/** HTTP transport for the pure Rust DICOMweb response parser. */

import {
  writeDicomwebResponse,
  type DicomwebResponseKind,
  type DicomwebResponseSink,
  type WasmMemory,
} from "./bulk.js";

export type DicomwebErrorCode =
  | "InvalidRequest"
  | "Network"
  | "Aborted"
  | "HttpStatus"
  | "ContentType";

/** A safe transport failure that never retains a URL, header, or body. */
export class DicomwebError extends Error {
  readonly code: DicomwebErrorCode;
  readonly status?: number;

  constructor(code: DicomwebErrorCode, status?: number) {
    super(messageFor(code));
    this.name = "DicomwebError";
    this.code = code;
    if (status !== undefined) {
      this.status = status;
    }
  }
}

export interface QidoSearchOptions {
  readonly filters?: ReadonlyArray<readonly [string, string]>;
  readonly includeFields?: readonly string[];
  readonly limit?: number;
  readonly offset?: number;
  readonly fuzzyMatching?: boolean;
  readonly signal?: AbortSignal;
}

export interface DicomwebClientOptions {
  readonly studiesServiceUrl: string;
  readonly wadoUriUrl: string;
  readonly fetch: typeof fetch;
  readonly wasm: WasmMemory;
  readonly sink: DicomwebResponseSink;
}

interface ParsedContentType {
  readonly base: string;
  readonly parameters: ReadonlyMap<string, string>;
}

/** A bounded QIDO-RS, WADO-RS, and WADO-URI GET client. */
export class DicomwebClient {
  readonly #studiesServiceUrl: URL;
  readonly #wadoUriUrl: URL;
  readonly #fetch: typeof fetch;
  readonly #wasm: WasmMemory;
  readonly #sink: DicomwebResponseSink;

  constructor(options: DicomwebClientOptions) {
    this.#studiesServiceUrl = safeUrl(options.studiesServiceUrl);
    this.#wadoUriUrl = safeUrl(options.wadoUriUrl);
    this.#fetch = options.fetch;
    this.#wasm = options.wasm;
    this.#sink = options.sink;
  }

  searchStudies(options: QidoSearchOptions = {}): Promise<void> {
    return this.#qido(["studies"], options);
  }

  async searchSeries(
    studyUid: string,
    options: QidoSearchOptions = {},
  ): Promise<void> {
    if (!validDicomUid(studyUid)) {
      throw new DicomwebError("InvalidRequest");
    }
    await this.#qido(["studies", studyUid, "series"], options);
  }

