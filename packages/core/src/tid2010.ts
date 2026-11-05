/**
 * TID 2010 key-object round trip for one title and one composite.
 *
 * Decision D10 keeps dcmjs for SR generation. PS3.16 TID 2010 requires a
 * coded document title from CID 7010 and at least one referenced object.
 * The caller title is the row 7 text, code (113012, DCM, "Key Object
 * Description"). The coded title stays (113000, DCM, "Of Interest").
 * The referenced object is a COMPOSITE, PS3.16 TID 2010 row 10.
 * The SOP class is Key Object Selection Document
 * 1.2.840.10008.5.1.4.1.1.88.59.
 */

import dcmjs from "dcmjs";

import { KEY_OBJECT_REQUEST_KEYS, refuseUnknownOptions } from "./options.js";

const SOP_CLASS = "1.2.840.10008.5.1.4.1.1.88.59";
const TRANSFER_SYNTAX = "1.2.840.10008.1.2.1";
const TEMPLATE = "2010";
const DOCUMENT_TITLE = "113000";
const DESCRIPTION = "113012";

export type KeyObjectRequest = {
  title: string;
  study: string;
  series: string;
  instance: string;
  referencedClass: string;
  referencedInstance: string;
};

export type KeyObjectDocument = {
  title: string;
  referencedClass: string;
  referencedInstance: string;
};

type RecordValue = Record<string, unknown>;

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

function textField(value: unknown): string | null {
  if (typeof value === "string" && value.length > 0) {
    return value;
  }
  if (typeof value === "number" && Number.isFinite(value)) {
    return String(value);
  }
  if (Array.isArray(value)) {
    return textField(value[0]);
  }
  return null;
}

function syntheticUid(value: string): boolean {
  return value.startsWith("2.25.") && dicomUid(value);
}

function dicomUid(value: string): boolean {
  if (value.length === 0 || value.length > 64 || value.startsWith(".") || value.endsWith(".")) {
    return false;
  }
  if (value.includes("..")) {
    return false;
  }
  const parts = value.split(".");
  for (const part of parts) {
    if (!/^[0-9]+$/u.test(part)) {
      return false;
    }
    if (part.length > 1 && part.startsWith("0")) {
      return false;
    }
  }
  return true;
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

function part10(dataset: RecordValue, sopClass: string, sopInstance: string): Uint8Array {
  const meta = dcmjs.data.DicomMetaDictionary.denaturalizeDataset({
    TransferSyntaxUID: TRANSFER_SYNTAX,
    MediaStorageSOPClassUID: sopClass,
    MediaStorageSOPInstanceUID: sopInstance,
  });
  const dict = new dcmjs.data.DicomDict(meta);
  dict.dict = dcmjs.data.DicomMetaDictionary.denaturalizeDataset(dataset);
  return asBytes(dict.write());
}

function codeIs(record: RecordValue, code: string): boolean {
  const concept = records(record.ConceptNameCodeSequence)[0];
  if (concept === undefined) {
    return false;
  }
  return (
    textField(concept.CodeValue) === code && textField(concept.CodingSchemeDesignator) === "DCM"
  );
}

function writeDocument(request: KeyObjectRequest): Uint8Array | null {
  if (request.title.trim().length === 0) {
    return null;
  }
  if (
    !syntheticUid(request.study) ||
    !syntheticUid(request.series) ||
    !syntheticUid(request.instance) ||
    !dicomUid(request.referencedInstance) ||
    !dicomUid(request.referencedClass)
  ) {
    return null;
  }
  return part10(
    {
      SOPClassUID: SOP_CLASS,
      SOPInstanceUID: request.instance,
      StudyInstanceUID: request.study,
      SeriesInstanceUID: request.series,
      Modality: "KO",
      ValueType: "CONTAINER",
      ContinuityOfContent: "SEPARATE",
      ConceptNameCodeSequence: {
        CodeValue: DOCUMENT_TITLE,
        CodingSchemeDesignator: "DCM",
        CodeMeaning: "Of Interest",
      },
      ContentTemplateSequence: {
        MappingResource: "DCMR",
        TemplateIdentifier: TEMPLATE,
      },
      ContentSequence: [
        {
          RelationshipType: "CONTAINS",
          ValueType: "TEXT",
          ConceptNameCodeSequence: {
            CodeValue: DESCRIPTION,
            CodingSchemeDesignator: "DCM",
            CodeMeaning: "Key Object Description",
          },
          TextValue: request.title,
        },
        {
          RelationshipType: "CONTAINS",
          ValueType: "COMPOSITE",
          ReferencedSOPSequence: {
            ReferencedSOPClassUID: request.referencedClass,
            ReferencedSOPInstanceUID: request.referencedInstance,
          },
        },
      ],
    },
    SOP_CLASS,
    request.instance,
  );
}

/** Write one key-object document. An empty title or UID returns nothing. */
export function writeKeyObject(request: KeyObjectRequest): Uint8Array | null {
  refuseUnknownOptions("KeyObjectRequest", request, KEY_OBJECT_REQUEST_KEYS);
  try {
    return writeDocument(request);
  } catch {
    return null;
  }
}

function readDocument(bytes: Uint8Array | ArrayBuffer): KeyObjectDocument | null {
  const dict = dcmjs.data.DicomMessage.readFile(exactBuffer(bytes));
  const dataset = dcmjs.data.DicomMetaDictionary.naturalizeDataset(dict.dict);
  const template = records(dataset.ContentTemplateSequence)[0];
  if (template === undefined || textField(template.TemplateIdentifier) !== TEMPLATE) {
    return null;
  }
  if (textField(template.MappingResource) !== "DCMR") {
    return null;
  }
  if (textField(dataset.SOPClassUID) !== SOP_CLASS || textField(dataset.Modality) !== "KO") {
    return null;
  }
  if (textField(dataset.ValueType) !== "CONTAINER") {
    return null;
  }
  if (textField(dataset.ContinuityOfContent) !== "SEPARATE" || !codeIs(dataset, DOCUMENT_TITLE)) {
    return null;
  }
  const items = records(dataset.ContentSequence);
  if (items.length !== 2) {
    return null;
  }
  let title: string | null = null;
  let referencedClass: string | null = null;
  let referencedInstance: string | null = null;
  for (const item of items) {
    if (textField(item.RelationshipType) !== "CONTAINS") {
      return null;
    }
    if (textField(item.ValueType) === "TEXT" && codeIs(item, DESCRIPTION)) {
      if (title !== null) {
        return null;
      }
      title = textField(item.TextValue);
    } else if (textField(item.ValueType) === "COMPOSITE") {
      if (referencedClass !== null) {
        return null;
      }
      const referenced = records(item.ReferencedSOPSequence)[0];
      if (referenced === undefined) {
        return null;
      }
      referencedClass = textField(referenced.ReferencedSOPClassUID);
      referencedInstance = textField(referenced.ReferencedSOPInstanceUID);
    } else {
      return null;
    }
  }
  if (
    title === null ||
    referencedClass === null ||
    referencedInstance === null ||
    !dicomUid(referencedClass) ||
    !dicomUid(referencedInstance)
  ) {
    return null;
  }
  return { title, referencedClass, referencedInstance };
}

/** Read one key object. Another template, including TID 1500, returns nothing. */
export function readKeyObject(bytes: Uint8Array | ArrayBuffer): KeyObjectDocument | null {
  try {
    return readDocument(bytes);
  } catch {
    return null;
  }
}
