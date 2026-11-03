import { describe, expect, it } from "vitest";

import { loadMeasurements, persistMeasurements } from "./measurement-persist.js";
import type { ContentItem } from "./sr.js";

const concept: ContentItem["concept"] = {
  scheme: "SCT",
  value: "410668003",
  meaning: "Length",
};

function textItem(label: string, root: boolean): ContentItem {
  return {
    relationship: root ? null : "CONTAINS",
    valueType: "TEXT",
    concept,
    text: label,
    children: [],
  };
}

function container(childCount: number): ContentItem {
  return {
    relationship: null,
    valueType: "CONTAINER",
    concept,
    children: Array.from({ length: childCount }, (_, index) => textItem(String(index), false)),
  };
}

describe("measurement persistence", () => {
  it("keeps two roots in order and does not keep the caller text", () => {
    const first = container(1);
    const second = textItem("later", true);
    const callerText = first.children[0];
    const document = persistMeasurements([first, second]);
    expect(document).not.toBeNull();
    const loaded = loadMeasurements(document ?? "");
    expect(loaded).not.toBeNull();
    if (loaded === null || callerText === undefined || callerText.valueType !== "TEXT") {
      throw new Error("expected two loaded roots");
    }
    expect(loaded).toHaveLength(2);
    const loadedFirst = loaded[0];
    const loadedSecond = loaded[1];
    if (
      loadedFirst === undefined ||
      loadedSecond === undefined ||
      loadedFirst.valueType !== "CONTAINER" ||
      loadedSecond.valueType !== "TEXT"
    ) {
      throw new Error("expected a container and a text");
    }
    const loadedChild = loadedFirst.children[0];
    if (loadedChild === undefined || loadedChild.valueType !== "TEXT") {
      throw new Error("expected a text child");
    }
    expect(loadedChild.text).toBe("0");
    expect(loadedChild).not.toBe(callerText);
    expect(loadedSecond.text).toBe("later");
    expect(loadedFirst).not.toBe(first);
    expect(callerText.text).toBe("0");
  });

  it("returns nothing for one bad node", () => {
    const bad: ContentItem = {
      relationship: "CONTAINS",
      valueType: "TEXT",
      concept,
      text: "no",
      children: [],
    };
    const previous = textItem("kept", true);
    expect(persistMeasurements([previous, bad])).toBeNull();
    if (previous.valueType !== "TEXT") {
      throw new Error("expected a text root");
    }
    expect(previous.text).toBe("kept");
  });

  it("persists 3999 nodes and refuses 4000", () => {
    const under = persistMeasurements([container(3998)]);
    expect(under).not.toBeNull();
    const loaded = loadMeasurements(under ?? "");
    expect(loaded).toHaveLength(1);
    expect(persistMeasurements([container(3999)])).toBeNull();
  });

  it("refuses a wrong format, a wrong version, and a non-object", () => {
    expect(loadMeasurements('{"format":"other","version":1,"items":[]}')).toBeNull();
    expect(loadMeasurements('{"format":"ocelli-measurement","version":2,"items":[]}')).toBeNull();
    expect(loadMeasurements("[]")).toBeNull();
    expect(loadMeasurements("null")).toBeNull();
    expect(loadMeasurements("{")).toBeNull();
  });

  it("loads an empty list as a new array", () => {
    const document = persistMeasurements([]);
    expect(document).not.toBeNull();
    const loaded = loadMeasurements(document ?? "");
    const again = loadMeasurements(document ?? "");
    expect(loaded).toEqual([]);
    expect(again).toEqual([]);
    expect(again).not.toBe(loaded);
  });
});
