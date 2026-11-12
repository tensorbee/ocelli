import { DicomwebClient, DicomwebError, type DicomwebBoundedDeliveryOptions, type QidoSearchOptions } from "./dicomweb.js";
import { refuseUnknownOptions, QIDO_SEARCH_OPTION_KEYS, DICOMWEB_PROVIDER_OPTION_KEYS } from "./options.js";
import { type DicomwebResponseKind } from "./bulk.js";
import { OcelliError } from "./ocelli-error.js";

/**
 * Private staged provider over the same origin. QIDO search and the bounded
 * WADO staged-instance path both live here; each request is bounded by the
 * captured hierarchy and validated headers. No stubs, no unresolved promises.
 */

export interface QidoStudyRow { readonly studyUid: string; }
export interface QidoSeriesRow { readonly seriesUid: string; readonly returnedStudyUid?: string; }
export interface QidoInstanceRow { readonly instanceUid: string; readonly returnedStudyUid?: string; readonly returnedSeriesUid?: string; }
export type QidoStudyCapturedScope = Readonly<Record<string, never>>;
export interface QidoSeriesCapturedScope { readonly studyUid: string; }
export interface QidoInstanceCapturedScope { readonly studyUid: string; readonly seriesUid: string; }
export interface QidoEnvelopeCommon { readonly sourceNonce: string; readonly generation: number; readonly rawBodySha256: string; }
export type QidoStudyEnvelope = QidoEnvelopeCommon & { readonly level: 'study'; readonly capturedScope: QidoStudyCapturedScope; readonly rows: readonly QidoStudyRow[]; };
export type QidoSeriesEnvelope = QidoEnvelopeCommon & { readonly level: 'series'; readonly capturedScope: QidoSeriesCapturedScope; readonly rows: readonly QidoSeriesRow[]; };
export type QidoInstanceEnvelope = QidoEnvelopeCommon & { readonly level: 'instance'; readonly capturedScope: QidoInstanceCapturedScope; readonly rows: readonly QidoInstanceRow[]; };
export type QidoEnvelope = QidoStudyEnvelope | QidoSeriesEnvelope | QidoInstanceEnvelope;

type QidoLevel = "study" | "series" | "instance";

function withQidoIncludeFields(level: QidoLevel, options: QidoSearchOptions): QidoSearchOptions {
  const required =
    level === "study"
      ? ["0020000D"]
      : level === "series"
        ? ["0020000D", "0020000E"]
        : ["0020000D", "0020000E", "00080018"];
  const seen = new Set<string>();
  const includeFields: string[] = [];
  for (const field of [...required, ...(options.includeFields ?? [])]) {
    if (!seen.has(field)) {
      seen.add(field);
      includeFields.push(field);
    }
  }
  return { ...options, includeFields };
}

const QIDO_MAX_RAW_BYTES = 2 * 1024 * 1024;
const QIDO_NATIVE_REPLY_MAX_BYTES = 16 * 1024 * 1024;
const QIDO_MAX_GENERATION = 9007199254740991;
const MAX_LIVE_OPERATIONS = 2;



interface PendingContext {
  readonly level: QidoLevel;
  readonly scope: ReadonlyMap<string, string>;
  readonly sourceNonce: string;
  readonly generation: number;
  readonly options: QidoSearchOptions;
  readonly controller: AbortController;
  readonly signal: AbortSignal;
}

interface LiveEntry {
  readonly controller: AbortController;
}

// Private queued job union. The prior QIDO-only QueuedJob is replaced by
// this operation-kind correlated union. No admission logic here; the same
// existing #queued slot holds the union.

interface QidoQueuedJob {
  readonly operation: "qido";
  readonly context: PendingContext;
  readonly resolve: (envelope: QidoEnvelope) => void;
  readonly reject: (error: OcelliError) => void;
  readonly externalSignal: AbortSignal | null;
  readonly externalListener: (() => void) | null;
}

interface WadoPendingContext {
  readonly identity: InstanceIdentity;
  readonly sourceNonce: string;
  readonly generation: number;
  readonly controller: AbortController;
  readonly signal: AbortSignal;
}

interface WadoQueuedJob {
  readonly operation: "wado";
  readonly context: WadoPendingContext;
  readonly resolve: (metadata: StagedMetadata) => void;
  readonly reject: (error: OcelliError) => void;
  readonly externalSignal: null;
  readonly externalListener: null;
}

type QueuedJob = QidoQueuedJob | WadoQueuedJob;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function exactKeys(value: Record<string, unknown>, keys: readonly string[]): boolean {
  const own = Object.keys(value);
  if (own.length !== keys.length) {
    return false;
  }
  return keys.every((key) => Object.prototype.hasOwnProperty.call(value, key));
}

function hex32(value: unknown): value is string {
  return typeof value === "string" && /^[0-9a-f]{32}$/.test(value);
}

function hex64(value: unknown): value is string {
  return typeof value === "string" && /^[0-9a-f]{64}$/.test(value);
}

function validUid(value: unknown): value is string {
  if (typeof value !== "string" || value.length === 0 || value.length > 64) {
    return false;
  }
  return /^(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*))*$/.test(value);
}

function isAbsoluteHttpUrl(value: string): boolean {
  try {
    const parsed = new URL(value);
    return parsed.protocol === "http:" || parsed.protocol === "https:";
  } catch {
    return false;
  }
}

function randomSourceNonce(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

async function sha256Hex(buffer: ArrayBuffer): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", buffer);
  return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, "0")).join("");
}

function nativeBodyError(): OcelliError {
  return new OcelliError("CommitRefused", "native endpoint returned an unusable body");
}

async function readBoundedBody(response: Response, maxBytes: number): Promise<Uint8Array> {
  const body = response.body;
  if (body === null) {
    throw new OcelliError("Network", "native response carried no body");
  }
  const reader = body.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;
  for (;;) {
    let step: ReadableStreamReadResult<Uint8Array>;
    try {
      step = await reader.read();
    } catch {
      throw new OcelliError("Network", "native response body could not be read");
    }
    if (step.done) {
      break;
    }
    total += step.value.byteLength;
    if (total > maxBytes) {
      void reader.cancel().catch(() => undefined);
      throw new OcelliError("AllocationRefused", "native response exceeded the bounded size");
    }
    chunks.push(step.value);
  }
  const merged = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    merged.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return merged;
}

