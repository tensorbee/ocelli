// Real worker/canvas latency and dedicated Chromium session CPU collection.
import { spawn } from "node:child_process";
import { performance } from "node:perf_hooks";
import { createServer } from "node:http";
import { createHash } from "node:crypto";
import { readFile, writeFile, mkdir, readdir } from "node:fs/promises";
import os from "node:os";
import { join } from "node:path";
import { repoPath, benchPath } from "./paths.mjs";
import { hostClass, HOST_CLASS_FIELDS } from "./hostclass.mjs";
import { resolveServedPath, median, atClockPrecision } from "./runners/wasm_cold_start.mjs";

const ARTEFACT = repoPath("crates", "ocelli-wasm", "pkg");
const RECEIPT = benchPath("out", "session-artifact.json");
const sha256 = bytes => createHash("sha256").update(bytes).digest("hex");

async function command(file, args) {
  await new Promise((resolve, reject) => {
    const process = spawn(file, args, { cwd: repoPath(), stdio: ["ignore", "inherit", "inherit"] });
    process.on("error", reject);
    process.on("exit", code => code === 0 ? resolve() : reject(new Error(`${file} exited ${code}`)));
  });
}

async function digestDirectories(directories, files = []) {
  const hash = createHash("sha256");
  async function visit(relative) {
    for (const entry of (await readdir(repoPath(relative), { withFileTypes: true })).sort((a, b) => a.name.localeCompare(b.name))) {
      if (["target", "pkg", "node_modules", ".git"].includes(entry.name)) continue;
      const path = `${relative}/${entry.name}`;
      if (entry.isDirectory()) await visit(path);
      else if (entry.isFile()) { hash.update(path); hash.update(await readFile(repoPath(path))); }
    }
  }
  for (const directory of directories) await visit(directory);
  for (const path of files) { hash.update(path); hash.update(await readFile(repoPath(path))); }
  return hash.digest("hex");
}

async function sourceIdentity() {
  return digestDirectories(["crates", "vendor", "packages/core/src", "tools/bench/page", "tools/bench/src"],
    ["Cargo.lock", "Cargo.toml", "rust-toolchain.toml", "package-lock.json", "packages/core/tsconfig.json"]);
}

async function servedIdentity() {
  return digestDirectories(["crates/ocelli-wasm/pkg", "packages/core/dist", "tools/bench/page"]);
}

async function prepare(build) {
  const source = await sourceIdentity();
  if (build) {
    await command(repoPath("bin", "ocelli.sh"), ["wasm"]);
    await command("npx", ["tsc", "--build"]);
    const bytes = await readFile(join(ARTEFACT, "ocelli_wasm_bg.wasm"));
    await mkdir(benchPath("out"), { recursive: true });
    await writeFile(RECEIPT, JSON.stringify({ sha256: sha256(bytes), bytes: bytes.length, profile: "release", source, served: await servedIdentity() }));
  }
  let receipt;
  try { receipt = JSON.parse(await readFile(RECEIPT, "utf8")); }
  catch { throw new Error("session release artifact receipt is absent. Run with build enabled first"); }
  const bytes = await readFile(join(ARTEFACT, "ocelli_wasm_bg.wasm"));
  if (receipt.profile !== "release" || receipt.sha256 !== sha256(bytes) || receipt.source !== source || receipt.served !== await servedIdentity()) {
    throw new Error("session release artifact differs from its receipt");
  }
  return receipt;
}

async function serve() {
  const server = createServer(async (request, response) => {
    const path = new URL(request.url, "http://127.0.0.1").pathname;
    let base = benchPath("page");
    let relative = path;
    if (path.startsWith("/wasm/")) { base = ARTEFACT; relative = path.slice(5); }
    else if (path.startsWith("/core/")) { base = repoPath("packages", "core", "dist"); relative = path.slice(5); }
    const file = resolveServedPath(base, relative);
    if (!file) { response.writeHead(403).end(); return; }
    try {
      const bytes = await readFile(file);
      response.writeHead(200, { "content-type": file.endsWith(".wasm") ? "application/wasm" : file.endsWith(".html") ? "text/html" : "text/javascript", "cache-control": "no-store" });
      response.end(bytes);
    } catch { response.writeHead(404).end(); }
  });
  await new Promise((resolve, reject) => { server.once("error", reject); server.listen(0, "127.0.0.1", resolve); });
  return { origin: `http://127.0.0.1:${server.address().port}`, close: () => new Promise(resolve => server.close(resolve)) };
}

export function cpuFraction(before, after, wallSeconds) {
  if (!(wallSeconds > 0) || !Number.isFinite(wallSeconds)) throw new Error("CPU wall interval is invalid");
  for (const sample of [before, after]) {
    if (new Set(sample.map(p => p.id)).size !== sample.length || sample.some(p => !Number.isFinite(p.cpuTime) || p.cpuTime < 0)) {
      throw new Error("CPU counter is invalid");
    }
  }
  const prior = new Map(before.map(p => [p.id, p.cpuTime]));
  if (after.length !== before.length || after.some(p => !prior.has(p.id))) {
    throw new Error("Chromium process set changed during CPU observation");
  }
  let seconds = 0;
  for (const process of after) {
    const delta = process.cpuTime - prior.get(process.id);
    if (!Number.isFinite(delta) || delta < 0) throw new Error("CPU counter is invalid");
    seconds += delta;
  }
  return seconds / wallSeconds;
}

async function representativeHost() {
  const file = process.env.OCELLI_CPU_BASELINE_HOST;
  if (!file) throw new Error("representative GPU-less host measurement is unavailable: configure OCELLI_CPU_BASELINE_HOST");
  const config = JSON.parse(await readFile(file, "utf8"));
  if (config.representativeGpuLess !== true || typeof config.incumbent !== "string" || !config.incumbent.trim() ||
      !HOST_CLASS_FIELDS.every(key => config.host?.[key] === hostClass(os)[key])) {
    throw new Error("CPU baseline host must identify this physical host and the incumbent workload");
  }
  return config;
}

