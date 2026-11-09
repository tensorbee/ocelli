import { describe, expect, it } from "vitest";

import dcmjs from "dcmjs";

import { MEASUREMENT_ITEM_LIMIT } from "./measurement-persist.js";
import type { ContentItem } from "./sr.js";
import {
  newUid,
  readTid1500,
  readTid1500Groups,
  writeTid1500,
  writeTid1500Groups,
  type ReportSource,
} from "./tid1500.js";

// Every expected value below is read from the standard, not from the writer.
// PS3.16 2026d TID 1500 row 1 title (126000, DCM, "Imaging Measurement
// Report"), row 3 Observation Context, row 6 (126010, DCM, "Imaging
// Measurements"), row 7 TID 1410 groups. TID 1410 row 1 (125007, DCM,
// "Measurement Group"), rows 2 and 3 (112039) and (112040), row 5 SCOORD
// (111030, DCM, "Image Region") with row 6 SELECTED FROM IMAGE. PS3.3 2026d
// C.17.1, C.17.2 and C.18.

const CT = "1.2.840.10008.5.1.4.1.1.2";
const ENHANCED_CT = "1.2.840.10008.5.1.4.1.1.2.1";

const source: ReportSource = {
  study: "1.2.3.4.5",
  studyDate: "20240102",
  studyTime: "101500",
  patientName: "Synthetic^Phantom",
  patientId: "SYNTH-1",
  accession: "ACC1",
  contentDate: "20261108",
  contentTime: "093000",
  evidence: [
    { series: "1.2.3.4.6", sopClass: CT, sopInstance: "1.2.3.4.7" },
    { series: "1.2.3.4.8", sopClass: ENHANCED_CT, sopInstance: "1.2.3.4.9" },
  ],
  predecessors: [{ series: "2.25.71", sopClass: "1.2.840.10008.5.1.4.1.1.88.34", sopInstance: "2.25.72" }],
};

function code(scheme: string, value: string, meaning: string): ContentItem["concept"] {
  return { scheme, value, meaning };
}

/** One TID 1410 group holding every value type the content tree has. */
function everyType(): ContentItem {
  return {
    relationship: null,
    valueType: "CONTAINER",
    concept: code("DCM", "125007", "Measurement Group"),
    children: [
      {
        relationship: "HAS_OBS_CONTEXT",
        valueType: "TEXT",
        concept: code("DCM", "112039", "Tracking Identifier"),
        text: "ocelli-folder-viewer:ellipse",
        children: [],
      },
      {
        relationship: "HAS_OBS_CONTEXT",
        valueType: "UIDREF",
        concept: code("DCM", "112040", "Tracking Unique Identifier"),
        uid: "2.25.12345",
        children: [],
      },
      {
        relationship: "CONTAINS",
        valueType: "SCOORD",
        concept: code("DCM", "111030", "Image Region"),
        graphic: "ELLIPSE",
        // Halves and integers, so FL holds each exactly.
        points: [[20, 20], [20, 60], [10.5, 40], [30.25, 40]],
        children: [
          {
            relationship: "SELECTED_FROM",
            valueType: "IMAGE",
            concept: code("DCM", "121112", "Source of Measurement"),
            sopClass: ENHANCED_CT,
            sopInstance: "1.2.3.4.9",
            frame: 3,
            children: [],
          },
        ],
      },
      {
        relationship: "CONTAINS",
        valueType: "NUM",
        concept: code("SCT", "410668003", "Length"),
        number: 85.4400374532,
        unit: "mm",
        children: [],
      },
      {
        relationship: "CONTAINS",
        valueType: "CODE",
        concept: code("DCM", "121071", "Finding"),
        code: code("SCT", "52988006", "Lesion"),
        children: [
          {
            relationship: "HAS_PROPERTIES",
            valueType: "TEXT",
            concept: code("DCM", "121106", "Comment"),
            text: "note",
            children: [],
          },
        ],
      },
      {
        relationship: "INFERRED_FROM",
        valueType: "SCOORD3D",
        concept: code("DCM", "111030", "Image Region"),
        graphic: "POINT",
        points: [[1, 2, 3]],
        frameOfReference: "2.25.44000",
        children: [],
      },
    ],
  };
}

function bytesOf(groups: readonly ContentItem[], at: ReportSource = source): Uint8Array {
  const bytes = writeTid1500Groups(groups, at);
  if (bytes === null) {
    throw new Error("expected a document");
  }
  return bytes;
}

function natural(bytes: Uint8Array): Record<string, unknown> {
  const copy = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(copy).set(bytes);
  const dict = dcmjs.data.DicomMessage.readFile(copy);
  return dcmjs.data.DicomMetaDictionary.naturalizeDataset(dict.dict);
}

/** The raw element for `tag`, so an empty Type 2 element can be seen. */
function raw(bytes: Uint8Array, tag: string): { vr: string; Value: unknown[] } | undefined {
  const copy = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(copy).set(bytes);
  const dict = dcmjs.data.DicomMessage.readFile(copy);
  const elements = dict.dict as Record<string, { vr: string; Value: unknown[] } | undefined>;
  return elements[tag];
}

