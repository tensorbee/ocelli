/**
 * @vitest-environment jsdom
 */
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { createAnnotationLayer } from "./annotation.js";
import { ANNOTATION_REFUSALS, createAnnotationLayer as publishedLayer } from "./index.js";
import type {
  AnnotationLayer,
  AnnotationLayerEvent,
  AnnotationMark,
  AnnotationPoint,
  AnnotationRefusal,
  AnnotationSyncResult,
} from "./index.js";

const HERE = dirname(fileURLToPath(import.meta.url));

function publishedTypes(
  layer: AnnotationLayer,
  event: AnnotationLayerEvent,
  mark: AnnotationMark,
  point: AnnotationPoint,
  refusal: AnnotationRefusal,
  result: AnnotationSyncResult,
): void {
  void layer;
  void event;
  void mark;
  void point;
  void refusal;
  void result;
}

function mark(overrides: Partial<AnnotationMark> = {}): AnnotationMark {
  return {
    id: "a",
    points: [
      { x: 1, y: 2 },
      { x: 3, y: 4 },
    ],
    text: null,
    visible: true,
    locked: false,
    selected: false,
    completed: false,
    ...overrides,
  };
}

function layer(): {
  svg: SVGSVGElement;
  events: AnnotationLayerEvent[];
  sync: AnnotationLayer["sync"];
} {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  const events: AnnotationLayerEvent[] = [];
  const drawing = createAnnotationLayer(svg, (event) => {
    events.push(event);
  });
  return { svg, events, sync: drawing.sync.bind(drawing) };
}

function group(svg: SVGSVGElement, id: string): Element | null {
  return svg.querySelector(`[data-ocelli-id="${id}"]`);
}

