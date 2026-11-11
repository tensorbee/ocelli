/**
 * TID 2010 key-object round trip for one title and one composite.
 *
 * Decision D10 keeps dcmjs for SR generation. PS3.16 TID 2010 requires a
 * coded document title from CID 7010 and at least one referenced object.
 * The caller title is the row 7 text, code (113012, DCM, "Key Object
 * Description"). The coded title stays (113000, DCM, "Of Interest").
 * The referenced object is an IMAGE item for an image storage class, PS3.16
 * TID 2010 row 8, a WAVEFORM item for a waveform class, row 9, and a
 * COMPOSITE item otherwise, row 10, since F-215.
 * The SOP class is Key Object Selection Document
 * 1.2.840.10008.5.1.4.1.1.88.59.
 */

import dcmjs from "dcmjs";

import { KEY_OBJECT_REQUEST_KEYS, refuseUnknownOptions } from "./options.js";
import { dicomUid, documentModules, newUid, part10, type DocumentReference, type ReportSource } from "./tid1500.js";

const SOP_CLASS = "1.2.840.10008.5.1.4.1.1.88.59";
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

function exactBuffer(bytes: Uint8Array | ArrayBuffer): ArrayBuffer {
  const view = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  const copy = new ArrayBuffer(view.byteLength);
  new Uint8Array(copy).set(view);
  return copy;
}

/**
 * The storage SOP classes a key object references as an IMAGE item, PS3.16
 * TID 2010 row 8. The rule: every class of PS3.4 2026d Table B.5-1 whose
 * IOD Module Table in PS3.3 includes the Image Pixel Module, C.7.6.3, or its
 * floating point form, C.7.6.24, which only Height Map Segmentation uses,
 * then the retired image storage classes of PS3.6 Annex A, whose IODs are no
 * longer in PS3.3. Each entry names the IOD section it was checked against.
 */