export async function runBrowserSession(subject, { build = true, iterations = 11, durationMs = 1000, diagnostic = false, workerTransform } = {}) {
  const known = ["render.first_frame", "interaction.window_level_drag", "interaction.scroll", "session.cpu_interactive", "session.cpu_idle", "cine.frame_change_rate"];
  if (!known.includes(subject)) throw new Error("unknown session measurement subject");
  if (workerTransform && !diagnostic) throw new Error("worker controls require diagnostic mode");
  const cpuSubject = subject === "session.cpu_interactive" || subject === "session.cpu_idle" || subject === "cine.frame_change_rate";
  if (!Number.isInteger(iterations) || iterations < 1 || durationMs < 250 || !Number.isFinite(durationMs)) {
    throw new Error("session measurement iterations or duration is invalid");
  }
  const host = cpuSubject && !diagnostic ? await representativeHost() : null;
  const artefact = await prepare(build);
  const { chromium } = await import("playwright");
  const server = await serve();
  let browser;
  try {
    browser = await chromium.launch({ headless: true, args: ["--disable-gpu", "--force-device-scale-factor=1"] });
    const measurements = [];
    let observation;
    for (let i = 0; i <= (cpuSubject ? 0 : iterations); i += 1) {
      const page = await browser.newPage({ viewport: { width: 256, height: 256 } });
      try {
        if (workerTransform) await page.route("**/session-worker.mjs", async route => {
          const response = await route.fetch();
          await route.fulfill({ response, body: workerTransform(await response.text()) });
        });
        await page.goto(`${server.origin}/session.html`);
        await page.waitForFunction(() => globalThis.__sessionReady === true || globalThis.__sessionError, null, { timeout: 30000 });
        const startupError = await page.evaluate(() => globalThis.__sessionError);
        if (startupError) throw new Error(startupError);
        if (subject === "render.first_frame") {
          const result = await page.evaluate(() => globalThis.__sessionBench.first());
          if (i > 0) measurements.push(atClockPrecision(result.value));
        } else if (subject === "interaction.window_level_drag") {
          await page.evaluate(() => globalThis.__sessionBench.first());
          await page.evaluate(() => globalThis.__sessionBench.armDrag());
          await page.mouse.move(32 + i, 32);
          const result = await page.evaluate(() => globalThis.__dragResult);
          if (result.error || result.commands < 1) throw new Error(result.error ?? "pointer batch did not apply");
          if (i > 0) measurements.push(atClockPrecision(result.value));
        } else if (subject === "interaction.scroll") {
          await page.evaluate(async () => { await globalThis.__sessionBench.first(); await globalThis.__sessionBench.stack(); globalThis.__sessionBench.armScroll(); });
          await page.mouse.move(40, 40);
          await page.mouse.wheel(0, 120);
          const result = await page.evaluate(() => globalThis.__scrollResult);
          if (result.error || result.key === result.previous) throw new Error(result.error ?? "scroll did not change resident key");
          if (i > 0) measurements.push(atClockPrecision(result.value));
        } else if (cpuSubject) {
          await page.evaluate(() => globalThis.__sessionBench.first());
          if (subject === "cine.frame_change_rate") await page.evaluate(() => globalThis.__sessionBench.verifyCine());
          const cdp = await browser.newBrowserCDPSession();
          const beforeStart = performance.now();
          const before = (await cdp.send("SystemInfo.getProcessInfo")).processInfo;
          const start = performance.now();
          const workload = subject === "session.cpu_idle" ? "idle" : subject === "cine.frame_change_rate" ? "cine" : "interactive";
          observation = await page.evaluate(async ({ workload, durationMs }) => globalThis.__sessionBench[workload](durationMs), { workload, durationMs });
          const end = performance.now();
          const after = (await cdp.send("SystemInfo.getProcessInfo")).processInfo;
          const afterEnd = performance.now();
          // Counter snapshots occur inside each CDP call. The midpoint gives
          // an estimate with an explicit worst-case boundary uncertainty.
          const wallSeconds = ((end + afterEnd) / 2 - (beforeStart + start) / 2) / 1000;
          const boundaryUncertaintySeconds = ((start - beforeStart) + (afterEnd - end)) / 2000;
          if (subject === "session.cpu_idle" && (observation.changedFrames !== 0 || observation.scheduled)) throw new Error("idle session scheduled rendering");
          const cpu = cpuFraction(before, after, wallSeconds);
          measurements.push(subject === "cine.frame_change_rate" ? observation.changedFrames / wallSeconds : cpu);
          observation = { ...observation, wallSeconds, boundaryUncertaintySeconds, workloadWallSeconds: (end - start) / 1000, collector: "CDP SystemInfo.getProcessInfo.cpuTime over all processes in a dedicated Chromium instance" };
          await cdp.detach();
        }
      } finally { await page.close(); }
    }
    return {
      value: cpuSubject ? median(measurements) : atClockPrecision(median(measurements)), instrument: { chromium: browser.version(), tier: "C", session: "worker-canvas-v1", harness: await digestDirectories(["tools/bench/page", "tools/bench/src"]) },
      detail: { tier: "C", profile: "release", artefact, samples: measurements, iterations: measurements.length,
        clock: "performance.now, page DOM event or command to later RAF ring drain", statistic: "median after one warm-up for latency",
        workload: "one 128 by 128 monochrome worker canvas, DPR 1", observation, host,
        diagnostic, representativeHostAcceptance: host !== null,
      },
    };
  } finally { await browser?.close(); await server.close(); }
}