function first(value: unknown): Record<string, unknown> {
  const item: unknown = Array.isArray(value) ? value[0] : value;
  if (item === null || typeof item !== "object") {
    throw new Error("expected an item");
  }
  return item as Record<string, unknown>;
}

function list(value: unknown): Record<string, unknown>[] {
  if (Array.isArray(value)) {
    return value as Record<string, unknown>[];
  }
  return value === undefined ? [] : [value as Record<string, unknown>];
}

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

describe("TID 1500 measurement groups", () => {
  it("reads every value type back, the frame number included", () => {
    const group = everyType();
    const loaded = readTid1500Groups(bytesOf([group]));
    expect(loaded).not.toBeNull();
    expect(loaded?.groups).toEqual([group]);
    expect(loaded?.study).toBe("1.2.3.4.5");
    expect(loaded?.sopClass).toBe("1.2.840.10008.5.1.4.1.1.88.34");
    expect(loaded?.predecessors).toEqual(["2.25.72"]);
    expect(loaded?.unread).toBe(0);
  });

  it("writes the relationship with a space and the frame as Referenced Frame Number", () => {
    const dataset = natural(bytesOf([everyType()]));
    const measurements = list(dataset.ContentSequence).find(
      (item) => first(item.ConceptNameCodeSequence).CodeValue === "126010",
    );
    expect(measurements?.RelationshipType).toBe("CONTAINS");
    const group = first(measurements?.ContentSequence);
    expect(group.ValueType).toBe("CONTAINER");
    expect(group.RelationshipType).toBe("CONTAINS");
    expect(first(group.ConceptNameCodeSequence).CodeValue).toBe("125007");
    const children = list(group.ContentSequence);
    expect(children.map((item) => item.RelationshipType)).toEqual([
      "HAS OBS CONTEXT",
      "HAS OBS CONTEXT",
      "CONTAINS",
      "CONTAINS",
      "CONTAINS",
      "INFERRED FROM",
    ]);
    const region = children[2];
    expect(region?.GraphicType).toBe("ELLIPSE");
    // Column then row, PS3.3 C.18.6.1.1.
    expect(region?.GraphicData).toEqual([20, 20, 20, 60, 10.5, 40, 30.25, 40]);
    const image = first(region?.ContentSequence);
    expect(image.RelationshipType).toBe("SELECTED FROM");
    expect(image.ValueType).toBe("IMAGE");
    const reference = first(image.ReferencedSOPSequence);
    expect(reference.ReferencedSOPInstanceUID).toBe("1.2.3.4.9");
    expect(Number(reference.ReferencedFrameNumber)).toBe(3);
    const num = children[3];
    const measured = first(num?.MeasuredValueSequence);
    expect(Number(measured.NumericValue)).toBe(85.4400374532);
    expect(first(measured.MeasurementUnitsCodeSequence).CodingSchemeDesignator).toBe("UCUM");
  });

  it("stores Graphic Data as FL, so 100.1 reads back as its 32-bit value", () => {
    // PS3.6: Graphic Data (0070,0022) is FL.
    const group = everyType();
    const region = group.children[2];
    if (region === undefined || region.valueType !== "SCOORD") {
      throw new Error("expected the region");
    }
    const moved: ContentItem = {
      ...group,
      children: [
        ...group.children.slice(0, 2),
        { ...region, graphic: "POINT", points: [[100.1, 0]] },
      ],
    };
    const bytes = bytesOf([moved]);
    expect(raw(bytes, "0040A730") === undefined).toBe(false);
    const loaded = readTid1500Groups(bytes);
    const point = loaded?.groups[0]?.children[2];
    if (point === undefined || point.valueType !== "SCOORD") {
      throw new Error("expected the point");
    }
    expect(point.points[0]?.[0]).toBe(Math.fround(100.1));
    expect(point.points[0]?.[0]).not.toBe(100.1);
    expect(point.points[0]?.[1]).toBe(0);
  });

  it("writes the document modules from the source", () => {
    const bytes = bytesOf([everyType()]);
    const dataset = natural(bytes);
    // PS3.3 C.17.1 SR Document Series.
    expect(dataset.Modality).toBe("SR");
    expect(String(dataset.SeriesNumber)).toBe("1");
    expect(String(dataset.SeriesInstanceUID).startsWith("2.25.")).toBe(true);
    expect(raw(bytes, "00081111")?.vr).toBe("SQ");
    // PS3.3 C.17.2 SR Document General.
    expect(String(dataset.InstanceNumber)).toBe("1");
    expect(dataset.CompletionFlag).toBe("COMPLETE");
    expect(dataset.VerificationFlag).toBe("UNVERIFIED");
    expect(dataset.ContentDate).toBe("20261108");
    expect(dataset.ContentTime).toBe("093000");
    expect(raw(bytes, "0040A372")?.vr).toBe("SQ");
    const predecessor = first(dataset.PredecessorDocumentsSequence);
    expect(predecessor.StudyInstanceUID).toBe("1.2.3.4.5");
    const predecessorSeries = first(predecessor.ReferencedSeriesSequence);
    expect(predecessorSeries.SeriesInstanceUID).toBe("2.25.71");
    expect(first(predecessorSeries.ReferencedSOPSequence).ReferencedSOPInstanceUID).toBe("2.25.72");
    const evidence = first(dataset.CurrentRequestedProcedureEvidenceSequence);
    expect(evidence.StudyInstanceUID).toBe("1.2.3.4.5");
    const series = list(evidence.ReferencedSeriesSequence);
    expect(series.map((item) => item.SeriesInstanceUID)).toEqual(["1.2.3.4.6", "1.2.3.4.8"]);
    expect(first(series[1]?.ReferencedSOPSequence).ReferencedSOPClassUID).toBe(ENHANCED_CT);
    // PS3.3 C.7.1.1 Patient, C.7.2.1 General Study, C.7.5.1 General Equipment.
    expect(dataset.PatientName).toEqual([{ Alphabetic: "Synthetic^Phantom" }]);
    expect(dataset.PatientID).toBe("SYNTH-1");
    expect(dataset.StudyInstanceUID).toBe("1.2.3.4.5");
    expect(dataset.StudyDate).toBe("20240102");
    expect(dataset.StudyTime).toBe("101500");
    expect(dataset.AccessionNumber).toBe("ACC1");
    for (const tag of ["00100030", "00100040", "00080090", "00200010", "00080070"]) {
      expect(raw(bytes, tag), tag).toBeDefined();
    }
    expect(dataset.SOPClassUID).toBe("1.2.840.10008.5.1.4.1.1.88.34");
    // TID 1500 row 1 and the template.
    expect(dataset.ValueType).toBe("CONTAINER");
    expect(first(dataset.ConceptNameCodeSequence).CodeValue).toBe("126000");
    expect(first(dataset.ContentTemplateSequence).TemplateIdentifier).toBe("1500");
  });

  it("writes the observation context TID 1500 row 3 requires", () => {
    // TID 1001 row 1, TID 1002 rows 1 and 3, TID 1004 row 1.
    const dataset = natural(bytesOf([everyType()]));
    const items = list(dataset.ContentSequence).filter((item) => item.RelationshipType === "HAS OBS CONTEXT");
    const codes = items.map((item) => first(item.ConceptNameCodeSequence).CodeValue);
    expect(codes).toEqual(["121005", "121012", "121013"]);
    expect(first(items[0]?.ConceptCodeSequence).CodeValue).toBe("121007");
    expect(items[1]?.ValueType).toBe("UIDREF");
  });

  it("refuses a root that is not a measurement group, and a short ellipse", () => {
    const num: ContentItem = {
      relationship: null,
      valueType: "NUM",
      concept: code("SCT", "410668003", "Length"),
      number: 1,
      unit: "mm",
      children: [],
    };
    expect(writeTid1500Groups([num], source)).toBeNull();
    const other: ContentItem = { ...everyType(), concept: code("DCM", "126010", "Imaging Measurements") };
    expect(writeTid1500Groups([other], source)).toBeNull();
    const group = everyType();
    const region = group.children[2];
    if (region === undefined || region.valueType !== "SCOORD") {
      throw new Error("expected the region");
    }
    const short: ContentItem = {
      ...group,
      children: [...group.children.slice(0, 2), { ...region, points: [[20, 20], [20, 60]] }],
    };
    expect(writeTid1500Groups([short], source)).toBeNull();
    // No groups and nothing to replace. D2's empty report is tested below.
    expect(writeTid1500Groups([], { ...source, predecessors: [] })).toBeNull();
  });

  it("refuses a source it cannot write", () => {
    const group = everyType();
    expect(writeTid1500Groups([group], { ...source, study: "" })).toBeNull();
    expect(writeTid1500Groups([group], { ...source, study: "1.2..3" })).toBeNull();
    expect(writeTid1500Groups([group], { ...source, contentDate: "2026-11-08" })).toBeNull();
    expect(writeTid1500Groups([group], { ...source, contentTime: "" })).toBeNull();
    expect(writeTid1500Groups([group], { ...source, studyTime: "10:15" })).toBeNull();
    expect(writeTid1500Groups([group], { ...source, evidence: [] })).toBeNull();
    expect(
      writeTid1500Groups([group], {
        ...source,
        evidence: [{ series: "1.2.3", sopClass: CT, sopInstance: "x" }],
      }),
    ).toBeNull();
    // Type 2 study attributes may be empty.
    expect(writeTid1500Groups([group], { ...source, studyDate: "", studyTime: "", accession: "" })).not.toBeNull();
  });

  it("refuses the item limit and writes one fewer", () => {
    const leaf = (index: number): ContentItem => ({
      relationship: "CONTAINS",
      valueType: "TEXT",
      concept: code("DCM", "121106", "Comment"),
      text: String(index),
      children: [],
    });
    const group = (count: number): ContentItem => ({
      relationship: null,
      valueType: "CONTAINER",
      concept: code("DCM", "125007", "Measurement Group"),
      children: Array.from({ length: count - 1 }, (_, index) => leaf(index)),
    });
    expect(writeTid1500Groups([group(MEASUREMENT_ITEM_LIMIT)], source)).toBeNull();
    expect(writeTid1500Groups([group(MEASUREMENT_ITEM_LIMIT - 1)], source)).not.toBeNull();
  });

  it("counts a group it cannot read and keeps the others", () => {
    const bytes = bytesOf([everyType(), everyType()]);
    const changed = rewrite(bytes, (dataset) => {
      const measurements = list(dataset.ContentSequence).find(
        (item) => first(item.ConceptNameCodeSequence).CodeValue === "126010",
      );
      const group = list(measurements?.ContentSequence)[1];
      if (group === undefined) {
        throw new Error("expected two groups");
      }
      group.ContentSequence = [
        ...list(group.ContentSequence),
        {
          RelationshipType: "HAS CONCEPT MOD",
          ValueType: "CODE",
          ConceptNameCodeSequence: [{ CodeValue: "121049", CodingSchemeDesignator: "DCM", CodeMeaning: "Language" }],
          ConceptCodeSequence: [{ CodeValue: "en", CodingSchemeDesignator: "RFC5646", CodeMeaning: "English" }],
        },
      ];
    });
    const loaded = readTid1500Groups(changed);
    expect(loaded?.groups).toEqual([everyType()]);
    expect(loaded?.unread).toBe(1);
  });

  it("reads an F-097 document as groups with no image region", () => {
    const bytes = writeTid1500(
      [
        {
          relationship: null,
          valueType: "NUM",
          concept: code("SCT", "410668003", "Length"),
          number: 12.5,
          unit: "mm",
          children: [],
        },
      ],
      { study: "1.2.3.4.5", series: "1.2.3.4.6", sopClass: CT, sopInstance: "1.2.3.4.7" },
    );
    if (bytes === null) {
      throw new Error("expected an F-097 document");
    }
    const loaded = readTid1500Groups(bytes);
    expect(loaded).not.toBeNull();
    // One TID 1501 group holding the NUM alone, so it has no geometry to draw.
    expect(loaded?.unread).toBe(0);
    expect(loaded?.groups.map((group) => group.children.map((item) => item.valueType))).toEqual([["NUM"]]);
  });

  it("does not read a key object or another template", () => {
    const bytes = bytesOf([everyType()]);
    const other = rewrite(bytes, (dataset) => {
      first(dataset.ContentTemplateSequence).TemplateIdentifier = "1499";
    });
    expect(readTid1500Groups(other)).toBeNull();
    expect(readTid1500Groups(new Uint8Array([1, 2, 3]))).toBeNull();
  });

  it("is read by the F-097 reader as its one NUM", () => {
    // Both writers put the groups under the same row 6 container, so the
    // older reader still finds the group's single NUM.
    expect(readTid1500(bytesOf([everyType()]))?.map((item) => item.valueType)).toEqual(["NUM"]);
  });

  it("reads text back whole in the file's own character set", () => {
    // Review pass 1, D1. Specific Character Set ISO_IR 192 is UTF-8 for every
    // string of the data set, sequence items included, PS3.3 C.12.1.1.2 and
    // PS3.5 6.1. UT is one value in which a backslash is ordinary text,
    // PS3.5 6.2.
    const group = everyType();
    const texts = ["Läsion", "肺", "a\\b", "Läsion 肺 a\\b"];
    const groups = texts.map((text): ContentItem => ({
      ...group,
      children: [
        ...group.children.slice(0, 3),
        {
          relationship: "CONTAINS",
          valueType: "TEXT",
          concept: code("DCM", "121106", "Comment"),
          text,
          children: [],
        },
      ],
    }));
    const bytes = bytesOf(groups);
    expect(natural(bytes).SpecificCharacterSet).toBe("ISO_IR 192");
    const loaded = readTid1500Groups(bytes);
    const read = loaded?.groups.map((one) => {
      const comment = one.children[3];
      return comment?.valueType === "TEXT" ? comment.text : null;
    });
    expect(read).toEqual(texts);
  });

  it("writes the two Type 1 file meta attributes PS3.10 Table 7.1-1 names", () => {
    // Review pass 1, D4. File Meta Information Version (0002,0001) is OB
    // 00H 01H, and Implementation Class UID (0002,0012) is a UID.
    // Review pass 2, D2. Written once, because every write makes new UIDs
    // whose lengths differ.
    const written = bytesOf([everyType()]);
    const copy = new ArrayBuffer(written.byteLength);
    new Uint8Array(copy).set(written);
    const meta = dcmjs.data.DicomMessage.readFile(copy).meta as Record<string, { vr: string; Value: unknown[] }>;
    const version = meta["00020001"];
    expect(version?.vr).toBe("OB");
    const value = version?.Value[0];
    expect(Array.from(new Uint8Array(value instanceof ArrayBuffer ? value : new ArrayBuffer(0)))).toEqual([0, 1]);
    const implementation = meta["00020012"]?.Value[0];
    expect(typeof implementation === "string" && /^[0-9.]+$/u.test(implementation)).toBe(true);
  });

  it("writes an empty measurements container that names what it replaces", () => {
    // Review pass 1, D2. PS3.16 TID 1500 row 6 is a container whose rows 7
    // to 9 are each U, so the container may be empty, and a document holding
    // no marks still replaces the one it names, PS3.3 C.17.2.
    const bytes = writeTid1500Groups([], { ...source, evidence: [] });
    if (bytes === null) {
      throw new Error("expected an empty report");
    }
    const loaded = readTid1500Groups(bytes);
    expect(loaded?.groups).toEqual([]);
    expect(loaded?.predecessors).toEqual(["2.25.72"]);
    // Review pass 2, S1. Current Requested Procedure Evidence Sequence is 1C,
    // required when instances are referenced, and when present holds one or
    // more items, PS3.3 C.17.2. This report references none.
    expect(natural(bytes).CurrentRequestedProcedureEvidenceSequence).toBeUndefined();
    expect(raw(bytes, "0040A375")).toBeUndefined();
    // With nothing to replace there is nothing to write.
    expect(writeTid1500Groups([], { ...source, evidence: [], predecessors: [] })).toBeNull();
    // A report with groups still needs its evidence.
    expect(writeTid1500Groups([everyType()], { ...source, evidence: [] })).toBeNull();
  });

  it("makes every UID a 2.25 value below 2^128", () => {
    // Review pass 1, S4. A 2.25 UID is the decimal of a 128-bit UUID,
    // PS3.5 B.2 and ISO/IEC 9834-8, so it is at most 2^128 - 1 and at most
    // 64 characters, PS3.5 9.1.
    const limit = 2n ** 128n;
    const drawn = Array.from({ length: 4000 }, () => newUid());
    for (const uid of drawn) {
      expect(uid).toMatch(/^2\.25\.(0|[1-9][0-9]*)$/u);
      expect(uid.length).toBeLessThanOrEqual(64);
      expect(BigInt(uid.slice(5)) < limit).toBe(true);
    }
    expect(new Set(drawn).size).toBe(drawn.length);
    const loaded = readTid1500Groups(bytesOf([everyType()]));
    for (const uid of [loaded?.sopInstance ?? "", loaded?.series ?? ""]) {
      expect(BigInt(uid.slice(5)) < limit).toBe(true);
    }
  });

  it("writes a unit's meaning when the tree gives one", () => {
    // Review pass 1, N2. CID 7460 gives (mm, UCUM, "millimeter").
    const group = everyType();
    const withMeaning: ContentItem = {
      ...group,
      children: group.children.map((item) =>
        item.valueType === "NUM" ? { ...item, unitMeaning: "millimeter" } : item,
      ),
    };
    const bytes = bytesOf([withMeaning]);
    const measurements = list(natural(bytes).ContentSequence).find(
      (item) => first(item.ConceptNameCodeSequence).CodeValue === "126010",
    );
    const num = list(first(measurements?.ContentSequence).ContentSequence)[3];
    const unit = first(first(num?.MeasuredValueSequence).MeasurementUnitsCodeSequence);
    expect([unit.CodeValue, unit.CodingSchemeDesignator, unit.CodeMeaning]).toEqual(["mm", "UCUM", "millimeter"]);
    expect(readTid1500Groups(bytes)?.groups).toEqual([withMeaning]);
  });

});