const IMAGE_CLASSES = new Set<string>([
  "1.2.840.10008.5.1.4.1.1.1", // Computed Radiography Image Storage, PS3.3 A.2.3
  "1.2.840.10008.5.1.4.1.1.1.1", // Digital X-Ray Image Storage, For Presentation, PS3.3 A.26.3
  "1.2.840.10008.5.1.4.1.1.1.1.1", // Digital X-Ray Image Storage, For Processing, PS3.3 A.26.3
  "1.2.840.10008.5.1.4.1.1.1.2", // Digital Mammography X-Ray Image Storage, For Presentation, PS3.3 A.27.3
  "1.2.840.10008.5.1.4.1.1.1.2.1", // Digital Mammography X-Ray Image Storage, For Processing, PS3.3 A.27.3
  "1.2.840.10008.5.1.4.1.1.1.3", // Digital Intra-Oral X-Ray Image Storage, For Presentation, PS3.3 A.28.3
  "1.2.840.10008.5.1.4.1.1.1.3.1", // Digital Intra-Oral X-Ray Image Storage, For Processing, PS3.3 A.28.3
  "1.2.840.10008.5.1.4.1.1.2", // CT Image Storage, PS3.3 A.3.3
  "1.2.840.10008.5.1.4.1.1.2.3", // CT Image Storage, For Processing, PS3.3 A.3.3
  "1.2.840.10008.5.1.4.1.1.2.1", // Enhanced CT Image Storage, PS3.3 A.38.1.3
  "1.2.840.10008.5.1.4.1.1.2.4", // Enhanced CT Image Storage, For Processing, PS3.3 A.38.1.3
  "1.2.840.10008.5.1.4.1.1.2.2", // Legacy Converted Enhanced CT Image Storage, PS3.3 A.70.3
  "1.2.840.10008.5.1.4.1.1.2.5", // Legacy Converted Enhanced CT Image Storage, For Processing, PS3.3 A.70.3
  "1.2.840.10008.5.1.4.1.1.3.1", // Ultrasound Multi-frame Image Storage, PS3.3 A.7.4
  "1.2.840.10008.5.1.4.1.1.4", // MR Image Storage, PS3.3 A.4.3
  "1.2.840.10008.5.1.4.1.1.4.1", // Enhanced MR Image Storage, PS3.3 A.36.2.3
  "1.2.840.10008.5.1.4.1.1.4.3", // Enhanced MR Color Image Storage, PS3.3 A.36.4.3
  "1.2.840.10008.5.1.4.1.1.4.4", // Legacy Converted Enhanced MR Image Storage, PS3.3 A.71.3
  "1.2.840.10008.5.1.4.1.1.6.1", // Ultrasound Image Storage, PS3.3 A.6.4
  "1.2.840.10008.5.1.4.1.1.6.2", // Enhanced US Volume Storage, PS3.3 A.59.3
  "1.2.840.10008.5.1.4.1.1.6.3", // Photoacoustic Image Storage, PS3.3 A.89.3
  "1.2.840.10008.5.1.4.1.1.7", // Secondary Capture Image Storage, PS3.3 A.8.1.3
  "1.2.840.10008.5.1.4.1.1.7.1", // Multi-frame Single Bit Secondary Capture Image Storage, PS3.3 A.8.2.3
  "1.2.840.10008.5.1.4.1.1.7.2", // Multi-frame Grayscale Byte Secondary Capture Image Storage, PS3.3 A.8.3.3
  "1.2.840.10008.5.1.4.1.1.7.3", // Multi-frame Grayscale Word Secondary Capture Image Storage, PS3.3 A.8.4.3
  "1.2.840.10008.5.1.4.1.1.7.4", // Multi-frame True Color Secondary Capture Image Storage, PS3.3 A.8.5.3
  "1.2.840.10008.5.1.4.1.1.12.1", // X-Ray Angiographic Image Storage, PS3.3 A.14.3
  "1.2.840.10008.5.1.4.1.1.12.1.1", // Enhanced XA Image Storage, PS3.3 A.47.3
  "1.2.840.10008.5.1.4.1.1.12.2", // X-Ray Radiofluoroscopic Image Storage, PS3.3 A.16.3
  "1.2.840.10008.5.1.4.1.1.12.2.1", // Enhanced XRF Image Storage, PS3.3 A.48.3
  "1.2.840.10008.5.1.4.1.1.13.1.1", // X-Ray 3D Angiographic Image Storage, PS3.3 A.53.3
  "1.2.840.10008.5.1.4.1.1.13.1.2", // X-Ray 3D Craniofacial Image Storage, PS3.3 A.54.3
  "1.2.840.10008.5.1.4.1.1.13.1.3", // Breast Tomosynthesis Image Storage, PS3.3 A.55.3
  "1.2.840.10008.5.1.4.1.1.13.1.4", // Breast Projection X-Ray Image Storage, For Presentation, PS3.3 A.74.3
  "1.2.840.10008.5.1.4.1.1.13.1.5", // Breast Projection X-Ray Image Storage, For Processing, PS3.3 A.74.3
  "1.2.840.10008.5.1.4.1.1.14.1", // Intravascular Optical Coherence Tomography Image Storage, For Presentation, PS3.3 A.66.3
  "1.2.840.10008.5.1.4.1.1.14.2", // Intravascular Optical Coherence Tomography Image Storage, For Processing, PS3.3 A.66.3
  "1.2.840.10008.5.1.4.1.1.20", // Nuclear Medicine Image Storage, PS3.3 A.5.4
  "1.2.840.10008.5.1.4.1.1.30", // Parametric Map Storage, PS3.3 A.75.3
  "1.2.840.10008.5.1.4.1.1.66.4", // Segmentation Storage, PS3.3 A.51.3
  "1.2.840.10008.5.1.4.1.1.66.7", // Label Map Segmentation Storage, PS3.3 A.51.3
  "1.2.840.10008.5.1.4.1.1.77.1.1", // VL Endoscopic Image Storage, PS3.3 A.32.1.3
  "1.2.840.10008.5.1.4.1.1.77.1.1.1", // Video Endoscopic Image Storage, PS3.3 A.32.5.3
  "1.2.840.10008.5.1.4.1.1.77.1.2", // VL Microscopic Image Storage, PS3.3 A.32.2.3
  "1.2.840.10008.5.1.4.1.1.77.1.2.1", // Video Microscopic Image Storage, PS3.3 A.32.6.3
  "1.2.840.10008.5.1.4.1.1.77.1.3", // VL Slide-Coordinates Microscopic Image Storage, PS3.3 A.32.3.3
  "1.2.840.10008.5.1.4.1.1.77.1.4", // VL Photographic Image Storage, PS3.3 A.32.4.3
  "1.2.840.10008.5.1.4.1.1.77.1.4.1", // Video Photographic Image Storage, PS3.3 A.32.7.3
  "1.2.840.10008.5.1.4.1.1.77.1.5.1", // Ophthalmic Photography 8 Bit Image Storage, PS3.3 A.41.3
  "1.2.840.10008.5.1.4.1.1.77.1.5.2", // Ophthalmic Photography 16 Bit Image Storage, PS3.3 A.42.3
  "1.2.840.10008.5.1.4.1.1.77.1.5.4", // Ophthalmic Tomography Image Storage, PS3.3 A.52.3
  "1.2.840.10008.5.1.4.1.1.77.1.5.5", // Wide Field Ophthalmic Photography Stereographic Projection Image Storage, PS3.3 A.76.3
  "1.2.840.10008.5.1.4.1.1.77.1.5.6", // Wide Field Ophthalmic Photography 3D Coordinates Image Storage, PS3.3 A.77.3
  "1.2.840.10008.5.1.4.1.1.77.1.5.7", // Ophthalmic Optical Coherence Tomography En Face Image Storage, PS3.3 A.83.3
  "1.2.840.10008.5.1.4.1.1.77.1.5.8", // Ophthalmic Optical Coherence Tomography B-scan Volume Analysis Storage, PS3.3 A.84.3
  "1.2.840.10008.5.1.4.1.1.77.1.6", // VL Whole Slide Microscopy Image Storage, PS3.3 A.32.8.3
  "1.2.840.10008.5.1.4.1.1.77.1.7", // Dermoscopic Photography Image Storage, PS3.3 A.32.11.3
  "1.2.840.10008.5.1.4.1.1.77.1.8", // Confocal Microscopy Image Storage, PS3.3 A.90.1.3
  "1.2.840.10008.5.1.4.1.1.77.1.9", // Confocal Microscopy Tiled Pyramidal Image Storage, PS3.3 A.90.2.3
  "1.2.840.10008.5.1.4.1.1.81.1", // Ophthalmic Thickness Map Storage, PS3.3 A.67.3
  "1.2.840.10008.5.1.4.1.1.82.1", // Corneal Topography Map Storage, PS3.3 A.73.3
  "1.2.840.10008.5.1.4.1.1.128", // Positron Emission Tomography Image Storage, PS3.3 A.21.3
  "1.2.840.10008.5.1.4.1.1.130", // Enhanced PET Image Storage, PS3.3 A.56.3
  "1.2.840.10008.5.1.4.1.1.128.1", // Legacy Converted Enhanced PET Image Storage, PS3.3 A.72.3
  "1.2.840.10008.5.1.4.1.1.481.1", // RT Image Storage, PS3.3 A.17.3
  "1.2.840.10008.5.1.4.1.1.481.2", // RT Dose Storage, PS3.3 A.18.3
  "1.2.840.10008.5.1.4.1.1.481.23", // Enhanced RT Image Storage, PS3.3 A.86.1.15.3
  "1.2.840.10008.5.1.4.1.1.481.24", // Enhanced Continuous RT Image Storage, PS3.3 A.86.1.16.3
  "1.2.840.10008.5.1.4.1.1.66.8", // Height Map Segmentation Storage, PS3.3 A.91.3
  "1.2.840.10008.5.1.1.29", // Hardcopy Grayscale Image Storage (Retired), PS3.6 Annex A
  "1.2.840.10008.5.1.1.30", // Hardcopy Color Image Storage (Retired), PS3.6 Annex A
  "1.2.840.10008.5.1.4.1.1.3", // Ultrasound Multi-frame Image Storage (Retired), PS3.6 Annex A
  "1.2.840.10008.5.1.4.1.1.5", // Nuclear Medicine Image Storage (Retired), PS3.6 Annex A
  "1.2.840.10008.5.1.4.1.1.6", // Ultrasound Image Storage (Retired), PS3.6 Annex A
  "1.2.840.10008.5.1.4.1.1.12.3", // X-Ray Angiographic Bi-Plane Image Storage (Retired), PS3.6 Annex A
  "1.2.840.10008.5.1.4.1.1.77.1", // VL Image Storage, Trial (Retired), PS3.6 Annex A
  "1.2.840.10008.5.1.4.1.1.77.2", // VL Multi-frame Image Storage, Trial (Retired), PS3.6 Annex A
]);

