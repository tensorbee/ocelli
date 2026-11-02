/**
 * SVG annotation drawing layer.
 *
 * HLD section 10 keeps this layer in the shell. It draws a caller-supplied
 * display list into one SVG element and emits the F-103 annotation names.
 * The list is not the SR content tree. F-094 owns that type. This module
 * does not listen and does not project world points.
 *
 * The display list and the emission order are the local decisions in
 * `.claude/plans/F-071-design.md`.
 */

const SVG_NS = "http://www.w3.org/2000/svg";

/** Refusals `sync` can return. The first failing check in list order wins. */
export const ANNOTATION_REFUSALS = [
  "bad-mark",
  "blank-id",
  "non-finite",
  "duplicate-id",
] as const;

/** One refusal from `ANNOTATION_REFUSALS`. */
export type AnnotationRefusal = (typeof ANNOTATION_REFUSALS)[number];

/** One canvas pixel. This module does not convert spaces. */
export interface AnnotationPoint {
  readonly x: number;
  readonly y: number;
}

/**
 * One display-list mark. Canvas pixels, not an SR content item.
 * `text` null means no label.
 */
export interface AnnotationMark {
  readonly id: string;
  readonly points: readonly AnnotationPoint[];
  readonly text: string | null;
  readonly visible: boolean;
  readonly locked: boolean;
  readonly selected: boolean;
  readonly completed: boolean;
}

/** Shell events this layer produces. Names match `events.ts`. */
export type AnnotationLayerEvent =
  | { readonly name: "annotation.added"; readonly id: string }
  | { readonly name: "annotation.removed"; readonly id: string }
  | { readonly name: "annotation.modified"; readonly id: string }
  | { readonly name: "annotation.completed"; readonly id: string }
  | {
      readonly name: "annotation.visibilityChanged";
      readonly id: string;
      readonly visible: boolean;
    }
  | {
      readonly name: "annotation.locked";
      readonly id: string;
      readonly locked: boolean;
    }
  | {
      readonly name: "annotation.selectionChanged";
      readonly id: string;
      readonly selected: boolean;
    }
  | { readonly name: "annotation.rendered"; readonly ids: readonly string[] };

/** Result of one `sync`. A refusal leaves the previous SVG in place. */
export type AnnotationSyncResult =
  | { readonly ok: true }
  | { readonly ok: false; readonly refusal: AnnotationRefusal };

