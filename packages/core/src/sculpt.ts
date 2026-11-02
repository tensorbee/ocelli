/**
 * Sculptor, annotation eraser and MIP jump.
 *
 * The HLD does not define these tools. The local rules are in
 * `.claude/plans/F-093-design.md`. This module does not read pixels,
 * does not compute a polygon area, and does not edit the annotation layer.
 */

type Point = { x: number; y: number };

type Mark = { id: string; x: number; y: number };

/**
 * Move one vertex by the pixel delta. At least three points are required.
 * The caller list is not written.
 */
export function sculptVertex(
  points: readonly Point[],
  index: number,
  originX: number,
  originY: number,
  lastX: number,
  lastY: number,
): readonly Point[] | null {
  if (!Number.isInteger(index) || index < 0 || index >= points.length || points.length < 3) {
    return null;
  }
  if (
    !Number.isFinite(originX) ||
    !Number.isFinite(originY) ||
    !Number.isFinite(lastX) ||
    !Number.isFinite(lastY)
  ) {
    return null;
  }
  const dx = lastX - originX;
  const dy = lastY - originY;
  const moved: Point[] = [];
  for (let cursor = 0; cursor < points.length; cursor += 1) {
    const point = points[cursor];
    if (point === undefined || !Number.isFinite(point.x) || !Number.isFinite(point.y)) {
      return null;
    }
    if (cursor === index) {
      const x = point.x + dx;
      const y = point.y + dy;
      if (!Number.isFinite(x) || !Number.isFinite(y)) {
        return null;
      }
      moved.push({ x, y });
    } else {
      moved.push({ x: point.x, y: point.y });
    }
  }
  return moved;
}

/**
 * The nearest mark at a distance no greater than the radius. An equal
 * distance keeps the earlier mark. The mark is not removed.
 */
export function eraseNearest(
  marks: readonly Mark[],
  x: number,
  y: number,
  radius: number,
): string | null {
  if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(radius) || !(radius > 0)) {
    return null;
  }
  const limit = radius * radius;
  if (!Number.isFinite(limit)) {
    return null;
  }
  let bestId: string | null = null;
  let bestDistance = limit;
  for (const mark of marks) {
    if (typeof mark.id !== "string" || mark.id.length === 0) {
      return null;
    }
    if (!Number.isFinite(mark.x) || !Number.isFinite(mark.y)) {
      return null;
    }
    const dx = mark.x - x;
    const dy = mark.y - y;
    const distance = dx * dx + dy * dy;
    if (!Number.isFinite(distance)) {
      return null;
    }
    if (distance <= limit && (bestId === null || distance < bestDistance)) {
      bestId = mark.id;
      bestDistance = distance;
    }
  }
  return bestId;
}

/**
 * The earlier index of the maximum sample. An empty list or a non-finite
 * sample is refused. The samples are caller numbers.
 */
export function mipIndex(samples: readonly number[]): number | null {
  const first = samples[0];
  if (first === undefined || !Number.isFinite(first)) {
    return null;
  }
  let best = first;
  let index = 0;
  for (let cursor = 1; cursor < samples.length; cursor += 1) {
    const sample = samples[cursor];
    if (sample === undefined || !Number.isFinite(sample)) {
      return null;
    }
    if (sample > best) {
      best = sample;
      index = cursor;
    }
  }
  return index;
}