/**
 * The storage SOP classes whose IOD Module Table includes the Waveform
 * Module, C.10.9, which a key object references as a WAVEFORM item, row 9.
 */
const WAVEFORM_CLASSES = new Set<string>([
  "1.2.840.10008.5.1.4.1.1.9.1.1", // 12-lead ECG Waveform Storage, PS3.3 A.34.3.3
  "1.2.840.10008.5.1.4.1.1.9.1.2", // General ECG Waveform Storage, PS3.3 A.34.4.3
  "1.2.840.10008.5.1.4.1.1.9.1.3", // Ambulatory ECG Waveform Storage, PS3.3 A.34.5.3
  "1.2.840.10008.5.1.4.1.1.9.1.4", // General 32-bit ECG Waveform Storage, PS3.3 A.34.18.3
  "1.2.840.10008.5.1.4.1.1.9.2.1", // Hemodynamic Waveform Storage, PS3.3 A.34.6.3
  "1.2.840.10008.5.1.4.1.1.9.3.1", // Cardiac Electrophysiology Waveform Storage, PS3.3 A.34.7.3
  "1.2.840.10008.5.1.4.1.1.9.4.1", // Basic Voice Audio Waveform Storage, PS3.3 A.34.2.3
  "1.2.840.10008.5.1.4.1.1.9.4.2", // General Audio Waveform Storage, PS3.3 A.34.10.3
  "1.2.840.10008.5.1.4.1.1.9.5.1", // Arterial Pulse Waveform Storage, PS3.3 A.34.8.3
  "1.2.840.10008.5.1.4.1.1.9.6.1", // Respiratory Waveform Storage, PS3.3 A.34.9.3
  "1.2.840.10008.5.1.4.1.1.9.6.2", // Multi-channel Respiratory Waveform Storage, PS3.3 A.34.16.3
  "1.2.840.10008.5.1.4.1.1.9.7.1", // Routine Scalp Electroencephalogram Waveform Storage, PS3.3 A.34.12.3
  "1.2.840.10008.5.1.4.1.1.9.7.2", // Electromyogram Waveform Storage, PS3.3 A.34.13.3
  "1.2.840.10008.5.1.4.1.1.9.7.3", // Electrooculogram Waveform Storage, PS3.3 A.34.14.3
  "1.2.840.10008.5.1.4.1.1.9.7.4", // Sleep Electroencephalogram Waveform Storage, PS3.3 A.34.15.3
  "1.2.840.10008.5.1.4.1.1.9.8.1", // Body Position Waveform Storage, PS3.3 A.34.17.3
]);

