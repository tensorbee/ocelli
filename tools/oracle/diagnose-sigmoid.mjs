// Synthetic paired reproducer. Writes only under ignored compare-out.
import { createRequire } from "node:module";
import { installedVersion } from "./src/pins.mjs";
import { spawnSync } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { chromium } from "playwright";
import { serveDirectory } from "./src/server.mjs";
import { readRenderParams, resolveRenderParams } from "./src/params.mjs";
import { digestOf, rowId } from "./src/manifest.mjs";
import { writeRow } from "./src/output.mjs";
import { isEntryPoint, oraclePath, repoPath } from "./src/paths.mjs";

function command(file, args, accepted = [0]) {
  const result = spawnSync(file, args, { cwd: repoPath(), encoding: "utf8" });
  if (!accepted.includes(result.status)) {
    throw new Error(`${file} exited ${result.status}: ${result.stderr}`);
  }
  return result;
}

// PS3.3 C.11.2.1.3.1. This equation is independent of Ocelli and vtk helpers.
export function expectedSigmoid(stored, width) {
  return 255 / (1 + Math.exp(-4 * (stored * 0.25 - 40) / width));
}

export async function run() {
  const out = oraclePath("compare-out", "sigmoid-diagnostic");
  const corpus = join(out, "corpus");
  const reference = join(out, "reference");
  await mkdir(reference, { recursive: true });
  command(repoPath(".venv", "bin", "python"), [
    oraclePath("fixtures", "sigmoid_ramp.py"), corpus,
  ]);
  command(process.execPath, [oraclePath("build-page.mjs")]);
  const path = "synthetic/diagnostic_sigmoid_ramp.dcm";
  const bytes = await readFile(join(corpus, path));
  const row = {
    path, modality: "CT", transferSyntax: "1.2.840.10008.1.2.1",
    category: "synthetic", categories: ["synthetic", "mono16"],
    source: "hand-derived synthetic ramp", licence: "CC0-1.0",
    licenceUrl: "https://creativecommons.org/publicdomain/zero/1.0/",
    sha256: digestOf(bytes),
  };
  const params = resolveRenderParams(readRenderParams(), row);
  params.canvas = { width: 80, height: 16 };
  const server = await serveDirectory(oraclePath("page", "dist"));
  let browser;
  let result;
  let environment;
  try {
    browser = await chromium.launch({ args: [
      "--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader",
      "--force-device-scale-factor=1",
    ] });
    const page = await browser.newPage({ viewport: { width: 200, height: 160 } });
    await page.goto(`${server.origin}/index.html`);
    await page.waitForFunction(() => globalThis.__oracleLoaded === true);
    environment = await page.evaluate(setup => globalThis.__oracle.ready(setup), {
      canvas: params.canvas, background: [0, 0, 0], wasmBasePath: `${server.origin}/wasm/`,
    });
    result = await page.evaluate(payload => globalThis.__oracle.render(payload), {
      id: rowId(path), bytesBase64: bytes.toString("base64"), params,
      includePixels: true, timeoutMs: 30000, fault: null,
    });
    if (!result.ok) throw new Error(`synthetic reference refused at ${result.boundary}`);
  } finally {
    await browser?.close();
    await server.close();
  }
  const require = createRequire(oraclePath("package.json"));
  const pins = require("./package.json").dependencies;
  const installed = Object.fromEntries(Object.keys(pins).map(name => [name, installedVersion(require, name)]));
  if (Object.entries(installed).some(([name, version]) => version !== pins[name])) throw new Error("reference dependency pin differs from its manifest");
  await writeRow(reference, { row, params, result }, environment, installed);
  await writeFile(join(reference, "run.json"), JSON.stringify({
    rows: [{ id: rowId(path), path, kind: "stack", ok: true }], volumes: [],
  }, null, 2));
  command(repoPath("bin", "ocelli.sh"), [
    "cargo", "run", "--release", "-p", "ocelli-oracle", "--example", "produce_stack",
    "--", reference, corpus, out,
  ], [0, 2]);
  const name = `${rowId(path)}.raw`;
  const ref = await readFile(join(reference, name));
  const candidate = await readFile(join(out, "candidate", name));
  const samples = Array.from({ length: 20 }, (_, i) => {
    const x = 2 + i * 4;
    const offset = (8 * 80 + x) * 4;
    return {
      stored: 151 + i, modality: (151 + i) * 0.25,
      expectedDisplay: expectedSigmoid(151 + i, 0.5),
      width1_5Display: expectedSigmoid(151 + i, 1.5),
      reference: ref[offset], candidate: candidate[offset],
    };
  });
  const report = {
    input: row.sha256, referencePin: installed, environment,
    referenceVoi: result.voi, samples,
    mismatchingSamples: samples.filter(s => s.reference !== s.candidate).length,
    candidateMaxDisplayError: Math.max(...samples.map(s => Math.abs(s.candidate - s.expectedDisplay))),
    referenceMaxWidth1_5Error: Math.max(...samples.map(s => Math.abs(s.reference - s.width1_5Display))),
  };
  await writeFile(join(out, "diagnosis.json"), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report, null, 2));
  return report;
}

if (isEntryPoint(import.meta.url)) await run();
