import { describe, expect, it } from "vitest";

import dcmjs from "dcmjs";

import {
  readSecondaryCapture,
  writeSecondaryCapture,
  type SecondaryCaptureRequest,
} from "./secondary-capture.js";

const request: SecondaryCaptureRequest = {
  study: "2.25.21",
  series: "2.25.22",
  instance: "2.25.20",
  columns: 2,
  rows: 2,
  pixels: new Uint8Array([0, 1, 2, 3]),
};

function natural(bytes: Uint8Array): Record<string, unknown> {
  const copy = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(copy).set(bytes);
  const dict = dcmjs.data.DicomMessage.readFile(copy);
  return dcmjs.data.DicomMetaDictionary.naturalizeDataset(dict.dict);
}

describe("Secondary Capture", () => {
  it("reads bytes 0, 1, 2 and 3 back", () => {
    const bytes = writeSecondaryCapture(request);
    expect(bytes).not.toBeNull();
    if (bytes === null) {
      throw new Error("expected bytes");
    }
    const loaded = readSecondaryCapture(bytes);
    expect(loaded).not.toBeNull();
    if (loaded === null) {
      throw new Error("expected pixels");
    }
    expect(loaded.columns).toBe(2);
    expect(loaded.rows).toBe(2);
    expect(Array.from(loaded.pixels)).toEqual([0, 1, 2, 3]);
    const dataset = natural(bytes);
    expect(dataset.SOPClassUID).toBe("1.2.840.10008.5.1.4.1.1.7");
    expect(dataset.PhotometricInterpretation).toBe("MONOCHROME2");
    expect(dataset.BitsAllocated).toBe(8);
    expect(dataset.ConversionType).toBe("WSD");
    expect(dataset.WindowCenter).toBeUndefined();
    expect(dataset.WindowWidth).toBeUndefined();
    expect(dataset.PatientName).toBeUndefined();
    expect(dataset.PatientID).toBeUndefined();
  });

  it("returns nothing for a short pixel buffer", () => {
    expect(
      writeSecondaryCapture({
        ...request,
        pixels: new Uint8Array([0, 1, 2]),
      }),
    ).toBeNull();
    expect(writeSecondaryCapture({ ...request, columns: 0 })).toBeNull();
    expect(writeSecondaryCapture({ ...request, study: "1.2.3" })).toBeNull();
    expect(
      writeSecondaryCapture({
        ...request,
        columns: 65536,
        rows: 1,
        pixels: new Uint8Array(65536),
      }),
    ).toBeNull();
  });

  it("reads an odd 3 by 3 ramp back without the even-length pad", () => {
    const ramp = new Uint8Array([0, 1, 2, 3, 4, 5, 6, 7, 8]);
    const bytes = writeSecondaryCapture({
      ...request,
      columns: 3,
      rows: 3,
      pixels: ramp,
    });
    expect(bytes).not.toBeNull();
    if (bytes === null) {
      throw new Error("expected bytes");
    }
    const loaded = readSecondaryCapture(bytes);
    expect(loaded).not.toBeNull();
    if (loaded === null) {
      throw new Error("expected pixels");
    }
    expect(loaded.columns).toBe(3);
    expect(loaded.rows).toBe(3);
    expect(Array.from(loaded.pixels)).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8]);
  });

  it("returns nothing when the even-length pad is not zero", () => {
    const ramp = new Uint8Array([0, 1, 2, 3, 4, 5, 6, 7, 8]);
    const bytes = writeSecondaryCapture({
      ...request,
      columns: 3,
      rows: 3,
      pixels: ramp,
    });
    expect(bytes).not.toBeNull();
    if (bytes === null) {
      throw new Error("expected bytes");
    }
    const copy = new ArrayBuffer(bytes.byteLength);
    new Uint8Array(copy).set(bytes);
    const dict = dcmjs.data.DicomMessage.readFile(copy);
    const dataset = dcmjs.data.DicomMetaDictionary.naturalizeDataset(dict.dict);
    const stored = dataset.PixelData;
    const buffer = Array.isArray(stored) ? stored[0] : stored;
    if (!(buffer instanceof ArrayBuffer)) {
      throw new Error("expected pixel bytes");
    }
    const pixels = new Uint8Array(buffer);
    const last = pixels.length - 1;
    const pad = pixels[last];
    if (pad === undefined || pad !== 0) {
      throw new Error("expected a zero pad");
    }
    pixels[last] = 1;
    dataset.PixelData = pixels;
    dataset._vrMap = { PixelData: "OB" };
    const rewritten = new dcmjs.data.DicomDict(dict.meta);
    rewritten.dict = dcmjs.data.DicomMetaDictionary.denaturalizeDataset(dataset);
    const view = rewritten.write();
    const next = view instanceof Uint8Array ? view : new Uint8Array(view);
    expect(readSecondaryCapture(next)).toBeNull();
  });

  it("writes the same bytes for the same request", () => {
    const first = writeSecondaryCapture(request);
    const second = writeSecondaryCapture(request);
    expect(first).not.toBeNull();
    expect(Array.from(first ?? [])).toEqual(Array.from(second ?? []));
  });

  it("writes the Type 1 file meta of PS3.10 Table 7.1-1", () => {
    // F-215 review pass 2. File Meta Information Version (0002,0001), OB
    // 00H 01H, and Implementation Class UID (0002,0012).
    const bytes = writeSecondaryCapture(request);
    if (bytes === null) {
      throw new Error("expected bytes");
    }
    const copy = new ArrayBuffer(bytes.byteLength);
    new Uint8Array(copy).set(bytes);
    const meta = dcmjs.data.DicomMessage.readFile(copy).meta as Record<string, { Value: unknown[] } | undefined>;
    const version = meta["00020001"]?.Value[0];
    expect(Array.from(new Uint8Array(version instanceof ArrayBuffer ? version : new ArrayBuffer(0)))).toEqual([0, 1]);
    expect(typeof meta["00020012"]?.Value[0]).toBe("string");
  });
});
