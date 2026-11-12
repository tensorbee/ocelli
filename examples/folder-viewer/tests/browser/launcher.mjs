// F-245 AC1. The page is served through the real npm launcher.
//
// bin/folder-viewer-dev.mjs is the script `npm run dev` runs. This control
// starts it the way a user does, with a private CARGO_TARGET_DIR under the
// repository's ignored target/, waits for the one URL the backend prints,
// opens that page in headless Chromium through tools/bench's pinned
// Playwright, and asserts it boots. It then fetches the same page from a
// plain native start of the same source and asserts the two bodies are
// identical, so the npm path serves exactly what the native path serves.
// The npm process group is signalled by the npm PID, and the owned cleanup
// is proved by the stopped port: a backend orphaned by an unhandled signal
// would still be serving.
//
// The binary is also identified at the process layer. The pid listening on
// the launcher's port is resolved with lsof, and its process command is
// read with ps, because comm alone is truncated to the last component on
// Linux, where this suite runs in CI, and names no directory. That command
// begins with the executable path the spawner set. It must begin with the
// private target directory followed by a path separator. A launcher that
// ignored the compiler artifact cargo named and ran a stale build from the
// default target directory would serve the same page with the same debug
// note, and would fail only here.
//
// The launcher rebuilds the core package dist in the checkout and builds
// the debug profile into the private target directory, so the first run of
// this control is a cold full debug graph build and later runs are
// incremental. bin/ocelli.sh registers this suite under viewer-browser,
// after dicomweb.mjs.

import { Buffer } from "node:buffer";
import { execFileSync, spawn } from "node:child_process";
import { mkdirSync } from "node:fs";
import { connect } from "node:net";
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

// The private target directory the launcher builds into, kept between runs
// so its debug build is incremental. It is distinct from the default
// target/debug, so a path under it can only be the one cargo's own
// compiler artifact named.
function privateTargetDir() {
  return path.join(repoRoot(), "target", "f245-launcher-debug");
}

function listeningPids(port) {
  try {
    const out = execFileSync("lsof", ["-nP", `-iTCP:${port}`, "-sTCP:LISTEN", "-t"], { encoding: "utf8" });
    return out.split("\n").map((line) => line.trim()).filter(Boolean).map(Number);
  } catch {
    return [];
  }
}

function psCommand(pid) {
  try {
    return execFileSync("ps", ["-o", "command=", "-p", String(pid)], { encoding: "utf8" }).trim();
  } catch {
    return "";
  }
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
  const targetDir = privateTargetDir();
  mkdirSync(targetDir, { recursive: true });
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
    // The process layer check. The page and the debug note are identical
    // for any debug build, so the binary itself is identified: whatever
    // listens on the launcher's port must run from the private target
    // directory the launcher was given.
    const pids = listeningPids(port);
    if (pids.length === 0) {
      problems.push(`no process was found listening on the launcher's port ${port}`);
    }
    for (const pid of pids) {
      const command = psCommand(pid);
      // The whole command is compared rather than a split on whitespace,
      // which would cut a checkout path containing a space into words that
      // no longer spell the directory, and the separator in the prefix
      // refuses a sibling directory whose name merely extends the
      // directory's name.
      if (!command.startsWith(targetDir + path.sep)) {
        problems.push(`the backend on port ${port} runs from "${command}", outside the private target directory`);
      }
    }
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
