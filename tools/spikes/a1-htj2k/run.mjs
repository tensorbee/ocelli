// THROWAWAY SPIKE CODE. F-X006, Appendix A gate A1.
//
// Drives four decodes of each of the three HTJ2K corpus rows, plus two JPEG 2000
// Part 1 CONTROL rows, and compares them through the ONE comparator in
// `tools/spikes/common/compare.mjs`:
//
//   D_wasm    `openjp2` 0.6.1 for wasm32-unknown-unknown, under node
//   D_simd    the same, built with `-C target-feature=+simd128`
//   D_native  the same `openjp2` 0.6.1 compiled for the host
//   D_ojph    `ojph_expand`, the independent decoder A1 names
//   R         `syntax/explicit_vr_le.dcm` PixelData, the uncompressed anchor
//
// `R` is exact for the REVERSIBLE rows only. `1.2.840.10008.1.2.4.203` and
// `1.2.840.10008.1.2.4.91` are irreversible, so no exact anchor exists for them
// and the record says so rather than implying one. `ojph_expand` is an HTJ2K
// decoder, so the Part 1 controls have no `D_ojph` and are not claimed to.
//
// A comparison whose two sides are not both available prints NOT MEASURED. It
// is never inferred and never silently dropped.
//
// Run: node tools/spikes/a1-htj2k/run.mjs

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

const CASES = [
  { index: 0, name: "htj2k_lossless", uid: "1.2.840.10008.1.2.4.201",
    reversible: true, control: false },
  { index: 1, name: "htj2k_lossless_rpcl", uid: "1.2.840.10008.1.2.4.202",
    reversible: true, control: false },
  { index: 2, name: "htj2k_lossy", uid: "1.2.840.10008.1.2.4.203",
    reversible: false, control: false },
  // CONTROLS, JPEG 2000 Part 1. Not A1's question. They separate "openjp2 does
  // not work on wasm32" from "openjp2's HTJ2K path does not work on wasm32".
  { index: 3, name: "j2k_lossless", uid: "1.2.840.10008.1.2.4.90",
    reversible: true, control: true },
  { index: 4, name: "j2k_lossy", uid: "1.2.840.10008.1.2.4.91",
    reversible: false, control: true },
];

const BUILDS = [
  { label: "D_wasm", dir: "wasm-plain" },
  { label: "D_simd", dir: "wasm-simd" },
];

/**
 * `openjp2` 0.6.1 leaves exactly one C symbol unresolved after the Rust-side
 * allocator shim: `strcpy`, from its event and error reporting. It is supplied
 * here, in JavaScript, over the module's own linear memory. Nothing in Rust
 * touches a pointer.
 */
function makeImports(memoryRef) {
  return {
    env: {
      strcpy(destination, source) {
        const bytes = new Uint8Array(memoryRef.value.buffer);
        let i = 0;
        while (bytes[source + i] !== 0) {
          bytes[destination + i] = bytes[source + i];
          i += 1;
        }
        bytes[destination + i] = 0;
        return destination;
      },
    },
  };
}

function decodeUnderNode(dir, caseIndex) {
  const path = join(
    HERE, "target", dir, "wasm32-unknown-unknown", "release", "a1_htj2k.wasm",
  );
  const module = new WebAssembly.Module(readFileSync(path));
  const memoryRef = { value: null };
  const instance = new WebAssembly.Instance(module, makeImports(memoryRef));
  memoryRef.value = instance.exports.memory;

  // `openjp2` 0.6.1 traps on wasm32 during tile teardown, which happens after
  // the pixels are stored. See the note above `decode` in `src/lib.rs`. A wasm
  // trap unwinds into JavaScript without poisoning the instance, so the pixels
  // are still readable and the trap is REPORTED rather than swallowed.
  let trap = null;
  let code = 0;
  try {
    code = instance.exports.decode(caseIndex);
  } catch (error) {
    trap = error.message;
  }
  if (trap === null && code !== 0) {
    throw new Error(`${dir} case ${caseIndex}: raw decoder error code ${code}`);
  }

  const length = instance.exports.out_len();
  if (length === 0) {
    // No pixels. Either the decode trapped or it refused. Both are reported,
    // and neither is turned into a buffer by reading `out_ptr()` anyway.
    return { bytes: null, trap, code };
  }
  if (length !== CANONICAL_BYTES) {
    throw new Error(
      `${dir} case ${caseIndex}: out_len ${length}, expected ${CANONICAL_BYTES}`,
    );
  }
  const pointer = instance.exports.out_ptr();
  if (pointer === 0) {
    throw new Error(`${dir} case ${caseIndex}: out_ptr is zero`);
  }
  // The copy matters. A view into a live linear memory is not a stable buffer.
  const bytes = new Uint8Array(
    instance.exports.memory.buffer.slice(pointer, pointer + length),
  );
  return { bytes, trap, code };
}

