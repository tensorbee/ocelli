// THROWAWAY SPIKE CODE. F-X006, Appendix A gate A2.
//
// Decodes the two JPEG-LS corpus rows through every candidate route and
// compares them through the ONE comparator in
// `tools/spikes/common/compare.mjs`.
//
//   R1        @cornerstonejs/codec-charls 1.2.5, the JS-side bridge
//   R3        pure_jpegls 2.0.0, native and wasm32, plain and SIMD128
//   R3b       ritk-codecs 0.6.0, native and wasm32, plain and SIMD128
//   R3b       dicom-toolkit-codec 0.5.0, native only, see the answer file
//   A_dcmtk   dcmdjpls, DCMTK 3.7.0. A SECOND CharLS READING, NOT INDEPENDENT
//   R         syntax/explicit_vr_le.dcm PixelData, exact for `.80` only
//
// **The correctness claim rests on different anchors for the two rows and the
// output says which.** `.80` is lossless, so `R` is exact and encoder
// independent. `.81` has no exact anchor: `pyjpegls` 1.5.1 encoded it and
// `dcmdjpls` decodes it and both wrap CharLS, so agreement between them is one
// implementation agreeing with itself. What IS independent for `.81` is
// ISO/IEC 14495-1's guarantee that the maximum absolute error is at most
// `NEAR`, which `scripts/corpus_synth.py` sets to `JPEG_LS_NEAR = 3`.
//
// Run: node tools/spikes/a2-jpeg-ls/run.mjs

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  CANONICAL_BYTES,
  CANONICAL_SAMPLES,
  compare,
  report,
  samples,
  sha256,
} from "../common/compare.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = join(HERE, "..", "out");

/// `scripts/corpus_synth.py`, `JPEG_LS_NEAR = 3`. ISO/IEC 14495-1 guarantees
/// the maximum absolute error of a near-lossless decode is at most this.
const JPEG_LS_NEAR = 3;

const CASES = [
  { index: 0, name: "jpegls_lossless", uid: "1.2.840.10008.1.2.4.80",
    lossless: true },
  { index: 1, name: "jpegls_near_lossless", uid: "1.2.840.10008.1.2.4.81",
    lossless: false },
];

const CANDIDATES = [
  { index: 0, label: "pure_jpegls", feature: "pure-jpegls", wasm: true },
  { index: 1, label: "dicom_toolkit", feature: "dicom-toolkit", wasm: false },
  { index: 2, label: "ritk_codecs", feature: "ritk", wasm: true },
];

function nativeDecode(candidate, caseIndex) {
  const binary = join(HERE, "target", "release", "a2-jpeg-ls");
  const target = join(OUT, `a2_c${candidate}_case${caseIndex}.raw`);
  try {
    execFileSync(binary, [String(candidate), String(caseIndex), target],
      { stdio: "pipe" });
  } catch (error) {
    const text = (error.stderr ?? Buffer.from("")).toString().trim();
    return { bytes: null, note: text.replaceAll("\n", " | ") };
  }
  return { bytes: new Uint8Array(readFileSync(target)), note: null };
}

function wasmDecode(feature, suffix, candidate, caseIndex) {
  const path = join(
    HERE, "target", `wasm-${feature}${suffix}`, "wasm32-unknown-unknown",
    "release", "a2_jpeg_ls.wasm",
  );
  const module = new WebAssembly.Module(readFileSync(path));
  const instance = new WebAssembly.Instance(module, {});
  let code = 0;
  try {
    code = instance.exports.decode(candidate, caseIndex);
  } catch (error) {
    return { bytes: null, note: `trap: ${error.message}` };
  }
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
  // The copy matters. A view into a live linear memory is not a stable buffer.
  return {
    bytes: new Uint8Array(
      instance.exports.memory.buffer.slice(pointer, pointer + length),
    ),
    note: null,
  };
}

/**
 * R1. `@cornerstonejs/codec-charls` 1.2.5, a second wasm module with its own
 * linear memory, which is what the route costs architecturally.
 */
async function charlsDecode(codestream) {
  const factory = (
    await import("@cornerstonejs/codec-charls/decode")
  ).default;
  const charls = await factory();
  const decoder = new charls.JpegLSDecoder();
  const encoded = decoder.getEncodedBuffer(codestream.length);
  encoded.set(codestream);
  decoder.decode();
  const info = decoder.getFrameInfo();
  const decoded = decoder.getDecodedBuffer();
  // The copy out of the OTHER module's memory. This is the per-frame cost R1
  // has and the pure-Rust routes do not.
  const bytes = new Uint8Array(decoded.slice(0, CANONICAL_BYTES));
  decoder.delete();
  return { bytes, info };
}

