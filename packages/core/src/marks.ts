/**
 * Arrow, label, key image and ETDRS display-list marks.
 *
 * The HLD does not define these marks. The local rules are in
 * `.claude/plans/F-078-design.md`. Each constructor returns an F-071
 * display-list mark. Nothing here builds an SR content tree or changes
 * the drawing layer.
 */

import type { AnnotationMark, AnnotationPoint } from "./annotation.js";

/** Refusals the constructors can return, in check order. */
export const MARK_REFUSALS = ["blank-id", "blank-text", "frame", "degenerate"] as const;

/** One refusal from `MARK_REFUSALS`. */
export type MarkRefusal = (typeof MARK_REFUSALS)[number];

/** One constructed mark, or the first refusal. */
export type MarkResult =
  | { readonly ok: true; readonly mark: AnnotationMark }
  | { readonly ok: false; readonly refusal: MarkRefusal };

/** Several constructed marks, or the first refusal. */
export type MarksResult =
  | { readonly ok: true; readonly marks: readonly AnnotationMark[] }
  | { readonly ok: false; readonly refusal: MarkRefusal };

/** A ring and a quadrant, or the centre and the outside of the grid. */
export type EtdrsRegion =
  | "central"
  | "outside"
  | `inner ${EtdrsQuadrant}`
  | `outer ${EtdrsQuadrant}`;

type EtdrsQuadrant = "superior" | "inferior" | "temporal" | "nasal";

const HEAD_LIMIT_PX = 12;
const HEAD_SHAFT_FRACTION = 4;
const BARB_TURN = Math.PI / 6;
const CIRCLE_STEPS = 32;
const CENTRAL_RADIUS_MM = 0.5;
const INNER_RADIUS_MM = 1.5;
const OUTER_RADIUS_MM = 3;

function blank(value: string): boolean {
  return value.trim().length === 0;
}

function finitePoint(point: AnnotationPoint): boolean {
  return Number.isFinite(point.x) && Number.isFinite(point.y);
}

function displayMark(
  id: string,
  points: readonly AnnotationPoint[],
  text: string | null,
): AnnotationMark {
  return {
    id,
    points,
    text,
    visible: true,
    locked: false,
    selected: false,
    completed: true,
  };
}

function refused(refusal: MarkRefusal): { readonly ok: false; readonly refusal: MarkRefusal } {
  return { ok: false, refusal };
}

/**
 * Shaft, then the two barbs. The head is at most 12 pixels and at most a
 * quarter of the shaft. A zero-length shaft is degenerate.
 */
export function arrowMark(
  id: string,
  start: AnnotationPoint,
  end: AnnotationPoint,
  text: string | null,
): MarkResult {
  if (blank(id)) {
    return refused("blank-id");
  }
  if (text !== null && blank(text)) {
    return refused("blank-text");
  }
  if (!finitePoint(start) || !finitePoint(end)) {
    return refused("frame");
  }
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const length = Math.hypot(dx, dy);
  if (length === 0) {
    return refused("degenerate");
  }
  const head = Math.min(HEAD_LIMIT_PX, length / HEAD_SHAFT_FRACTION);
  const reverseX = -dx / length;
  const reverseY = -dy / length;
  const turnCos = Math.cos(BARB_TURN);
  const turnSin = Math.sin(BARB_TURN);
  const positive = {
    x: end.x + (reverseX * turnCos - reverseY * turnSin) * head,
    y: end.y + (reverseX * turnSin + reverseY * turnCos) * head,
  };
  const negative = {
    x: end.x + (reverseX * turnCos + reverseY * turnSin) * head,
    y: end.y + (-reverseX * turnSin + reverseY * turnCos) * head,
  };
  return {
    ok: true,
    mark: displayMark(id, [start, end, positive, end, negative], text),
  };
}

/** One point and a non-blank label. */
export function labelMark(id: string, point: AnnotationPoint, text: string): MarkResult {
  if (blank(id)) {
    return refused("blank-id");
  }
  if (blank(text)) {
    return refused("blank-text");
  }
  if (!finitePoint(point)) {
    return refused("frame");
  }
  return { ok: true, mark: displayMark(id, [point], text) };
}

/** One point labelled `KEY`. A second call with the same inputs is equal. */
export function keyImageMark(id: string, point: AnnotationPoint): MarkResult {
  if (blank(id)) {
    return refused("blank-id");
  }
  if (!finitePoint(point)) {
    return refused("frame");
  }
  return { ok: true, mark: displayMark(id, [point], "KEY") };
}

