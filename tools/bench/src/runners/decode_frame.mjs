// `decode.frame`, one real Decoder::decode call over one synthetic corpus
// frame. The Rust executable owns the clock so process startup, fixture setup,
// decoder construction, and output allocation remain outside the boundary.

import { spawn } from "node:child_process";
import { access } from "node:fs/promises";

import { repoPath } from "../paths.mjs";

export const id = "decode.frame";

const BINARY = repoPath("target", "release", "examples", "decode_frame");

function child(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const process = spawn(command, args, {
      cwd: repoPath(),
      stdio: ["ignore", "pipe", "inherit"],
      ...options,
    });
    let stdout = "";
    process.stdout.setEncoding("utf8");
    process.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    process.on("error", reject);
    process.on("exit", (code) => {
      if (code === 0) {
        resolve(stdout);
        return;
      }
      reject(new Error(`${command} exited ${code}`));
    });
  });
}

export function parseDecodeFrame(text) {
  const parsed = JSON.parse(text);
  if (!Number.isFinite(parsed.value) || parsed.value <= 0) {
    throw new Error("decode.frame produced no positive finite duration");
  }
  if (!Number.isInteger(parsed.iterations) || parsed.iterations < 1) {
    throw new Error("decode.frame produced no iteration count");
  }
  if (!Array.isArray(parsed.range_ms) || parsed.range_ms.length !== 2 ||
      !parsed.range_ms.every(Number.isFinite)) {
    throw new Error("decode.frame produced no finite observed range");
  }
  if (!Number.isSafeInteger(parsed.checksum) || parsed.checksum <= 0) {
    throw new Error("decode.frame produced no output checksum");
  }
  return parsed;
}

export async function requireReleaseBinary(binary) {
  try {
    await access(binary);
  } catch {
    throw new Error(
      "target/release/examples/decode_frame is not there. This runner " +
        "measures the release executable and refuses a substitute. Run " +
        "bin/ocelli.sh bench, or build that example before --no-build.",
    );
  }
}

export async function run({ build = true }) {
  if (build) {
    await child(repoPath("bin", "ocelli.sh"), [
      "cargo",
      "build",
      "--release",
      "-p",
      "ocelli-codec",
      "--example",
      "decode_frame",
    ]);
  } else {
    await requireReleaseBinary(BINARY);
  }

  const result = parseDecodeFrame(await child(BINARY, []));
  return {
    value: result.value,
    detail: {
      statistic: "median",
      iterations_run: result.iterations + result.warmup_iterations,
      iterations_kept: result.iterations,
      discarded: "one decoder warm-up call",
      observed_range_ms: result.range_ms,
      clock: "std::time::Instant around each Decoder::decode call",
      rounding: "0.0001 ms, below the observed run-to-run spread",
      output_checksum: result.checksum,
      transfer_syntax: "1.2.840.10008.1.2.4.51",
      rows: 64,
      columns: 96,
      bits_stored: 12,
      profile: "release",
      fixture: "synthetic repository corpus row jpeg_extended_12",
      excludes: [
        "process startup",
        "fixture setup",
        "decoder construction",
        "caller output allocation",
      ],
    },
  };
}
