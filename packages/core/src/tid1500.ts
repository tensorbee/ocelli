/**
 * TID 1500 round trip for one NUM per content-tree root, and since F-215
 * for TID 1410 measurement groups of any value type.
 *
 * Decision D10 keeps dcmjs for SR generation. The HLD does not map a NUM
 * onto MeasuredValueSequence. The local rules are in
 * `.claude/plans/F-097-design.md`.
 */

import dcmjs from "dcmjs";

import { MEASUREMENT_ITEM_LIMIT } from "./measurement-persist.js";
import { contentTree, type ContentItem, type GraphicType, type Relationship } from "./sr.js";

const SOP_CLASS = "1.2.840.10008.5.1.4.1.1.88.34";
const TRANSFER_SYNTAX = "1.2.840.10008.1.2.1";
const TEMPLATE = "1500";

export type TidSource = {
  study: string;
  series: string;
  sopClass: string;
  sopInstance: string;
};

type RecordValue = Record<string, unknown>;

function text(value: string): string | null {
  return value.length > 0 ? value : null;
}

function isRecord(value: unknown): value is RecordValue {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function records(value: unknown): RecordValue[] {
  if (Array.isArray(value)) {
    const found: RecordValue[] = [];
    for (const item of value) {
      if (isRecord(item)) {
        found.push(item);
      }
    }
    return found;
  }
  return isRecord(value) ? [value] : [];
}

function stringField(value: unknown): string | null {
  if (typeof value === "string" && value.length > 0) {
    return value;
  }
  if (Array.isArray(value) && typeof value[0] === "string" && value[0].length > 0) {
    return value[0];
  }
  return null;
}

function countNodes(item: ContentItem): number {
  let count = 1;
  for (const child of item.children) {
    count += countNodes(child);
  }
  return count;
}

function collect(
  item: ContentItem,
  nums: ContentItem[],
  operator: string | null,
): string | null {
  let name = operator;
  if (item.valueType === "NUM") {
    nums.push(item);
  }
  if (
    name === null &&
    item.valueType === "TEXT" &&
    item.concept.scheme === "OCELLI" &&
    item.concept.value === "operator"
  ) {
    name = item.text;
  }
  for (const child of item.children) {
    name = collect(child, nums, name);
  }
  return name;
}

function measurement(num: ContentItem & { valueType: "NUM" }, source: TidSource): unknown {
  return {
    contentItem(): unknown[] {
      return [
        {
          RelationshipType: "CONTAINS",
          ValueType: "NUM",
          ConceptNameCodeSequence: {
            CodeValue: num.concept.value,
            CodingSchemeDesignator: num.concept.scheme,
            CodeMeaning: num.concept.meaning,
          },
          MeasuredValueSequence: {
            NumericValue: num.number,
            MeasurementUnitsCodeSequence: {
              CodeValue: num.unit,
              CodingSchemeDesignator: "UCUM",
              CodeMeaning: num.unit,
            },
          },
        },
      ];
    },
    ReferencedSOPSequence: {
      ReferencedSOPClassUID: source.sopClass,
      ReferencedSOPInstanceUID: source.sopInstance,
    },
  };
}

function asBytes(value: ArrayBuffer | Uint8Array): Uint8Array {
  const view = value instanceof Uint8Array ? value : new Uint8Array(value);
  return new Uint8Array(view);
}

function exactBytes(bytes: Uint8Array | ArrayBuffer): Uint8Array {
  return bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
}

function exactBuffer(bytes: Uint8Array | ArrayBuffer): ArrayBuffer {
  const view = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  const copy = new ArrayBuffer(view.byteLength);
  new Uint8Array(copy).set(view);
  return copy;
}

function writeReport(items: readonly ContentItem[], source: TidSource): Uint8Array | null {
  if (
    text(source.study) === null ||
    text(source.series) === null ||
    text(source.sopClass) === null ||
    text(source.sopInstance) === null
  ) {
    return null;
  }
  const nums: Array<ContentItem & { valueType: "NUM" }> = [];
  let count = 0;
  let operator: string | null = null;
  for (const item of items) {
    const copied = contentTree(item);
    if (copied === null) {
      return null;
    }
    count += countNodes(copied);
    if (count >= MEASUREMENT_ITEM_LIMIT) {
      return null;
    }
    const found: ContentItem[] = [];
    operator = collect(copied, found, operator);
    if (found.length !== 1) {
      return null;
    }
    const num = found[0];
    if (num === undefined || num.valueType !== "NUM") {
      return null;
    }
    nums.push(num);
  }
  const groups = nums.map((num) => {
    return new dcmjs.utilities.TID1500.TID1501MeasurementGroup([measurement(num, source)]);
  });
  const report = new dcmjs.utilities.TID1500.TID1500MeasurementReport({
    TID1501MeasurementGroups: groups,
  });
  const derivation = {
    StudyInstanceUID: source.study,
    SeriesInstanceUID: source.series,
  };
  const shell =
    operator === null
      ? report.contentItem(derivation)
      : report.contentItem(derivation, { PersonName: operator });
  const sopInstance = newUid();
  const dataset = {
    ...shell,
    SOPClassUID: SOP_CLASS,
    SOPInstanceUID: sopInstance,
    StudyInstanceUID: source.study,
    SeriesInstanceUID: newUid(),
    Modality: "SR",
  };
  return part10(dataset, SOP_CLASS, sopInstance);
}

/** Write one Comprehensive 3D SR document. A refused tree returns nothing. */
export function writeTid1500(items: readonly ContentItem[], source: TidSource): Uint8Array | null {
  try {
    return writeReport(items, source);
  } catch {
    return null;
  }
}

function conceptOf(value: unknown): ContentItem["concept"] | null {
  const record = records(value)[0];
  if (record === undefined) {
    return null;
  }
  const scheme = stringField(record.CodingSchemeDesignator);
  const code = stringField(record.CodeValue);
  const meaning = stringField(record.CodeMeaning);
  if (scheme === null || code === null || meaning === null) {
    return null;
  }
  return { scheme, value: code, meaning };
}

function numberOf(value: unknown): number | null {
  if (typeof value === "number" && Number.isFinite(value)) {
    return value;
  }
  if (typeof value === "string" && value.length > 0) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : null;
  }
  const first = Array.isArray(value) ? value[0] : undefined;
  if (typeof first === "number" && Number.isFinite(first)) {
    return first;
  }
  if (typeof first === "string" && first.length > 0) {
    const parsed = Number(first);
    return Number.isFinite(parsed) ? parsed : null;
  }
  return null;
}