function decodeEnvelope(
  raw: Uint8Array,
  context: PendingContext,
  expectedSha: string,
): QidoEnvelope {
  let parsed: unknown;
  try {
    parsed = JSON.parse(new TextDecoder().decode(raw));
  } catch {
    throw nativeBodyError();
  }
  if (
    !isRecord(parsed) ||
    !exactKeys(parsed, ["level", "rows", "capturedScope", "sourceNonce", "generation", "rawBodySha256"])
  ) {
    throw nativeBodyError();
  }
  const level = parsed["level"];
  if (typeof level !== "string" || level !== context.level) {
    throw nativeBodyError();
  }
  if (!Array.isArray(parsed["rows"])) {
    throw nativeBodyError();
  }
  const scopeValue = parsed["capturedScope"];
  if (!isRecord(scopeValue)) {
    throw nativeBodyError();
  }
  const sourceNonce = parsed["sourceNonce"];
  if (!hex32(sourceNonce) || sourceNonce !== context.sourceNonce) {
    throw nativeBodyError();
  }
  const generation = parsed["generation"];
  if (
    typeof generation !== "number" ||
    !Number.isSafeInteger(generation) ||
    generation !== context.generation
  ) {
    throw nativeBodyError();
  }
  const rawBodySha256 = parsed["rawBodySha256"];
  if (!hex64(rawBodySha256) || rawBodySha256 !== expectedSha) {
    throw nativeBodyError();
  }
  if (level === "study") {
    if (!exactKeys(scopeValue, [])) {
      throw nativeBodyError();
    }
    const rows: QidoStudyRow[] = [];
    for (const candidate of parsed["rows"]) {
      if (!isRecord(candidate) || !exactKeys(candidate, ["studyUid"])) {
        throw nativeBodyError();
      }
      const studyUid = candidate["studyUid"];
      if (!validUid(studyUid)) {
        throw nativeBodyError();
      }
      rows.push(Object.freeze({ studyUid }));
    }
    const envelope: QidoStudyEnvelope = {
      level: "study",
      capturedScope: Object.freeze({}),
      rows: Object.freeze(rows),
      sourceNonce,
      generation,
      rawBodySha256,
    };
    return Object.freeze(envelope);
  }
  const captured = context.scope;
  if (level === "series") {
    if (!exactKeys(scopeValue, ["studyUid"])) {
      throw nativeBodyError();
    }
    const expectedStudy = captured.get("studyUid");
    const capturedStudy = scopeValue["studyUid"];
    if (expectedStudy === undefined || capturedStudy !== expectedStudy) {
      throw nativeBodyError();
    }
    if (!validUid(capturedStudy)) {
      throw nativeBodyError();
    }
    const rows: QidoSeriesRow[] = [];
    for (const candidate of parsed["rows"]) {
      if (
        !isRecord(candidate) ||
        !(exactKeys(candidate, ["seriesUid"]) || exactKeys(candidate, ["seriesUid", "returnedStudyUid"]))
      ) {
        throw nativeBodyError();
      }
      const seriesUid = candidate["seriesUid"];
      if (!validUid(seriesUid)) {
        throw nativeBodyError();
      }
      const row: { seriesUid: string; returnedStudyUid?: string } = { seriesUid };
      if ("returnedStudyUid" in candidate) {
        const returned = candidate["returnedStudyUid"];
        if (!validUid(returned) || returned !== expectedStudy) {
          throw nativeBodyError();
        }
        row.returnedStudyUid = returned;
      }
      rows.push(Object.freeze(row));
    }
    const envelope: QidoSeriesEnvelope = {
      level: "series",
      capturedScope: Object.freeze({ studyUid: capturedStudy }),
      rows: Object.freeze(rows),
      sourceNonce,
      generation,
      rawBodySha256,
    };
    return Object.freeze(envelope);
  }
  if (!exactKeys(scopeValue, ["studyUid", "seriesUid"])) {
    throw nativeBodyError();
  }
  const expectedStudyUid = captured.get("studyUid");
  const expectedSeriesUid = captured.get("seriesUid");
  const capturedStudyUid = scopeValue["studyUid"];
  const capturedSeriesUid = scopeValue["seriesUid"];
  if (
    expectedStudyUid === undefined ||
    expectedSeriesUid === undefined ||
    capturedStudyUid !== expectedStudyUid ||
    capturedSeriesUid !== expectedSeriesUid ||
    !validUid(capturedStudyUid) ||
    !validUid(capturedSeriesUid)
  ) {
    throw nativeBodyError();
  }
  const rows: QidoInstanceRow[] = [];
  for (const candidate of parsed["rows"]) {
    if (
      !isRecord(candidate) ||
      !(
        exactKeys(candidate, ["instanceUid"]) ||
        exactKeys(candidate, ["instanceUid", "returnedStudyUid"]) ||
        exactKeys(candidate, ["instanceUid", "returnedStudyUid", "returnedSeriesUid"]) ||
        exactKeys(candidate, ["instanceUid", "returnedSeriesUid"])
      )
    ) {
      throw nativeBodyError();
    }
    const instanceUid = candidate["instanceUid"];
    if (!validUid(instanceUid)) {
      throw nativeBodyError();
    }
    const row: { instanceUid: string; returnedStudyUid?: string; returnedSeriesUid?: string } = {
      instanceUid,
    };
    if ("returnedStudyUid" in candidate) {
      const returned = candidate["returnedStudyUid"];
      if (!validUid(returned) || returned !== expectedStudyUid) {
        throw nativeBodyError();
      }
      row.returnedStudyUid = returned;
    }
    if ("returnedSeriesUid" in candidate) {
      const returned = candidate["returnedSeriesUid"];
      if (!validUid(returned) || returned !== expectedSeriesUid) {
        throw nativeBodyError();
      }
      row.returnedSeriesUid = returned;
    }
    rows.push(Object.freeze(row));
  }
  const envelope: QidoInstanceEnvelope = {
    level: "instance",
    capturedScope: Object.freeze({ studyUid: capturedStudyUid, seriesUid: capturedSeriesUid }),
    rows: Object.freeze(rows),
    sourceNonce,
    generation,
    rawBodySha256,
  };
  return Object.freeze(envelope);
}

export type InstanceIdentity = Readonly<{
  studyUid: string;
  seriesUid: string;
  instanceUid: string;
}>;

export type StagedMetadata = Readonly<{
  identity: InstanceIdentity;
  rawBodySha256: string;
  sourceNonce: string;
  generation: number;
  byteLength: number;
  representation: Readonly<{ field: "wado-rs-instances"; boundary: string }>;
}>;

export type OwnedEncodedInstance = { body: ArrayBuffer; metadata: StagedMetadata };