// Review pass 2, D1. Another writer's character set. The bytes are changed
// in place, which is safe because every sequence and item this package
// writes has undefined length, PS3.5 7.5.

const COMMENT_TAG = [0x40, 0x00, 0x60, 0xa1, 0x55, 0x54, 0x00, 0x00];

function concat(parts: readonly Uint8Array[]): Uint8Array {
  const total = parts.reduce((sum, part) => sum + part.length, 0);
  const out = new Uint8Array(total);
  let at = 0;
  for (const part of parts) {
    out.set(part, at);
    at += part.length;
  }
  return out;
}

function indexOf(haystack: Uint8Array, needle: readonly number[], from = 0): number {
  outer: for (let at = from; at <= haystack.length - needle.length; at += 1) {
    for (let k = 0; k < needle.length; k += 1) {
      if (haystack[at + k] !== needle[k]) {
        continue outer;
      }
    }
    return at;
  }
  return -1;
}

/**
 * The file with its Specific Character Set value replaced by `set`, padded
 * to an even length with a space, PS3.5 6.2, and the comment whose UTF-8 is
 * `text` stored as `stored` instead.
 */
function recoded(bytes: Uint8Array, set: string, text: string, stored: readonly number[]): Uint8Array {
  const charset = indexOf(bytes, [0x08, 0x00, 0x05, 0x00, 0x43, 0x53]);
  const oldLength = (bytes[charset + 6] ?? 0) | ((bytes[charset + 7] ?? 0) << 8);
  const setBytes = Array.from(set, (char) => char.charCodeAt(0));
  if (setBytes.length % 2 === 1) {
    setBytes.push(0x20);
  }
  const head = concat([
    bytes.slice(0, charset + 6),
    new Uint8Array([setBytes.length & 0xff, setBytes.length >> 8]),
    new Uint8Array(setBytes),
    bytes.slice(charset + 8 + oldLength),
  ]);
  const utf8 = Array.from(new TextEncoder().encode(text));
  let at = indexOf(head, COMMENT_TAG);
  while (at >= 0 && indexOf(head.slice(at + 12, at + 12 + utf8.length), utf8) !== 0) {
    at = indexOf(head, COMMENT_TAG, at + 1);
  }
  if (at < 0) {
    throw new Error("expected the comment");
  }
  const view = new DataView(head.buffer, head.byteOffset);
  const length = view.getUint32(at + 8, true);
  const value = [...stored];
  if (value.length % 2 === 1) {
    value.push(0x20);
  }
  const size = new Uint8Array(4);
  new DataView(size.buffer).setUint32(0, value.length, true);
  return concat([head.slice(0, at + 8), size, new Uint8Array(value), head.slice(at + 12 + length)]);
}

