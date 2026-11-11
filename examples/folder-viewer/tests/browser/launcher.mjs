// F-245 AC1. The page is served through the real npm launcher.
//
// bin/folder-viewer-dev.mjs is the script `npm run dev` runs. This control
// starts it the way a user does, with a custom CARGO_TARGET_DIR so the
// binary can only come from the directory cargo's own compiler artifact
// names, waits for the one URL the backend prints, opens that page in
// headless Chromium through tools/bench's pinned Playwright, and asserts it
// boots. It then fetches the same page from a plain native start of the
// same source and asserts the two bodies are identical, so the npm path
// serves exactly what the native path serves. The npm process group is
// signalled by the npm PID, and the owned cleanup is proved by the stopped
// port: a backend orphaned by an unhandled signal would still be serving.
//
// The launcher builds into the private target directory this script makes
// and removes, so nothing outside the run is written. bin/ocelli.sh
// registers this suite under viewer-browser, after dicomweb.mjs.

import { Buffer } from "node:buffer";
import { spawn } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { connect } from "node:net";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { ensureViewerBinary, startViewer } from "../../../../tools/bench/src/viewer_work.mjs";
import { loadPlaywright, stopByPid } from "./aspect.mjs";

const URL_TIMEOUT_MS = 300000;
const CLOSE_TIMEOUT_MS = 30000;
const INITIAL_STATUS = "Choose Synthetic study or Folder.";
const DEBUG_NOTE = "This is the slow debug build.";

function repoRoot() {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../..");
}

function startLauncher(targetDir) {
  // Detached, so the npm process leads its own group and the stop below can
  // reach the whole tree with one signal. The backend the launcher spawns
  // is detached in turn, so only the launcher's owned cleanup can stop it,
  // which is what the stopped port proves.
  const child = spawn("npm", ["run", "dev"], {
    cwd: repoRoot(),
    env: { ...process.env, OCELLI_PORT: "0", CARGO_TARGET_DIR: targetDir },
    detached: true,
    stdio: ["ignore", "pipe", "pipe"],
  });
  const url = new Promise((resolve, reject) => {
    let stdout = "";
    let stderr = "";
    let settled = false;
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      reject(new Error(`the launcher printed no URL in ${URL_TIMEOUT_MS} ms: ${stderr.slice(-2000)}`));
    }, URL_TIMEOUT_MS);
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      if (settled) return;
      stdout += chunk;
      const found = stdout.match(/http:\/\/127\.0\.0\.1:(\d+)\//);
      if (!found) return;
      settled = true;
      clearTimeout(timer);
      resolve({ url: found[0], port: Number(found[1]) });
    });
    child.stderr.setEncoding("utf8");
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.once("close", (code) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      reject(new Error(`the launcher exited ${code} before printing a URL: ${stderr.slice(-2000)}`));
    });
  });
  return { child, url };
}

function portServes(port) {
  return new Promise((resolve) => {
    const socket = connect({ host: "127.0.0.1", port });
    const done = (value) => {
      socket.destroy();
      resolve(value);
    };
    socket.once("connect", () => done(true));
    socket.once("error", () => done(false));
  });
}

async function waitForPortClosed(port) {
  const started = Date.now();
  for (;;) {
    if (!(await portServes(port))) return;
    if (Date.now() - started > 10000) {
      throw new Error("the backend kept serving after the launcher stopped");
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
}

function signalGroup(pid, signal) {
  try {
    process.kill(-pid, signal);
  } catch {
    // The group is already gone, which is the state the stop wants.
  }
}

async function main() {
  const { chromium } = await loadPlaywright();
  const problems = [];
  const targetDir = mkdtempSync(path.join(tmpdir(), "f245-launcher-target-"));
  let launcher = null;
  let browser = null;
  try {
    // The native half of the comparison: the same source started directly
    // from the cargo-selected release binary.
    const binary = await ensureViewerBinary("release", true);
    const native = await startViewer(binary);
    const nativePage = await (await fetch(native.url)).arrayBuffer();
    await stopByPid(native.pid);

    launcher = startLauncher(targetDir);
    const { url, port } = await launcher.url;
    const launcherPage = await (await fetch(url)).arrayBuffer();
    if (!Buffer.from(nativePage).equals(Buffer.from(launcherPage))) {
      problems.push("the launcher serves a different page than the native start");
    }

    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
    page.setDefaultTimeout(60000);
    await page.goto(url);
    await page.waitForFunction(
      (text) => (document.getElementById("status")?.textContent || "").startsWith(text),
      INITIAL_STATUS,
    );
    if (!(await page.locator("#source-synthetic").isVisible())) {
      problems.push("the launcher-served page did not boot its source chooser");
    }
    // The launcher builds the debug profile and the page says so on boot. A
    // launcher that ran a hardcoded release binary instead of the artifact
    // cargo named would boot without this note.
    const debugNoted = await page.evaluate(
      (note) => (document.getElementById("status")?.textContent || "").includes(note),
      DEBUG_NOTE,
    );
    if (!debugNoted) {
      problems.push("the launcher-served page did not report the launcher's own debug build");
    }

    signalGroup(launcher.child.pid, "SIGTERM");
    const closed = new Promise((resolve) => launcher.child.once("close", resolve));
    const timedOut = await Promise.race([
      closed.then(() => false),
      new Promise((resolve) => setTimeout(() => resolve(true), CLOSE_TIMEOUT_MS)),
    ]);
    if (timedOut) signalGroup(launcher.child.pid, "SIGKILL");
    await closed.catch(() => undefined);
    try {
      await waitForPortClosed(port);
    } catch (error) {
      problems.push(error instanceof Error ? error.message : String(error));
    }
  } finally {
    if (browser !== null) await browser.close();
    if (launcher !== null && launcher.child.exitCode === null && launcher.child.signalCode === null) {
      // Best effort owned stop on any path that left early: the group signal
      // reaches npm and the launcher it runs, and the launcher cleans up its
      // own child before it exits.
      signalGroup(launcher.child.pid, "SIGTERM");
      await new Promise((resolve) => {
        const timer = setTimeout(resolve, 5000);
        launcher.child.once("close", () => {
          clearTimeout(timer);
          resolve(undefined);
        });
      });
      signalGroup(launcher.child.pid, "SIGKILL");
    }
    rmSync(targetDir, { recursive: true, force: true });
  }
  if (problems.length) {
    for (const problem of problems) console.error(problem);
    process.exitCode = 1;
    return;
  }
  console.log("ok");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
