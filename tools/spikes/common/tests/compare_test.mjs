// THROWAWAY SPIKE CODE, see the header of `../compare.mjs`.
//
// The comparator is observed red before any digest it prints is believed. This
// file feeds it two buffers differing in one sample and asserts it reports
// exactly that sample, and feeds it a short buffer and asserts a refusal.
//
// S03's stated gate defect class, applied to a spike: a guard mutated in the
// same command that adds it has been observed to fire once and has nothing
// watching it afterwards. The mutation runs as its own command.
//
// Run:  node tools/spikes/common/tests/compare_test.mjs

import assert from "node:assert/strict";
import {
  CANONICAL_BYTES,
  CANONICAL_SAMPLES,
  assertCanonical,
  compare,
  pgmToCanonical,
  sha256,
} from "../compare.mjs";

let failures = 0;

function check(name, body) {
  try {
    body();
    console.log(`ok    ${name}`);
  } catch (error) {
    failures += 1;
    console.log(`FAIL  ${name}: ${error.message}`);
  }
}

/** A canonical buffer whose sample i is (i * 7) mod 65536, little-endian. */
function synthetic() {
  const bytes = new Uint8Array(CANONICAL_BYTES);
  for (let i = 0; i < CANONICAL_SAMPLES; i += 1) {
    const value = (i * 7) % 65536;
    bytes[2 * i] = value & 0xff;
    bytes[2 * i + 1] = (value >> 8) & 0xff;
  }
  return bytes;
}

check("identical buffers compare equal", () => {
  const a = synthetic();
  const b = synthetic();
  const result = compare(a, b, "a", "b");
  assert.equal(result.equal, true);
  assert.equal(result.differing, 0);
  assert.equal(result.first, null);
  assert.equal(result.maxAbs, 0);
  assert.equal(result.digestA, result.digestB);
});

check("one differing sample is reported at its index", () => {
  const a = synthetic();
  const b = synthetic();
  // Sample 4097 has value (4097 * 7) mod 65536 = 28679. Raise it by 5.
  const index = 4097;
  const before = a[2 * index] | (a[2 * index + 1] << 8);
  assert.equal(before, 28679);
  const after = before + 5;
  b[2 * index] = after & 0xff;
  b[2 * index + 1] = (after >> 8) & 0xff;

  const result = compare(a, b, "a", "b");
  assert.equal(result.equal, false);
  assert.notEqual(result.digestA, result.digestB);
  assert.equal(result.differing, 1);
  assert.equal(result.first.index, index);
  assert.equal(result.first.a, 28679);
  assert.equal(result.first.b, 28684);
  assert.equal(result.maxAbs, 5);
  assert.deepEqual(result.histogram, [[5, 1]]);
});

check("a buffer that is not exactly 12288 bytes is refused", () => {
  assert.throws(
    () => assertCanonical(new Uint8Array(CANONICAL_BYTES - 2), "short"),
    /12286 bytes, expected exactly 12288/,
  );
  assert.throws(
    () => assertCanonical(new Uint8Array(CANONICAL_BYTES + 2), "long"),
    /12290 bytes, expected exactly 12288/,
  );
  assert.throws(
    () => compare(new Uint8Array(4), synthetic(), "short", "b"),
    /expected exactly 12288/,
  );
});

check("PGM big-endian body is byte-swapped into canonical order", () => {
  const canonical = synthetic();
  const header = Buffer.from("P5\n96 64\n65535\n", "ascii");
  const body = new Uint8Array(CANONICAL_BYTES);
  for (let i = 0; i < CANONICAL_SAMPLES; i += 1) {
    body[2 * i] = canonical[2 * i + 1]; // big-endian high byte first
    body[2 * i + 1] = canonical[2 * i];
  }
  const pgm = Buffer.concat([header, Buffer.from(body)]);
  const converted = pgmToCanonical(pgm, "pgm");
  assert.equal(sha256(converted), sha256(canonical));
});

check("a PGM of the wrong shape is refused", () => {
  const pgm = Buffer.concat([
    Buffer.from("P5\n95 64\n65535\n", "ascii"),
    Buffer.alloc(CANONICAL_BYTES),
  ]);
  assert.throws(() => pgmToCanonical(pgm, "pgm"), /95x64, expected 96x64/);
});

if (failures > 0) {
  console.log(`\n${failures} failure(s)`);
  process.exit(1);
}
console.log("\nall comparator checks green");