  async searchInstances(
    studyUid: string,
    seriesUid: string,
    options: QidoSearchOptions = {},
  ): Promise<void> {
    if (!validDicomUid(studyUid) || !validDicomUid(seriesUid)) {
      throw new DicomwebError("InvalidRequest");
    }
    await this.#qido(
      ["studies", studyUid, "series", seriesUid, "instances"],
      options,
    );
  }

  async retrieveInstance(
    studyUid: string,
    seriesUid: string,
    instanceUid: string,
    signal?: AbortSignal,
  ): Promise<void> {
    if (
      !validDicomUid(studyUid) ||
      !validDicomUid(seriesUid) ||
      !validDicomUid(instanceUid)
    ) {
      throw new DicomwebError("InvalidRequest");
    }
    const url = this.#resourceUrl([
      "studies",
      studyUid,
      "series",
      seriesUid,
      "instances",
      instanceUid,
    ]);
    const response = await this.#get(
      url,
      'multipart/related; type="application/dicom"; transfer-syntax=*',
      signal,
    );
    const boundary = requireMultipart(response, "application/dicom");
    await this.#commit(response, { type: "wado-rs-instances", boundary }, signal);
  }

  async retrieveFrames(
    studyUid: string,
    seriesUid: string,
    instanceUid: string,
    frames: readonly number[],
    mediaType: string,
    signal?: AbortSignal,
  ): Promise<void> {
    if (
      !validDicomUid(studyUid) ||
      !validDicomUid(seriesUid) ||
      !validDicomUid(instanceUid) ||
      frames.length === 0 ||
      frames.some((frame) => !Number.isSafeInteger(frame) || frame < 1) ||
      frames.some((frame, index) => index > 0 && frame <= (frames[index - 1] ?? 0)) ||
      !validMediaType(mediaType)
    ) {
      throw new DicomwebError("InvalidRequest");
    }
    const url = this.#resourceUrl([
      "studies",
      studyUid,
      "series",
      seriesUid,
      "instances",
      instanceUid,
    ]);
    url.pathname = `${url.pathname}/frames/${frames.join(",")}`;
    const response = await this.#get(
      url,
      `multipart/related; type="${mediaType}"`,
      signal,
    );
    const boundary = requireMultipart(response, mediaType);
    await this.#commit(
      response,
      {
        type: "wado-rs-frames",
        boundary,
        mediaType,
      },
      signal,
    );
  }

  async retrieveWadoUri(
    studyUid: string,
    seriesUid: string,
    objectUid: string,
    signal?: AbortSignal,
  ): Promise<void> {
    if (
      !validDicomUid(studyUid) ||
      !validDicomUid(seriesUid) ||
      !validDicomUid(objectUid)
    ) {
      throw new DicomwebError("InvalidRequest");
    }
    const url = new URL(this.#wadoUriUrl);
    url.searchParams.set("requestType", "WADO");
    url.searchParams.set("studyUID", studyUid);
    url.searchParams.set("seriesUID", seriesUid);
    url.searchParams.set("objectUID", objectUid);
    url.searchParams.set("contentType", "application/dicom");
    const response = await this.#get(url, "application/dicom", signal);
    requireSimple(response, "application/dicom");
    await this.#commit(response, { type: "wado-uri-part10" }, signal);
  }

  async #qido(path: readonly string[], options: QidoSearchOptions): Promise<void> {
    if (!validUint(options.limit) || !validUint(options.offset)) {
      throw new DicomwebError("InvalidRequest");
    }
    const url = this.#resourceUrl(path);
    const matchingAttributes = new Set<string>();
    for (const [key, value] of options.filters ?? []) {
      const normalized = key.toLowerCase();
      if (
        [
          "includefield",
          "limit",
          "offset",
          "fuzzymatching",
          "emptyvaluematching",
          "multiplevaluematching",
        ].includes(normalized) ||
        matchingAttributes.has(normalized)
      ) {
        throw new DicomwebError("InvalidRequest");
      }
      matchingAttributes.add(normalized);
      url.searchParams.append(key, value);
    }
    for (const field of options.includeFields ?? []) {
      url.searchParams.append("includefield", field);
    }
    if (options.limit !== undefined) {
      url.searchParams.set("limit", String(options.limit));
    }
    if (options.offset !== undefined) {
      url.searchParams.set("offset", String(options.offset));
    }
    if (options.fuzzyMatching !== undefined) {
      url.searchParams.set("fuzzymatching", String(options.fuzzyMatching));
    }
    const response = await this.#get(
      url,
      "application/dicom+json",
      options.signal,
    );
    requireSimple(response, "application/dicom+json");
    await this.#commit(response, { type: "qido-json" }, options.signal);
  }

  #resourceUrl(segments: readonly string[]): URL {
    const url = new URL(this.#studiesServiceUrl);
    const prefix = url.pathname.endsWith("/")
      ? url.pathname.slice(0, -1)
      : url.pathname;
    url.pathname = `${prefix}/${segments.map(encodeURIComponent).join("/")}`;
    url.search = "";
    url.hash = "";
    return url;
  }

  async #get(url: URL, accept: string, signal?: AbortSignal): Promise<Response> {
    let response: Response;
    try {
      response = await this.#fetch(url, {
        method: "GET",
        headers: { Accept: accept },
        ...(signal === undefined ? {} : { signal }),
      });
    } catch (error: unknown) {
      if (
        signal?.aborted === true ||
        (error instanceof DOMException && error.name === "AbortError")
      ) {
        throw new DicomwebError("Aborted");
      }
      throw new DicomwebError("Network");
    }
    if (!response.ok) {
      throw new DicomwebError("HttpStatus", response.status);
    }
    return response;
  }

  async #commit(
    response: Response,
    kind: DicomwebResponseKind,
    signal?: AbortSignal,
  ): Promise<void> {
    let buffer: ArrayBuffer;
    try {
      buffer = await response.arrayBuffer();
    } catch (error: unknown) {
      throw transportError(error, signal);
    }
    const bytes = new Uint8Array(buffer);
    writeDicomwebResponse(this.#wasm, this.#sink, bytes, kind);
  }
}

function parseContentType(response: Response): ParsedContentType {
  const value = response.headers.get("Content-Type");
  if (value === null) {
    throw new DicomwebError("ContentType");
  }
  const [basePart, ...parameterParts] = splitParameters(value);
  const base = basePart?.trim().toLowerCase();
  if (base === undefined || base === "") {
    throw new DicomwebError("ContentType");
  }
  const parameters = new Map<string, string>();
  for (const part of parameterParts) {
    const separator = part.indexOf("=");
    if (separator < 1) {
      throw new DicomwebError("ContentType");
    }
    const key = part.slice(0, separator).trim().toLowerCase();
    const parameter = unquoteParameter(part.slice(separator + 1).trim());
    if (key === "" || parameter === "" || parameters.has(key)) {
      throw new DicomwebError("ContentType");
    }
    parameters.set(key, parameter);
  }
  return { base, parameters };
}

