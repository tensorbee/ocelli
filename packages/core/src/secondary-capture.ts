/**
 * One 8-bit MONOCHROME2 Secondary Capture image.
 *
 * PS3.3 A.8.1 is the Secondary Capture Image IOD. The SOP class is
 * 1.2.840.10008.5.1.4.1.1.7. C.7.6.3 is the image pixel module.
 * C.8.6.2 Conversion Type is WSD, a workstation capture.
 * The pixel bytes are the caller's display bytes. This writer does not
 * window and it does not copy a patient module.
 */

import dcmjs from "dcmjs";

import { SECONDARY_CAPTURE_REQUEST_KEYS, refuseUnknownOptions } from "./options.js";

const SOP_CLASS = "1.2.840.10008.5.1.4.1.1.7";
const TRANSFER_SYNTAX = "1.2.840.10008.1.2.1";

export type SecondaryCaptureRequest = {
  study: string;
  series: string;
  instance: string;
  columns: number;
  rows: number;
  pixels: Uint8Array;
};

export type SecondaryCaptureImage = {
  columns: number;
  rows: number;
  pixels: Uint8Array;
};

type RecordValue = Record<string, unknown>;

function isRecord(value: unknown): value is RecordValue {
  return value !== null && typeof value === "object" && !Array.isArray(value);
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

function whole(value: unknown): number | null {
  const text = textField(value);
  if (text === null || !/^[0-9]+$/u.test(text)) {
    return null;
  }
  const parsed = Number(text);
  return Number.isSafeInteger(parsed) ? parsed : null;
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

function pixelBytes(value: unknown): Uint8Array | null {
  if (value instanceof Uint8Array) {
    return new Uint8Array(value);
  }
  if (value instanceof ArrayBuffer) {
    return new Uint8Array(value);
  }
  if (Array.isArray(value) && value.length === 1) {
    return pixelBytes(value[0]);
  }
  return null;
}

const US_MAX = 65535;

function pixelCount(columns: number, rows: number): number | null {
  if (
    !Number.isSafeInteger(columns) ||
    !Number.isSafeInteger(rows) ||
    columns < 1 ||
    rows < 1 ||
    columns > US_MAX ||
    rows > US_MAX
  ) {
    return null;
  }
  return columns * rows;
}

function displayedPixels(columns: number, rows: number, stored: Uint8Array): Uint8Array | null {
  const count = pixelCount(columns, rows);
  if (count === null) {
    return null;
  }
  if (stored.length === count) {
    return stored;
  }
  const pad = stored[count];
  if (count % 2 === 1 && stored.length === count + 1 && pad === 0) {
    return stored.subarray(0, count);
  }
  return null;
}

function writeImage(request: SecondaryCaptureRequest): Uint8Array | null {
  if (
    !syntheticUid(request.study) ||
    !syntheticUid(request.series) ||
    !syntheticUid(request.instance)
  ) {
    return null;
  }
  if (!(request.pixels instanceof Uint8Array)) {
    return null;
  }
  const count = pixelCount(request.columns, request.rows);
  if (count === null || request.pixels.length !== count) {
    return null;
  }
  const dataset = {
    SOPClassUID: SOP_CLASS,
    SOPInstanceUID: request.instance,
    StudyInstanceUID: request.study,
    SeriesInstanceUID: request.series,
    Modality: "OT",
    ConversionType: "WSD",
    InstanceNumber: 1,
    SamplesPerPixel: 1,
    PhotometricInterpretation: "MONOCHROME2",
    Rows: request.rows,
    Columns: request.columns,
    BitsAllocated: 8,
    BitsStored: 8,
    HighBit: 7,
    PixelRepresentation: 0,
    PixelData: new Uint8Array(request.pixels),
    _vrMap: { PixelData: "OB" },
  };
  const meta = dcmjs.data.DicomMetaDictionary.denaturalizeDataset({
    TransferSyntaxUID: TRANSFER_SYNTAX,
    MediaStorageSOPClassUID: SOP_CLASS,
    MediaStorageSOPInstanceUID: request.instance,
  });
  const dict = new dcmjs.data.DicomDict(meta);
  dict.dict = dcmjs.data.DicomMetaDictionary.denaturalizeDataset(dataset);
  return asBytes(dict.write());
}

/** Write one secondary capture. A short pixel buffer returns nothing. */
export function writeSecondaryCapture(request: SecondaryCaptureRequest): Uint8Array | null {
  refuseUnknownOptions("SecondaryCaptureRequest", request, SECONDARY_CAPTURE_REQUEST_KEYS);
  try {
    return writeImage(request);
  } catch {
    return null;
  }
}

function readImage(bytes: Uint8Array | ArrayBuffer): SecondaryCaptureImage | null {
  const dict = dcmjs.data.DicomMessage.readFile(exactBuffer(bytes));
  const dataset = dcmjs.data.DicomMetaDictionary.naturalizeDataset(dict.dict);
  if (!isRecord(dataset)) {
    return null;
  }
  if (textField(dataset.SOPClassUID) !== SOP_CLASS) {
    return null;
  }
  if (textField(dataset.PhotometricInterpretation) !== "MONOCHROME2") {
    return null;
  }
  if (textField(dataset.ConversionType) !== "WSD" || textField(dataset.Modality) !== "OT") {
    return null;
  }
  if (
    whole(dataset.SamplesPerPixel) !== 1 ||
    whole(dataset.BitsAllocated) !== 8 ||
    whole(dataset.BitsStored) !== 8 ||
    whole(dataset.HighBit) !== 7 ||
    whole(dataset.PixelRepresentation) !== 0
  ) {
    return null;
  }
  if (
    dataset.WindowCenter !== undefined ||
    dataset.WindowWidth !== undefined ||
    dataset.PatientName !== undefined ||
    dataset.PatientID !== undefined
  ) {
    return null;
  }
  const columns = whole(dataset.Columns);
  const rows = whole(dataset.Rows);
  const stored = pixelBytes(dataset.PixelData);
  if (columns === null || rows === null || stored === null) {
    return null;
  }
  const pixels = displayedPixels(columns, rows, stored);
  if (pixels === null) {
    return null;
  }
  return { columns, rows, pixels: new Uint8Array(pixels) };
}

/** Read the display bytes back. A windowed or patient-bearing file returns nothing. */
export function readSecondaryCapture(
  bytes: Uint8Array | ArrayBuffer,
): SecondaryCaptureImage | null {
  try {
    return readImage(bytes);
  } catch {
    return null;
  }
}