/**
 * Classify one offset in millimetres. Positive `y` is superior. A boundary
 * belongs to the outer side. `temporalIsPositiveX` names the positive `x`
 * side and does not change a vertical region.
 */
export function etdrsRegion(
  dxMm: number,
  dyMm: number,
  temporalIsPositiveX: boolean,
): EtdrsRegion | "frame" {
  if (!Number.isFinite(dxMm) || !Number.isFinite(dyMm)) {
    return "frame";
  }
  const radius = Math.hypot(dxMm, dyMm);
  if (radius < CENTRAL_RADIUS_MM) {
    return "central";
  }
  if (radius >= OUTER_RADIUS_MM) {
    return "outside";
  }
  const ring = radius < INNER_RADIUS_MM ? "inner" : "outer";
  const quadrant = quadrantOf(dxMm, dyMm, temporalIsPositiveX);
  return `${ring} ${quadrant}`;
}

function quadrantOf(dxMm: number, dyMm: number, temporalIsPositiveX: boolean): EtdrsQuadrant {
  if (Math.abs(dyMm) >= Math.abs(dxMm)) {
    return dyMm > 0 ? "superior" : "inferior";
  }
  const positiveIsTemporal = dxMm > 0 ? temporalIsPositiveX : !temporalIsPositiveX;
  return positiveIsTemporal ? "temporal" : "nasal";
}

function closedCircle(centre: AnnotationPoint, radiusPx: number): AnnotationPoint[] {
  const start = { x: centre.x + radiusPx, y: centre.y };
  const points: AnnotationPoint[] = [start];
  for (let step = 1; step < CIRCLE_STEPS; step += 1) {
    const theta = (step * 2 * Math.PI) / CIRCLE_STEPS;
    points.push({
      x: centre.x + radiusPx * Math.cos(theta),
      y: centre.y + radiusPx * Math.sin(theta),
    });
  }
  points.push({ x: start.x, y: start.y });
  return points;
}

function spoke(
  centre: AnnotationPoint,
  innerPx: number,
  outerPx: number,
  xSign: -1 | 0 | 1,
  ySign: -1 | 0 | 1,
): AnnotationPoint[] {
  return [
    { x: centre.x + xSign * innerPx, y: centre.y + ySign * innerPx },
    { x: centre.x + xSign * outerPx, y: centre.y + ySign * outerPx },
  ];
}

/**
 * Three closed circles and four cardinal spokes. The spokes are axis
 * offsets. `temporalIsPositiveX` is accepted and does not move them.
 */
export function etdrsMarks(
  prefix: string,
  centre: AnnotationPoint,
  mmPerPixel: number,
  temporalIsPositiveX: boolean,
): MarksResult {
  if (blank(prefix)) {
    return refused("blank-id");
  }
  if (
    !finitePoint(centre) ||
    !Number.isFinite(mmPerPixel) ||
    typeof temporalIsPositiveX !== "boolean"
  ) {
    return refused("frame");
  }
  if (mmPerPixel <= 0) {
    return refused("degenerate");
  }
  const innerPx = CENTRAL_RADIUS_MM / mmPerPixel;
  const outerPx = OUTER_RADIUS_MM / mmPerPixel;
  const circles: readonly (readonly [string, number])[] = [
    ["c1", CENTRAL_RADIUS_MM],
    ["c3", INNER_RADIUS_MM],
    ["c6", OUTER_RADIUS_MM],
  ];
  const marks: AnnotationMark[] = circles.map(([suffix, radiusMm]) =>
    displayMark(
      `${prefix}-${suffix}`,
      closedCircle(centre, radiusMm / mmPerPixel),
      null,
    ),
  );
  const axes: readonly (readonly [-1 | 0 | 1, -1 | 0 | 1])[] = [
    [1, 0],
    [0, 1],
    [-1, 0],
    [0, -1],
  ];
  for (let index = 0; index < axes.length; index += 1) {
    const axis = axes[index];
    if (axis === undefined) {
      return refused("degenerate");
    }
    marks.push(
      displayMark(`${prefix}-axis-${String(index)}`, spoke(centre, innerPx, outerPx, axis[0], axis[1]), null),
    );
  }
  return { ok: true, marks };
}