// Test-only peeker type. It is registered under vitest's in-source testing
// by the static block in DicomwebProvider and read by that module's own
// test block at the foot of this file. It never appears in the emitted
// declaration file.
type StagedBodyPeek = (provider: DicomwebProvider) => OwnedEncodedInstance | null;

function decodeInstanceMetadata(
  bytes: Uint8Array,
  expected: {
    readonly identity: InstanceIdentity;
    readonly sourceNonce: string;
    readonly generation: number;
    readonly rawBodySha256: string;
    readonly byteLength: number;
    readonly boundary: string;
  },
): StagedMetadata {
  let parsed: unknown;
  try {
    parsed = JSON.parse(new TextDecoder().decode(bytes));
  } catch {
    throw nativeBodyError();
  }
  if (
    !isRecord(parsed) ||
    !exactKeys(parsed, ["identity", "rawBodySha256", "sourceNonce", "generation", "byteLength", "representation"])
  ) {
    throw nativeBodyError();
  }
  const identityValue = parsed["identity"];
  if (!isRecord(identityValue) || !exactKeys(identityValue, ["studyUid", "seriesUid", "instanceUid"])) {
    throw nativeBodyError();
  }
  const studyUid = identityValue["studyUid"];
  const seriesUid = identityValue["seriesUid"];
  const instanceUid = identityValue["instanceUid"];
  if (!validUid(studyUid) || !validUid(seriesUid) || !validUid(instanceUid)) {
    throw nativeBodyError();
  }
  if (
    studyUid !== expected.identity.studyUid ||
    seriesUid !== expected.identity.seriesUid ||
    instanceUid !== expected.identity.instanceUid
  ) {
    throw nativeBodyError();
  }
  const rawBodySha256 = parsed["rawBodySha256"];
  if (!hex64(rawBodySha256) || rawBodySha256 !== expected.rawBodySha256) {
    throw nativeBodyError();
  }
  const sourceNonce = parsed["sourceNonce"];
  if (!hex32(sourceNonce) || sourceNonce !== expected.sourceNonce) {
    throw nativeBodyError();
  }
  const generation = parsed["generation"];
  if (
    typeof generation !== "number" ||
    !Number.isSafeInteger(generation) ||
    generation < 0 ||
    generation !== expected.generation
  ) {
    throw nativeBodyError();
  }
  const byteLength = parsed["byteLength"];
  if (
    typeof byteLength !== "number" ||
    !Number.isSafeInteger(byteLength) ||
    byteLength < 0 ||
    byteLength !== expected.byteLength
  ) {
    throw nativeBodyError();
  }
  const representationValue = parsed["representation"];
  if (!isRecord(representationValue) || !exactKeys(representationValue, ["field", "boundary"])) {
    throw nativeBodyError();
  }
  const field = representationValue["field"];
  const boundary = representationValue["boundary"];
  if (field !== "wado-rs-instances") {
    throw nativeBodyError();
  }
  if (typeof boundary !== "string" || boundary !== expected.boundary) {
    throw nativeBodyError();
  }
  const identity: InstanceIdentity = Object.freeze({
    studyUid,
    seriesUid,
    instanceUid,
  });
  const representation = Object.freeze({
    field: "wado-rs-instances" as const,
    boundary,
  });
  const metadata: StagedMetadata = Object.freeze({
    identity,
    rawBodySha256,
    sourceNonce,
    generation,
    byteLength,
    representation,
  });
  return metadata;
}

export type DicomwebProviderOptions = { studiesServiceUrl: string; wadoUriUrl: string; fetch: typeof fetch };

export class DicomwebProvider {
  readonly #studiesServiceUrl: URL;
  readonly #wadoUriUrl: URL;
  readonly #fetch: typeof fetch;
  #generation: number = 0;
  #disposed: boolean = false;
  readonly #live: Set<LiveEntry> = new Set();
  #queued: QueuedJob | null = null;

  #staged: OwnedEncodedInstance | null = null;