/** The value type of one reference: row 8, 9 or 10 of TID 2010, PS3.3 C.18.3. */
function itemType(sopClass: string): "IMAGE" | "WAVEFORM" | "COMPOSITE" {
  if (IMAGE_CLASSES.has(sopClass)) {
    return "IMAGE";
  }
  return WAVEFORM_CLASSES.has(sopClass) ? "WAVEFORM" : "COMPOSITE";
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
          ValueType: itemType(request.referencedClass),
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
    } else if (["IMAGE", "WAVEFORM", "COMPOSITE"].includes(textField(item.ValueType) ?? "")) {
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

/** One key image, TID 2010 row 8. `frame` is a Referenced Frame Number counted from 1. */
export type KeyReference = {
  sopClass: string;
  sopInstance: string;
  frame?: number;
};

/** One Key Object Selection read as its references. */
export type KeyObjects = {
  study: string;
  series: string;
  sopInstance: string;
  /** Content Date and Content Time, DA and TM text, PS3.3 C.17.6.2. */
  contentDate: string;
  contentTime: string;
  instanceNumber: number;
  /** Manufacturer, PS3.3 C.7.5.1, empty when absent. */
  manufacturer: string;
  references: readonly KeyReference[];
};

function frameOk(frame: number | undefined): boolean {
  return frame === undefined || (Number.isSafeInteger(frame) && frame >= 1);
}

function writeDocuments(references: readonly KeyReference[], source: ReportSource): Uint8Array | null {
  if (references.length === 0) {
    return null;
  }
  const listed = new Map<string, DocumentReference>();
  for (const reference of source.evidence) {
    listed.set(reference.sopInstance, reference);
  }
  const evidence: DocumentReference[] = [];
  for (const reference of references) {
    const known = listed.get(reference.sopInstance);
    if (known === undefined || !dicomUid(reference.sopClass) || !frameOk(reference.frame)) {
      return null;
    }
    evidence.push(known);
  }
  const sopInstance = newUid();
  const modules = documentModules(source, "KO", SOP_CLASS, sopInstance);
  if (modules === null) {
    return null;
  }
  return part10(
    {
      ...modules,
      CurrentRequestedProcedureEvidenceSequence: evidenceOf(source.study, evidence),
      ValueType: "CONTAINER",
      ContinuityOfContent: "SEPARATE",
      ConceptNameCodeSequence: [
        { CodeValue: DOCUMENT_TITLE, CodingSchemeDesignator: "DCM", CodeMeaning: "Of Interest" },
      ],
      ContentTemplateSequence: [{ MappingResource: "DCMR", TemplateIdentifier: TEMPLATE }],
      ContentSequence: references.map((reference) => {
        const sop: RecordValue = {
          ReferencedSOPClassUID: reference.sopClass,
          ReferencedSOPInstanceUID: reference.sopInstance,
        };
        if (reference.frame !== undefined) {
          sop.ReferencedFrameNumber = reference.frame;
        }
        return { RelationshipType: "CONTAINS", ValueType: itemType(reference.sopClass), ReferencedSOPSequence: [sop] };
      }),
    },
    SOP_CLASS,
    sopInstance,
  );
}

/** The Hierarchical SOP Instance Reference Macro for the referenced instances only. */
function evidenceOf(study: string, references: readonly DocumentReference[]): RecordValue[] {
  const bySeries = new Map<string, RecordValue[]>();
  const seen = new Set<string>();
  for (const reference of references) {
    if (seen.has(reference.sopInstance)) {
      continue;
    }
    seen.add(reference.sopInstance);
    const instances = bySeries.get(reference.series) ?? [];
    instances.push({
      ReferencedSOPClassUID: reference.sopClass,
      ReferencedSOPInstanceUID: reference.sopInstance,
    });
    bySeries.set(reference.series, instances);
  }
  return [
    {
      StudyInstanceUID: study,
      ReferencedSeriesSequence: Array.from(bySeries, ([series, instances]) => ({
        SeriesInstanceUID: series,
        ReferencedSOPSequence: instances,
      })),
    },
  ];
}

/**
 * Write one Key Object Selection listing every reference, an image as a TID
 * 2010 row 8 IMAGE item, a waveform as a row 9 WAVEFORM item and any other
 * object as a row 10 COMPOSITE item.
 * Each reference must be in `source.evidence`, or nothing is written, and
 * Current Requested Procedure Evidence Sequence lists exactly those. The
 * study UID is the user's, so any well-formed UID is accepted.
 * `source.predecessors` is not written, because C.17.6.2 has no such
 * attribute. An empty list or a refused source returns nothing.
 */
export function writeKeyObjects(references: readonly KeyReference[], source: ReportSource): Uint8Array | null {
  try {
    return writeDocuments(references, source);
  } catch {
    return null;
  }
}

function referenceOf(item: RecordValue): KeyReference | null {
  const listed = records(item.ReferencedSOPSequence);
  const sop = listed[0];
  if (listed.length !== 1 || sop === undefined) {
    return null;
  }
  const sopClass = textField(sop.ReferencedSOPClassUID);
  const sopInstance = textField(sop.ReferencedSOPInstanceUID);
  if (sopClass === null || sopInstance === null || !dicomUid(sopClass) || !dicomUid(sopInstance)) {
    return null;
  }
  const frames = sop.ReferencedFrameNumber;
  if (frames === undefined || frames === null || (Array.isArray(frames) && frames.length === 0)) {
    return { sopClass, sopInstance };
  }
  const values = Array.isArray(frames) ? frames : [frames];
  if (values.length !== 1) {
    return null;
  }
  const frame = Number(values[0]);
  return Number.isSafeInteger(frame) && frame >= 1 ? { sopClass, sopInstance, frame } : null;
}

function readDocuments(bytes: Uint8Array | ArrayBuffer): KeyObjects | null {
  const dict = dcmjs.data.DicomMessage.readFile(exactBuffer(bytes));
  const dataset = dcmjs.data.DicomMetaDictionary.naturalizeDataset(dict.dict);
  const template = records(dataset.ContentTemplateSequence)[0];
  if (
    template === undefined ||
    textField(template.TemplateIdentifier) !== TEMPLATE ||
    textField(template.MappingResource) !== "DCMR" ||
    textField(dataset.SOPClassUID) !== SOP_CLASS
  ) {
    return null;
  }
  const study = textField(dataset.StudyInstanceUID);
  const series = textField(dataset.SeriesInstanceUID);
  const sopInstance = textField(dataset.SOPInstanceUID);
  if (study === null || series === null || sopInstance === null) {
    return null;
  }
  const references: KeyReference[] = [];
  for (const item of records(dataset.ContentSequence)) {
    const kind = textField(item.ValueType);
    if (kind !== "IMAGE" && kind !== "WAVEFORM" && kind !== "COMPOSITE") {
      continue;
    }
    const reference = referenceOf(item);
    if (reference === null) {
      return null;
    }
    references.push(reference);
  }
  if (references.length === 0) {
    return null;
  }
  const instance = Number(textField(dataset.InstanceNumber));
  return {
    study,
    series,
    sopInstance,
    contentDate: textField(dataset.ContentDate) ?? "",
    contentTime: textField(dataset.ContentTime) ?? "",
    instanceNumber: Number.isFinite(instance) ? instance : 0,
    manufacturer: textField(dataset.Manufacturer) ?? "",
    references,
  };
}

/**
 * Read every IMAGE, WAVEFORM and COMPOSITE reference of a Key Object Selection.
 * Another template, another SOP class or a malformed reference returns nothing.
 */
export function readKeyObjects(bytes: Uint8Array | ArrayBuffer): KeyObjects | null {
  try {
    return readDocuments(bytes);
  } catch {
    return null;
  }
}
