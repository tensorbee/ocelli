/**
 * Trackball, volume and planar rotation, plus the orientation quadrant.
 *
 * The HLD does not define a second rotation. The drag is the F-069 delta.
 * The quadrant rule is in `.claude/plans/F-090-design.md`. This module does
 * not write a camera and does not build a matrix.
 */

import type { ToolBinding } from "./viewport.js";

/** Pixels that add one radian. The F-069 scale. */
const DRAG_SCALE = 256;

const QUARTERS = [0, 1, 2, 3] as const;

/** Three names for the existing Rotate command. */
export const ROTATION_TOOLS: readonly ToolBinding[] = [
  { tool: "TrackballRotate", command: "Rotate" },
  { tool: "VolumeRotate", command: "Rotate" },
  { tool: "PlanarRotate", command: "Rotate" },
];

/**
 * `radians + (lastX - originX) / 256`. A non-finite input or result is refused.
 */
export function rotationDrag(radians: number, originX: number, lastX: number): number | null {
  if (!Number.isFinite(radians) || !Number.isFinite(originX) || !Number.isFinite(lastX)) {
    return null;
  }
  const next = radians + (lastX - originX) / DRAG_SCALE;
  if (!Number.isFinite(next)) {
    return null;
  }
  return next;
}

/**
 * Quarter turns of `radians`, wrapped into 0, 1, 2 or 3.
 * Truncation is toward zero. A negative remainder wraps. A non-finite angle
 * is refused. This is the OrientationController. It is not a registry tool.
 */
export function orientationQuadrant(radians: number): 0 | 1 | 2 | 3 | null {
  if (!Number.isFinite(radians)) {
    return null;
  }
  const quarter = Math.PI / 2;
  const raw = Math.trunc(radians / quarter);
  const wrapped = ((raw % 4) + 4) % 4;
  const index = QUARTERS[wrapped];
  return index === undefined ? null : index;
}
