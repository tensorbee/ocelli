// One real `.203` Decoder::decode call over the synthetic corpus frame.
//
// The irreversible row. The three HTJ2K syntaxes are reported per syntax and
// never as one average, because a lossless and a lossy codestream are
// different work.

import { spawn } from "node:child_process";
import { access } from "node:fs/promises";
import { repoPath } from "../paths.mjs";

export const id = "decode.transfer_syntax.htj2k";
const BINARY = repoPath("target", "release", "examples", "decode_htj2k");
const ITERATIONS_KEPT = 31;
const WARMUP_ITERATIONS = 1;
const DECODES_PER_SAMPLE = 4;

function child(command, args) {
  return new Promise((resolve, reject) => {
    const process = spawn(command, args, { cwd: repoPath(), stdio: ["ignore", "pipe", "inherit"] });
    let stdout = "";
    process.stdout.setEncoding("utf8");
    process.stdout.on("data", (chunk) => { stdout += chunk; });
    process.on("error", reject);
    process.on("exit", (code) => code === 0 ? resolve(stdout) : reject(new Error(`${command} exited ${code}`)));
  });
}

export function parseHtJ2k(text) {
  const parsed = JSON.parse(text);
  if (!Number.isFinite(parsed.value) || parsed.value <= 0) {
    throw new Error("HTJ2K benchmark produced no positive finite duration");
  }
  if (!Number.isInteger(parsed.iterations) || parsed.iterations !== ITERATIONS_KEPT) {
    throw new Error("HTJ2K benchmark iteration count does not match the runner contract of 31 timing samples");
  }
  if (!Number.isInteger(parsed.warmup_iterations) ||
      parsed.warmup_iterations !== WARMUP_ITERATIONS) {
    throw new Error("HTJ2K benchmark warm-up iteration count does not match the one-call runner contract");
  }
  if (!Number.isInteger(parsed.decodes_per_sample) ||
      parsed.decodes_per_sample !== DECODES_PER_SAMPLE) {
    throw new Error("HTJ2K benchmark decodes per timing sample do not match the four-call runner contract");
  }
  if (!Array.isArray(parsed.range_ms) || parsed.range_ms.length !== 2 ||
      !parsed.range_ms.every(Number.isFinite)) {
    throw new Error("HTJ2K benchmark produced no finite observed range");
  }
  const [minimum, maximum] = parsed.range_ms;
  if (minimum <= 0 || maximum <= 0) {
    throw new Error("HTJ2K benchmark produced no positive observed range");
  }
  if (minimum > maximum) {
    throw new Error("HTJ2K benchmark produced no ordered observed range");
  }
  if (parsed.value < minimum || parsed.value > maximum) {
    throw new Error("HTJ2K benchmark observed range does not enclose the median");
  }
  if (!Number.isSafeInteger(parsed.checksum) || parsed.checksum <= 0) {
    throw new Error("HTJ2K benchmark produced no output checksum");
  }
  return parsed;
}

export async function run({ build = true }) {
  if (build) {
    await child(repoPath("bin", "ocelli.sh"), ["cargo", "build", "--release", "-p", "ocelli-codec", "--example", "decode_htj2k"]);
  } else {
    await access(BINARY);
  }
  const result = parseHtJ2k(await child(BINARY, []));
  return {
    value: result.value,
    detail: {
      statistic: "median of per-call normalized batch durations",
      iterations_run: result.iterations * result.decodes_per_sample + result.warmup_iterations,
      iterations_kept: result.iterations, discarded: "one decoder warm-up call",
      decodes_per_sample: result.decodes_per_sample,
      observed_range_ms: result.range_ms,
      clock: "std::time::Instant surrounds each four-decode batch, then elapsed time is normalized by four",
      rounding: "0.0001 ms, below the observed run-to-run spread",
      output_checksum: result.checksum, transfer_syntax: "1.2.840.10008.1.2.4.203",
      rows: 64, columns: 96, bits_stored: 16, profile: "release",
      fixture: "synthetic manifest-backed htj2k_lossy corpus frame",
      excludes: ["process startup", "fixture setup", "decoder construction", "caller output allocation"],
    },
  };
}