function numItem(record: RecordValue): ContentItem | null {
  if (record.ValueType !== "NUM") {
    return null;
  }
  const concept = conceptOf(record.ConceptNameCodeSequence);
  const measured = records(record.MeasuredValueSequence)[0];
  if (concept === null || measured === undefined) {
    return null;
  }
  const number = numberOf(measured.NumericValue);
  const units = records(measured.MeasurementUnitsCodeSequence)[0];
  const unit = units === undefined ? null : stringField(units.CodeValue);
  if (number === null || unit === null) {
    return null;
  }
  return contentTree({
    relationship: null,
    valueType: "NUM",
    concept,
    number,
    unit,
    children: [],
  });
}

function numsUnder(record: RecordValue, found: ContentItem[]): boolean {
  if (record.ValueType === "NUM") {
    const item = numItem(record);
    if (item === null) {
      return false;
    }
    found.push(item);
  }
  for (const child of records(record.ContentSequence)) {
    if (!numsUnder(child, found)) {
      return false;
    }
  }
  return true;
}

function readReport(bytes: Uint8Array | ArrayBuffer): readonly ContentItem[] | null {
  const dict = dcmjs.data.DicomMessage.readFile(exactBuffer(bytes));
  const dataset = dcmjs.data.DicomMetaDictionary.naturalizeDataset(dict.dict);
  const template = records(dataset.ContentTemplateSequence)[0];
  if (template === undefined || stringField(template.TemplateIdentifier) !== TEMPLATE) {
    return null;
  }
  let groups: RecordValue[] | null = null;
  for (const item of records(dataset.ContentSequence)) {
    const concept = records(item.ConceptNameCodeSequence)[0];
    if (concept !== undefined && stringField(concept.CodeValue) === "126010") {
      groups = records(item.ContentSequence);
    }
  }
  if (groups === null) {
    return null;
  }
  const loaded: ContentItem[] = [];
  for (const group of groups) {
    const found: ContentItem[] = [];
    if (!numsUnder(group, found) || found.length !== 1) {
      return null;
    }
    const item = found[0];
    if (item === undefined) {
      return null;
    }
    loaded.push(item);
  }
  if (loaded.length >= MEASUREMENT_ITEM_LIMIT) {
    return null;
  }
  return loaded;
}

/** Read NUM roots from a TID 1500 document. Another template returns nothing. */
export function readTid1500(bytes: Uint8Array | ArrayBuffer): readonly ContentItem[] | null {
  try {
    return readReport(bytes);
  } catch {
    return null;
  }
}

// TID 1500 groups, F-215. PS3.16 TID 1500 row 7 includes TID 1410 groups
// under row 6, and each group is written as the caller's content tree. The
// local rules are in `.claude/plans/F-215-design.md`.

/** One referenced instance, by series. */
export type DocumentReference = {
  series: string;
  sopClass: string;
  sopInstance: string;
};

/**
 * The patient, study and document attributes of one written document.
 *
 * `study`, `patientName`, `patientId`, `studyDate`, `studyTime` and
 * `accession` are copied from the referenced images. `contentDate` and
 * `contentTime` are DA and TM text. `evidence` lists every referenced
 * instance and `predecessors` every document this one replaces.
 */
export type ReportSource = {
  study: string;
  patientName: string;
  patientId: string;
  studyDate: string;
  studyTime: string;
  accession: string;
  contentDate: string;
  contentTime: string;
  evidence: readonly DocumentReference[];
  predecessors: readonly DocumentReference[];
};