describe("annotation layer", () => {
  it("a new mark is added and drawn", () => {
    const drawing = layer();
    const result = drawing.sync([mark()]);
    expect(result).toEqual({ ok: true });
    expect(drawing.events).toEqual([
      { name: "annotation.added", id: "a" },
      { name: "annotation.rendered", ids: ["a"] },
    ]);
    const node = group(drawing.svg, "a");
    expect(node?.getAttribute("data-selected")).toBe("false");
    expect(node?.getAttribute("data-locked")).toBe("false");
    expect(node?.getAttribute("display")).toBeNull();
    expect(node?.querySelector("polyline")?.getAttribute("points")).toBe("1,2 3,4");
    expect(node?.querySelector("text")).toBeNull();
  });

  it("a new completed mark completes", () => {
    const drawing = layer();
    drawing.sync([mark({ completed: true })]);
    expect(drawing.events.map((event) => event.name)).toEqual([
      "annotation.added",
      "annotation.completed",
      "annotation.rendered",
    ]);
  });

  it("the initial flags are not changes", () => {
    const drawing = layer();
    drawing.sync([
      mark({ visible: false, locked: true, selected: true, completed: false }),
    ]);
    expect(drawing.events.map((event) => event.name)).toEqual([
      "annotation.added",
      "annotation.rendered",
    ]);
  });

  it("a point change is modified", () => {
    const drawing = layer();
    drawing.sync([mark()]);
    drawing.events.length = 0;
    drawing.sync([mark({ points: [{ x: 8, y: 9 }] })]);
    expect(drawing.events).toEqual([
      { name: "annotation.modified", id: "a" },
      { name: "annotation.rendered", ids: ["a"] },
    ]);
    expect(group(drawing.svg, "a")?.querySelector("polyline")?.getAttribute("points")).toBe(
      "8,9",
    );
  });

  it("a text change is modified", () => {
    const drawing = layer();
    drawing.sync([mark()]);
    drawing.events.length = 0;
    drawing.sync([mark({ text: "mm" })]);
    expect(drawing.events[0]).toEqual({ name: "annotation.modified", id: "a" });
    const text = group(drawing.svg, "a")?.querySelector("text");
    expect(text?.textContent).toBe("mm");
    expect(text?.getAttribute("x")).toBe("3");
    expect(text?.getAttribute("y")).toBe("4");
    const children = group(drawing.svg, "a")?.children;
    expect(children?.[0]?.localName).toBe("polyline");
    expect(children?.[1]?.localName).toBe("text");
  });

  it("an empty label is not a missing label", () => {
    const drawing = layer();
    drawing.sync([mark()]);
    drawing.events.length = 0;
    drawing.sync([mark({ text: "" })]);
    expect(drawing.events[0]).toEqual({ name: "annotation.modified", id: "a" });
    expect(group(drawing.svg, "a")?.querySelector("text")?.textContent).toBe("");
  });

  it("completion is a rising edge", () => {
    const drawing = layer();
    drawing.sync([mark()]);
    drawing.events.length = 0;
    drawing.sync([mark({ completed: true })]);
    expect(drawing.events.map((event) => event.name)).toEqual([
      "annotation.completed",
      "annotation.rendered",
    ]);
    drawing.events.length = 0;
    drawing.sync([mark({ completed: true })]);
    expect(drawing.events.map((event) => event.name)).toEqual(["annotation.rendered"]);
  });

  it("leaving completed is not an event", () => {
    const drawing = layer();
    drawing.sync([mark({ completed: true })]);
    drawing.events.length = 0;
    drawing.sync([mark({ completed: false })]);
    expect(drawing.events.map((event) => event.name)).toEqual(["annotation.rendered"]);
  });

  it("a hidden mark stays in the svg", () => {
    const drawing = layer();
    drawing.sync([mark()]);
    drawing.events.length = 0;
    drawing.sync([mark({ visible: false })]);
    expect(drawing.events).toEqual([
      { name: "annotation.visibilityChanged", id: "a", visible: false },
      { name: "annotation.rendered", ids: ["a"] },
    ]);
    const node = group(drawing.svg, "a");
    expect(node?.getAttribute("display")).toBe("none");
    expect(node?.querySelector("polyline")).not.toBeNull();
    drawing.events.length = 0;
    drawing.sync([mark({ visible: true })]);
    expect(drawing.events[0]).toEqual({
      name: "annotation.visibilityChanged",
      id: "a",
      visible: true,
    });
    expect(group(drawing.svg, "a")?.getAttribute("display")).toBeNull();
  });

  it("lock and selection are separate", () => {
    const drawing = layer();
    drawing.sync([mark()]);
    drawing.events.length = 0;
    drawing.sync([mark({ locked: true, selected: true })]);
    expect(drawing.events).toEqual([
      { name: "annotation.locked", id: "a", locked: true },
      { name: "annotation.selectionChanged", id: "a", selected: true },
      { name: "annotation.rendered", ids: ["a"] },
    ]);
    expect(group(drawing.svg, "a")?.getAttribute("data-locked")).toBe("true");
    expect(group(drawing.svg, "a")?.getAttribute("data-selected")).toBe("true");
    drawing.events.length = 0;
    drawing.sync([mark({ locked: false, selected: false })]);
    expect(drawing.events).toEqual([
      { name: "annotation.locked", id: "a", locked: false },
      { name: "annotation.selectionChanged", id: "a", selected: false },
      { name: "annotation.rendered", ids: ["a"] },
    ]);
  });

  it("one id reports every change in order", () => {
    const drawing = layer();
    drawing.sync([mark({ points: [{ x: 0, y: 0 }] })]);
    drawing.events.length = 0;
    drawing.sync([
      mark({
        points: [{ x: 1, y: 0 }],
        text: "n",
        completed: true,
        visible: false,
        locked: true,
        selected: true,
      }),
    ]);
    expect(drawing.events).toEqual([
      { name: "annotation.modified", id: "a" },
      { name: "annotation.completed", id: "a" },
      { name: "annotation.visibilityChanged", id: "a", visible: false },
      { name: "annotation.locked", id: "a", locked: true },
      { name: "annotation.selectionChanged", id: "a", selected: true },
      { name: "annotation.rendered", ids: ["a"] },
    ]);
  });

  it("removal keeps the previous order", () => {
    const drawing = layer();
    drawing.sync([mark({ id: "b" }), mark({ id: "a" })]);
    drawing.events.length = 0;
    drawing.sync([]);
    expect(drawing.events).toEqual([
      { name: "annotation.removed", id: "b" },
      { name: "annotation.removed", id: "a" },
      { name: "annotation.rendered", ids: [] },
    ]);
    expect(drawing.svg.childElementCount).toBe(0);
  });

  it("an unchanged id is silent", () => {
    const drawing = layer();
    drawing.sync([mark({ id: "a" }), mark({ id: "b" })]);
    drawing.events.length = 0;
    drawing.sync([mark({ id: "b", points: [{ x: 9, y: 9 }] }), mark({ id: "c" })]);
    expect(drawing.events).toEqual([
      { name: "annotation.removed", id: "a" },
      { name: "annotation.modified", id: "b" },
      { name: "annotation.added", id: "c" },
      { name: "annotation.rendered", ids: ["b", "c"] },
    ]);
  });

  it("an identical sync still draws", () => {
    const drawing = layer();
    drawing.sync([mark()]);
    const node = group(drawing.svg, "a");
    node?.querySelector("polyline")?.setAttribute("points", "9,9");
    drawing.events.length = 0;
    drawing.sync([mark()]);
    expect(drawing.events).toEqual([{ name: "annotation.rendered", ids: ["a"] }]);
    expect(group(drawing.svg, "a")?.querySelector("polyline")?.getAttribute("points")).toBe(
      "1,2 3,4",
    );
  });

  it("a refusal is silent", () => {
    const drawing = layer();
    drawing.sync([mark()]);
    const before = drawing.svg.innerHTML;
    drawing.events.length = 0;
    const result = drawing.sync([mark(), mark()]);
    expect(result).toEqual({ ok: false, refusal: "duplicate-id" });
    expect(drawing.events).toEqual([]);
    expect(drawing.svg.innerHTML).toBe(before);
  });

  it("the first bad mark wins", () => {
    const drawing = layer();
    const blank = drawing.sync([
      mark(),
      mark({ id: " ", points: [{ x: Number.NaN, y: 1 }] }),
      mark(),
    ]);
    expect(blank).toEqual({ ok: false, refusal: "blank-id" });
    const nonFinite = drawing.sync([
      mark({ points: [{ x: Number.POSITIVE_INFINITY, y: 1 }] }),
      mark(),
    ]);
    expect(nonFinite).toEqual({ ok: false, refusal: "non-finite" });
    const finiteDuplicate = drawing.sync([mark({ id: "a" }), mark({ id: "a" })]);
    expect(finiteDuplicate).toEqual({ ok: false, refusal: "duplicate-id" });
    const nanDuplicate = drawing.sync([
      mark({ id: "a" }),
      mark({ id: "a", points: [{ x: Number.NaN, y: 1 }] }),
    ]);
    expect(nanDuplicate).toEqual({ ok: false, refusal: "non-finite" });
    expect(drawing.events).toEqual([]);
    expect(drawing.svg.childElementCount).toBe(0);
  });

  it("a blank id is whitespace and a spaced id is kept", () => {
    const drawing = layer();
    expect(drawing.sync([mark({ id: "\t" })])).toEqual({ ok: false, refusal: "blank-id" });
    drawing.sync([mark({ id: " a" })]);
    expect(group(drawing.svg, " a")).not.toBeNull();
  });

  it("a structural mistake is bad-mark", () => {
    const drawing = layer();
    expect(drawing.sync([{ ...mark(), id: 1 } as unknown as AnnotationMark])).toEqual({
      ok: false,
      refusal: "bad-mark",
    });
    expect(
      drawing.sync([mark({ text: "mm", points: [] })]),
    ).toEqual({ ok: false, refusal: "bad-mark" });
    expect(drawing.sync(null as unknown as AnnotationMark[])).toEqual({
      ok: false,
      refusal: "bad-mark",
    });
  });

  it("structure finishes before blank and before non-finite", () => {
    const drawing = layer();
    const spaced = drawing.sync([
      mark({ id: " ", points: [{ x: "1" as unknown as number, y: 2 }] }),
    ]);
    expect(spaced).toEqual({ ok: false, refusal: "bad-mark" });
    const later = drawing.sync([
      mark({
        points: [
          { x: Number.POSITIVE_INFINITY, y: 1 },
          { x: "bad" as unknown as number, y: 1 },
        ],
      }),
    ]);
    expect(later).toEqual({ ok: false, refusal: "bad-mark" });
    const labelled = drawing.sync([mark({ id: "\n", points: [], text: "mm" })]);
    expect(labelled).toEqual({ ok: false, refusal: "bad-mark" });
    const blankNan = drawing.sync([
      mark({ id: " ", points: [{ x: Number.NaN, y: 1 }] }),
    ]);
    expect(blankNan).toEqual({ ok: false, refusal: "blank-id" });
    expect(drawing.events).toEqual([]);
    expect(drawing.svg.childElementCount).toBe(0);
  });

  it("an empty mark draws a group with no polyline", () => {
    const drawing = layer();
    drawing.sync([mark({ points: [], text: null })]);
    const node = group(drawing.svg, "a");
    expect(node).not.toBeNull();
    expect(node?.querySelector("polyline")).toBeNull();
    expect(node?.querySelector("text")).toBeNull();
  });

  it("the layer keeps its own copy", () => {
    const drawing = layer();
    const points = [{ x: 1, y: 2 }];
    drawing.sync([mark({ points })]);
    const first = points[0];
    if (first === undefined) {
      throw new Error("missing point");
    }
    first.x = 99;
    drawing.events.length = 0;
    drawing.sync([mark({ points: [{ x: 1, y: 2 }] })]);
    expect(drawing.events.map((event) => event.name)).toEqual(["annotation.rendered"]);
  });

  it("a listener throw leaves the svg written", () => {
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    let fail = true;
    const events: AnnotationLayerEvent[] = [];
    const drawing = createAnnotationLayer(svg, (event) => {
      events.push(event);
      if (fail) {
        throw new Error("listener");
      }
    });
    expect(() => drawing.sync([mark()])).toThrow("listener");
    expect(group(svg, "a")).not.toBeNull();
    fail = false;
    events.length = 0;
    drawing.sync([]);
    expect(events[0]).toEqual({ name: "annotation.removed", id: "a" });
  });

  it("an svg with no document is refused", () => {
    const events: AnnotationLayerEvent[] = [];
    const host = {
      ownerDocument: null,
      firstChild: null,
      removeChild(): never {
        throw new Error("removed");
      },
      appendChild(): never {
        throw new Error("appended");
      },
    };
    const drawing = createAnnotationLayer(host as unknown as SVGSVGElement, (event) => {
      events.push(event);
    });
    expect(drawing.sync([mark()])).toEqual({ ok: false, refusal: "bad-mark" });
    expect(events).toEqual([]);
  });

  it("the annotation module does not listen", () => {
    const source = readFileSync(join(HERE, "annotation.ts"), "utf8");
    expect(source.includes("addEventListener")).toBe(false);
    expect(source.includes("pointer.ts")).toBe(false);
    expect(source.includes("pointer.js")).toBe(false);
    expect(ANNOTATION_REFUSALS).toEqual([
      "bad-mark",
      "blank-id",
      "non-finite",
      "duplicate-id",
    ]);
    const sample = mark();
    const point = sample.points[0];
    if (point === undefined) {
      throw new Error("missing point");
    }
    publishedTypes(
      publishedLayer(document.createElementNS("http://www.w3.org/2000/svg", "svg"), () => {
        return undefined;
      }),
      { name: "annotation.rendered", ids: [] },
      sample,
      point,
      "bad-mark",
      { ok: true },
    );
  });
});
