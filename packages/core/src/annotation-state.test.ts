import { describe, expect, it } from "vitest";

import {
  annotationsForFrame,
  createAnnotationState,
  putAnnotation,
  redoAnnotation,
  undoAnnotation,
  type AnnotationState,
} from "./annotation-state.js";
import type { ContentItem } from "./sr.js";

const length: ContentItem["concept"] = {
  scheme: "SCT",
  value: "410668003",
  meaning: "Length",
};

function report(number: number): ContentItem {
  return {
    relationship: null,
    valueType: "CONTAINER",
    concept: { scheme: "DCM", value: "126000", meaning: "Imaging Measurement Report" },
    children: [
      {
        relationship: "CONTAINS",
        valueType: "NUM",
        concept: length,
        number,
        unit: "mm",
        children: [],
      },
    ],
  };
}

function storedNumber(state: AnnotationState, id: string): number | null {
  const record = state.records.find((item) => item.id === id);
  const child = record?.tree.children[0];
  if (child?.valueType !== "NUM") {
    return null;
  }
  return child.number;
}

function put(
  state: AnnotationState,
  id: string,
  frameOfReference: string,
  number: number,
): AnnotationState {
  const next = putAnnotation(state, id, frameOfReference, report(number));
  expect(next).not.toBeNull();
  if (next === null) {
    throw new Error("put");
  }
  return next;
}

describe("annotation state", () => {
  it("lists records for one frame of reference in insertion order", () => {
    let state = createAnnotationState();
    state = put(state, "a", "1.2.3", 1);
    state = put(state, "b", "1.2.3", 2);
    state = put(state, "c", "1.2.4", 3);
    expect(annotationsForFrame(state, "1.2.3")?.map((record) => record.id)).toEqual(["a", "b"]);
    expect(annotationsForFrame(state, "1.2.4")?.map((record) => record.id)).toEqual(["c"]);
    const listed = annotationsForFrame(state, "1.2.3");
    expect(listed?.[0]).not.toBe(state.records[0]);
  });

  it("replaces the same id and keeps the new number", () => {
    let state = createAnnotationState();
    state = put(state, "a", "1.2.3", 1);
    state = put(state, "a", "1.2.3", 4);
    const listed = annotationsForFrame(state, "1.2.3");
    expect(listed).toHaveLength(1);
    expect(storedNumber(state, "a")).toBe(4);
  });

  it("undoes two puts back to empty and then refuses", () => {
    let state = createAnnotationState();
    expect(undoAnnotation(state)).toBeNull();
    state = put(state, "a", "1.2.3", 1);
    state = put(state, "b", "1.2.3", 2);
    const once = undoAnnotation(state);
    expect(once?.records.map((record) => record.id)).toEqual(["a"]);
    const twice = once === null ? null : undoAnnotation(once);
    expect(twice?.records).toEqual([]);
    expect(twice === null ? null : undoAnnotation(twice)).toBeNull();
  });

  it("redoes one undo and drops redo after a new put", () => {
    let state = createAnnotationState();
    state = put(state, "a", "1.2.3", 1);
    state = put(state, "b", "1.2.3", 2);
    const undone = undoAnnotation(state);
    expect(undone).not.toBeNull();
    if (undone === null) {
      return;
    }
    const redone = redoAnnotation(undone);
    expect(redone?.records.map((record) => record.id)).toEqual(["a", "b"]);
    const replaced = put(undone, "c", "1.2.3", 5);
    expect(redoAnnotation(replaced)).toBeNull();
  });

  it("refuses a blank id and a tree the content tree refuses", () => {
    const state = createAnnotationState();
    const records = state.records;
    expect(putAnnotation(state, "", "1.2.3", report(1))).toBeNull();
    expect(putAnnotation(state, "a", "", report(1))).toBeNull();
    const refused: ContentItem = {
      relationship: "CONTAINS",
      valueType: "CONTAINER",
      concept: length,
      children: [],
    };
    expect(putAnnotation(state, "a", "1.2.3", refused)).toBeNull();
    expect(state.records).toBe(records);
    expect(state.undo).toHaveLength(0);
    const input = report(1);
    const stored = putAnnotation(state, "a", "1.2.3", input);
    expect(stored?.records[0]?.tree).not.toBe(input);
    expect(state.records).toBe(records);
  });
});
