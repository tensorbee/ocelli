/** HTTP transport for the pure Rust DICOMweb response parser. */

import {
  writeDicomwebResponse,
  type DicomwebResponseKind,
  type DicomwebResponseSink,
  type WasmMemory,
} from "./bulk.js";
import { OcelliError, type DicomwebErrorCode } from "./ocelli-error.js";
import {
  DICOMWEB_CLIENT_OPTION_KEYS,
  DICOMWEB_BOUNDED_DELIVERY_OPTION_KEYS,
  QIDO_SEARCH_OPTION_KEYS,
  refuseUnknownOptions,
} from "./options.js";

export type { DicomwebErrorCode } from "./ocelli-error.js";

/** A safe transport failure that never retains a URL, header, or body. */
export class DicomwebError extends OcelliError {
  override readonly code: DicomwebErrorCode;
  readonly status?: number;

  constructor(code: DicomwebErrorCode, status?: number) {
    super(code, messageFor(code));
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

/** The fetch cache modes a `DicomwebClient` accepts. */
export type DicomwebCacheMode = "no-cache" | "no-store" | "reload" | "default";

const CACHE_MODES: readonly DicomwebCacheMode[] = [
  "no-cache",
  "no-store",
  "reload",
  "default",
];

export interface DicomwebClientOptions {
  readonly studiesServiceUrl: string;
  readonly wadoUriUrl: string;
  readonly fetch: typeof fetch;
  readonly wasm: WasmMemory;
  readonly sink: DicomwebResponseSink;
  /**
   * The cache mode of every request. `"no-cache"` revalidates a stored
   * response with the server before using it, and is the default, so a
   * re-processed object is never served from the browser's HTTP cache
   * without asking. `"default"` is an explicit opt-in to the browser's
   * heuristic freshness. `"force-cache"` and `"only-if-cached"` can serve a
   * stored response that is stale, and are refused at construction. A proxy
   * or a service worker between the browser and the server is outside what
   * a cache mode controls.
   */
  readonly cache?: DicomwebCacheMode;
}

export type DicomwebBoundedReceive = (
  raw: Uint8Array,
  kind: DicomwebResponseKind,
) => Promise<void>;

export interface DicomwebBoundedDeliveryOptions {
  readonly studiesServiceUrl: string;
  readonly wadoUriUrl: string;
  readonly fetch: typeof fetch;
  /**
   * The cache mode of every request. `"no-cache"` revalidates a stored
   * response with the server before using it, and is the default, so a
   * re-processed object is never served from the browser's HTTP cache
   * without asking. `"default"` is an explicit opt-in to the browser's
   * heuristic freshness. `"force-cache"` and `"only-if-cached"` can serve a
   * stored response that is stale, and are refused at construction. A proxy
   * or a service worker between the browser and the server is outside what
   * a cache mode controls.
   */
  readonly cache?: DicomwebCacheMode;
  /** Largest total response size accepted, in bytes. */
  readonly maxResponseBytes: number;
  /** Receives one final assembled raw response body, not decoded pixels and not chunks. */
  readonly receive: DicomwebBoundedReceive;
  readonly wasm?: never;
  readonly sink?: never;
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
  readonly #cache: DicomwebCacheMode;
  readonly #delivery:
    | {
        mode: "legacy";
        wasm: DicomwebClientOptions["wasm"];
        sink: DicomwebClientOptions["sink"];
      }
    | {
        mode: "bounded";
        maxResponseBytes: number;
        receive: NonNullable<DicomwebBoundedDeliveryOptions["receive"]>;
      };

  constructor(options: DicomwebClientOptions | DicomwebBoundedDeliveryOptions) {
    const bounded = DicomwebClient.isBoundedDeliveryOptions(options);
    if (bounded) {
      refuseUnknownOptions(
        "DicomwebBoundedDeliveryOptions",
        options,
        DICOMWEB_BOUNDED_DELIVERY_OPTION_KEYS,
      );
      const receive = options.receive;
      const maxResponseBytes = options.maxResponseBytes;
      if (Object.hasOwn(options, "wasm") || Object.hasOwn(options, "sink")) {
        throw new OcelliError(
          "InvalidOptions",
          "Bounded delivery options must not include wasm or sink",
        );
      }
      if (typeof receive !== "function") {
        throw new OcelliError(
          "InvalidOptions",
          "receive must be a callable function",
        );
      }
      if (
        typeof maxResponseBytes !== "number" ||
        !Number.isSafeInteger(maxResponseBytes) ||
        maxResponseBytes <= 0
      ) {
        throw new OcelliError(
          "InvalidOptions",
          "maxResponseBytes must be a positive safe integer",
        );
      }
      if (options.cache !== undefined && !validCacheMode(options.cache)) {
        throw new OcelliError(
          "InvalidOptions",
          "cache is not a valid DicomwebCacheMode",
        );
      }
      try {
        this.#studiesServiceUrl = safeUrl(options.studiesServiceUrl);
      } catch {
        throw new OcelliError("InvalidOptions", "studiesServiceUrl is not a valid absolute URL");
      }
      try {
        this.#wadoUriUrl = safeUrl(options.wadoUriUrl);
      } catch {
        throw new OcelliError("InvalidOptions", "wadoUriUrl is not a valid absolute URL");
      }
      this.#fetch = options.fetch;
      this.#cache = options.cache ?? "no-cache";
      this.#delivery = {
        mode: "bounded",
        maxResponseBytes,
        receive,
      };
    } else {
      refuseUnknownOptions("DicomwebClientOptions", options, DICOMWEB_CLIENT_OPTION_KEYS);
      if (options.cache !== undefined && !validCacheMode(options.cache)) {
        throw new DicomwebError("InvalidRequest");
      }
      this.#studiesServiceUrl = safeUrl(options.studiesServiceUrl);
      this.#wadoUriUrl = safeUrl(options.wadoUriUrl);
      this.#fetch = options.fetch;
      this.#cache = options.cache ?? "no-cache";
      this.#delivery = {
        mode: "legacy",
        wasm: options.wasm,
        sink: options.sink,
      };
    }
  }

