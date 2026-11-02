/**
 * @vitest-environment jsdom
 */
import { describe, expect, it } from "vitest";

import { createAnnotationLayer } from "./annotation.js";
import {
  MARK_REFUSALS,
  arrowMark,
  etdrsMarks,
  etdrsRegion,
  keyImageMark,
  labelMark,
} from "./index.js";

const POSITIVE_BARB = { x: 7.834936490538903, y: -1.2499999999999998 };
const NEGATIVE_BARB = { x: 7.834936490538903, y: 1.2499999999999998 };

describe("display marks", () => {
  it("the_horizontal_arrow_barbs_are_hand_computed", () => {
    const built = arrowMark("arrow", { x: 0, y: 0 }, { x: 10, y: 0 }, null);
    expect(built).toEqual({
      ok: true,
      mark: {
        id: "arrow",
        points: [
          { x: 0, y: 0 },
          { x: 10, y: 0 },
          POSITIVE_BARB,
          { x: 10, y: 0 },
          NEGATIVE_BARB,
        ],
        text: null,
        visible: true,
        locked: false,
        selected: false,
        completed: true,
      },
    });
    expect(arrowMark(" ", { x: 0, y: 0 }, { x: Number.NaN, y: 0 }, " ")).toEqual({
      ok: false,
      refusal: "blank-id",
    });
    expect(arrowMark("arrow", { x: 0, y: 0 }, { x: Number.NaN, y: 0 }, " ")).toEqual({
      ok: false,
      refusal: "blank-text",
    });
    expect(arrowMark("arrow", { x: 0, y: 0 }, { x: Number.NaN, y: 0 }, null)).toEqual({
      ok: false,
      refusal: "frame",
    });
  });

  it("a_zero_shaft_is_degenerate", () => {
    expect(arrowMark("arrow", { x: 1, y: 2 }, { x: 1, y: 2 }, null)).toEqual({
      ok: false,
      refusal: "degenerate",
    });
  });

  it("a_blank_label_is_refused", () => {
    expect(labelMark(" ", { x: Number.NaN, y: 2 }, " ")).toEqual({
      ok: false,
      refusal: "blank-id",
    });
    expect(labelMark("label", { x: Number.NaN, y: 2 }, " ")).toEqual({
      ok: false,
      refusal: "blank-text",
    });
    expect(labelMark("label", { x: Number.NaN, y: 2 }, "mm")).toEqual({
      ok: false,
      refusal: "frame",
    });
    expect(labelMark("label", { x: 1, y: 2 }, "mm")).toEqual({
      ok: true,
      mark: {
        id: "label",
        points: [{ x: 1, y: 2 }],
        text: "mm",
        visible: true,
        locked: false,
        selected: false,
        completed: true,
      },
    });
  });

  it("a_key_image_is_the_text_key", () => {
    const first = keyImageMark("key", { x: 4, y: 5 });
    const second = keyImageMark("key", { x: 4, y: 5 });
    expect(first).toEqual(second);
    expect(first).toEqual({
      ok: true,
      mark: {
        id: "key",
        points: [{ x: 4, y: 5 }],
        text: "KEY",
        visible: true,
        locked: false,
        selected: false,
        completed: true,
      },
    });
    expect(keyImageMark(" ", { x: 4, y: 5 })).toEqual({ ok: false, refusal: "blank-id" });
    expect(keyImageMark("key", { x: Number.NaN, y: 5 })).toEqual({
      ok: false,
      refusal: "frame",
    });
  });

  it("the_etdrs_regions_follow_the_radii", () => {
    expect(etdrsRegion(0, 0, true)).toBe("central");
    expect(etdrsRegion(0, 0, false)).toBe("central");
    expect(etdrsRegion(1, 1, true)).toBe("inner superior");
    expect(etdrsRegion(1, 1, false)).toBe("inner superior");
    expect(etdrsRegion(0, 0.5, true)).toBe("inner superior");
    expect(etdrsRegion(0, 1.5, true)).toBe("outer superior");
    expect(etdrsRegion(1, 0, true)).toBe("inner temporal");
    expect(etdrsRegion(1, 0, false)).toBe("inner nasal");
    expect(etdrsRegion(Number.NaN, 0, true)).toBe("frame");
  });

  it("outside_the_outer_radius_is_outside", () => {
    expect(etdrsRegion(4, 0, true)).toBe("outside");
    expect(etdrsRegion(0, 3, false)).toBe("outside");
  });

  it("the_outer_circle_starts_on_the_axis", () => {
    const centre = { x: 10, y: 20 };
    const drawn = etdrsMarks("grid", centre, 0.5, true);
    const flipped = etdrsMarks("grid", centre, 0.5, false);
    expect(drawn.ok).toBe(true);
    expect(flipped).toEqual(drawn);
    if (!drawn.ok) {
      return;
    }
    expect(drawn.marks.map((mark) => mark.id)).toEqual([
      "grid-c1",
      "grid-c3",
      "grid-c6",
      "grid-axis-0",
      "grid-axis-1",
      "grid-axis-2",
      "grid-axis-3",
    ]);
    const outer = drawn.marks[2];
    expect(outer).toBeDefined();
    if (outer === undefined) {
      return;
    }
    expect(outer.points).toHaveLength(33);
    expect(outer.points[0]).toEqual({ x: 16, y: 20 });
    expect(outer.points[32]).toEqual({ x: 16, y: 20 });
    expect(drawn.marks[3]?.points).toEqual([
      { x: 11, y: 20 },
      { x: 16, y: 20 },
    ]);
    expect(drawn.marks[4]?.points).toEqual([
      { x: 10, y: 21 },
      { x: 10, y: 26 },
    ]);
    expect(etdrsMarks(" ", centre, Number.NaN, true)).toEqual({
      ok: false,
      refusal: "blank-id",
    });
    expect(etdrsMarks("grid", { x: Number.NaN, y: 20 }, 0.5, true)).toEqual({
      ok: false,
      refusal: "frame",
    });
    expect(etdrsMarks("grid", centre, 0, true)).toEqual({
      ok: false,
      refusal: "degenerate",
    });
  });

  it("the_layer_draws_the_arrow_polyline", () => {
    const built = arrowMark("arrow", { x: 0, y: 0 }, { x: 10, y: 0 }, null);
    expect(built.ok).toBe(true);
    if (!built.ok) {
      return;
    }
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    const synced = createAnnotationLayer(svg, () => undefined).sync([built.mark]);
    expect(synced).toEqual({ ok: true });
    expect(svg.querySelector("polyline")?.getAttribute("points")).toBe(
      "0,0 10,0 7.834936490538903,-1.2499999999999998 10,0 7.834936490538903,1.2499999999999998",
    );
  });

  it("refusals stay in check order", () => {
    expect(MARK_REFUSALS).toEqual(["blank-id", "blank-text", "frame", "degenerate"]);
  });
});
