/**
 * TID 1500 round trip for one NUM per content-tree root.
 *
 * Decision D10 keeps dcmjs for SR generation. The HLD does not map a NUM
 * onto MeasuredValueSequence. The local rules are in
 * `.claude/plans/F-097-design.md`.
 */

import dcmjs from "dcmjs";

import { MEASUREMENT_ITEM_LIMIT } from "./measurement-persist.js";
import { contentTree, type ContentItem } from "./sr.js";

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
  const sopInstance = dcmjs.data.DicomMetaDictionary.uid();
  const dataset = {
    ...shell,
    SOPClassUID: SOP_CLASS,
    SOPInstanceUID: sopInstance,
    StudyInstanceUID: source.study,
    SeriesInstanceUID: dcmjs.data.DicomMetaDictionary.uid(),
    Modality: "SR",
  };
  const meta = dcmjs.data.DicomMetaDictionary.denaturalizeDataset({
    TransferSyntaxUID: TRANSFER_SYNTAX,
    MediaStorageSOPClassUID: SOP_CLASS,
    MediaStorageSOPInstanceUID: sopInstance,
  });
  const dict = new dcmjs.data.DicomDict(meta);
  dict.dict = dcmjs.data.DicomMetaDictionary.denaturalizeDataset(dataset);
  return asBytes(dict.write());
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