function nativeDecode(caseIndex) {
  const binary = join(HERE, "target", "release", "a1-htj2k");
  const target = join(OUT, `native_case${caseIndex}.raw`);
  execFileSync(binary, [String(caseIndex), target], { stdio: "pipe" });
  return new Uint8Array(readFileSync(target));
}

function ojphDecode(name) {
  const source = join(OUT, `${name}.j2c`);
  const target = join(OUT, `ojph_${name}.pgm`);
  execFileSync("ojph_expand", ["-i", source, "-o", target], { stdio: "pipe" });
  return pgmToCanonical(readFileSync(target), `ojph:${name}`);
}

const reference = new Uint8Array(readFileSync(join(OUT, "reference.raw")));

console.log("=== A1, HTJ2K through openjp2 0.6.1 ===");
console.log(`R (syntax/explicit_vr_le.dcm PixelData): ${sha256(reference)}`);

let failures = 0;
let traps = 0;
let unavailable = 0;
let skipped = 0;
for (const testCase of CASES) {
  console.log(`\n--- ${testCase.name}, ${testCase.uid}, ` +
    `${testCase.reversible ? "reversible" : "irreversible"}` +
    `${testCase.control ? ", CONTROL" : ""} ---`);

  const decodes = new Map();
  for (const build of BUILDS) {
    const { bytes, trap, code } = decodeUnderNode(build.dir, testCase.index);
    if (bytes === null) {
      console.log(
        `  ${build.label.padEnd(9)} NO PIXELS. ` +
          `trap: ${trap === null ? "none" : trap}, raw error code ${code}`,
      );
      unavailable += 1;
      if (trap !== null) traps += 1;
    } else {
      decodes.set(build.label, bytes);
      console.log(
        `  ${build.label.padEnd(9)} decoded, trap: ` +
          `${trap === null ? "none" : trap}`,
      );
    }
  }
  decodes.set("D_native", nativeDecode(testCase.index));
  if (!testCase.control) {
    // `ojph_expand` is an HTJ2K decoder. The Part 1 controls have no OpenJPH
    // anchor and are not claimed to have one.
    decodes.set("D_ojph", ojphDecode(testCase.name));
  }

  for (const [label, bytes] of decodes) {
    console.log(`  ${label.padEnd(9)} ${sha256(bytes)}`);
  }

  const pairs = [
    ["D_wasm", "D_native"],
    ["D_simd", "D_wasm"],
  ];
  if (!testCase.control) {
    pairs.push(["D_wasm", "D_ojph"], ["D_native", "D_ojph"]);
  }
  if (testCase.reversible) {
    decodes.set("R", reference);
    pairs.push(["D_native", "R"], ["D_wasm", "R"]);
    if (!testCase.control) pairs.push(["D_ojph", "R"]);
  }

  for (const [left, right] of pairs) {
    if (!decodes.has(left) || !decodes.has(right)) {
      // NOT MEASURED, and said so rather than silently omitted.
      console.log(`  NOT MEASURED  ${left} vs ${right}, one side has no pixels`);
      skipped += 1;
      continue;
    }
    const result = compare(
      decodes.get(left), decodes.get(right), left, right,
    );
    if (!result.equal) failures += 1;
    console.log(report(result).split("\n").map((l) => `  ${l}`).join("\n"));
  }
}

console.log(
  `\n${failures} differing comparison(s), ${skipped} not measured, ` +
    `${unavailable} decode(s) produced no pixels, ${traps} wasm trap(s)`,
);