/** One TID 1500 document read as measurement groups. */
export type TidGroups = {
  study: string;
  series: string;
  sopClass: string;
  sopInstance: string;
  /** The SOP Instance UIDs Predecessor Documents Sequence names. */
  predecessors: readonly string[];
  /** Each group as a content tree whose root relationship is null. */
  groups: readonly ContentItem[];
  /** Groups holding a value type or relationship the content tree does not have. */
  unread: number;
};

const GROUP = "125007";
const MEASUREMENTS = "126010";
const REPORT_TITLE = "126000";
/** The device observer of every document this package writes, TID 1004 row 1. */
const DEVICE_OBSERVER = "2.25.273392777930776228441174147069498315568";
const MANUFACTURER = "Ocelli";

/** A UID of PS3.5 9.1: digits and single dots, at most 64 characters. */
export function dicomUid(value: string): boolean {
  if (value.length === 0 || value.length > 64 || value.startsWith(".") || value.endsWith(".")) {
    return false;
  }
  for (const part of value.split(".")) {
    if (!/^[0-9]+$/u.test(part) || (part.length > 1 && part.startsWith("0"))) {
      return false;
    }
  }
  return true;
}

const DATE = /^[0-9]{8}$/u;
const TIME = /^[0-9]{2}(?:[0-9]{2}(?:[0-9]{2}(?:\.[0-9]{1,6})?)?)?$/u;

function referenceOk(reference: DocumentReference): boolean {
  return dicomUid(reference.series) && dicomUid(reference.sopClass) && dicomUid(reference.sopInstance);
}

/**
 * A 2.25 UID from 128 random bits, PS3.5 B.2: the decimal of a UUID, so at
 * most 2^128 - 1 and at most 44 characters. The folder viewer's page makes
 * its Tracking Unique Identifiers the same way.
 */
export function newUid(): string {
  const bytes = new Uint8Array(16);
  globalThis.crypto.getRandomValues(bytes);
  let value = 0n;
  for (const byte of bytes) {
    value = (value << 8n) | BigInt(byte);
  }
  return "2.25." + value.toString();
}

/**
 * PS3.10 Table 7.1-1 Implementation Class UID and Implementation Version
 * Name of every file this package writes. A UID fixed at design time, so
 * one value for every installation.
 */
const IMPLEMENTATION_CLASS = "2.25.181940322218151432116404530960524932737";
const IMPLEMENTATION_VERSION = "OCELLI_0_1";

/** `evidence` may be empty only for a document that references no instance. */
function sourceOk(source: ReportSource, evidence = true): boolean {
  return (
    dicomUid(source.study) &&
    DATE.test(source.contentDate) &&
    TIME.test(source.contentTime) &&
    (source.studyDate === "" || DATE.test(source.studyDate)) &&
    (source.studyTime === "" || TIME.test(source.studyTime)) &&
    (!evidence || source.evidence.length > 0) &&
    source.evidence.every(referenceOk) &&
    source.predecessors.every(referenceOk)
  );
}

/** PS3.3 Table 10-4, the Hierarchical SOP Instance Reference Macro, for one study. */
function hierarchy(study: string, references: readonly DocumentReference[]): RecordValue[] {
  const bySeries = new Map<string, RecordValue[]>();
  const seen = new Set<string>();
  for (const reference of references) {
    if (seen.has(reference.sopInstance)) {
      continue;
    }
    seen.add(reference.sopInstance);
    const listed = bySeries.get(reference.series) ?? [];
    listed.push({
      ReferencedSOPClassUID: reference.sopClass,
      ReferencedSOPInstanceUID: reference.sopInstance,
    });
    bySeries.set(reference.series, listed);
  }
  const series = Array.from(bySeries, ([uid, instances]) => ({
    SeriesInstanceUID: uid,
    ReferencedSOPSequence: instances,
  }));
  return [{ StudyInstanceUID: study, ReferencedSeriesSequence: series }];
}

/**
 * The modules every document F-215 writes shares. PS3.3 C.7.1.1 Patient,
 * C.7.2.1 General Study, C.17.1 and C.17.6.1 Series, C.7.5.1 General
 * Equipment and C.12.1 SOP Common. Type 2 attributes with no value are
 * written empty. ISO_IR 192 lets a copied name keep its characters. A
 * source with a malformed UID, date or time, or no evidence, returns nothing.
 */
export function documentModules(
  source: ReportSource,
  modality: "SR" | "KO",
  sopClass: string,
  sopInstance: string,
  evidence = true,
): RecordValue | null {
  if (!sourceOk(source, evidence)) {
    return null;
  }
  return {
    SpecificCharacterSet: "ISO_IR 192",
    SOPClassUID: sopClass,
    SOPInstanceUID: sopInstance,
    PatientName: source.patientName,
    PatientID: source.patientId,
    PatientBirthDate: "",
    PatientSex: "",
    StudyInstanceUID: source.study,
    StudyDate: source.studyDate,
    StudyTime: source.studyTime,
    ReferringPhysicianName: "",
    StudyID: "",
    AccessionNumber: source.accession,
    Modality: modality,
    SeriesInstanceUID: newUid(),
    SeriesNumber: "1",
    ReferencedPerformedProcedureStepSequence: [],
    Manufacturer: MANUFACTURER,
    InstanceNumber: "1",
    ContentDate: source.contentDate,
    ContentTime: source.contentTime,
  };
}