function commentGroup(text: string): ContentItem {
  const group = everyType();
  return {
    ...group,
    children: [
      ...group.children.slice(0, 3),
      { relationship: "CONTAINS", valueType: "TEXT", concept: code("DCM", "121106", "Comment"), text, children: [] },
    ],
  };
}

function commentOf(bytes: Uint8Array): { text: string | null; unread: number } {
  const loaded = readTid1500Groups(bytes);
  const comment = loaded?.groups[0]?.children[3];
  return { text: comment?.valueType === "TEXT" ? comment.text : null, unread: loaded?.unread ?? -1 };
}

/**
 * WHATWG Encoding's windows-1252 index for 0x80 to 0x9F, which is what a
 * browser's TextDecoder("latin1") uses. Node's decoder keeps those bytes as
 * U+0080 to U+009F instead, so a Node test sees a different string from the
 * page unless the browser's decoder is put in its place.
 */
const WINDOWS_1252 = [
  0x20ac, 0x81, 0x201a, 0x192, 0x201e, 0x2026, 0x2020, 0x2021, 0x2c6, 0x2030, 0x160, 0x2039, 0x152, 0x8d, 0x17d, 0x8f,
  0x90, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014, 0x2dc, 0x2122, 0x161, 0x203a, 0x153, 0x9d, 0x17e, 0x178,
];

