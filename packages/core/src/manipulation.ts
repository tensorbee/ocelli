/**
 * Window, region window and stack scroll.
 *
 * The HLD counts the manipulation classes and does not define these drags.
 * The local rules are in `.claude/plans/F-087-design.md`. Pan and zoom stay
 * in the F-069 dispatcher. This module names those tools and does not copy
 * their expressions.
 */

import type { ToolBinding } from "./viewport.js";

/** Pixels that move a window or a stack by one unit. The F-069 scale. */
const DRAG_SCALE = 256;

/** The five manipulation names, each bound to an existing command. */
export const MANIPULATION_TOOLS: readonly ToolBinding[] = [
  { tool: "Pan", command: "Pan" },
  { tool: "Zoom", command: "Zoom" },
  { tool: "WindowLevel", command: "SetWindow" },
  { tool: "WindowLevelRegion", command: "SetWindow" },
  { tool: "StackScroll", command: "Scroll" },
];

function finite(value: number): boolean {
  return Number.isFinite(value);
}

/**
 * Centre follows the horizontal drag. Width follows the upward drag.
 * A width that is not greater than 0 is refused. The basis is not written.
 */
export function windowDrag(
  basis: { readonly centre: number; readonly width: number },
  originX: number,
  originY: number,
  lastX: number,
  lastY: number,
): { centre: number; width: number } | null {
  if (
    !finite(basis.centre) ||
    !finite(basis.width) ||
    !finite(originX) ||
    !finite(originY) ||
    !finite(lastX) ||
    !finite(lastY)
  ) {
    return null;
  }
  const centre = basis.centre + (lastX - originX) / DRAG_SCALE;
  const width = basis.width + (originY - lastY) / DRAG_SCALE;
  if (!finite(centre) || !finite(width) || !(width > 0)) {
    return null;
  }
  return { centre, width };
}

/**
 * Centre is the mid-span of the samples. Width is the span.
 * An empty list, a non-finite sample, or a span of 0 is refused.
 */
export function windowRegion(
  samples: readonly number[],
): { centre: number; width: number } | null {
  const first = samples[0];
  if (first === undefined || !finite(first)) {
    return null;
  }
  let min = first;
  let max = first;
  for (const sample of samples) {
    if (!finite(sample)) {
      return null;
    }
    if (sample < min) {
      min = sample;
    }
    if (sample > max) {
      max = sample;
    }
  }
  if (!(max > min)) {
    return null;
  }
  const centre = (min + max) / 2;
  const width = max - min;
  if (!finite(centre) || !finite(width)) {
    return null;
  }
  return { centre, width };
}

/**
 * Truncation toward zero of the upward drag, in units of `DRAG_SCALE`.
 * A step of 0 is refused.
 */
export function stackDrag(originY: number, lastY: number): number | null {
  if (!finite(originY) || !finite(lastY)) {
    return null;
  }
  const step = Math.trunc((originY - lastY) / DRAG_SCALE);
  if (!finite(step) || step === 0) {
    return null;
  }
  return step;
}