/**
 * Part 10 bytes in Explicit VR Little Endian, with every Type 1 attribute of
 * the file meta, PS3.10 Table 7.1-1. File Meta Information Version is the
 * two bytes 00H 01H.
 */
export function part10(dataset: RecordValue, sopClass: string, sopInstance: string): Uint8Array {
  const meta = dcmjs.data.DicomMetaDictionary.denaturalizeDataset({
    FileMetaInformationVersion: new Uint8Array([0, 1]).buffer,
    ImplementationClassUID: IMPLEMENTATION_CLASS,
    ImplementationVersionName: IMPLEMENTATION_VERSION,
    TransferSyntaxUID: TRANSFER_SYNTAX,
    MediaStorageSOPClassUID: sopClass,
    MediaStorageSOPInstanceUID: sopInstance,
  });
  const dict = new dcmjs.data.DicomDict(meta);
  dict.dict = dcmjs.data.DicomMetaDictionary.denaturalizeDataset(dataset);
  return asBytes(dict.write());
}

function codeRecord(concept: ContentItem["concept"]): RecordValue[] {
  return [
    {
      CodeValue: concept.value,
      CodingSchemeDesignator: concept.scheme,
      CodeMeaning: concept.meaning,
    },
  ];
}

/** PS3.3 C.17.3 Table C.17-6 relationship text: the type with a space for each underscore. */
function relationshipText(item: ContentItem, relationship: string | null): string | null {
  const named = relationship ?? item.relationship;
  return named === null ? null : named.replace(/_/gu, " ");
}

/** One content item as PS3.3 C.17.3 attributes. A root is written with `relationship`. */
function encode(item: ContentItem, relationship: string | null): RecordValue {
  const record: RecordValue = {
    RelationshipType: relationshipText(item, relationship),
    ValueType: item.valueType,
    ConceptNameCodeSequence: codeRecord(item.concept),
  };
  if (item.valueType === "CONTAINER") {
    record.ContinuityOfContent = "SEPARATE";
  } else if (item.valueType === "TEXT") {
    record.TextValue = item.text;
  } else if (item.valueType === "NUM") {
    record.MeasuredValueSequence = [
      {
        NumericValue: item.number,
        MeasurementUnitsCodeSequence: [
          { CodeValue: item.unit, CodingSchemeDesignator: "UCUM", CodeMeaning: item.unitMeaning ?? item.unit },
        ],
      },
    ];
  } else if (item.valueType === "CODE") {
    record.ConceptCodeSequence = codeRecord(item.code);
  } else if (item.valueType === "UIDREF") {
    record.UID = item.uid;
  } else if (item.valueType === "IMAGE") {
    const reference: RecordValue = {
      ReferencedSOPClassUID: item.sopClass,
      ReferencedSOPInstanceUID: item.sopInstance,
    };
    if (item.frame !== undefined) {
      reference.ReferencedFrameNumber = item.frame;
    }
    record.ReferencedSOPSequence = [reference];
  } else {
    record.GraphicType = item.graphic;
    record.GraphicData = item.points.flat();
    if (item.valueType === "SCOORD3D") {
      record.ReferencedFrameOfReferenceUID = item.frameOfReference;
    }
  }
  if (item.children.length > 0) {
    record.ContentSequence = item.children.map((child) => encode(child, null));
  }
  return record;
}

function concept(value: string, meaning: string): RecordValue[] {
  return [{ CodeValue: value, CodingSchemeDesignator: "DCM", CodeMeaning: meaning }];
}

/** TID 1001 row 1, TID 1002 rows 1 and 3, and TID 1004 rows 1 and 2. */
function observerContext(): RecordValue[] {
  return [
    {
      RelationshipType: "HAS OBS CONTEXT",
      ValueType: "CODE",
      ConceptNameCodeSequence: concept("121005", "Observer Type"),
      ConceptCodeSequence: concept("121007", "Device"),
    },
    {
      RelationshipType: "HAS OBS CONTEXT",
      ValueType: "UIDREF",
      ConceptNameCodeSequence: concept("121012", "Device Observer UID"),
      UID: DEVICE_OBSERVER,
    },
    {
      RelationshipType: "HAS OBS CONTEXT",
      ValueType: "TEXT",
      ConceptNameCodeSequence: concept("121013", "Device Observer Name"),
      TextValue: MANUFACTURER,
    },
  ];
}