function asBrowser<T>(run: () => T): T {
  const Real = globalThis.TextDecoder;
  class BrowserDecoder extends Real {
    readonly #windows: boolean;
    constructor(label?: string, options?: TextDecoderOptions) {
      super(label, options);
      this.#windows = this.encoding === "windows-1252";
    }
    override decode(input?: AllowSharedBufferSource, options?: TextDecodeOptions): string {
      if (!this.#windows || input === undefined) {
        return super.decode(input, options);
      }
      const view = ArrayBuffer.isView(input)
        ? new Uint8Array(input.buffer, input.byteOffset, input.byteLength)
        : new Uint8Array(input);
      let out = "";
      for (const byte of view) {
        out += String.fromCharCode(byte >= 0x80 && byte <= 0x9f ? (WINDOWS_1252[byte - 0x80] ?? byte) : byte);
      }
      return out;
    }
  }
  globalThis.TextDecoder = BrowserDecoder;
  try {
    return run();
  } finally {
    globalThis.TextDecoder = Real;
  }
}

describe("text in the file's own character set", () => {
  it("reads ISO_IR 192 text whole under a browser's latin1 decoder", () => {
    // PS3.3 C.12.1.1.2 ISO_IR 192 is UTF-8. "肺" is E8 82 BA, and 0x82 is
    // U+201A in windows-1252.
    const text = "肺 Läsion a\\b";
    const bytes = bytesOf([commentGroup(text)]);
    expect(asBrowser(() => commentOf(bytes))).toEqual({ text, unread: 0 });
    expect(commentOf(bytes)).toEqual({ text, unread: 0 });
  });

  it("reads an ISO_IR 100 file another writer made", () => {
    // ISO_IR 100 is ISO/IEC 8859-1, PS3.3 C.12.1.1.2, so "ä" is the one byte
    // E4 and "é" is E9.
    const written = bytesOf([commentGroup("Läsion é")]);
    const latin = recoded(written, "ISO_IR 100", "Läsion é", [0x4c, 0xe4, 0x73, 0x69, 0x6f, 0x6e, 0x20, 0xe9]);
    expect(commentOf(latin)).toEqual({ text: "Läsion é", unread: 0 });
    expect(asBrowser(() => commentOf(latin))).toEqual({ text: "Läsion é", unread: 0 });
  });

  it("reads an ISO_IR 144 file in Cyrillic", () => {
    // ISO_IR 144 is ISO/IEC 8859-5, where "Д" is B4 and "а" is D0.
    const written = bytesOf([commentGroup("Да")]);
    const cyrillic = recoded(written, "ISO_IR 144", "Да", [0xb4, 0xd0]);
    expect(asBrowser(() => commentOf(cyrillic))).toEqual({ text: "Да", unread: 0 });
  });

  it("counts a group unread under code extensions or an unknown set", () => {
    // ISO 2022 IR 87 needs code extensions, PS3.3 C.12.1.1.2, and the reader
    // does not guess, so the group is unread rather than shown wrong.
    const written = bytesOf([commentGroup("Läsion")]);
    for (const set of ["ISO 2022 IR 87", "ISO_IR 999"]) {
      const coded = recoded(written, set, "Läsion", [0x4c, 0xe4, 0x73, 0x69, 0x6f, 0x6e]);
      expect(commentOf(coded), set).toEqual({ text: null, unread: 1 });
    }
    // The default repertoire, ISO-IR 6, has no byte above 7F.
    const ascii = recoded(written, "", "Läsion", [0x4c, 0xe4, 0x73, 0x69, 0x6f, 0x6e]);
    expect(commentOf(ascii), "default").toEqual({ text: null, unread: 1 });
  });
});

