/**
 * Slice intersection, segmentation intersection and an overlay grid.
 *
 * The HLD does not define these overlays. The local rules are in
 * `.claude/plans/F-089-design.md`. This module does not project and does
 * not import the crosshair module.
 */

export type Segment = {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
};

function positiveSize(width: number, height: number): boolean {
  return Number.isFinite(width) && Number.isFinite(height) && width > 0 && height > 0;
}

/**
 * Liang-Barsky against the viewport. Edges are inside. A zero delta is
 * not a line. A miss returns nothing.
 */
function clipSegment(
  x0: number,
  y0: number,
  x1: number,
  y1: number,
  width: number,
  height: number,
): Segment | null {
  if (
    !Number.isFinite(x0) ||
    !Number.isFinite(y0) ||
    !Number.isFinite(x1) ||
    !Number.isFinite(y1) ||
    !positiveSize(width, height)
  ) {
    return null;
  }
  const dx = x1 - x0;
  const dy = y1 - y0;
  if (dx === 0 && dy === 0) {
    return null;
  }
  let t0 = 0;
  let t1 = 1;
  const edges: readonly (readonly [number, number])[] = [
    [-dx, x0],
    [dx, width - x0],
    [-dy, y0],
    [dy, height - y0],
  ];
  for (const edge of edges) {
    const p = edge[0];
    const q = edge[1];
    if (p === 0) {
      if (q < 0) {
        return null;
      }
      continue;
    }
    const t = q / p;
    if (p < 0) {
      if (t > t1) {
        return null;
      }
      if (t > t0) {
        t0 = t;
      }
    } else if (t < t0) {
      return null;
    } else if (t < t1) {
      t1 = t;
    }
  }
  const leftX = x0 + t0 * dx;
  const leftY = y0 + t0 * dy;
  const rightX = x0 + t1 * dx;
  const rightY = y0 + t1 * dy;
  if (
    !Number.isFinite(leftX) ||
    !Number.isFinite(leftY) ||
    !Number.isFinite(rightX) ||
    !Number.isFinite(rightY)
  ) {
    return null;
  }
  return { x0: leftX, y0: leftY, x1: rightX, y1: rightY };
}

/** One caller segment clipped to the viewport. */
export function sliceIntersection(
  x0: number,
  y0: number,
  x1: number,
  y1: number,
  width: number,
  height: number,
): Segment | null {
  return clipSegment(x0, y0, x1, y1, width, height);
}

/**
 * The same clip on each segment, in order. A miss is dropped. One
 * non-finite coordinate refuses the whole call.
 */
export function segmentationIntersection(
  segments: readonly Segment[],
  width: number,
  height: number,
): readonly Segment[] | null {
  if (!positiveSize(width, height)) {
    return null;
  }
  const kept: Segment[] = [];
  for (const segment of segments) {
    if (
      !Number.isFinite(segment.x0) ||
      !Number.isFinite(segment.y0) ||
      !Number.isFinite(segment.x1) ||
      !Number.isFinite(segment.y1)
    ) {
      return null;
    }
    const clipped = clipSegment(segment.x0, segment.y0, segment.x1, segment.y1, width, height);
    if (clipped !== null) {
      kept.push(clipped);
    }
  }
  return kept;
}

/**
 * Interior grid lines. Counts are integers of at least 1. One by one
 * returns an empty list. A fractional count returns nothing.
 */
export function overlayGrid(
  columns: number,
  rows: number,
  width: number,
  height: number,
): readonly Segment[] | null {
  if (!Number.isInteger(columns) || !Number.isInteger(rows) || columns < 1 || rows < 1) {
    return null;
  }
  if (!positiveSize(width, height)) {
    return null;
  }
  const lines: Segment[] = [];
  for (let column = 1; column < columns; column += 1) {
    const x = (width * column) / columns;
    lines.push({ x0: x, y0: 0, x1: x, y1: height });
  }
  for (let row = 1; row < rows; row += 1) {
    const y = (height * row) / rows;
    lines.push({ x0: 0, y0: y, x1: width, y1: y });
  }
  return lines;
}