  #beginIntent(): {
    sourceNonce: string;
    generation: number;
    controller: AbortController;
  } {
    if (this.#disposed) {
      throw new OcelliError("Aborted", "provider is disposed");
    }
    if (this.#generation >= QIDO_MAX_GENERATION) {
      throw new OcelliError("InvalidRequest", "generation overflow");
    }
    for (const entry of [...this.#live]) {
      entry.controller.abort();
    }
    this.#release();
    const replaced = this.#queued;
    if (replaced !== null) {
      this.#queued = null;
      this.#removeExternalListener(
        replaced.externalSignal,
        replaced.externalListener,
      );
      replaced.context.controller.abort();
      replaced.reject(
        new OcelliError("Aborted", "operation was replaced by a newer request"),
      );
    }
    this.#generation += 1;
    const generation = this.#generation;
    const sourceNonce = randomSourceNonce();
    const controller = new AbortController();
    return { sourceNonce, generation, controller };
  }

  constructor(opts: DicomwebProviderOptions) {
    if (opts === null || typeof opts !== "object" || Array.isArray(opts)) {
      throw new OcelliError("InvalidOptions", "DicomwebProvider options must be an object");
    }
    try {
      refuseUnknownOptions("DicomwebProvider options", opts, DICOMWEB_PROVIDER_OPTION_KEYS);
    } catch (error: unknown) {
      if (error instanceof OcelliError) {
        throw error;
      }
      throw new OcelliError("InvalidOptions", "DicomwebProvider options are invalid");
    }
    const candidates = opts;
    if (
      typeof candidates.studiesServiceUrl !== "string" ||
      !isAbsoluteHttpUrl(candidates.studiesServiceUrl)
    ) {
      throw new OcelliError(
        "InvalidOptions",
        "studiesServiceUrl must be an absolute http(s) URL string",
      );
    }
    if (typeof candidates.wadoUriUrl !== "string" || !isAbsoluteHttpUrl(candidates.wadoUriUrl)) {
      throw new OcelliError("InvalidOptions", "wadoUriUrl must be an absolute http(s) URL string");
    }
    if (typeof candidates.fetch !== "function") {
      throw new OcelliError("InvalidOptions", "fetch must be a function");
    }
    this.#studiesServiceUrl = new URL(candidates.studiesServiceUrl);
    this.#wadoUriUrl = new URL(candidates.wadoUriUrl);
    this.#fetch = candidates.fetch;
  }

  async searchStudies(options?: QidoSearchOptions): Promise<QidoStudyEnvelope> {
    const envelope = await this.#run("study", new Map(), options);
    if (envelope.level !== "study") {
      throw new OcelliError("CommitRefused", "native endpoint returned a mismatched level");
    }
    return envelope;
  }

  async searchSeries(studyUid: string, options?: QidoSearchOptions): Promise<QidoSeriesEnvelope> {
    if (!validUid(studyUid)) {
      throw new OcelliError("InvalidRequest", "studyUid is not a valid UI VM1 UID");
    }
    const envelope = await this.#run("series", new Map([["studyUid", studyUid]]), options);
    if (envelope.level !== "series") {
      throw new OcelliError("CommitRefused", "native endpoint returned a mismatched level");
    }
    return envelope;
  }

  async searchInstances(
    studyUid: string,
    seriesUid: string,
    options?: QidoSearchOptions,
  ): Promise<QidoInstanceEnvelope> {
    if (!validUid(studyUid)) {
      throw new OcelliError("InvalidRequest", "studyUid is not a valid UI VM1 UID");
    }
    if (!validUid(seriesUid)) {
      throw new OcelliError("InvalidRequest", "seriesUid is not a valid UI VM1 UID");
    }
    const envelope = await this.#run(
      "instance",
      new Map([
        ["studyUid", studyUid],
        ["seriesUid", seriesUid],
      ]),
      options,
    );
    if (envelope.level !== "instance") {
      throw new OcelliError("CommitRefused", "native endpoint returned a mismatched level");
    }
    return envelope;
  }

  cancelPending(): void {
    this.#generation += 1;
    for (const entry of [...this.#live]) {
      entry.controller.abort();
    }
    this.#release();
    const queued = this.#queued;
    this.#queued = null;
    if (queued !== null) {
      this.#removeExternalListener(queued.externalSignal, queued.externalListener);
      queued.context.controller.abort();
      queued.reject(new OcelliError("Aborted", "operation was canceled"));
    }
  }

  dispose(): void {
    if (this.#disposed) {
      return;
    }
    this.#disposed = true;
    // Released directly, so an override of cancelPending cannot keep the
    // staged body either. The release is idempotent.
    this.#release();
    this.cancelPending();
  }

  #removeExternalListener(signal: AbortSignal | null, listener: (() => void) | null): void {
    if (signal !== null && listener !== null) {
      signal.removeEventListener("abort", listener);
    }
  }

  #checkFresh(
    context: PendingContext | WadoPendingContext,
    stage: string,
  ): void {
    if (
      this.#disposed ||
      this.#generation !== context.generation ||
      context.controller.signal.aborted
    ) {
      throw new OcelliError("Aborted", `${stage}: operation was replaced`);
    }
  }

  #validateQidoOptions(options?: QidoSearchOptions): QidoSearchOptions {
    if (options === undefined) {
      return {};
    }
    try {
      refuseUnknownOptions("search options", options, QIDO_SEARCH_OPTION_KEYS);
    } catch (error: unknown) {
      if (error instanceof OcelliError) {
        throw error;
      }
      throw new OcelliError("InvalidOptions", "search options are invalid");
    }
    const observed = options;
    const next: {
      filters?: ReadonlyArray<readonly [string, string]>;
      includeFields?: readonly string[];
      limit?: number;
      offset?: number;
      fuzzyMatching?: boolean;
      signal?: AbortSignal;
    } = {};
    if (observed.filters !== undefined) {
      if (!Array.isArray(observed.filters)) {
        throw new OcelliError("InvalidOptions", "filters must be an array of tuples");
      }
      const tuples: [string, string][] = [];
      for (const entry of observed.filters) {
        if (!Array.isArray(entry) || entry.length !== 2) {
          throw new OcelliError("InvalidOptions", "filters entries must be [string, string] tuples");
        }
        if (typeof entry[0] !== "string" || typeof entry[1] !== "string") {
          throw new OcelliError("InvalidOptions", "filters entries must be [string, string] tuples");
        }
        tuples.push([entry[0], entry[1]]);
      }
      next.filters = tuples;
    }
    if (observed.includeFields !== undefined) {
      if (!Array.isArray(observed.includeFields)) {
        throw new OcelliError("InvalidOptions", "includeFields must be an array of strings");
      }
      const fields: string[] = [];
      for (const field of observed.includeFields) {
        if (typeof field !== "string") {
          throw new OcelliError("InvalidOptions", "includeFields entries must be strings");
        }
        fields.push(field);
      }
      next.includeFields = fields;
    }
    if (observed.limit !== undefined) {
      if (typeof observed.limit !== "number" || !Number.isSafeInteger(observed.limit) || observed.limit < 0) {
        throw new OcelliError("InvalidOptions", "limit must be a non-negative safe integer");
      }
      next.limit = observed.limit;
    }
    if (observed.offset !== undefined) {
      if (typeof observed.offset !== "number" || !Number.isSafeInteger(observed.offset) || observed.offset < 0) {
        throw new OcelliError("InvalidOptions", "offset must be a non-negative safe integer");
      }
      next.offset = observed.offset;
    }
    if (observed.fuzzyMatching !== undefined) {
      if (typeof observed.fuzzyMatching !== "boolean") {
        throw new OcelliError("InvalidOptions", "fuzzyMatching must be a boolean");
      }
      next.fuzzyMatching = observed.fuzzyMatching;
    }
    if (observed.signal !== undefined) {
      if (!(observed.signal instanceof AbortSignal)) {
        throw new OcelliError("InvalidOptions", "signal must be an AbortSignal");
      }
      next.signal = observed.signal;
    }
    return next;
  }

  async #run(
    level: QidoLevel,
    scope: ReadonlyMap<string, string>,
    options?: QidoSearchOptions,
  ): Promise<QidoEnvelope> {
    const validated = withQidoIncludeFields(
      level,
      this.#validateQidoOptions(options),
    );
    const externalSignal = validated.signal ?? null;
    if (externalSignal !== null && externalSignal.aborted) {
      throw new OcelliError("Aborted", "operation was aborted before admission");
    }
    const { sourceNonce, generation, controller } = this.#beginIntent();
    let externalListener: (() => void) | null = null;
    if (externalSignal !== null) {
      externalListener = () => {
        controller.abort();
        const job = this.#queued;
        if (job !== null && job.context.controller === controller) {
          this.#queued = null;
          this.#removeExternalListener(externalSignal, externalListener);
          job.reject(new OcelliError("Aborted", "operation was aborted"));
        }
      };
      externalSignal.addEventListener("abort", externalListener, { once: true });
    }
    const context: PendingContext = {
      level,
      scope: new Map(scope),
      sourceNonce,
      generation,
      options: validated,
      controller,
      signal: controller.signal,
    };
    return await new Promise<QidoEnvelope>((resolve, reject) => {
      const job: QidoQueuedJob = {
        operation: "qido",
        context,
        resolve,
        reject,
        externalSignal,
        externalListener,
      };
      this.#startOrQueue(job);
    });
  }

  #drainQueued(): void {
    const job = this.#queued;
    if (job === null) {
      return;
    }
    const stale =
      this.#disposed ||
      this.#generation !== job.context.generation ||
      job.context.controller.signal.aborted;
    if (stale) {
      this.#queued = null;
      if (job.operation === "qido") {
        this.#removeExternalListener(job.externalSignal, job.externalListener);
      } else {
        this.#removeExternalListener(null, null);
      }
      job.context.controller.abort();
      job.reject(new OcelliError("Aborted", "operation was replaced"));
      return;
    }
    if (this.#live.size >= MAX_LIVE_OPERATIONS) {
      return;
    }
    this.#queued = null;
    this.#startOrQueue(job);
  }

  async #execute(
    context: PendingContext,
    resolve: (envelope: QidoEnvelope) => void,
    reject: (error: OcelliError) => void,
    entry: LiveEntry,
    externalSignal: AbortSignal | null,
    externalListener: (() => void) | null,
  ): Promise<void> {
    try {
      this.#checkFresh(context, "collect");
      const raw = await this.#collectUpstream(context);
      this.#checkFresh(context, "hash");
      const owned = new ArrayBuffer(raw.byteLength);
      new Uint8Array(owned).set(raw);
      const digest = await sha256Hex(owned);
      this.#checkFresh(context, "post");
      const native = await this.#postNative(context, owned);
      this.#checkFresh(context, "body");
      const reply = await readBoundedBody(native, QIDO_NATIVE_REPLY_MAX_BYTES);
      this.#checkFresh(context, "decode");
      const envelope = decodeEnvelope(reply, context, digest);
      this.#checkFresh(context, "success");
      resolve(envelope);
    } catch (error: unknown) {
      const stale =
        this.#disposed ||
        this.#generation !== context.generation ||
        context.controller.signal.aborted;
      if (stale) {
        reject(new OcelliError("Aborted", "operation was replaced"));
        return;
      }
      if (error instanceof OcelliError) {
        reject(error);
        return;
      }
      reject(new OcelliError("Network", "qido search failed"));
    } finally {
      this.#live.delete(entry);
      this.#removeExternalListener(externalSignal, externalListener);
      this.#drainQueued();
    }
  }

  async #postNative(context: PendingContext, body: ArrayBuffer): Promise<Response> {
    this.#checkFresh(context, "native-post");
    const headers = new Headers();
    headers.set("X-Qido-Level", context.level);
    const parentStudy = context.scope.get("studyUid");
    if (parentStudy !== undefined) {
      headers.set("X-Qido-Parent-Study", parentStudy);
    }
    const parentSeries = context.scope.get("seriesUid");
    if (parentSeries !== undefined) {
      headers.set("X-Qido-Parent-Series", parentSeries);
    }
    headers.set("X-Qido-Source-Nonce", context.sourceNonce);
    headers.set("X-Qido-Generation", String(context.generation));
    headers.set("Content-Type", "application/dicom+json");
    let response: Response;
    try {
      response = await fetch("/dicweb/qido", {
        method: "POST",
        headers,
        body,
        signal: context.controller.signal,
      });
    } catch {
      if (context.controller.signal.aborted) {
        throw new OcelliError("Aborted", "operation was aborted");
      }
      throw new OcelliError("Network", "native endpoint unreachable");
    }
    this.#checkFresh(context, "native-response");
    if (response.status === 200) {
      return response;
    }
    if (response.status === 400) {
      throw new OcelliError("InvalidRequest", "native request was rejected");
    }
    if (response.status === 413) {
      throw new OcelliError("AllocationRefused", "native request exceeded allocation limits");
    }
    if (response.status === 415) {
      throw new OcelliError("ContentType", "native endpoint refused the content type");
    }
    if (response.status === 422) {
      const code = await this.#readNativeErrorCode(context, response);
      if (code === "InvalidValue") {
        throw new OcelliError("InvalidValue", "native endpoint refused the value");
      }
      throw new OcelliError("CommitRefused", "native endpoint returned an unexpected code");
    }
    if (response.status === 503) {
      throw new DicomwebError("HttpStatus", 503);
    }
    throw new DicomwebError("HttpStatus", response.status);
  }

  async #readNativeErrorCode(
    context: PendingContext | WadoPendingContext,
    response: Response,
  ): Promise<string | null> {
    let bytes: Uint8Array;
    try {
      bytes = await readBoundedBody(response, 16 * 1024 * 1024);
    } catch (error) {
      if (
        error instanceof OcelliError &&
        error.code === "AllocationRefused"
      ) {
        throw error;
      }
      this.#checkFresh(context, "native-error");
      return null;
    }
    this.#checkFresh(context, "native-error");
    let parsed: unknown;
    try {
      parsed = JSON.parse(new TextDecoder().decode(bytes)) as unknown;
    } catch {
      return null;
    }
    if (
      isRecord(parsed) &&
      exactKeys(parsed, ["code"]) &&
      typeof parsed["code"] === "string"
    ) {
      this.#checkFresh(context, "native-error");
      return parsed["code"];
    }
    return null;
  }

  async #collectUpstream(context: PendingContext): Promise<Uint8Array> {
    this.#checkFresh(context, "upstream");
    let result: { readonly bytes: Uint8Array } | null = null;
    let received = false;
    const receive = async (
      raw: Uint8Array,
      kind: DicomwebResponseKind,
    ): Promise<void> => {
      if (kind.type !== "qido-json") {
        throw new OcelliError("ContentType", "unexpected upstream response kind");
      }
      this.#checkFresh(context, "upstream-delivery");
      received = true;
      result = { bytes: raw };
    };
    const client = new DicomwebClient({
      studiesServiceUrl: this.#studiesServiceUrl.href,
      wadoUriUrl: this.#wadoUriUrl.href,
      fetch: this.#fetch,
      maxResponseBytes: QIDO_MAX_RAW_BYTES,
      receive,
    } satisfies DicomwebBoundedDeliveryOptions);
    const upstreamOptions: QidoSearchOptions = {
      ...context.options,
      signal: context.controller.signal,
    };
    try {
      if (context.level === "study") {
        await client.searchStudies(upstreamOptions);
      } else if (context.level === "series") {
        const studyUid = context.scope.get("studyUid");
        if (studyUid === undefined) {
          throw new OcelliError("InvalidRequest", "missing study UID in captured scope");
        }
        await client.searchSeries(studyUid, upstreamOptions);
      } else {
        const studyUid = context.scope.get("studyUid");
        const seriesUid = context.scope.get("seriesUid");
        if (studyUid === undefined || seriesUid === undefined) {
          throw new OcelliError("InvalidRequest", "missing UIDs in captured scope");
        }
        await client.searchInstances(studyUid, seriesUid, upstreamOptions);
      }
    } catch (error: unknown) {
      if (
        this.#disposed ||
        this.#generation !== context.generation ||
        context.controller.signal.aborted
      ) {
        throw new OcelliError("Aborted", "operation was replaced");
      }
      if (error instanceof OcelliError) {
        if (error.code === "Aborted") {
          throw new OcelliError("Aborted", "operation was aborted");
        }
        throw error;
      }
      throw new OcelliError("Network", "upstream search failed");
    }
    this.#checkFresh(context, "upstream-collected");
    if (received === false || result === null) {
      throw new OcelliError("CommitRefused", "upstream body was not delivered");
    }
    const holder: { readonly bytes: Uint8Array } = result;
    return holder.bytes;
  }

  #startOrQueue(job: QueuedJob): void {
    if (this.#live.size >= MAX_LIVE_OPERATIONS) {
      this.#queued = job;
      return;
    }
    const entry: LiveEntry = { controller: job.context.controller };
    this.#live.add(entry);
    if (job.operation === "qido") {
      void this.#execute(
        job.context,
        job.resolve,
        job.reject,
        entry,
        job.externalSignal,
        job.externalListener,
      ).finally(() => {
        if (job.externalSignal !== null && job.externalListener !== null) {
          job.externalSignal.removeEventListener("abort", job.externalListener);
        }
      });
    } else {
      void this.#executeInstance(job.context, job.resolve, job.reject, entry);
    }
  }

  async fetchSelectedInstance(identity: InstanceIdentity): Promise<StagedMetadata> {
    if (!isRecord(identity)) {
      throw new OcelliError("InvalidRequest", "identity must be an object");
    }
    const { studyUid, seriesUid, instanceUid } = identity;
    if (
      !validUid(studyUid) ||
      !validUid(seriesUid) ||
      !validUid(instanceUid)
    ) {
      throw new OcelliError("InvalidRequest", "identity UIDs are invalid");
    }
    const ownedIdentity: InstanceIdentity = Object.freeze({
      studyUid,
      seriesUid,
      instanceUid,
    });
    const intent = this.#beginIntent();
    const context: WadoPendingContext = {
      identity: ownedIdentity,
      sourceNonce: intent.sourceNonce,
      generation: intent.generation,
      controller: intent.controller,
      signal: intent.controller.signal,
    };
    return await new Promise<StagedMetadata>((resolve, reject) => {
      const job: WadoQueuedJob = {
        operation: "wado",
        context,
        resolve,
        reject,
        externalSignal: null,
        externalListener: null,
      };
      this.#startOrQueue(job);
    });
  }

  async #collectInstance(
    context: WadoPendingContext,
  ): Promise<{ raw: Uint8Array; boundary: string }> {
    this.#checkFresh(context, "wado");
    let result: { readonly raw: Uint8Array; readonly boundary: string } | null =
      null;
    let received = false;
    const receive = async (
      raw: Uint8Array,
      kind: DicomwebResponseKind,
    ): Promise<void> => {
      if (kind.type !== "wado-rs-instances") {
        throw new OcelliError("ContentType", "unexpected wado response kind");
      }
      this.#checkFresh(context, "wado-delivery");
      received = true;
      result = { raw, boundary: kind.boundary };
    };
    const client = new DicomwebClient({
      studiesServiceUrl: this.#studiesServiceUrl.href,
      wadoUriUrl: this.#wadoUriUrl.href,
      fetch: this.#fetch,
      maxResponseBytes: 32 * 1024 * 1024,
      receive,
    } satisfies DicomwebBoundedDeliveryOptions);
    try {
      await client.retrieveInstance(
        context.identity.studyUid,
        context.identity.seriesUid,
        context.identity.instanceUid,
        context.controller.signal,
      );
    } catch (error: unknown) {
      if (
        this.#disposed ||
        this.#generation !== context.generation ||
        context.controller.signal.aborted
      ) {
        throw new OcelliError("Aborted", "operation was replaced");
      }
      if (error instanceof OcelliError) {
        if (error.code === "Aborted") {
          throw new OcelliError("Aborted", "operation was aborted");
        }
        throw error;
      }
      throw new OcelliError("Network", "upstream instance retrieval failed");
    }
    this.#checkFresh(context, "wado-collected");
    if (received === false || result === null) {
      throw new OcelliError("CommitRefused", "wado body was not delivered");
    }
    const holder: { readonly raw: Uint8Array; readonly boundary: string } =
      result;
    return { raw: holder.raw, boundary: holder.boundary };
  }

  async #postInstance(context: WadoPendingContext, body: ArrayBuffer, boundary: string): Promise<Response> {
    this.#checkFresh(context, "wado-post");
    const headers = new Headers();
    headers.set("X-Wado-Study-Instance-UID", context.identity.studyUid);
    headers.set("X-Wado-Series-Instance-UID", context.identity.seriesUid);
    headers.set("X-Wado-SOP-Instance-UID", context.identity.instanceUid);
    headers.set("X-Wado-Source-Nonce", context.sourceNonce);
    headers.set("X-Wado-Generation", String(context.generation));
    headers.set(
      "Content-Type",
      `multipart/related; type="application/dicom"; boundary="${boundary}"`,
    );
    let response: Response;
    try {
      response = await fetch("/dicweb/instance", {
        method: "POST",
        headers,
        body,
        signal: context.controller.signal,
      });
    } catch {
      if (context.controller.signal.aborted) {
        throw new OcelliError("Aborted", "operation was aborted");
      }
      throw new OcelliError("Network", "native endpoint unreachable");
    }
    this.#checkFresh(context, "wado-response");
    if (response.status === 200) {
      return response;
    }
    if (response.status === 400) {
      throw new OcelliError("InvalidRequest", "native request was rejected");
    }
    if (response.status === 413) {
      throw new OcelliError("AllocationRefused", "native request exceeded allocation limits");
    }
    if (response.status === 415) {
      throw new OcelliError("ContentType", "native endpoint refused the content type");
    }
    if (response.status === 422) {
      const code = await this.#readNativeErrorCode(context, response);
      if (code === "InvalidValue") {
        throw new OcelliError("InvalidValue", "native endpoint refused the value");
      }
      throw new OcelliError("CommitRefused", "native endpoint returned an unexpected code");
    }
    if (response.status === 503) {
      throw new DicomwebError("HttpStatus", 503);
    }
    throw new DicomwebError("HttpStatus", response.status);
  }

  async #executeInstance(
    context: WadoPendingContext,
    resolve: (metadata: StagedMetadata) => void,
    reject: (error: OcelliError) => void,
    entry: LiveEntry,
  ): Promise<void> {
    try {
      this.#checkFresh(context, "wado-collect");
      const { raw, boundary } = await this.#collectInstance(context);
      this.#checkFresh(context, "wado-copy");
      const owned = new ArrayBuffer(raw.byteLength);
      new Uint8Array(owned).set(raw);
      this.#checkFresh(context, "wado-hash");
      const computedDigest = await sha256Hex(owned);
      this.#checkFresh(context, "wado-post");
      const response = await this.#postInstance(context, owned, boundary);
      this.#checkFresh(context, "wado-body");
      const bytes = await readBoundedBody(response, 16 * 1024 * 1024);
      this.#checkFresh(context, "wado-decode");
      const metadata = decodeInstanceMetadata(bytes, {
        identity: context.identity,
        sourceNonce: context.sourceNonce,
        generation: context.generation,
        rawBodySha256: computedDigest,
        byteLength: raw.byteLength,
        boundary,
      });
      this.#checkFresh(context, "wado-stage");
      if (owned.byteLength !== raw.byteLength) {
        throw new OcelliError(
          "CommitRefused",
          "staged buffer length did not match the collected body",
        );
      }
      this.#staged = { body: owned, metadata };
      resolve(metadata);
    } catch (error: unknown) {
      const stale =
        this.#disposed ||
        this.#generation !== context.generation ||
        context.controller.signal.aborted;
      if (stale) {
        reject(new OcelliError("Aborted", "operation was replaced"));
        return;
      }
      if (error instanceof OcelliError) {
        reject(error);
        return;
      }
      reject(new OcelliError("Network", "wado instance retrieval failed"));
    } finally {
      this.#live.delete(entry);
      this.#drainQueued();
    }
  }

  takeStagedInstance(expected: StagedMetadata): OwnedEncodedInstance | null {
    const snapshot = this.#staged;
    if (snapshot === null || this.#disposed || snapshot.metadata.generation !== this.#generation) {
      return null;
    }
    const expectedRecord: unknown = expected;
    const meta = snapshot.metadata;
    try {
      if (!isRecord(expectedRecord)) {
        return null;
      }
      if (
        !exactKeys(expectedRecord, [
          "identity",
          "rawBodySha256",
          "sourceNonce",
          "generation",
          "byteLength",
          "representation",
        ])
      ) {
        return null;
      }
      const identityValue = expectedRecord["identity"];
      if (!isRecord(identityValue) || !exactKeys(identityValue, ["studyUid", "seriesUid", "instanceUid"])) {
        return null;
      }
      if (
        identityValue["studyUid"] !== meta.identity.studyUid ||
        identityValue["seriesUid"] !== meta.identity.seriesUid ||
        identityValue["instanceUid"] !== meta.identity.instanceUid
      ) {
        return null;
      }
      const representationValue = expectedRecord["representation"];
      if (!isRecord(representationValue) || !exactKeys(representationValue, ["field", "boundary"])) {
        return null;
      }
      if (representationValue["field"] !== "wado-rs-instances") {
        return null;
      }
      const expectedBoundary = representationValue["boundary"];
      const expectedSha = expectedRecord["rawBodySha256"];
      const expectedNonce = expectedRecord["sourceNonce"];
      const expectedGeneration = expectedRecord["generation"];
      const expectedByteLength = expectedRecord["byteLength"];
      if (
        typeof expectedSha !== "string" ||
        expectedSha !== meta.rawBodySha256 ||
        !hex64(expectedSha)
      ) {
        return null;
      }
      if (typeof expectedNonce !== "string" || expectedNonce !== meta.sourceNonce || !hex32(expectedNonce)) {
        return null;
      }
      if (
        typeof expectedGeneration !== "number" ||
        !Number.isSafeInteger(expectedGeneration) ||
        expectedGeneration !== meta.generation
      ) {
        return null;
      }
      if (
        typeof expectedByteLength !== "number" ||
        !Number.isSafeInteger(expectedByteLength) ||
        expectedByteLength !== meta.byteLength
      ) {
        return null;
      }
      if (typeof expectedBoundary !== "string" || expectedBoundary !== meta.representation.boundary) {
        return null;
      }
    } catch {
      return null;
    }
    if (this.#staged !== snapshot || this.#disposed || snapshot.metadata.generation !== this.#generation) {
      return null;
    }
    this.#staged = null;
    let transferredBody: ArrayBuffer;
    try {
      transferredBody = structuredClone(snapshot.body, { transfer: [snapshot.body] });
    } catch {
      throw new OcelliError("CommitRefused", "Staged instance transfer failed; refetch to recover.");
    }
    if (transferredBody.byteLength !== meta.byteLength) {
      throw new OcelliError("CommitRefused", "Staged instance transfer failed; refetch to recover.");
    }
    return { body: transferredBody, metadata: snapshot.metadata };
  }

  readStagedInfo(): StagedMetadata | null {
    return this.#staged !== null && !this.#disposed && this.#staged.metadata.generation === this.#generation
      ? this.#staged.metadata
      : null;
  }

  releaseStaged(): void {
    // The documented public release. The intent, cancel and dispose paths
    // release through the private #release, so an override of this method
    // can neither block nor observe an internal release.
    this.#release();
  }

  #release(): void {
    this.#staged = null;
  }

  // Test-only observation seam. Every public staged reader gates on the
  // current generation, which every intent and cancel advances, so no
  // public reader can separate a released body from a stale one. Under
  // vitest's in-source testing this block registers a peeker that reports
  // the staged body with no gate. The guard is undefined in every other
  // context, so the built module carries the block inert and nothing
  // registers on it.
  static {
    if (import.meta.vitest) {
      (DicomwebProvider as unknown as { __stagedBodyPeek?: StagedBodyPeek }).__stagedBodyPeek = (
        provider: DicomwebProvider,
      ): OwnedEncodedInstance | null => provider.#staged;
    }
  }
}