// Review pass 3. More character sets and transfer syntaxes. The files are
// this package's own report, transcoded by dcmjs or changed byte by byte.

function transcoded(bytes: Uint8Array, syntax: string): Uint8Array {
  const copy = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(copy).set(bytes);
  const dict = dcmjs.data.DicomMessage.readFile(copy);
  const meta = dcmjs.data.DicomMetaDictionary.naturalizeDataset(dict.meta);
  meta.TransferSyntaxUID = syntax;
  const next = new dcmjs.data.DicomDict(dcmjs.data.DicomMetaDictionary.denaturalizeDataset(meta));
  next.dict = dict.dict;
  const view = next.write();
  return view instanceof Uint8Array ? view : new Uint8Array(view);
}

/** The file with its (0008,0005) element taken out. */
function withoutCharacterSet(bytes: Uint8Array, bigEndian: boolean): Uint8Array {
  const header = bigEndian ? [0x00, 0x08, 0x00, 0x05, 0x43, 0x53] : [0x08, 0x00, 0x05, 0x00, 0x43, 0x53];
  const at = indexOf(bytes, header, 132);
  const length = bigEndian ? ((bytes[at + 6] ?? 0) << 8) | (bytes[at + 7] ?? 0) : (bytes[at + 6] ?? 0) | ((bytes[at + 7] ?? 0) << 8);
  return concat([bytes.slice(0, at), bytes.slice(at + 8 + length)]);
}