function writeGroups(groups: readonly ContentItem[], source: ReportSource): Uint8Array | null {
  // An empty row 6 container is written only to replace an earlier document,
  // so a report of no groups must name a predecessor.
  const empty = groups.length === 0;
  if (empty && source.predecessors.length === 0) {
    return null;
  }
  const copied: ContentItem[] = [];
  let count = 0;
  for (const group of groups) {
    const tree = contentTree(group);
    if (tree === null || tree.valueType !== "CONTAINER" || tree.concept.value !== GROUP || tree.concept.scheme !== "DCM") {
      return null;
    }
    count += countNodes(tree);
    if (count >= MEASUREMENT_ITEM_LIMIT) {
      return null;
    }
    copied.push(tree);
  }
  const sopInstance = newUid();
  const modules = documentModules(source, "SR", SOP_CLASS, sopInstance, !empty);
  if (modules === null) {
    return null;
  }
  const dataset: RecordValue = {
    ...modules,
    CompletionFlag: "COMPLETE",
    VerificationFlag: "UNVERIFIED",
    PerformedProcedureCodeSequence: [],
    ValueType: "CONTAINER",
    ConceptNameCodeSequence: concept(REPORT_TITLE, "Imaging Measurement Report"),
    ContinuityOfContent: "SEPARATE",
    ContentTemplateSequence: [{ MappingResource: "DCMR", TemplateIdentifier: TEMPLATE }],
    ContentSequence: [
      ...observerContext(),
      {
        RelationshipType: "CONTAINS",
        ValueType: "CONTAINER",
        ConceptNameCodeSequence: concept(MEASUREMENTS, "Imaging Measurements"),
        ContinuityOfContent: "SEPARATE",
        ContentSequence: copied.map((group) => encode(group, "CONTAINS")),
      },
    ],
  };
  // PS3.3 C.17.2: Current Requested Procedure Evidence Sequence is 1C, there
  // when the content references instances.
  if (source.evidence.length > 0) {
    dataset.CurrentRequestedProcedureEvidenceSequence = hierarchy(source.study, source.evidence);
  }
  if (source.predecessors.length > 0) {
    dataset.PredecessorDocumentsSequence = hierarchy(source.study, source.predecessors);
  }
  return part10(dataset, SOP_CLASS, sopInstance);
}

/**
 * Write one Comprehensive 3D SR whose TID 1500 row 6 container holds one
 * TID 1410 group per root. Each root must be a `(125007, DCM)` container.
 * A refused tree or source returns nothing.
 */
export function writeTid1500Groups(groups: readonly ContentItem[], source: ReportSource): Uint8Array | null {
  try {
    return writeGroups(groups, source);
  } catch {
    return null;
  }
}

const RELATIONSHIP_NAMES: Readonly<Record<string, Relationship>> = {
  CONTAINS: "CONTAINS",
  "HAS OBS CONTEXT": "HAS_OBS_CONTEXT",
  "HAS PROPERTIES": "HAS_PROPERTIES",
  "INFERRED FROM": "INFERRED_FROM",
  "SELECTED FROM": "SELECTED_FROM",
};

const GRAPHIC_TYPES = new Set<string>(["POINT", "POLYLINE", "CIRCLE", "ELLIPSE"]);

function numbersOf(value: unknown): number[] | null {
  const values = Array.isArray(value) ? value : [value];
  const found: number[] = [];
  for (const entry of values) {
    const parsed = typeof entry === "number" ? entry : typeof entry === "string" && entry.length > 0 ? Number(entry) : Number.NaN;
    if (!Number.isFinite(parsed)) {
      return null;
    }
    found.push(parsed);
  }
  return found;
}

function pairs(data: readonly number[], width: number): number[][] | null {
  if (data.length === 0 || data.length % width !== 0) {
    return null;
  }
  const found: number[][] = [];
  for (let at = 0; at < data.length; at += width) {
    found.push(data.slice(at, at + width));
  }
  return found;
}

/**
 * One string of the content tree as the file's Specific Character Set says,
 * PS3.5 6.1, or null when it cannot be decoded.
 */
type Strings = (value: string) => string | null;

/**
 * WHATWG Encoding's windows-1252 index for the bytes 0x80 to 0x9F. dcmjs
 * reads a string inside a sequence item with `TextDecoder("latin1")`, which
 * is windows-1252 in a browser, while Node keeps those bytes as U+0080 to
 * U+009F. Either way every byte has its own image, so the inverse is exact.
 */
const WINDOWS_1252_HIGH = [
  0x20ac, 0x81, 0x201a, 0x192, 0x201e, 0x2026, 0x2020, 0x2021, 0x2c6, 0x2030, 0x160, 0x2039, 0x152, 0x8d, 0x17d, 0x8f,
  0x90, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014, 0x2dc, 0x2122, 0x161, 0x203a, 0x153, 0x9d, 0x17e, 0x178,
];

const BYTE_OF = new Map<number, number>();
for (let byte = 0; byte < 0x100; byte += 1) {
  BYTE_OF.set(byte, byte);
}
WINDOWS_1252_HIGH.forEach((code, at) => {
  BYTE_OF.set(code, 0x80 + at);
});

/** The bytes dcmjs decoded `value` from, or null for a character no byte decodes to. */
function bytesOf(value: string): Uint8Array | null {
  const bytes = new Uint8Array(value.length);
  for (let at = 0; at < value.length; at += 1) {
    const byte = BYTE_OF.get(value.charCodeAt(at));
    if (byte === undefined) {
      return null;
    }
    bytes[at] = byte;
  }
  return bytes;
}

/**
 * PS3.3 C.12.1.1.2. The single-byte sets without code extensions, each a
 * G0 of ISO-IR 6 and a G1, by the TextDecoder label of that G1. The G1 is
 * 96 characters except ISO_IR 166's, TIS 620-2533, which is 88.
 * ISO_IR 100 is decoded byte for byte, because WHATWG's "iso-8859-1" is
 * windows-1252. ISO_IR 13 is left out: its G0 is JIS X 0201 Romaji, not
 * ISO-IR 6.
 */