function requireSimple(response: Response, expected: string): void {
  if (parseContentType(response).base !== expected.toLowerCase()) {
    throw new DicomwebError("ContentType");
  }
}

function requireMultipart(response: Response, expectedPartType: string): string {
  const contentType = parseContentType(response);
  const partType = contentType.parameters.get("type");
  const boundary = contentType.parameters.get("boundary");
  if (
    contentType.base !== "multipart/related" ||
    partType === undefined ||
    !sameMediaType(partType, expectedPartType) ||
    boundary === undefined ||
    !validBoundary(boundary)
  ) {
    throw new DicomwebError("ContentType");
  }
  return boundary;
}

function safeUrl(value: string): URL {
  try {
    return new URL(value);
  } catch {
    throw new DicomwebError("InvalidRequest");
  }
}

function validUint(value: number | undefined): boolean {
  return value === undefined || (Number.isSafeInteger(value) && value >= 0);
}

function validDicomUid(value: string): boolean {
  return (
    value.length <= 64 &&
    /^(?:0|[1-9][0-9]*)(?:\.(?:0|[1-9][0-9]*))*$/u.test(value)
  );
}

function transportError(error: unknown, signal?: AbortSignal): DicomwebError {
  if (
    signal?.aborted === true ||
    (error instanceof DOMException && error.name === "AbortError")
  ) {
    return new DicomwebError("Aborted");
  }
  return new DicomwebError("Network");
}

function splitParameters(value: string): string[] {
  const parts: string[] = [];
  let start = 0;
  let quoted = false;
  let escaped = false;
  for (let index = 0; index < value.length; index += 1) {
    const character = value[index];
    if (escaped) {
      escaped = false;
    } else if (quoted && character === "\\") {
      escaped = true;
    } else if (character === '"') {
      quoted = !quoted;
    } else if (!quoted && character === ";") {
      parts.push(value.slice(start, index));
      start = index + 1;
    }
  }
  if (quoted || escaped) {
    throw new DicomwebError("ContentType");
  }
  parts.push(value.slice(start));
  return parts;
}

function unquoteParameter(value: string): string {
  if (!value.startsWith('"')) {
    return value;
  }
  if (!value.endsWith('"') || value.length < 2) {
    throw new DicomwebError("ContentType");
  }
  return value.slice(1, -1).replace(/\\(.)/gu, "$1");
}

function validMediaType(value: string): boolean {
  return normalizeMediaType(value) !== undefined;
}

function normalizeMediaType(value: string): string | undefined {
  try {
    const [base, ...parameters] = splitParameters(value);
    if (!base?.trim().match(/^[!#$&^_.+\-\w]+\/[!#$&^_.+\-\w]+$/u)) {
      return undefined;
    }
    const normalizedParameters = new Map<string, string>();
    for (const part of parameters) {
      const separator = part.indexOf("=");
      const name = part.slice(0, separator).trim().toLowerCase();
      const parameter = unquoteParameter(part.slice(separator + 1).trim()).toLowerCase();
      if (
        separator < 1 ||
        name === "" ||
        parameter === "" ||
        normalizedParameters.has(name)
      ) {
        return undefined;
      }
      normalizedParameters.set(name, parameter);
    }
    const sorted = [...normalizedParameters].sort(([left], [right]) =>
      left.localeCompare(right),
    );
    return [base.trim().toLowerCase(), ...sorted.map(([key, item]) => `${key}=${item}`)].join(
      ";",
    );
  } catch {
    return undefined;
  }
}

function sameMediaType(left: string, right: string): boolean {
  const normalizedLeft = normalizeMediaType(left);
  return normalizedLeft !== undefined && normalizedLeft === normalizeMediaType(right);
}

function validBoundary(boundary: string): boolean {
  return (
    boundary.length >= 1 &&
    boundary.length <= 70 &&
    !boundary.endsWith(" ") &&
    /^[A-Za-z0-9'()+_,\-./:=? ]+$/u.test(boundary)
  );
}

function messageFor(code: DicomwebErrorCode): string {
  switch (code) {
    case "InvalidRequest":
      return "DICOMweb request parameters are invalid";
    case "Network":
      return "DICOMweb request failed";
    case "Aborted":
      return "DICOMweb request was cancelled";
    case "HttpStatus":
      return "DICOMweb server returned an unsuccessful status";
    case "ContentType":
      return "DICOMweb response content type is incompatible";
  }
}