/** The file with its data set deflated, PS3.5 A.5, and the meta naming it. */
async function deflated(bytes: Uint8Array): Promise<Uint8Array> {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const metaEnd = 132 + 12 + view.getUint32(140, true);
  const syntax = indexOf(bytes, [0x02, 0x00, 0x10, 0x00, 0x55, 0x49], 132);
  const oldLength = view.getUint16(syntax + 6, true);
  const name = Array.from("1.2.840.10008.1.2.1.99", (char) => char.charCodeAt(0));
  const meta = concat([
    bytes.slice(0, syntax + 6),
    new Uint8Array([name.length & 0xff, name.length >> 8]),
    new Uint8Array(name),
    bytes.slice(syntax + 8 + oldLength, metaEnd),
  ]);
  new DataView(meta.buffer).setUint32(140, view.getUint32(140, true) + name.length - oldLength, true);
  const stream = new Blob([bytes.slice(metaEnd)]).stream().pipeThrough(new CompressionStream("deflate-raw"));
  const packed = new Uint8Array(await new Response(stream).arrayBuffer());
  return concat([meta, packed]);
}

/** `recoded` for an implicit VR file, whose headers hold no VR, PS3.5 7.1.3. */
function recodedImplicit(bytes: Uint8Array, set: string, placeholder: string, stored: readonly number[]): Uint8Array {
  const charset = indexOf(bytes, [0x08, 0x00, 0x05, 0x00], 132);
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const oldLength = view.getUint32(charset + 4, true);
  const setBytes = Array.from(set, (char) => char.charCodeAt(0));
  if (setBytes.length % 2 === 1) {
    setBytes.push(0x20);
  }
  const size = (count: number): Uint8Array => {
    const out = new Uint8Array(4);
    new DataView(out.buffer).setUint32(0, count, true);
    return out;
  };
  const head = concat([bytes.slice(0, charset + 4), size(setBytes.length), new Uint8Array(setBytes), bytes.slice(charset + 8 + oldLength)]);
  const wanted = Array.from(placeholder, (char) => char.charCodeAt(0));
  let at = indexOf(head, [0x40, 0x00, 0x60, 0xa1]);
  while (at >= 0 && indexOf(head.slice(at + 8, at + 8 + wanted.length), wanted) !== 0) {
    at = indexOf(head, [0x40, 0x00, 0x60, 0xa1], at + 1);
  }
  const length = new DataView(head.buffer).getUint32(at + 4, true);
  const value = [...stored];
  if (value.length % 2 === 1) {
    value.push(0x20);
  }
  return concat([head.slice(0, at + 4), size(value.length), new Uint8Array(value), head.slice(at + 8 + length)]);
}

