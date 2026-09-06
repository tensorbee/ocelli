// THROWAWAY SPIKE CODE. Do not import this from `crates/` or `tools/oracle/`.
//
// F-X006 answers Appendix A gates A1 and A2. This comparator is exact equality
// over a DECODED PIXEL BUFFER and it carries no tolerance of any kind, because
// none of its comparisons is a rendering comparison and HLD section 25.1 does
// not reach them. F-011 builds the real comparator, over RENDERED frames, with
// section 25.1's per-modality tolerance policy. The two must never converge:
// this one is deleted when A1 and A2 are closed, and the sprint must not end
// with two general comparators.
//
// The canonical form every decode is reduced to before comparison:
// 12288 bytes, little-endian u16, 6144 samples, row-major, 64 rows by 96
// columns. That shape is not chosen, it is what `scripts/corpus_synth.py`
// produced, and all five compressed corpus rows were encoded from
// `syntax/explicit_vr_le.dcm`.

import { createHash } from "node:crypto";

export const CANONICAL_BYTES = 12288;
export const CANONICAL_SAMPLES = 6144;
export const CANONICAL_ROWS = 64;
export const CANONICAL_COLS = 96;

/** sha256 of a byte buffer, lowercase hex. */
export function sha256(bytes) {
  return createHash("sha256").update(Buffer.from(bytes)).digest("hex");
}

/**
 * Refuse anything that is not exactly the canonical length.
 *
 * A truncation must never pass as an equality over a shorter buffer, which is
 * why this is a hard refusal and not a minimum-length check.
 */
export function assertCanonical(bytes, label) {
  if (!(bytes instanceof Uint8Array)) {
    throw new Error(`${label}: not a Uint8Array`);
  }
  if (bytes.length !== CANONICAL_BYTES) {
    throw new Error(
      `${label}: ${bytes.length} bytes, expected exactly ${CANONICAL_BYTES}`,
    );
  }
  return bytes;
}

/** Little-endian u16 view over canonical bytes. */
export function samples(bytes, label) {
  assertCanonical(bytes, label);
  const out = new Uint16Array(CANONICAL_SAMPLES);
  for (let i = 0; i < CANONICAL_SAMPLES; i += 1) {
    out[i] = bytes[2 * i] | (bytes[2 * i + 1] << 8);
  }
  return out;
}

/**
 * Parse a binary PGM (`P5`) into canonical little-endian bytes.
 *
 * `ojph_expand` writes PGM, and `P5` with a maxval above 255 is BIG-ENDIAN by
 * the PGM specification, so this is a header skip and a byte swap. The swap
 * happens here and never inside a decoder, so no decode is touched by our
 * arithmetic.
 */
export function pgmToCanonical(buffer, label) {
  const bytes = new Uint8Array(buffer);
  if (bytes[0] !== 0x50 || bytes[1] !== 0x35) {
    throw new Error(`${label}: not a P5 PGM`);
  }
  // Header: P5, width, height, maxval. Whitespace separated, `#` comments run
  // to end of line. Exactly one whitespace byte follows maxval.
  const fields = [];
  let i = 2;
  while (fields.length < 3) {
    while (i < bytes.length && /\s/.test(String.fromCharCode(bytes[i]))) {
      i += 1;
    }
    if (bytes[i] === 0x23) {
      while (i < bytes.length && bytes[i] !== 0x0a) i += 1;
      continue;
    }
    let token = "";
    while (i < bytes.length && !/\s/.test(String.fromCharCode(bytes[i]))) {
      token += String.fromCharCode(bytes[i]);
      i += 1;
    }
    fields.push(Number(token));
  }
  i += 1; // the single whitespace byte after maxval
  const [width, height, maxval] = fields;
  if (width !== CANONICAL_COLS || height !== CANONICAL_ROWS) {
    throw new Error(
      `${label}: PGM is ${width}x${height}, expected ` +
        `${CANONICAL_COLS}x${CANONICAL_ROWS}`,
    );
  }
  if (maxval <= 255) {
    throw new Error(`${label}: PGM maxval ${maxval}, expected a 16-bit image`);
  }
  const body = bytes.subarray(i);
  if (body.length !== CANONICAL_BYTES) {
    throw new Error(
      `${label}: PGM body ${body.length} bytes, expected ${CANONICAL_BYTES}`,
    );
  }
  const out = new Uint8Array(CANONICAL_BYTES);
  for (let s = 0; s < CANONICAL_SAMPLES; s += 1) {
    out[2 * s] = body[2 * s + 1]; // low byte, from big-endian high position
    out[2 * s + 1] = body[2 * s];
  }
  return out;
}

/**
 * Compare two canonical buffers exactly.
 *
 * Returns the headline digests and, when they differ, an index-level report:
 * how many samples differ, the first differing sample with both values, the
 * maximum absolute difference, and a histogram of differences by magnitude.
 * The digest says equal or not equal. The index report says what to do next.
 *
 * A difference is never absorbed. It is either explained by a named property
 * of the format, which publishes a bound, or it is a defect.
 */
export function compare(a, b, labelA, labelB) {
  const sa = samples(a, labelA);
  const sb = samples(b, labelB);
  const digestA = sha256(a);
  const digestB = sha256(b);

  let differing = 0;
  let first = null;
  let maxAbs = 0;
  const histogram = new Map();

  for (let i = 0; i < CANONICAL_SAMPLES; i += 1) {
    if (sa[i] === sb[i]) continue;
    differing += 1;
    const delta = Math.abs(sa[i] - sb[i]);
    if (first === null) first = { index: i, a: sa[i], b: sb[i] };
    if (delta > maxAbs) maxAbs = delta;
    histogram.set(delta, (histogram.get(delta) ?? 0) + 1);
  }

  return {
    labelA,
    labelB,
    digestA,
    digestB,
    equal: digestA === digestB,
    differing,
    first,
    maxAbs,
    histogram: [...histogram.entries()].sort((x, y) => x[0] - y[0]),
  };
}

/** One line per comparison, plus the index report when they differ. */
export function report(result) {
  const lines = [];
  const verdict = result.equal ? "EQUAL" : "DIFFER";
  lines.push(`${verdict}  ${result.labelA} vs ${result.labelB}`);
  lines.push(`  ${result.labelA}: ${result.digestA}`);
  lines.push(`  ${result.labelB}: ${result.digestB}`);
  if (!result.equal) {
    lines.push(
      `  differing samples: ${result.differing} of ${CANONICAL_SAMPLES}`,
    );
    lines.push(
      `  first difference: index ${result.first.index}, ` +
        `${result.labelA}=${result.first.a} ${result.labelB}=${result.first.b}`,
    );
    lines.push(`  max absolute difference: ${result.maxAbs}`);
    const shown = result.histogram
      .slice(0, 16)
      .map(([delta, count]) => `${delta}:${count}`)
      .join(" ");
    const tail = result.histogram.length > 16 ? " ..." : "";
    lines.push(`  histogram by magnitude: ${shown}${tail}`);
  }
  return lines.join("\n");
}
