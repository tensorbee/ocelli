// THROWAWAY SPIKE CODE. F-X013, Appendix A gate A1 follow-up.
//
// Compares openjph-core 0.1.0 on native and wasm against the synthetic ramp
// and ojph_expand 0.31.0. The two reversible transfer syntaxes must reproduce
// the ramp exactly. The irreversible syntax is bound to its measured D14
// divergence against ojph_expand because it has no independent exact anchor.

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  CANONICAL_BYTES,
  compare,
  pgmToCanonical,
  report,
  sha256,
} from "../common/compare.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = join(HERE, "..", "out");
const BINARY = join(HERE, "target", "release", "x013-htj2k-route");
const REFERENCE_DIGEST =
  "b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609";

const CASES = [
  {
    index: 0,
    name: "htj2k_lossless",
    uid: "1.2.840.10008.1.2.4.201",
    lossless: true,
    codestreamDigest:
      "964adb7fc83054985c3cd56d49eadedefa98c8eab0cb719eab1184a8c6ac193f",
    candidateDigest: REFERENCE_DIGEST,
    ojphDigest: REFERENCE_DIGEST,
  },
  {
    index: 1,
    name: "htj2k_lossless_rpcl",
    uid: "1.2.840.10008.1.2.4.202",
    lossless: true,
    codestreamDigest:
      "5b165000827b8d7ef8b4e36e9a2a09cd9db8593c9b083721a03210a69a47c4cc",
    candidateDigest: REFERENCE_DIGEST,
    ojphDigest: REFERENCE_DIGEST,
  },
  {
    index: 2,
    name: "htj2k_lossy",
    uid: "1.2.840.10008.1.2.4.203",
    lossless: false,
    codestreamDigest:
      "0d6be6d6217e93b2f28b5b849029f87ad2d8e02bb8945a62441af14759b6f12a",
    candidateDigest:
      "ce4a2bb9d75b897292a4e9e9e7455447e976f5ff1e18b4ecb3219bb17d4d062c",
    ojphDigest:
      "41a94cc4db2b871e16e50c738512413db800657b0dbeab7872415504be87c138",
    divergence: {
      differing: 41,
      first: { index: 69, a: 756, b: 757 },
      maxAbs: 1,
      histogram: [[1, 41]],
    },
  },
];

function digestMatches(actual, expected, label) {
  if (actual === expected) return true;
  console.log(`  MISMATCH ${label}: expected ${expected}`);
  return false;
}

function divergenceMatches(actual, expected) {
  return (
    actual.differing === expected.differing &&
    JSON.stringify(actual.first) === JSON.stringify(expected.first) &&
    actual.maxAbs === expected.maxAbs &&
    JSON.stringify(actual.histogram) === JSON.stringify(expected.histogram)
  );
}

function nativeDecode(testCase) {
  const target = join(OUT, `x013_native_${testCase.name}.raw`);
  execFileSync(BINARY, [String(testCase.index), target], { stdio: "pipe" });
  return new Uint8Array(readFileSync(target));
}

function wasmDecode(profile, testCase) {
  const path = join(
    HERE,
    "target",
    `wasm-${profile}`,
    "wasm32-unknown-unknown",
    "release",
    "x013_htj2k_route.wasm",
  );
  const module = new WebAssembly.Module(readFileSync(path));
  const instance = new WebAssembly.Instance(module, {});
  const code = instance.exports.decode(testCase.index);
  if (code !== 0) {
    return { bytes: null, note: `raw error code ${code}` };
  }
  const length = instance.exports.out_len();
  if (length !== CANONICAL_BYTES) {
    return { bytes: null, note: `out_len ${length}` };
  }
  const pointer = instance.exports.out_ptr();
  if (pointer === 0) {
    return { bytes: null, note: "out_ptr is zero" };
  }
  return {
    bytes: new Uint8Array(
      instance.exports.memory.buffer.slice(pointer, pointer + length),
    ),
    note: null,
  };
}

function ojphDecode(testCase) {
  const source = join(OUT, `${testCase.name}.j2c`);
  const target = join(OUT, `x013_ojph_${testCase.name}.pgm`);
  execFileSync("ojph_expand", ["-i", source, "-o", target], {
    stdio: "pipe",
  });
  return pgmToCanonical(readFileSync(target), `ojph/${testCase.name}`);
}

const reference = new Uint8Array(readFileSync(join(OUT, "reference.raw")));
let failures = 0;

console.log("=== F-X013, openjph-core 0.1.0 ===");
const referenceDigest = sha256(reference);
console.log(`R  ${referenceDigest}`);
if (!digestMatches(referenceDigest, REFERENCE_DIGEST, "reference")) failures += 1;

for (const testCase of CASES) {
  console.log(`\n--- ${testCase.name}, ${testCase.uid} ---`);
  const codestreamDigest = sha256(
    new Uint8Array(readFileSync(join(OUT, `${testCase.name}.j2c`))),
  );
  console.log(`  codestream               ${codestreamDigest}`);
  if (
    !digestMatches(
      codestreamDigest,
      testCase.codestreamDigest,
      `${testCase.name} codestream`,
    )
  ) {
    failures += 1;
  }
  const routes = new Map([["openjph-core/native", nativeDecode(testCase)]]);
  for (const [profile, label] of [
    ["plain", "openjph-core/wasm"],
    ["simd", "openjph-core/wasm-simd128"],
  ]) {
    const result = wasmDecode(profile, testCase);
    if (result.bytes === null) {
      console.log(`  ${label.padEnd(24)} UNAVAILABLE: ${result.note}`);
      failures += 1;
    } else {
      routes.set(label, result.bytes);
    }
  }
  const native = routes.get("openjph-core/native");
  const ojph = ojphDecode(testCase);
  const ojphDigest = sha256(ojph);
  console.log(`  ojph_expand              ${ojphDigest}`);
  if (!digestMatches(ojphDigest, testCase.ojphDigest, "ojph_expand")) {
    failures += 1;
  }
  for (const [label, bytes] of routes) {
    const candidateDigest = sha256(bytes);
    console.log(`  ${label.padEnd(24)} ${candidateDigest}`);
    if (!digestMatches(candidateDigest, testCase.candidateDigest, label)) {
      failures += 1;
    }
    const againstOjph = compare(bytes, ojph, label, "ojph_expand");
    if (testCase.lossless && !againstOjph.equal) failures += 1;
    if (
      testCase.divergence !== undefined &&
      !divergenceMatches(againstOjph, testCase.divergence)
    ) {
      console.log(`  MISMATCH ${label}: unexpected measured divergence`);
      failures += 1;
    }
    console.log(
      report(againstOjph).split("\n").map((line) => `  ${line}`).join("\n"),
    );
    if (testCase.lossless) {
      const againstReference = compare(bytes, reference, label, "R");
      if (!againstReference.equal) failures += 1;
      console.log(
        report(againstReference)
          .split("\n")
          .map((line) => `  ${line}`)
          .join("\n"),
      );
    }
  }
  console.log("  -- cross-target identity --");
  for (const [label, bytes] of routes) {
    if (label === "openjph-core/native") continue;
    const againstNative = compare(bytes, native, label, "openjph-core/native");
    if (!againstNative.equal) failures += 1;
    console.log(
      report(againstNative)
        .split("\n")
        .map((line) => `  ${line}`)
        .join("\n"),
    );
  }
}

console.log(`\n${failures} failing comparison(s)`);
if (failures !== 0) process.exit(1);
