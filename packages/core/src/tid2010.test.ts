import { describe, expect, it } from "vitest";

import dcmjs from "dcmjs";

import type { ContentItem } from "./sr.js";
import { writeTid1500, type ReportSource, type TidSource } from "./tid1500.js";
import {
  readKeyObject,
  readKeyObjects,
  writeKeyObject,
  writeKeyObjects,
  type KeyObjectRequest,
  type KeyReference,
} from "./tid2010.js";

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

  it("writes the reference as an IMAGE item, TID 2010 row 8", () => {
    // Lead amendment 5. A key image is an IMAGE content item, PS3.3 C.18.4.
    const bytes = writeKeyObject(request);
    if (bytes === null) {
      throw new Error("expected bytes");
    }
    const items = natural(bytes).ContentSequence;
    const kinds = (Array.isArray(items) ? items : [items]).map((item: Record<string, unknown>) => item.ValueType);
    expect(kinds).toEqual(["TEXT", "IMAGE"]);
  });

  it("still reads a COMPOSITE reference an F-196 file holds", () => {
    const bytes = writeKeyObject(request);
    if (bytes === null) {
      throw new Error("expected bytes");
    }
    const older = rewrite(bytes, (dataset) => {
      const items = dataset.ContentSequence as Record<string, unknown>[];
      const reference = items[1];
      if (reference !== undefined) {
        reference.ValueType = "COMPOSITE";
      }
    });
    expect(readKeyObject(older)?.referencedInstance).toBe("2.25.13");
  });
});

const CT = "1.2.840.10008.5.1.4.1.1.2";
const ENHANCED_CT = "1.2.840.10008.5.1.4.1.1.2.1";

const report: ReportSource = {
  study: "1.2.3.4.5",
  patientName: "Synthetic^Phantom",
  patientId: "SYNTH-1",
  studyDate: "20240102",
  studyTime: "101500",
  accession: "ACC1",
  contentDate: "20261108",
  contentTime: "093000",
  evidence: [
    { series: "1.2.3.4.6", sopClass: CT, sopInstance: "1.2.3.4.7" },
    { series: "1.2.3.4.8", sopClass: ENHANCED_CT, sopInstance: "1.2.3.4.9" },
    { series: "1.2.3.4.8", sopClass: CT, sopInstance: "1.2.3.4.10" },
  ],
  predecessors: [],
};

const references: KeyReference[] = [
  { sopClass: CT, sopInstance: "1.2.3.4.7" },
  { sopClass: ENHANCED_CT, sopInstance: "1.2.3.4.9", frame: 2 },
];

function rewrite(bytes: Uint8Array, change: (dataset: Record<string, unknown>) => void): Uint8Array {
  const copy = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(copy).set(bytes);
  const dict = dcmjs.data.DicomMessage.readFile(copy);
  const dataset = dcmjs.data.DicomMetaDictionary.naturalizeDataset(dict.dict);
  change(dataset);
  const next = new dcmjs.data.DicomDict(dict.meta);
  next.dict = dcmjs.data.DicomMetaDictionary.denaturalizeDataset(dataset);
  const view = next.write();
  return view instanceof Uint8Array ? view : new Uint8Array(view);
}

function listOf(value: unknown): Record<string, unknown>[] {
  if (Array.isArray(value)) {
    return value as Record<string, unknown>[];
  }
  return value === undefined ? [] : [value as Record<string, unknown>];
}

