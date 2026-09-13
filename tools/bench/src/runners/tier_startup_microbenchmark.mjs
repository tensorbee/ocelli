// The permanent runner for F-004's existing fill-rate instrument.

import { spawn } from "node:child_process";
import { repoPath } from "../paths.mjs";

export const id = "tier.startup_microbenchmark";

export function parsePixelRate(output) {
  const match = /pixels per second\s+(\d+)/.exec(output);
  const value = Number(match?.[1]);
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error("fill-rate instrument produced no positive pixel rate");
  }
  return value;
}

function measure() {
  return new Promise((resolve, reject) => {
    const child = spawn(repoPath("bin", "ocelli.sh"), ["test", "ocelli-render", "--release", "--", "--ignored", "--nocapture", "measures_a_fill_rate_on_this_machine"], {
      cwd: repoPath(), stdio: ["ignore", "pipe", "inherit"],
    });
    let output = "";
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk) => { output += chunk; });
    child.on("error", reject);
    child.on("exit", (code) => code === 0 ? resolve(output) : reject(new Error(`fill-rate instrument exited ${code}`)));
  });
}

export async function run() {
  const output = await measure();
  const value = parsePixelRate(output);
  return { value, detail: { statistic: "single calibrated probe", instrument: "ocelli-render F-004 fill-rate probe" } };
}
