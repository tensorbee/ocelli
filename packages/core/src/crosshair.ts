/**
 * Crosshair segments and the reference cursor.
 *
 * The HLD does not define these lines. The local rules are in
 * `.claude/plans/F-088-design.md`. The caller supplies the canvas
 * projection. This module does not project and does not read a frame
 * of reference.
 */

/** One canvas segment. Coordinates are viewport pixels. */
export interface Segment {
  readonly x0: number;
  readonly y0: number;
  readonly x1: number;
  readonly y1: number;
}

function inside(x: number, y: number, width: number, height: number): boolean {
  return (
    Number.isFinite(x) &&
    Number.isFinite(y) &&
    Number.isFinite(width) &&
    Number.isFinite(height) &&
    width > 0 &&
    height > 0 &&
    x >= 0 &&
    y >= 0 &&
    x <= width &&
    y <= height
  );
}

/**
 * Horizontal span at `y` and vertical span at `x`.
 * A point outside the viewport, including a non-positive side, returns null.
 * The edges are inside.
 */
export function crosshairLines(
  x: number,
  y: number,
  width: number,
  height: number,
): readonly [Segment, Segment] | null {
  if (!inside(x, y, width, height)) {
    return null;
  }
  return [
    { x0: 0, y0: y, x1: width, y1: y },
    { x0: x, y0: 0, x1: x, y1: height },
  ];
}

/** The same two segments as `crosshairLines`. */
export const referenceLines = crosshairLines;

/** The projected point when it lies inside the viewport, edges included. */
export function referenceCursor(
  x: number,
  y: number,
  width: number,
  height: number,
): { x: number; y: number } | null {
  if (!inside(x, y, width, height)) {
    return null;
  }
  return { x, y };
}