  private static isBoundedDeliveryOptions(
    options: DicomwebClientOptions | DicomwebBoundedDeliveryOptions,
  ): options is DicomwebBoundedDeliveryOptions {
    if (typeof options !== "object" || options === null) {
      return false;
    }
    return (
      Object.hasOwn(options, "receive") || Object.hasOwn(options, "maxResponseBytes")
    );
  }

  async searchStudies(options: QidoSearchOptions = {}): Promise<void> {
    refuseUnknownOptions("QidoSearchOptions", options, QIDO_SEARCH_OPTION_KEYS);
    await this.#qido(["studies"], options);
  }

  async searchSeries(
    studyUid: string,
    options: QidoSearchOptions = {},
  ): Promise<void> {
    refuseUnknownOptions("QidoSearchOptions", options, QIDO_SEARCH_OPTION_KEYS);
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
    refuseUnknownOptions("QidoSearchOptions", options, QIDO_SEARCH_OPTION_KEYS);
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
        cache: this.#cache,
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
    if (this.#delivery.mode === "bounded" && signal?.aborted) {
      throw new DicomwebError("Aborted");
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
    if (this.#delivery.mode === "legacy") {
      let buffer: ArrayBuffer;
      try {
        buffer = await response.arrayBuffer();
      } catch (error: unknown) {
        throw transportError(error, signal);
      }
      const bytes = new Uint8Array(buffer);
      writeDicomwebResponse(this.#delivery.wasm, this.#delivery.sink, bytes, kind);
      return;
    }
    const bytes = await this.#readBounded(response, this.#delivery.maxResponseBytes, signal);
    if (signal?.aborted) {
      throw new DicomwebError("Aborted");
    }
    try {
      const receive = this.#delivery.receive;
      await receive(bytes, kind);
    } catch (error: unknown) {
      if (signal?.aborted) {
        throw new DicomwebError("Aborted");
      }
      if (error instanceof OcelliError) {
        throw error;
      }
      throw new OcelliError("CommitRefused", "Delivery callback refused the response");
    }
    if (signal?.aborted) {
      throw new DicomwebError("Aborted");
    }
  }

  async #readBounded(
    response: Response,
    cap: number,
    signal?: AbortSignal,
  ): Promise<Uint8Array> {
    if (signal?.aborted) {
      throw new DicomwebError("Aborted");
    }
    // Content-Length is never trusted as a refusal basis. The response body is
    // counted independently during streaming; only bytes actually exceeding the
    // cap cancel the transfer, and a body ending exactly at the cap is accepted
    // even if an overreported header claims a larger size.

    let reader: ReadableStreamDefaultReader<Uint8Array> | undefined;
    const onAbort = (): void => {
      // Controlled readers ignore the fetch signal, cancel wakes any pending read.
      if (reader !== undefined) {
        try {
          void reader.cancel().catch(() => undefined);
        } catch {
          // Synchronous cancel failure must not mask the primary abort.
        }
      }
    };
    if (signal !== undefined) {
      signal.addEventListener("abort", onAbort, { once: true });
    }

    let chunks: Uint8Array[] = [];
    let received = 0;
    let completed = false;
    try {
      if (signal?.aborted) {
        throw new DicomwebError("Aborted");
      }
      try {
        reader = response.body?.getReader();
      } catch (error: unknown) {
        reader = undefined;
        throw transportError(error, signal);
      }
      if (reader === undefined) {
        completed = true;
        return new Uint8Array(0);
      }
      while (true) {
        if (signal?.aborted) {
          throw new DicomwebError("Aborted");
        }
        let result: ReadableStreamReadResult<Uint8Array>;
        try {
          result = await reader.read();
        } catch (error: unknown) {
          throw transportError(error, signal);
        }
        // Post-read abort wins over done or chunk classification.
        if (signal?.aborted) {
          throw new DicomwebError("Aborted");
        }
        if (result.done) {
          break;
        }
        const chunk = result.value;
        received += chunk.byteLength;
        if (received > cap) {
          // Refuse before assembly or callback, no oversized copy is ever made.
          throw new OcelliError("AllocationRefused", "Streamed length exceeds the response byte cap");
        }
        chunks.push(chunk);
      }
      // Exactly-cap success requires actual EOF, no premature refusal is invented.
      const assembled = new Uint8Array(received);
      let offset = 0;
      for (const chunk of chunks) {
        assembled.set(chunk, offset);
        offset += chunk.byteLength;
      }
      chunks = [];
      completed = true;
      return assembled;
    } finally {
      if (signal !== undefined) {
        signal.removeEventListener("abort", onAbort);
      }
      if (reader !== undefined) {
        if (!completed) {
          try {
            void reader.cancel().catch(() => undefined);
          } catch {
            // Cancel failure must not mask the primary cap, abort or transport error.
          }
        }
        try {
          reader.releaseLock();
        } catch {
          // Release failure must not replace a primary cap, abort or transport error.
        }
      }
    }
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

function validCacheMode(value: string): boolean {
  return CACHE_MODES.some((mode) => mode === value);
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
