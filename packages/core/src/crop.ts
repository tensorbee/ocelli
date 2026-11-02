/**
 * Volume crop plane count and origin drag.
 *
 * The HLD does not define a crop drag. The local rules are in
 * `.claude/plans/F-092-design.md`. F-062 already tests the kept side.
 * This module does not call `set_clip_planes` and does not repeat that test.
 */

/** Pixels that add one multiple of the caller normal. The F-069 scale. */
const DRAG_SCALE = 256;

export type CropPlane = {
  origin: readonly [number, number, number];
  normal: readonly [number, number, number];
};

function component(values: readonly number[], index: number): number | null {
  const value = values[index];
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function triple(x: number, y: number, z: number): readonly [number, number, number] {
  return [x, y, z];
}

function copyPlane(plane: CropPlane): CropPlane | null {
  const ox = component(plane.origin, 0);
  const oy = component(plane.origin, 1);
  const oz = component(plane.origin, 2);
  const nx = component(plane.normal, 0);
  const ny = component(plane.normal, 1);
  const nz = component(plane.normal, 2);
  if (ox === null || oy === null || oz === null || nx === null || ny === null || nz === null) {
    return null;
  }
  const lengthSquared = nx * nx + ny * ny + nz * nz;
  if (!Number.isFinite(lengthSquared) || !(lengthSquared > 0)) {
    return null;
  }
  return { origin: triple(ox, oy, oz), normal: triple(nx, ny, nz) };
}

/**
 * A copy of 0 to 6 planes. More than six, a non-finite component, or a
 * zero normal is refused. An empty list is an empty copy.
 */
export function cropPlanes(planes: readonly CropPlane[]): readonly CropPlane[] | null {
  if (planes.length > 6) {
    return null;
  }
  const copied: CropPlane[] = [];
  for (const plane of planes) {
    const next = copyPlane(plane);
    if (next === null) {
      return null;
    }
    copied.push(next);
  }
  return copied;
}

/**
 * `origin + normal * (lastX - originX) / 256`. The normal is not
 * normalised. A zero normal or a non-finite result is refused.
 */
export function cropDrag(
  origin: readonly [number, number, number],
  normal: readonly [number, number, number],
  originX: number,
  lastX: number,
): readonly [number, number, number] | null {
  if (!Number.isFinite(originX) || !Number.isFinite(lastX)) {
    return null;
  }
  const ox = component(origin, 0);
  const oy = component(origin, 1);
  const oz = component(origin, 2);
  const nx = component(normal, 0);
  const ny = component(normal, 1);
  const nz = component(normal, 2);
  if (ox === null || oy === null || oz === null || nx === null || ny === null || nz === null) {
    return null;
  }
  if (nx === 0 && ny === 0 && nz === 0) {
    return null;
  }
  const step = (lastX - originX) / DRAG_SCALE;
  const x = ox + nx * step;
  const y = oy + ny * step;
  const z = oz + nz * step;
  if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(z)) {
    return null;
  }
  return triple(x, y, z);
}
