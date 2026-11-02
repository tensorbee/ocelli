import { describe, expect, it } from "vitest";

import { contentTree, provenanceItems, type ContentItem } from "./sr.js";

const length: ContentItem["concept"] = { scheme: "SCT", value: "410668003", meaning: "Length" };

function num(number: number): ContentItem {
  return {
    relationship: "CONTAINS",
    valueType: "NUM",
    concept: length,
    number,
    unit: "mm",
    children: [],
  };
}

describe("SR content tree", () => {
  it("copies a container and its numeric child", () => {
    const child = num(12.5);
    const root: ContentItem = {
      relationship: null,
      valueType: "CONTAINER",
      concept: { scheme: "DCM", value: "126000", meaning: "Imaging Measurement Report" },
      children: [child],
    };
    const copied = contentTree(root);
    expect(copied?.valueType).toBe("CONTAINER");
    expect(copied?.children).toEqual([child]);
    expect(copied?.children[0]).not.toBe(child);
    expect(copied).not.toBe(root);
  });

  it("keeps one 2D point", () => {
    const root: ContentItem = {
      relationship: null,
      valueType: "SCOORD",
      concept: length,
      graphic: "POINT",
      points: [[1, 2]],
      children: [],
    };
    expect(contentTree(root)).toEqual(root);
  });

  it("refuses a short circle, a non-finite number, and an empty concept", () => {
    const circle: ContentItem = {
      relationship: null,
      valueType: "SCOORD",
      concept: length,
      graphic: "CIRCLE",
      points: [[1, 2]],
      children: [],
    };
    expect(contentTree(circle)).toBeNull();
    expect(contentTree({ ...num(Number.NaN), relationship: null })).toBeNull();
    const blank: ContentItem = {
      relationship: null,
      valueType: "CONTAINER",
      concept: { scheme: "DCM", value: "", meaning: "Report" },
      children: [],
    };
    expect(contentTree(blank)).toBeNull();
  });

  it("refuses a value type outside the eight", () => {
    const foreign = {
      relationship: "CONTAINS",
      valueType: "DATE",
      concept: length,
      graphic: "POINT",
      points: [[1, 2, 3]],
      frameOfReference: "1.2.3",
      children: [],
    };
    const root: ContentItem = {
      relationship: null,
      valueType: "CONTAINER",
      concept: length,
      children: [foreign as ContentItem],
    };
    expect(contentTree(root)).toBeNull();
    expect(
      contentTree({
        relationship: null,
        valueType: "DATE",
        concept: length,
        children: [],
      } as unknown as ContentItem),
    ).toBeNull();
  });

  it("omits a null algorithm and refuses an empty tool", () => {
    const items = provenanceItems("Length", "0.1.0", null, "reader");
    expect(items).toHaveLength(3);
    expect(items?.map((item) => item.valueType === "TEXT" ? item.text : "")).toEqual([
      "Length",
      "0.1.0",
      "reader",
    ]);
    expect(provenanceItems("", "0.1.0", null, "reader")).toBeNull();
  });

  it("copies each provenance concept", () => {
    const first = provenanceItems("Length", "0.1.0", null, "reader");
    const second = provenanceItems("Length", "0.1.0", null, "reader");
    const firstTool = first?.[0];
    const secondTool = second?.[0];
    expect(firstTool?.concept).not.toBe(secondTool?.concept);
    if (firstTool !== undefined) {
      firstTool.concept.scheme = "MUTATED";
    }
    const third = provenanceItems("Length", "0.1.0", null, "reader");
    expect(third?.[0]?.concept.scheme).toBe("OCELLI");
  });
});