/** The drawing layer for one SVG element. */
export interface AnnotationLayer {
  sync(marks: readonly AnnotationMark[]): AnnotationSyncResult;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function pointIsNumberPair(value: unknown): boolean {
  if (!isRecord(value)) {
    return false;
  }
  return typeof value["x"] === "number" && typeof value["y"] === "number";
}

function pointIsFinite(value: unknown): boolean {
  if (!isRecord(value)) {
    return false;
  }
  const x = value["x"];
  const y = value["y"];
  return (
    typeof x === "number" &&
    typeof y === "number" &&
    Number.isFinite(x) &&
    Number.isFinite(y)
  );
}

function refusalFor(mark: unknown, seen: Set<string>): AnnotationRefusal | null {
  if (!isRecord(mark) || !Array.isArray(mark["points"])) {
    return "bad-mark";
  }
  const id = mark["id"];
  const text = mark["text"];
  const visible = mark["visible"];
  const locked = mark["locked"];
  const selected = mark["selected"];
  const completed = mark["completed"];
  const points: unknown[] = mark["points"];
  if (typeof id !== "string") {
    return "bad-mark";
  }
  if (text !== null && typeof text !== "string") {
    return "bad-mark";
  }
  if (
    typeof visible !== "boolean" ||
    typeof locked !== "boolean" ||
    typeof selected !== "boolean" ||
    typeof completed !== "boolean"
  ) {
    return "bad-mark";
  }
  for (const point of points) {
    if (!pointIsNumberPair(point)) {
      return "bad-mark";
    }
  }
  if (text !== null && points.length === 0) {
    return "bad-mark";
  }
  if (id.trim().length === 0) {
    return "blank-id";
  }
  for (const point of points) {
    if (!pointIsFinite(point)) {
      return "non-finite";
    }
  }
  if (seen.has(id)) {
    return "duplicate-id";
  }
  seen.add(id);
  return null;
}

function copyMark(mark: AnnotationMark): AnnotationMark {
  return {
    id: mark.id,
    points: mark.points.map((point) => ({ x: point.x, y: point.y })),
    text: mark.text,
    visible: mark.visible,
    locked: mark.locked,
    selected: mark.selected,
    completed: mark.completed,
  };
}

function geometryChanged(prior: AnnotationMark, next: AnnotationMark): boolean {
  if (prior.text !== next.text) {
    return true;
  }
  if (prior.points.length !== next.points.length) {
    return true;
  }
  for (let index = 0; index < prior.points.length; index += 1) {
    const left = prior.points[index];
    const right = next.points[index];
    if (left === undefined || right === undefined) {
      return true;
    }
    if (left.x !== right.x || left.y !== right.y) {
      return true;
    }
  }
  return false;
}

function diff(
  previous: readonly AnnotationMark[],
  next: readonly AnnotationMark[],
): AnnotationLayerEvent[] {
  const events: AnnotationLayerEvent[] = [];
  const nextIds = new Set(next.map((mark) => mark.id));
  for (const mark of previous) {
    if (!nextIds.has(mark.id)) {
      events.push({ name: "annotation.removed", id: mark.id });
    }
  }
  const previousById = new Map(previous.map((mark) => [mark.id, mark]));
  for (const mark of next) {
    const prior = previousById.get(mark.id);
    if (prior === undefined) {
      events.push({ name: "annotation.added", id: mark.id });
      if (mark.completed) {
        events.push({ name: "annotation.completed", id: mark.id });
      }
      continue;
    }
    if (geometryChanged(prior, mark)) {
      events.push({ name: "annotation.modified", id: mark.id });
    }
    if (!prior.completed && mark.completed) {
      events.push({ name: "annotation.completed", id: mark.id });
    }
    if (prior.visible !== mark.visible) {
      events.push({
        name: "annotation.visibilityChanged",
        id: mark.id,
        visible: mark.visible,
      });
    }
    if (prior.locked !== mark.locked) {
      events.push({ name: "annotation.locked", id: mark.id, locked: mark.locked });
    }
    if (prior.selected !== mark.selected) {
      events.push({
        name: "annotation.selectionChanged",
        id: mark.id,
        selected: mark.selected,
      });
    }
  }
  events.push({ name: "annotation.rendered", ids: next.map((mark) => mark.id) });
  return events;
}

function pointsAttribute(points: readonly AnnotationPoint[]): string {
  return points.map((point) => `${String(point.x)},${String(point.y)}`).join(" ");
}

function writeSvg(
  svg: SVGSVGElement,
  owner: Document,
  marks: readonly AnnotationMark[],
): void {
  while (svg.firstChild !== null) {
    svg.removeChild(svg.firstChild);
  }
  for (const mark of marks) {
    const group = owner.createElementNS(SVG_NS, "g");
    group.setAttribute("data-ocelli-id", mark.id);
    group.setAttribute("data-selected", mark.selected ? "true" : "false");
    group.setAttribute("data-locked", mark.locked ? "true" : "false");
    if (!mark.visible) {
      group.setAttribute("display", "none");
    }
    if (mark.points.length > 0) {
      const polyline = owner.createElementNS(SVG_NS, "polyline");
      polyline.setAttribute("points", pointsAttribute(mark.points));
      group.appendChild(polyline);
    }
    const last = mark.points[mark.points.length - 1];
    if (mark.text !== null && last !== undefined) {
      const text = owner.createElementNS(SVG_NS, "text");
      text.setAttribute("x", String(last.x));
      text.setAttribute("y", String(last.y));
      text.textContent = mark.text;
      group.appendChild(text);
    }
    svg.appendChild(group);
  }
}

/**
 * Bind one SVG element. `sync` replaces that element's children from the
 * display list and then emits. A refusal leaves the element alone.
 */
export function createAnnotationLayer(
  svg: SVGSVGElement,
  listener: (event: AnnotationLayerEvent) => void,
): AnnotationLayer {
  let previous: AnnotationMark[] = [];
  return {
    sync(marks: readonly AnnotationMark[]): AnnotationSyncResult {
      const owner = svg.ownerDocument;
      if (!Array.isArray(marks) || owner === null) {
        return { ok: false, refusal: "bad-mark" };
      }
      const seen = new Set<string>();
      for (const mark of marks) {
        const refusal = refusalFor(mark, seen);
        if (refusal !== null) {
          return { ok: false, refusal };
        }
      }
      const next = marks.map((mark) => copyMark(mark));
      const events = diff(previous, next);
      writeSvg(svg, owner, next);
      previous = next;
      for (const event of events) {
        listener(event);
      }
      return { ok: true };
    },
  };
}