describe("review pass 3", () => {
  it("reads an ISO_IR 203 file, which dcmjs does not map", () => {
    // D1. ISO_IR 203 is ISO/IEC 8859-15, PS3.3 C.12.1.1.2: "€" is A4, "Œ" BC
    // and "Ÿ" BE.
    const written = bytesOf([commentGroup("E O Y")]);
    const latin9 = recoded(written, "ISO_IR 203", "E O Y", [0xa4, 0x20, 0xbc, 0x20, 0xbe]);
    expect(commentOf(latin9)).toEqual({ text: "€ Œ Ÿ", unread: 0 });
  });

  it("reads an ASCII report from a big endian file and from a deflated one", async () => {
    // D2. Explicit VR Big Endian, PS3.5 A.3, and Deflated Explicit VR Little
    // Endian, A.5. With no Specific Character Set the default repertoire
    // applies, PS3.3 C.12.1.1.2, and needs no decoding.
    const written = bytesOf([commentGroup("plain")]);
    const big = withoutCharacterSet(transcoded(written, "1.2.840.10008.1.2.2"), true);
    expect(commentOf(big)).toEqual({ text: "plain", unread: 0 });
    const packed = await deflated(withoutCharacterSet(written, false));
    expect(commentOf(packed)).toEqual({ text: "plain", unread: 0 });
    // The big endian walk reads the file's own set too.
    expect(commentOf(transcoded(written, "1.2.840.10008.1.2.2"))).toEqual({ text: "plain", unread: 0 });
  });

  it("refuses a C1 byte in a single-byte set", () => {
    // S1. PS3.5 6.1.2: characters from the C1 set shall not be used. 0x85 is
    // NEL in ISO/IEC 8859-1.
    const written = bytesOf([commentGroup("Ab")]);
    const control = recoded(written, "ISO_IR 100", "Ab", [0x41, 0x85]);
    expect(commentOf(control)).toEqual({ text: null, unread: 1 });
  });

  it("refuses a Specific Character Set of more than one value", () => {
    // S1. A second value means code extensions, PS3.3 C.12.1.1.2. The first
    // value empty is the default repertoire as G0, still with extensions.
    const written = bytesOf([commentGroup("plain")]);
    const extended = recoded(written, "\\ISO 2022 IR 87", "plain", Array.from("plain", (char) => char.charCodeAt(0)));
    expect(commentOf(extended)).toEqual({ text: null, unread: 1 });
  });

  it("reads an implicit VR ISO_IR 100 file", () => {
    // S1. Implicit VR Little Endian, PS3.5 A.1, has no VR in its headers.
    const written = transcoded(bytesOf([commentGroup("XXXXXX")]), "1.2.840.10008.1.2");
    const latin = recodedImplicit(written, "ISO_IR 100", "XXXXXX", [0x4c, 0xe4, 0x73, 0x69, 0x6f, 0x6e]);
    expect(commentOf(latin)).toEqual({ text: "Läsion", unread: 0 });
  });

  it("reads ISO_IR 6 as the default repertoire", () => {
    // N2. Table C.12-2 names the default repertoire "none", and other tools
    // write ISO_IR 6 for it.
    const written = bytesOf([commentGroup("plain")]);
    const six = recoded(written, "ISO_IR 6", "plain", Array.from("plain", (char) => char.charCodeAt(0)));
    expect(commentOf(six)).toEqual({ text: "plain", unread: 0 });
  });

  it("refuses the eight positions ISO-IR 166 says shall not be used", () => {
    // N3, corrected in review pass 4, D1. ISO_IR 166 is ISO-IR 166, PS3.3
    // Table C.12-2: 88 characters in a 96-set. The registration names 2/0,
    // byte A0, NO-BREAK SPACE, and says positions 5/11 to 5/14 and 7/12 to
    // 7/15, bytes DB to DE and FC to FF, shall not be used. ISO/IEC 8859-11
    // maps A0 to U+00A0. A1 is the first Thai letter, U+0E01.
    const written = bytesOf([commentGroup("Ab")]);
    expect(commentOf(recoded(written, "ISO_IR 166", "Ab", [0xa1, 0x41]))).toEqual({ text: "\u0e01A", unread: 0 });
    expect(commentOf(recoded(written, "ISO_IR 166", "Ab", [0xa0, 0x41]))).toEqual({ text: "\u00a0A", unread: 0 });
    for (const byte of [0xdb, 0xdc, 0xdd, 0xde, 0xfc, 0xfd, 0xfe, 0xff]) {
      expect(commentOf(recoded(written, "ISO_IR 166", "Ab", [byte, 0x41])), byte.toString(16)).toEqual({ text: null, unread: 1 });
    }
  });

  it("decodes a 200000 byte text", () => {
    // N4. A UT may be up to 2^32 - 2 bytes, PS3.5 6.2.
    const written = bytesOf([commentGroup("Ab")]);
    const long = recoded(written, "ISO_IR 100", "Ab", new Array<number>(200000).fill(0xe4));
    expect(commentOf(long).text).toBe("ä".repeat(200000));
  });

  it("counts a deflated report in a set dcmjs refuses unread, not missing", async () => {
    // Review pass 4, D2. A deflated file, PS3.5 A.5, is not walked, so its
    // set is only what dcmjs reports. dcmjs refuses ISO_IR 203, and the
    // report is still read, with every group unread.
    const written = bytesOf([commentGroup("E O Y")]);
    const latin9 = await deflated(recoded(written, "ISO_IR 203", "E O Y", [0xa4, 0x20, 0xbc, 0x20, 0xbe]));
    expect(commentOf(latin9)).toEqual({ text: null, unread: 1 });
  });

  it("counts a deflated report that names a set unread, even when its text is ASCII", async () => {
    // Review pass 4, S1. The walk cannot confirm a deflated file's set, and
    // dcmjs reports ISO_IR 192 for any set present, so a named set is never
    // taken as the default repertoire, PS3.3 C.12.1.1.2.
    const written = bytesOf([commentGroup("plain")]);
    const named = await deflated(recoded(written, "ISO_IR 100", "plain", Array.from("plain", (char) => char.charCodeAt(0))));
    expect(commentOf(named)).toEqual({ text: null, unread: 1 });
  });
});