// The release of the staged body is observable only inside the class, so
// the release tests live here under vitest's in-source testing. The public
// readers gate on the generation every release advances, so they cannot
// show the release itself. These tests prove both internal release lines
// and the override contract: an override of releaseStaged can neither
// observe nor block an internal release.
if (import.meta.vitest) {
  const { it, afterEach, expect, vi } = import.meta.vitest;

  const peekStagedBody = (provider: DicomwebProvider): OwnedEncodedInstance | null => {
    const peek = (DicomwebProvider as unknown as { __stagedBodyPeek?: StagedBodyPeek }).__stagedBodyPeek;
    if (peek === undefined) {
      throw new Error("the staged body peeker did not register");
    }
    return peek(provider);
  };

  const sha256Hex = async (buffer: BufferSource): Promise<string> =>
    Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256", buffer)), (byte) =>
      byte.toString(16).padStart(2, "0"),
    ).join("");

  const STUDY_UID = "2.25.1";
  const SERIES_UID = "2.25.2";
  const INSTANCE_UID = "2.25.3";
  const IDENTITY = { studyUid: STUDY_UID, seriesUid: SERIES_UID, instanceUid: INSTANCE_UID };

  // One provider whose upstream serves `raw` as one synthetic multipart
  // instance and whose native route echoes the posted body in its receipt,
  // the same shapes the unit suite's richer fixtures use.
  function stagedProvider<C extends typeof DicomwebProvider>(
    raw: Uint8Array,
    boundary: string,
    ctor: C = DicomwebProvider as C,
  ): InstanceType<C> {
    const upstreamFetch: typeof fetch = async () =>
      new Response(raw.slice(), {
        status: 200,
        headers: { "Content-Type": `multipart/related; type="application/dicom"; boundary=${boundary}` },
      });
    const nativeFetch: typeof fetch = async (_input, init) => {
      // The provider posts the staged instance as one ArrayBuffer body.
      const body = new Uint8Array((init?.body ?? new ArrayBuffer(0)) as ArrayBuffer);
      const headers = new Headers(init?.headers);
      const envelope = {
        identity: IDENTITY,
        rawBodySha256: await sha256Hex(body),
        sourceNonce: headers.get("X-Wado-Source-Nonce") ?? "",
        generation: Number(headers.get("X-Wado-Generation") ?? "0"),
        byteLength: body.byteLength,
        representation: { field: "wado-rs-instances", boundary },
      };
      return new Response(JSON.stringify(envelope), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    };
    vi.stubGlobal("fetch", nativeFetch);
    // The constructor is constrained to the provider type, so the instance
    // it builds is the caller's refined class.
    return new ctor({
      studiesServiceUrl: "https://upstream.example/dicomweb",
      wadoUriUrl: "https://upstream.example/wado",
      fetch: upstreamFetch,
    }) as InstanceType<C>;
  }

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("releases the staged body when a cancel runs with no new intent", async () => {
    const provider = stagedProvider(new Uint8Array([10, 20, 30, 40]), "f245-insource-cancel");
    try {
      const staged = await provider.fetchSelectedInstance(IDENTITY);
      expect(staged.byteLength).toBe(4);
      provider.cancelPending();
      expect(peekStagedBody(provider)).toBeNull();
      expect(provider.readStagedInfo()).toBeNull();
    } finally {
      provider.dispose();
    }
  });

  it("clears a staged body when a newer intent begins", async () => {
    const provider = stagedProvider(new Uint8Array([2, 7, 1, 8]), "f245-insource-intent");
    try {
      const staged = await provider.fetchSelectedInstance(IDENTITY);
      expect(staged.byteLength).toBe(4);
      const pending = provider.fetchSelectedInstance(IDENTITY);
      // The intent releases before the newer fetch stages, so this read
      // sits between the two and cannot be satisfied by the newer staging.
      expect(peekStagedBody(provider)).toBeNull();
      const settled = await pending;
      expect(settled.generation).toBe(staged.generation + 1);
    } finally {
      provider.dispose();
    }
  });

  it("releases on cancel even when an override of releaseStaged forgets super", async () => {
    class OverrideWithoutSuper extends DicomwebProvider {
      override releaseStaged(): void {
        // A consumer override that releases nothing.
      }
    }
    const provider = stagedProvider(new Uint8Array([3, 1, 4, 1, 5]), "f245-insource-override", OverrideWithoutSuper);
    try {
      const staged = await provider.fetchSelectedInstance(IDENTITY);
      expect(staged.byteLength).toBe(5);
      provider.cancelPending();
      expect(peekStagedBody(provider)).toBeNull();
    } finally {
      provider.dispose();
    }
  });

  it("releases on dispose even when an override of releaseStaged forgets super", async () => {
    class OverrideWithoutSuper extends DicomwebProvider {
      override releaseStaged(): void {
        // A consumer override that releases nothing.
      }
    }
    const provider = stagedProvider(new Uint8Array([1, 6, 1, 8]), "f245-insource-dispose", OverrideWithoutSuper);
    try {
      const staged = await provider.fetchSelectedInstance(IDENTITY);
      expect(staged.byteLength).toBe(4);
      provider.dispose();
      expect(peekStagedBody(provider)).toBeNull();
    } finally {
      provider.dispose();
    }
  });

  it("releases on dispose even when an override of cancelPending forgets super", async () => {
    class CancelOverrideWithoutSuper extends DicomwebProvider {
      override cancelPending(): void {
        // A consumer override that cancels nothing.
      }
    }
    const provider = stagedProvider(new Uint8Array([5, 1, 4]), "f245-insource-cancel-override", CancelOverrideWithoutSuper);
    try {
      const staged = await provider.fetchSelectedInstance(IDENTITY);
      expect(staged.byteLength).toBe(3);
      provider.dispose();
      expect(peekStagedBody(provider)).toBeNull();
    } finally {
      provider.dispose();
    }
  });

  it("does not route an internal release through the public releaseStaged", async () => {
    class RecordingOverride extends DicomwebProvider {
      routed = 0;
      override releaseStaged(): void {
        this.routed += 1;
        super.releaseStaged();
      }
    }
    const provider = stagedProvider(new Uint8Array([9, 9, 9, 9]), "f245-insource-routing", RecordingOverride);
    try {
      await provider.fetchSelectedInstance(IDENTITY);
      provider.cancelPending();
      expect(provider.routed).toBe(0);
      expect(peekStagedBody(provider)).toBeNull();
    } finally {
      provider.dispose();
    }
  });
}
