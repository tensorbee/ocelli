import { describe, expect, it } from "vitest";

import dcmjs from "dcmjs";

import type { ContentItem } from "./sr.js";
import { readTid1500, writeTid1500, type TidSource } from "./tid1500.js";

const concept: ContentItem["concept"] = {
  scheme: "SCT",
  value: "410668003",
  meaning: "Length",
};

const source: TidSource = {
  study: "1.2.3.4.5",
  series: "1.2.3.4.6",
  sopClass: "1.2.840.10008.5.1.4.1.1.2",
  sopInstance: "1.2.3.4.7",
};

function numItem(number: number, root: boolean): ContentItem & { valueType: "NUM" } {
  return {
    relationship: root ? null : "CONTAINS",
    valueType: "NUM",
    concept,
    number,
    unit: "mm",
    children: [],
  };
}

function textItem(label: string): ContentItem {
  return {
    relationship: "CONTAINS",
    valueType: "TEXT",
    concept: { scheme: "OCELLI", value: "operator", meaning: "Operator" },
    text: label,
    children: [],
  };
}

function container(childCount: number): ContentItem {
  const children: ContentItem[] = [numItem(12.5, false)];
  for (let index = 1; index < childCount; index += 1) {
    children.push({
      relationship: "CONTAINS",
      valueType: "TEXT",
      concept,
      text: String(index),
      children: [],
    });
  }
  return {
    relationship: null,
    valueType: "CONTAINER",
    concept,
    children,
  };
}

function latin1(bytes: Uint8Array): string {
  return new TextDecoder("latin1").decode(bytes);
}

describe("TID 1500", () => {
  it("reads 12.5 mm back with the same concept", () => {
    const root = numItem(12.5, true);
    const bytes = writeTid1500([root], source);
    expect(bytes).not.toBeNull();
    if (bytes === null) {
      throw new Error("expected bytes");
    }
    const loaded = readTid1500(bytes);
    expect(loaded).not.toBeNull();
    const item = loaded?.[0];
    if (item === undefined || item.valueType !== "NUM") {
      throw new Error("expected a numeric root");
    }
    expect(item.number).toBe(12.5);
    expect(item.unit).toBe("mm");
    expect(item.concept).toEqual(concept);
    expect(item.relationship).toBeNull();
    expect(item.children).toEqual([]);
    expect(item).not.toBe(root);
    expect(root.number).toBe(12.5);
  });

  it("names template 1500 and the comprehensive 3D SR class", () => {
    const bytes = writeTid1500([numItem(1, true)], source);
    expect(bytes).not.toBeNull();
    if (bytes === null) {
      throw new Error("expected bytes");
    }
    const text = latin1(bytes);
    expect(text.includes("1.2.840.10008.5.1.4.1.1.88.34")).toBe(true);
    expect(text.includes("1500")).toBe(true);
    const copy = new ArrayBuffer(bytes.byteLength);
    new Uint8Array(copy).set(bytes);
    const dict = dcmjs.data.DicomMessage.readFile(copy);
    const dataset = dcmjs.data.DicomMetaDictionary.naturalizeDataset(dict.dict);
    const template = dataset.ContentTemplateSequence;
    const identifier = Array.isArray(template) ? template[0]?.TemplateIdentifier : undefined;
    expect(identifier).toBe("1500");
    expect(dataset.SOPClassUID).toBe("1.2.840.10008.5.1.4.1.1.88.34");
  });

  it("returns nothing for a refused tree, a root with no NUM, and an empty source", () => {
    const bad: ContentItem = {
      relationship: "CONTAINS",
      valueType: "NUM",
      concept,
      number: 1,
      unit: "mm",
      children: [],
    };
    const kept = numItem(4, true);
    expect(writeTid1500([kept, bad], source)).toBeNull();
    expect(kept.number).toBe(4);
    const noNumber: ContentItem = {
      relationship: null,
      valueType: "TEXT",
      concept,
      text: "note",
      children: [],
    };
    expect(writeTid1500([noNumber], source)).toBeNull();
    expect(writeTid1500([numItem(1, true)], { ...source, study: "" })).toBeNull();
    const two: ContentItem = {
      relationship: null,
      valueType: "CONTAINER",
      concept,
      children: [numItem(1, false), numItem(2, false)],
    };
    expect(writeTid1500([two], source)).toBeNull();
  });

  it("reads template 1499 as nothing", () => {
    const bytes = writeTid1500([numItem(3, true)], source);
    expect(bytes).not.toBeNull();
    if (bytes === null) {
      throw new Error("expected bytes");
    }
    const copy = new ArrayBuffer(bytes.byteLength);
    new Uint8Array(copy).set(bytes);
    const dict = dcmjs.data.DicomMessage.readFile(copy);
    const dataset = dcmjs.data.DicomMetaDictionary.naturalizeDataset(dict.dict);
    const template = dataset.ContentTemplateSequence;
    if (!Array.isArray(template) || template[0] === undefined) {
      throw new Error("expected a template");
    }
    template[0].TemplateIdentifier = "1499";
    const rewritten = new dcmjs.data.DicomDict(dict.meta);
    rewritten.dict = dcmjs.data.DicomMetaDictionary.denaturalizeDataset(dataset);
    const produced = rewritten.write();
    const view = produced instanceof Uint8Array ? produced : new Uint8Array(produced);
    expect(readTid1500(view)).toBeNull();
  });

  it("reads two measurements in caller order", () => {
    const first = numItem(1, true);
    const second = numItem(2, true);
    const bytes = writeTid1500([first, second], source);
    expect(bytes).not.toBeNull();
    const loaded = readTid1500(bytes ?? new Uint8Array());
    expect(loaded).toHaveLength(2);
    const left = loaded?.[0];
    const right = loaded?.[1];
    if (left === undefined || right === undefined || left.valueType !== "NUM" || right.valueType !== "NUM") {
      throw new Error("expected two numbers");
    }
    expect(left.number).toBe(1);
    expect(right.number).toBe(2);
    expect(left.unit).toBe("mm");
    expect(right.unit).toBe("mm");
  });

  it("stores an operator name and does not return it", () => {
    const root: ContentItem = {
      relationship: null,
      valueType: "CONTAINER",
      concept,
      children: [numItem(8, false), textItem("Ada")],
    };
    const bytes = writeTid1500([root], source);
    expect(bytes).not.toBeNull();
    if (bytes === null) {
      throw new Error("expected bytes");
    }
    expect(latin1(bytes).includes("Ada")).toBe(true);
    const loaded = readTid1500(bytes);
    expect(loaded).toHaveLength(1);
    const item = loaded?.[0];
    if (item === undefined || item.valueType !== "NUM") {
      throw new Error("expected the numeric root");
    }
    expect(item.number).toBe(8);
    expect(JSON.stringify(item).includes("Ada")).toBe(false);
  });

  it("reads the same measurement from two different documents", () => {
    const root = numItem(12.5, true);
    const first = writeTid1500([root], source);
    const second = writeTid1500([root], source);
    expect(first).not.toBeNull();
    expect(second).not.toBeNull();
    if (first === null || second === null) {
      throw new Error("expected two documents");
    }
    expect(latin1(first)).not.toBe(latin1(second));
    expect(readTid1500(first)).toEqual(readTid1500(second));
  });

  it("refuses 4000 nodes and writes 3999", () => {
    expect(writeTid1500([container(3999)], source)).toBeNull();
    const bytes = writeTid1500([container(3998)], source);
    expect(bytes).not.toBeNull();
    const loaded = readTid1500(bytes ?? new Uint8Array());
    expect(loaded).toHaveLength(1);
  });
});