const SINGLE_BYTE: Readonly<Record<string, string>> = {
  "ISO_IR 100": "latin1",
  "ISO_IR 101": "iso-8859-2",
  "ISO_IR 109": "iso-8859-3",
  "ISO_IR 110": "iso-8859-4",
  "ISO_IR 144": "iso-8859-5",
  "ISO_IR 127": "iso-8859-6",
  "ISO_IR 126": "iso-8859-7",
  "ISO_IR 138": "iso-8859-8",
  "ISO_IR 148": "iso-8859-9",
  "ISO_IR 203": "iso-8859-15",
  "ISO_IR 166": "windows-874",
};

/** The multi-byte sets without code extensions, PS3.3 C.12.1.1.2. */
const MULTI_BYTE: Readonly<Record<string, string>> = {
  "ISO_IR 192": "utf-8",
  GB18030: "gb18030",
  GBK: "gbk",
};

/**
 * The G1 positions ISO-IR 166 says shall not be used, 5/11 to 5/14 and 7/12
 * to 7/15, which are the bytes DB to DE and FC to FF, and which windows-874
 * maps to private use code points. Byte A0, position 2/0, is NO-BREAK
 * SPACE, one of the set's 88 characters, as ISO/IEC 8859-11 also has it.
 */
function tis620Undefined(byte: number): boolean {
  return (byte >= 0xdb && byte <= 0xde) || byte >= 0xfc;
}

/** Bytes as the code points of the same value, built in chunks, so a long value does not overflow the call stack. */
function latin1(bytes: Uint8Array): string {
  let out = "";
  for (let at = 0; at < bytes.length; at += 8192) {
    out += String.fromCharCode(...bytes.subarray(at, at + 8192));
  }
  return out;
}

function strict(label: string, bytes: Uint8Array): string | null {
  try {
    return new TextDecoder(label, { fatal: true }).decode(bytes);
  } catch {
    return null;
  }
}

/**
 * The decoder for one Specific Character Set, or null for a set this reader
 * does not decode: code extensions, a value it does not know, or a file whose
 * (0008,0005) could not be read.
 */
function stringsFor(set: readonly string[] | null): Strings | null {
  if (set === null || set.length > 1) {
    return null;
  }
  const term = set[0] ?? "";
  // Table C.12-2 names the default repertoire "none", and other writers put
  // ISO_IR 6 in the element for it.
  if (term === "" || term === "ISO_IR 6") {
    // The default repertoire, ISO-IR 6, holds no byte above 7F.
    return (value) => {
      const bytes = bytesOf(value);
      return bytes === null || bytes.some((byte) => byte > 0x7f) ? null : value;
    };
  }
  const single = SINGLE_BYTE[term];
  if (single !== undefined) {
    return (value) => {
      const bytes = bytesOf(value);
      // 0x80 to 0x9F are C1 controls, which PS3.5 6.1.2 says shall not be used.
      if (bytes === null || bytes.some((byte) => byte >= 0x80 && byte <= 0x9f)) {
        return null;
      }
      if (term === "ISO_IR 166" && bytes.some(tis620Undefined)) {
        return null;
      }
      return single === "latin1" ? latin1(bytes) : strict(single, bytes);
    };
  }
  const multi = MULTI_BYTE[term];
  if (multi !== undefined) {
    return (value) => {
      const bytes = bytesOf(value);
      return bytes === null ? null : strict(multi, bytes);
    };
  }
  return null;
}

/** Long-length explicit VRs, PS3.5 Table 7.1-1. */
const LONG_VRS = new Set(["OB", "OD", "OF", "OL", "OV", "OW", "SQ", "SV", "UC", "UN", "UR", "UT", "UV"]);

/**
 * The file's own Specific Character Set (0008,0005), read from its bytes,
 * because dcmjs rewrites the parsed value to ISO_IR 192 whenever the element
 * is present. An empty list is an absent element. Null is a file this reader
 * cannot walk to that element, such as a deflated one, PS3.5 A.5.
 */
