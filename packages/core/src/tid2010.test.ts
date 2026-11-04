import { describe, expect, it } from "vitest";

import dcmjs from "dcmjs";

import type { ContentItem } from "./sr.js";
import { writeTid1500, type TidSource } from "./tid1500.js";
import { readKeyObject, writeKeyObject, type KeyObjectRequest } from "./tid2010.js";

const request: KeyObjectRequest = {
  title: "Key image",
  study: "2.25.11",
  series: "2.25.12",
  instance: "2.25.10",
  referencedClass: "1.2.840.10008.5.1.4.1.1.2",
  referencedInstance: "2.25.13",
};

const source: TidSource = {
  study: "1.2.3.4.5",
  series: "1.2.3.4.6",
  sopClass: "1.2.840.10008.5.1.4.1.1.2",
  sopInstance: "1.2.3.4.7",
};

function lengthItem(): ContentItem & { valueType: "NUM" } {
  return {
    relationship: null,
    valueType: "NUM",
    concept: { scheme: "SCT", value: "410668003", meaning: "Length" },
    number: 12.5,
    unit: "mm",
    children: [],
  };
}

function natural(bytes: Uint8Array): Record<string, unknown> {
  const copy = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(copy).set(bytes);
  const dict = dcmjs.data.DicomMessage.readFile(copy);
  return dcmjs.data.DicomMetaDictionary.naturalizeDataset(dict.dict);
}

describe("TID 2010", () => {
  it("reads the title and the referenced SOP back", () => {
    const bytes = writeKeyObject(request);
    expect(bytes).not.toBeNull();
    if (bytes === null) {
      throw new Error("expected bytes");
    }
    const loaded = readKeyObject(bytes);
    expect(loaded).toEqual({
      title: "Key image",
      referencedClass: "1.2.840.10008.5.1.4.1.1.2",
      referencedInstance: "2.25.13",
    });
    const dataset = natural(bytes);
    expect(dataset.SOPClassUID).toBe("1.2.840.10008.5.1.4.1.1.88.59");
    expect(dataset.PatientName).toBeUndefined();
    expect(dataset.PatientID).toBeUndefined();
  });

  it("does not read a TID 1500 document as a key object", () => {
    const bytes = writeTid1500([lengthItem()], source);
    expect(bytes).not.toBeNull();
    if (bytes === null) {
      throw new Error("expected a TID 1500 document");
    }
    expect(readKeyObject(bytes)).toBeNull();
  });

  it("returns nothing for an empty title or an empty UID", () => {
    expect(writeKeyObject({ ...request, title: "" })).toBeNull();
    expect(writeKeyObject({ ...request, title: " " })).toBeNull();
    expect(writeKeyObject({ ...request, study: "" })).toBeNull();
    expect(writeKeyObject({ ...request, referencedInstance: "" })).toBeNull();
    expect(writeKeyObject({ ...request, referencedClass: "" })).toBeNull();
    expect(writeKeyObject({ ...request, study: "1.2.3" })).toBeNull();
  });

  it("reads a referenced instance that is not a 2.25 UID", () => {
    const referencedInstance = "1.2.840.113619.2.55.3.1";
    const bytes = writeKeyObject({ ...request, referencedInstance });
    expect(bytes).not.toBeNull();
    if (bytes === null) {
      throw new Error("expected bytes");
    }
    expect(readKeyObject(bytes)?.referencedInstance).toBe(referencedInstance);
  });

  it("does not read a key object whose template was changed to 1500", () => {
    const bytes = writeKeyObject(request);
    expect(bytes).not.toBeNull();
    if (bytes === null) {
      throw new Error("expected bytes");
    }
    const copy = new ArrayBuffer(bytes.byteLength);
    new Uint8Array(copy).set(bytes);
    const dict = dcmjs.data.DicomMessage.readFile(copy);
    const dataset = dcmjs.data.DicomMetaDictionary.naturalizeDataset(dict.dict);
    const templates = dataset.ContentTemplateSequence;
    const template: unknown = Array.isArray(templates) ? templates[0] : templates;
    if (template === null || typeof template !== "object" || !("TemplateIdentifier" in template)) {
      throw new Error("expected a template");
    }
    template.TemplateIdentifier = "1500";
    const rewritten = new dcmjs.data.DicomDict(dict.meta);
    rewritten.dict = dcmjs.data.DicomMetaDictionary.denaturalizeDataset(dataset);
    const view = rewritten.write();
    const next = view instanceof Uint8Array ? view : new Uint8Array(view);
    expect(readKeyObject(next)).toBeNull();
  });

  it("writes the same bytes for the same request", () => {
    const first = writeKeyObject(request);
    const second = writeKeyObject(request);
    expect(first).not.toBeNull();
    expect(Array.from(first ?? [])).toEqual(Array.from(second ?? []));
  });
});