describe("Key Object Selection of many images", () => {
  it("reads both references back, the frame included", () => {
    const bytes = writeKeyObjects(references, report);
    if (bytes === null) {
      throw new Error("expected bytes");
    }
    const loaded = readKeyObjects(bytes);
    expect(loaded?.references).toEqual(references);
    expect(loaded?.study).toBe("1.2.3.4.5");
    expect(loaded?.contentDate).toBe("20261108");
    expect(loaded?.contentTime).toBe("093000");
    expect(loaded?.instanceNumber).toBe(1);
    expect(loaded?.manufacturer).toBe("Ocelli");
  });

  it("writes TID 2010 rows 1 and 8 and the C.17.6 modules", () => {
    const bytes = writeKeyObjects(references, report);
    if (bytes === null) {
      throw new Error("expected bytes");
    }
    const dataset = natural(bytes);
    expect(dataset.SOPClassUID).toBe("1.2.840.10008.5.1.4.1.1.88.59");
    expect(dataset.Modality).toBe("KO");
    expect(String(dataset.SeriesNumber)).toBe("1");
    expect(String(dataset.InstanceNumber)).toBe("1");
    expect(dataset.ContentDate).toBe("20261108");
    expect(dataset.ContentTime).toBe("093000");
    expect(dataset.PatientID).toBe("SYNTH-1");
    expect(dataset.StudyInstanceUID).toBe("1.2.3.4.5");
    // C.17.6.2 has no Predecessor Documents Sequence.
    expect(dataset.PredecessorDocumentsSequence).toBeUndefined();
    const title = listOf(dataset.ConceptNameCodeSequence)[0];
    expect(title?.CodeValue).toBe("113000");
    const items = listOf(dataset.ContentSequence);
    expect(items.map((item) => [item.RelationshipType, item.ValueType])).toEqual([
      ["CONTAINS", "IMAGE"],
      ["CONTAINS", "IMAGE"],
    ]);
    // Rows 8 to 10 carry no concept name and no purpose of reference.
    expect(items.every((item) => item.ConceptNameCodeSequence === undefined)).toBe(true);
    const framed = listOf(items[1]?.ReferencedSOPSequence)[0];
    expect(Number(framed?.ReferencedFrameNumber)).toBe(2);
    expect(listOf(items[0]?.ReferencedSOPSequence)[0]?.ReferencedFrameNumber).toBeUndefined();
    // TID 2010 rows 8 to 10: every evidence instance is referenced, so the
    // unreferenced 1.2.3.4.10 is not listed.
    const evidence = listOf(dataset.CurrentRequestedProcedureEvidenceSequence)[0];
    expect(evidence?.StudyInstanceUID).toBe("1.2.3.4.5");
    const series = listOf(evidence?.ReferencedSeriesSequence);
    expect(series.map((item) => item.SeriesInstanceUID)).toEqual(["1.2.3.4.6", "1.2.3.4.8"]);
    expect(
      series.map((item) => listOf(item.ReferencedSOPSequence).map((sop) => sop.ReferencedSOPInstanceUID)),
    ).toEqual([["1.2.3.4.7"], ["1.2.3.4.9"]]);
  });

  it("accepts a study UID that is not 2.25, and refuses what it cannot write", () => {
    expect(writeKeyObjects(references, report)).not.toBeNull();
    expect(writeKeyObjects([], report)).toBeNull();
    expect(writeKeyObjects([{ sopClass: CT, sopInstance: "1.2.3.4.99" }], report)).toBeNull();
    expect(writeKeyObjects([{ sopClass: CT, sopInstance: "1.2.3.4.7", frame: 0 }], report)).toBeNull();
    expect(writeKeyObjects(references, { ...report, study: "1..2" })).toBeNull();
  });

  it("does not read a TID 1500 document", () => {
    const bytes = writeTid1500([lengthItem()], source);
    if (bytes === null) {
      throw new Error("expected a TID 1500 document");
    }
    expect(readKeyObjects(bytes)).toBeNull();
  });

  it("reads an F-196 file as one reference", () => {
    const bytes = writeKeyObject(request);
    if (bytes === null) {
      throw new Error("expected bytes");
    }
    expect(readKeyObjects(bytes)?.references).toEqual([
      { sopClass: "1.2.840.10008.5.1.4.1.1.2", sopInstance: "2.25.13" },
    ]);
  });

  it("writes COMPOSITE for a reference that is not an image", () => {
    // Review pass 1, S3. TID 2010 row 8 is IMAGE for an image and row 10 is
    // COMPOSITE for any other composite object, PS3.3 C.18.3 and C.18.4.
    // Encapsulated PDF Storage is 1.2.840.10008.5.1.4.1.1.104.1, PS3.4
    // Table B.5-1, and holds no image.
    const pdf = "1.2.840.10008.5.1.4.1.1.104.1";
    const withPdf: ReportSource = {
      ...report,
      evidence: [...report.evidence, { series: "1.2.3.4.8", sopClass: pdf, sopInstance: "1.2.3.4.11" }],
    };
    const mixed: KeyReference[] = [...references, { sopClass: pdf, sopInstance: "1.2.3.4.11" }];
    const bytes = writeKeyObjects(mixed, withPdf);
    if (bytes === null) {
      throw new Error("expected bytes");
    }
    expect(listOf(natural(bytes).ContentSequence).map((item) => item.ValueType)).toEqual(["IMAGE", "IMAGE", "COMPOSITE"]);
    expect(readKeyObjects(bytes)?.references).toEqual(mixed);
    const single = writeKeyObject({ ...request, referencedClass: pdf });
    if (single === null) {
      throw new Error("expected bytes");
    }
    expect(listOf(natural(single).ContentSequence).map((item) => item.ValueType)).toEqual(["TEXT", "COMPOSITE"]);
    expect(readKeyObject(single)?.referencedClass).toBe(pdf);
  });

  it("writes the Type 1 file meta from both writers", () => {
    // Review pass 1, D4. PS3.10 Table 7.1-1, File Meta Information Version
    // (0002,0001) and Implementation Class UID (0002,0012).
    for (const bytes of [writeKeyObjects(references, report), writeKeyObject(request)]) {
      if (bytes === null) {
        throw new Error("expected bytes");
      }
      const copy = new ArrayBuffer(bytes.byteLength);
      new Uint8Array(copy).set(bytes);
      const meta = dcmjs.data.DicomMessage.readFile(copy).meta as Record<string, { Value: unknown[] } | undefined>;
      expect(meta["00020001"]).toBeDefined();
      expect(typeof meta["00020012"]?.Value[0]).toBe("string");
    }
  });

  it("writes IMAGE for every class whose IOD holds the Image Pixel Module, and WAVEFORM for a waveform", () => {
    // Review pass 2, D6 and S2. TID 2010 row 8 is IMAGE, row 9 WAVEFORM and
    // row 10 COMPOSITE for any other object, PS3.3 C.18.3. The Ophthalmic
    // Thickness Map IOD, PS3.3 A.67.3, Corneal Topography Map IOD, A.73.3,
    // and OCT B-scan Volume Analysis IOD, A.84.3, hold the Image Pixel
    // Module, and the Height Map Segmentation IOD, A.91.3, its floating point
    // form. Hardcopy Grayscale Image Storage is a retired image class of
    // PS3.6 Annex A. 12-lead ECG Waveform Storage is a waveform, A.34.3.3.
    const classes: [string, string][] = [
      ["1.2.840.10008.5.1.4.1.1.81.1", "IMAGE"],
      ["1.2.840.10008.5.1.4.1.1.82.1", "IMAGE"],
      ["1.2.840.10008.5.1.4.1.1.77.1.5.8", "IMAGE"],
      ["1.2.840.10008.5.1.4.1.1.66.8", "IMAGE"],
      ["1.2.840.10008.5.1.1.29", "IMAGE"],
      ["1.2.840.10008.5.1.4.1.1.9.1.1", "WAVEFORM"],
      ["1.2.840.10008.5.1.4.1.1.11.1", "COMPOSITE"],
    ];
    const evidence = classes.map(([sopClass], at) => ({ series: "1.2.3.4.8", sopClass, sopInstance: "1.2.3.4." + String(20 + at) }));
    const keys: KeyReference[] = evidence.map(({ sopClass, sopInstance }) => ({ sopClass, sopInstance }));
    const bytes = writeKeyObjects(keys, { ...report, evidence });
    if (bytes === null) {
      throw new Error("expected bytes");
    }
    expect(listOf(natural(bytes).ContentSequence).map((item) => item.ValueType)).toEqual(classes.map(([, kind]) => kind));
    expect(readKeyObjects(bytes)?.references).toEqual(keys);
    const ecg = writeKeyObject({ ...request, referencedClass: "1.2.840.10008.5.1.4.1.1.9.1.1" });
    if (ecg === null) {
      throw new Error("expected bytes");
    }
    expect(listOf(natural(ecg).ContentSequence).map((item) => item.ValueType)).toEqual(["TEXT", "WAVEFORM"]);
    expect(readKeyObject(ecg)?.referencedClass).toBe("1.2.840.10008.5.1.4.1.1.9.1.1");
  });
});
