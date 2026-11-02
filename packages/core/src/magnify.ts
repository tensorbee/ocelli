/**
 * Magnify circle, scale overlay and orientation marker.
 *
 * The HLD does not define these overlays. The local rules are in
 * `.claude/plans/F-091-design.md`. This module does not read pixels,
 * DICOM spacing, or the annotation layer.
 */

/**
 * A circle at the caller centre. A null scale is plain magnify and is stored
 * as 1. An advanced scale must be finite and greater than 1.
 */
export function magnifyCircle(
  x: number,
  y: number,
  radius: number,
  scale: number | null,
): { x: number; y: number; radius: number; scale: number } | null {
  if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(radius) || !(radius > 0)) {
    return null;
  }
  if (scale === null) {
    return { x, y, radius, scale: 1 };
  }
  if (!Number.isFinite(scale) || !(scale > 1)) {
    return null;
  }
  return { x, y, radius, scale };
}

/**
 * Pixel length is the product. A non-finite factor, or a pixels-per-millimetre
 * that is not greater than 0, is refused.
 */
export function scaleLength(millimetres: number, pixelsPerMillimetre: number): number | null {
  if (!Number.isFinite(millimetres) || !Number.isFinite(pixelsPerMillimetre)) {
    return null;
  }
  if (!(pixelsPerMillimetre > 0)) {
    return null;
  }
  const pixels = millimetres * pixelsPerMillimetre;
  if (!Number.isFinite(pixels)) {
    return null;
  }
  return pixels;
}

function letterAt(letters: readonly string[], index: number): string | null {
  const letter = letters[index];
  return typeof letter === "string" ? letter : null;
}

/**
 * Quadrant 0 returns the four letters unchanged. Each later quadrant moves
 * the first letter to the end. Any other quadrant is refused and does not wrap.
 */
export function orientationMarker(
  letters: readonly [string, string, string, string],
  quadrant: number,
): readonly [string, string, string, string] | null {
  const first = letterAt(letters, 0);
  const second = letterAt(letters, 1);
  const third = letterAt(letters, 2);
  const fourth = letterAt(letters, 3);
  if (
    letters.length !== 4 ||
    first === null ||
    second === null ||
    third === null ||
    fourth === null
  ) {
    return null;
  }
  if (quadrant !== 0 && quadrant !== 1 && quadrant !== 2 && quadrant !== 3) {
    return null;
  }
  let a = first;
  let b = second;
  let c = third;
  let d = fourth;
  for (let step = 0; step < quadrant; step += 1) {
    const moved = a;
    a = b;
    b = c;
    c = d;
    d = moved;
  }
  return [a, b, c, d];
}