/** The ISO/IEC 14495-1 NEAR bound, checked against the uncompressed ramp. */
function nearBound(decoded, reference, label) {
  const a = samples(decoded, label);
  const b = samples(reference, "R");
  let maxAbs = 0;
  let over = 0;
  for (let i = 0; i < CANONICAL_SAMPLES; i += 1) {
    const delta = Math.abs(a[i] - b[i]);
    if (delta > maxAbs) maxAbs = delta;
    if (delta > JPEG_LS_NEAR) over += 1;
  }
  return { maxAbs, over };
}

const reference = new Uint8Array(readFileSync(join(OUT, "reference.raw")));

console.log("=== A2, JPEG-LS candidate routes ===");
console.log(`R (syntax/explicit_vr_le.dcm PixelData): ${sha256(reference)}`);

let failures = 0;
let unavailable = 0;

for (const testCase of CASES) {
  console.log(`\n--- ${testCase.name}, ${testCase.uid}, ` +
    `${testCase.lossless ? "lossless" : `near-lossless NEAR ${JPEG_LS_NEAR}`}` +
    " ---");

  const anchor = new Uint8Array(
    readFileSync(join(OUT, `dcmtk_${testCase.name}.raw`)),
  );
  const decodes = new Map([["A_dcmtk", anchor]]);

  // R1
  const codestream = new Uint8Array(
    readFileSync(join(OUT, `${testCase.name}.jls`)),
  );
  try {
    const { bytes, info } = await charlsDecode(codestream);
    decodes.set("R1_charls_js", bytes);
    console.log(`  R1 frame info: ${JSON.stringify(info)}`);
  } catch (error) {
    console.log(`  R1_charls_js  UNAVAILABLE: ${error.message}`);
    unavailable += 1;
  }

  // R3 and R3b, native then wasm
  for (const candidate of CANDIDATES) {
    const native = nativeDecode(candidate.index, testCase.index);
    if (native.bytes === null) {
      console.log(
        `  ${candidate.label}/native      UNAVAILABLE: ${native.note}`,
      );
      unavailable += 1;
    } else {
      decodes.set(`${candidate.label}/native`, native.bytes);
    }
    if (!candidate.wasm) {
      console.log(
        `  ${candidate.label}/wasm        NOT BUILT: ` +
          "does not compile for wasm32-unknown-unknown",
      );
      continue;
    }
    for (const [suffixName, dirSuffix] of [["wasm", ""], ["simd", "-simd"]]) {
      const result = wasmDecode(
        candidate.feature, dirSuffix, candidate.index, testCase.index,
      );
      if (result.bytes === null) {
        console.log(
          `  ${candidate.label}/${suffixName}  UNAVAILABLE: ${result.note}`,
        );
        unavailable += 1;
      } else {
        decodes.set(`${candidate.label}/${suffixName}`, result.bytes);
      }
    }
  }

  for (const [label, bytes] of decodes) {
    console.log(`  ${label.padEnd(24)} ${sha256(bytes)}`);
  }

  console.log("  -- against the DCMTK anchor, which is a second CharLS " +
    "reading and not independent --");
  for (const [label, bytes] of decodes) {
    if (label === "A_dcmtk") continue;
    const result = compare(bytes, anchor, label, "A_dcmtk");
    if (!result.equal) failures += 1;
    console.log(report(result).split("\n").map((l) => `  ${l}`).join("\n"));
  }

  if (testCase.lossless) {
    console.log("  -- against R, which IS independent and IS exact for " +
      "a lossless syntax --");
    for (const [label, bytes] of decodes) {
      const result = compare(bytes, reference, label, "R");
      if (!result.equal) failures += 1;
      console.log(report(result).split("\n").map((l) => `  ${l}`).join("\n"));
    }
  } else {
    console.log(`  -- against ISO/IEC 14495-1's NEAR bound, ` +
      `max abs error at most ${JPEG_LS_NEAR} --`);
    for (const [label, bytes] of decodes) {
      const { maxAbs, over } = nearBound(bytes, reference, label);
      const verdict = over === 0 ? "WITHIN" : "VIOLATES";
      if (over !== 0) failures += 1;
      console.log(
        `  ${verdict.padEnd(8)} ${label.padEnd(24)} ` +
          `max abs error ${maxAbs}, ${over} sample(s) over ${JPEG_LS_NEAR}`,
      );
    }
  }
}

console.log(
  `\n${failures} failing comparison(s), ${unavailable} route(s) unavailable`,
);