function characterSetOf(raw: Uint8Array): string[] | null {
  const view = new DataView(raw.buffer, raw.byteOffset, raw.byteLength);
  if (raw.length < 132 || String.fromCharCode(...raw.subarray(128, 132)) !== "DICM") {
    return null;
  }
  let at = 132;
  let syntax = "";
  // The file meta is always explicit VR little endian, PS3.10 7.1.
  while (at + 8 <= raw.length && view.getUint16(at, true) === 0x0002) {
    const element = view.getUint16(at + 2, true);
    const vr = String.fromCharCode(raw[at + 4] ?? 0, raw[at + 5] ?? 0);
    const long = LONG_VRS.has(vr);
    const length = long ? view.getUint32(at + 8, true) : view.getUint16(at + 6, true);
    const value = at + (long ? 12 : 8);
    if (element === 0x0010) {
      syntax = String.fromCharCode(...raw.subarray(value, value + length)).replace(/[\0 ]+$/u, "");
    }
    at = value + length;
  }
  // PS3.5 A.1 Implicit VR Little Endian, A.2 Explicit VR Little Endian and
  // A.3 Explicit VR Big Endian, where only the byte order differs.
  const implicit = syntax === "1.2.840.10008.1.2";
  const little = syntax !== "1.2.840.10008.1.2.2";
  if (!implicit && little && syntax !== "1.2.840.10008.1.2.1") {
    return null;
  }
  while (at + 8 <= raw.length) {
    const group = view.getUint16(at, little);
    const element = view.getUint16(at + 2, little);
    if (group > 0x0008 || (group === 0x0008 && element > 0x0005)) {
      return [];
    }
    let length: number;
    let value: number;
    if (implicit) {
      length = view.getUint32(at + 4, true);
      value = at + 8;
    } else {
      const vr = String.fromCharCode(raw[at + 4] ?? 0, raw[at + 5] ?? 0);
      const long = LONG_VRS.has(vr);
      length = long ? view.getUint32(at + 8, little) : view.getUint16(at + 6, little);
      value = at + (long ? 12 : 8);
    }
    if (length === 0xffffffff) {
      return null;
    }
    if (group === 0x0008 && element === 0x0005) {
      const text = String.fromCharCode(...raw.subarray(value, value + length));
      return text.split("\\").map((part) => part.trim());
    }
    at = value + length;
  }
  return [];
}

/** A string field of one or more values, rejoined with the backslash dcmjs split it on. */
function joinedField(value: unknown): string | null {
  if (typeof value === "string") {
    return value;
  }
  if (Array.isArray(value) && value.length > 0 && value.every((part) => typeof part === "string")) {
    return value.join("\\");
  }
  return null;
}

function textIn(value: unknown, strings: Strings): string | null {
  const found = joinedField(value);
  return found === null || found.length === 0 ? null : strings(found);
}

function conceptIn(value: unknown, strings: Strings): ContentItem["concept"] | null {
  const record = records(value)[0];
  if (record === undefined) {
    return null;
  }
  const scheme = textIn(record.CodingSchemeDesignator, strings);
  const code = textIn(record.CodeValue, strings);
  const meaning = textIn(record.CodeMeaning, strings);
  if (scheme === null || code === null || meaning === null) {
    return null;
  }
  return { scheme, value: code, meaning };
}

/**
 * A TextValue. UT holds one value in which a backslash is ordinary text,
 * PS3.5 6.2, so the values dcmjs split it into are joined with a backslash.
 */
function textValueOf(value: unknown, strings: Strings): string | null {
  const found = joinedField(value);
  return found === null ? null : strings(found);
}

/** One record as a content item, or null for anything the content tree does not hold. */
function decode(record: RecordValue, root: boolean, strings: Strings): ContentItem | null {
  let relationship: Relationship | null = null;
  if (!root) {
    const named = stringField(record.RelationshipType);
    relationship = named === null ? null : (RELATIONSHIP_NAMES[named] ?? null);
    if (relationship === null) {
      return null;
    }
  }
  const coded = conceptIn(record.ConceptNameCodeSequence, strings);
  if (coded === null) {
    return null;
  }
  const children: ContentItem[] = [];
  for (const child of records(record.ContentSequence)) {
    const decoded = decode(child, false, strings);
    if (decoded === null) {
      return null;
    }
    children.push(decoded);
  }
  const base = { relationship, concept: coded, children };
  const valueType = stringField(record.ValueType);
  if (valueType === "CONTAINER") {
    return { ...base, valueType };
  }
  if (valueType === "TEXT") {
    const text = textValueOf(record.TextValue, strings);
    return text === null ? null : { ...base, valueType, text };
  }
  if (valueType === "NUM") {
    const measured = records(record.MeasuredValueSequence)[0];
    const number = measured === undefined ? null : numberOf(measured.NumericValue);
    const units = measured === undefined ? undefined : records(measured.MeasurementUnitsCodeSequence)[0];
    const unit = units === undefined ? null : textIn(units.CodeValue, strings);
    const meaning = units === undefined ? null : textIn(units.CodeMeaning, strings);
    if (number === null || unit === null) {
      return null;
    }
    return meaning === null || meaning === unit
      ? { ...base, valueType, number, unit }
      : { ...base, valueType, number, unit, unitMeaning: meaning };
  }
  if (valueType === "CODE") {
    const code = conceptIn(record.ConceptCodeSequence, strings);
    return code === null ? null : { ...base, valueType, code };
  }
  if (valueType === "UIDREF") {
    const uid = stringField(record.UID);
    return uid === null ? null : { ...base, valueType, uid };
  }
  if (valueType === "IMAGE") {
    const listed = records(record.ReferencedSOPSequence);
    const reference = listed[0];
    if (listed.length !== 1 || reference === undefined) {
      return null;
    }
    const sopClass = stringField(reference.ReferencedSOPClassUID);
    const sopInstance = stringField(reference.ReferencedSOPInstanceUID);
    if (sopClass === null || sopInstance === null) {
      return null;
    }
    if (reference.ReferencedFrameNumber === undefined || reference.ReferencedFrameNumber === null) {
      return { ...base, valueType, sopClass, sopInstance };
    }
    const frames = numbersOf(reference.ReferencedFrameNumber);
    const frame = frames !== null && frames.length === 1 ? frames[0] : undefined;
    return frame === undefined ? null : { ...base, valueType, sopClass, sopInstance, frame };
  }
  if (valueType === "SCOORD" || valueType === "SCOORD3D") {
    const graphic = stringField(record.GraphicType);
    const data = numbersOf(record.GraphicData);
    if (graphic === null || !GRAPHIC_TYPES.has(graphic) || data === null) {
      return null;
    }
    const points = pairs(data, valueType === "SCOORD" ? 2 : 3);
    if (points === null) {
      return null;
    }
    const typed = graphic as GraphicType;
    if (valueType === "SCOORD") {
      return { ...base, valueType, graphic: typed, points };
    }
    const frameOfReference = stringField(record.ReferencedFrameOfReferenceUID);
    return frameOfReference === null ? null : { ...base, valueType, graphic: typed, points, frameOfReference };
  }
  return null;
}

/** The SR storage classes whose content is a TID 1500 tree this reader accepts. */
const SR_CLASSES = new Set<string>([
  "1.2.840.10008.5.1.4.1.1.88.22",
  "1.2.840.10008.5.1.4.1.1.88.33",
  "1.2.840.10008.5.1.4.1.1.88.34",
]);

/**
 * The parsed file. dcmjs refuses a character set it does not map, such as
 * ISO_IR 203, and more than one value. This reader never uses dcmjs's decode
 * of a content-tree string, so a file is always parsed past that refusal,
 * review pass 4, D2. A truncated file still returns nothing. A file damaged
 * inside its content can read as fewer groups, with or without the retry,
 * review pass 5, D2.
 */
function parsed(bytes: Uint8Array | ArrayBuffer): { dict: unknown } {
  try {
    return dcmjs.data.DicomMessage.readFile(exactBuffer(bytes));
  } catch {
    return dcmjs.data.DicomMessage.readFile(exactBuffer(bytes), { ignoreErrors: true });
  }
}

function readGroups(bytes: Uint8Array | ArrayBuffer): TidGroups | null {
  const set = characterSetOf(exactBytes(bytes));
  const dict = parsed(bytes);
  const dataset = dcmjs.data.DicomMetaDictionary.naturalizeDataset(dict.dict);
  // Review pass 3, D2. A file the walk cannot read, such as a deflated one,
  // whose parsed data set has no Specific Character Set, holds the default
  // repertoire, because dcmjs adds that element only when the file has it.
  const strings = set !== null ? stringsFor(set) : dataset.SpecificCharacterSet === undefined ? stringsFor([]) : null;
  const template = records(dataset.ContentTemplateSequence)[0];
  if (template === undefined || stringField(template.TemplateIdentifier) !== TEMPLATE) {
    return null;
  }
  const sopClass = stringField(dataset.SOPClassUID);
  const sopInstance = stringField(dataset.SOPInstanceUID);
  const study = stringField(dataset.StudyInstanceUID);
  const series = stringField(dataset.SeriesInstanceUID);
  if (sopClass === null || !SR_CLASSES.has(sopClass) || sopInstance === null || study === null || series === null) {
    return null;
  }
  const predecessors: string[] = [];
  for (const studyItem of records(dataset.PredecessorDocumentsSequence)) {
    for (const seriesItem of records(studyItem.ReferencedSeriesSequence)) {
      for (const instance of records(seriesItem.ReferencedSOPSequence)) {
        const uid = stringField(instance.ReferencedSOPInstanceUID);
        if (uid !== null) {
          predecessors.push(uid);
        }
      }
    }
  }
  // Review pass 2, D1. A set this reader does not decode counts every group
  // unread rather than showing a guess.
  const groups: ContentItem[] = [];
  let unread = 0;
  let found = false;
  for (const item of records(dataset.ContentSequence)) {
    const named = records(item.ConceptNameCodeSequence)[0];
    if (named === undefined || stringField(named.CodeValue) !== MEASUREMENTS) {
      continue;
    }
    found = true;
    for (const group of records(item.ContentSequence)) {
      const decoded = strings === null ? null : decode(group, true, strings);
      const copied = decoded === null ? null : contentTree(decoded);
      if (copied === null || copied.valueType !== "CONTAINER" || copied.concept.value !== GROUP) {
        unread += 1;
      } else {
        groups.push(copied);
      }
    }
  }
  if (!found || groups.length + unread >= MEASUREMENT_ITEM_LIMIT) {
    return null;
  }
  return { study, series, sopClass, sopInstance, predecessors, groups, unread };
}

/**
 * Read every TID 1410 or TID 1501 group of a TID 1500 document. A group
 * holding something the content tree cannot is counted in `unread`.
 * Another template or a file that is not an SR returns nothing.
 */
export function readTid1500Groups(bytes: Uint8Array | ArrayBuffer): TidGroups | null {
  try {
    return readGroups(bytes);
  } catch {
    return null;
  }
}
