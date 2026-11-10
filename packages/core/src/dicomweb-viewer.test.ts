import type { InstanceIdentity, StagedMetadata } from "./index.js";
import { afterEach, expect, vi } from "vitest";
import { DicomwebProvider } from "./index.js";
import { OcelliError } from "./ocelli-error.js";

const STUDY_UID = "2.25.1";
const SERIES_UID = "2.25.2";
const UPSTREAM_BYTES = new Uint8Array([
  ...new TextEncoder().encode("[]"),
  0x20,
]);
const UPSTREAM_URL = "https://upstream.example/dicomweb";
const WADO_URL = "https://upstream.example/wado";
const NATIVE_BASE = "http://native.example";

interface NativeCall {
  url: string;
  method: string;
  headers: Headers;
  bodyBytes: Uint8Array;
}

interface UpstreamCall {
  url: string;
  headers: Headers;
}

interface CapturedCalls {
  native: NativeCall[];
  upstream: UpstreamCall[];
}

type QidoLevel = "study" | "series" | "instance";

interface NativeEnvelope {
  level: QidoLevel;
  rows: readonly unknown[];
  capturedScope: Record<string, string>;
  sourceNonce: string;
  generation: number;
  rawBodySha256: string;
}

function observedUrl(input: RequestInfo | URL): string {
  if (typeof input === "string") {
    return input;
  }
  if (input instanceof URL) {
    return input.href;
  }
  return input.url;
}

function observedHeaders(input: RequestInfo | URL, init: RequestInit | undefined): Headers {
  if (init?.headers !== undefined) {
    return new Headers(init.headers);
  }
  if (input instanceof Request) {
    return input.headers;
  }
  return new Headers();
}

function observedMethod(input: RequestInfo | URL, init: RequestInit | undefined): string {
  if (init?.method !== undefined) {
    return init.method;
  }
  if (input instanceof Request) {
    return input.method;
  }
  return "GET";
}

async function readBodyBytes(input: RequestInfo | URL, init: RequestInit | undefined): Promise<Uint8Array> {
  let newRequest: Request;
  if (input instanceof Request) {
    newRequest = new Request(input.clone(), init);
  } else {
    newRequest = new Request(new URL(observedUrl(input), NATIVE_BASE), init);
  }
  const buffer = await newRequest.arrayBuffer();
  return new Uint8Array(buffer);
}

function makeProvider(
  nativeResponseFactory: (call: NativeCall) => Promise<Response>,
  captured: CapturedCalls,
): DicomwebProvider {
  const upstreamFetch: typeof fetch = async (input, init) => {
    captured.upstream.push({
      url: observedUrl(input),
      headers: observedHeaders(input, init),
    });
    return new Response(UPSTREAM_BYTES.slice(), {
      status: 200,
      headers: { "Content-Type": "application/dicom+json" },
    });
  };
  const nativeFetch: typeof fetch = async (input, init) => {
    const call: NativeCall = {
      url: observedUrl(input),
      method: observedMethod(input, init),
      headers: observedHeaders(input, init),
      bodyBytes: await readBodyBytes(input, init),
    };
    captured.native.push(call);
    return nativeResponseFactory(call);
  };
  vi.stubGlobal("fetch", nativeFetch);
  return new DicomwebProvider({
    studiesServiceUrl: UPSTREAM_URL,
    wadoUriUrl: WADO_URL,
    fetch: upstreamFetch,
  });
}

async function sha256Hex(bytes: Uint8Array): Promise<string> {
  const owned = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(owned).set(bytes);
  const digest = await crypto.subtle.digest("SHA-256", owned);
  return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, "0")).join("");
}

async function nativeOkEnvelope(call: NativeCall, level: QidoLevel, rows: readonly unknown[]): Promise<NativeEnvelope> {
  return {
    level,
    rows,
    capturedScope:
      level === "study"
        ? {}
        : level === "series"
          ? { studyUid: STUDY_UID }
          : { studyUid: STUDY_UID, seriesUid: SERIES_UID },
    sourceNonce: call.headers.get("X-Qido-Source-Nonce") ?? "",
    generation: Number(call.headers.get("X-Qido-Generation") ?? "0"),
    rawBodySha256: await sha256Hex(UPSTREAM_BYTES),
  };
}

async function nativeOkResponse(
  call: NativeCall,
  level: QidoLevel,
  rows: readonly unknown[],
): Promise<Response> {
  const body = await nativeOkEnvelope(call, level, rows);
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { "Content-Type": "application/json" },
  });
}

async function expectOcelliCode(promise: Promise<unknown>, code: string): Promise<void> {
  try {
    await promise;
  } catch (error: unknown) {
    if (!(error instanceof OcelliError)) {
      throw error;
    }
    expect(error.code).toBe(code);
    return;
  }
  expect.fail("expected rejection");
}

import { describe, it } from "vitest";
import type { QidoEnvelope, QidoStudyCapturedScope } from "./index.js";

const VALID_STUDY_ROWS: readonly { studyUid: string }[] = [
  { studyUid: "2.25.10.1" },
];

interface PositiveCase {
  readonly name: string;
  readonly level: QidoLevel;
  readonly run: (provider: DicomwebProvider) => Promise<QidoEnvelope>;
  readonly rows: readonly unknown[];
  readonly expectedHeaders: Readonly<Record<string, string>>;
  readonly absentHeaders: readonly string[];
  readonly capturedScope: Record<string, string>;
}

const POSITIVE_CASES: readonly PositiveCase[] = [
  {
    name: "study level forwards rows with absent parents and echoes nonce/generation",
    level: "study",
    run: (provider) => provider.searchStudies(),
    rows: [
      { studyUid: "2.25.100.1" },
      { studyUid: "2.25.100.2" },
      { studyUid: "2.25.100.3" },
    ],
    expectedHeaders: { "X-Qido-Level": "study" },
    absentHeaders: ["X-Qido-Parent-Study", "X-Qido-Parent-Series"],
    capturedScope: {},
  },
  {
    name: "series level forwards study parent with absent returned parent row",
    level: "series",
    run: (provider) => provider.searchSeries(STUDY_UID),
    rows: [{ seriesUid: "2.25.200.1" }],
    expectedHeaders: { "X-Qido-Level": "series", "X-Qido-Parent-Study": STUDY_UID },
    absentHeaders: ["X-Qido-Parent-Series"],
    capturedScope: { studyUid: STUDY_UID },
  },
  {
    name: "series level forwards study parent with returned parent row",
    level: "series",
    run: (provider) => provider.searchSeries(STUDY_UID),
    rows: [{ seriesUid: "2.25.200.2", returnedStudyUid: STUDY_UID }],
    expectedHeaders: { "X-Qido-Level": "series", "X-Qido-Parent-Study": STUDY_UID },
    absentHeaders: ["X-Qido-Parent-Series"],
    capturedScope: { studyUid: STUDY_UID },
  },
  {
    name: "series level empty rows with study parent",
    level: "series",
    run: (provider) => provider.searchSeries(STUDY_UID),
    rows: [],
    expectedHeaders: { "X-Qido-Level": "series", "X-Qido-Parent-Study": STUDY_UID },
    absentHeaders: ["X-Qido-Parent-Series"],
    capturedScope: { studyUid: STUDY_UID },
  },
  {
    name: "instance level forwards both parents with absent returned parents row",
    level: "instance",
    run: (provider) => provider.searchInstances(STUDY_UID, SERIES_UID),
    rows: [{ instanceUid: "2.25.300.1" }],
    expectedHeaders: {
      "X-Qido-Level": "instance",
      "X-Qido-Parent-Study": STUDY_UID,
      "X-Qido-Parent-Series": SERIES_UID,
    },
    absentHeaders: [],
    capturedScope: { studyUid: STUDY_UID, seriesUid: SERIES_UID },
  },
  {
    name: "instance level rows match returned parents",
    level: "instance",
    run: (provider) => provider.searchInstances(STUDY_UID, SERIES_UID),
    rows: [
      {
        instanceUid: "2.25.300.2",
        returnedStudyUid: STUDY_UID,
        returnedSeriesUid: SERIES_UID,
      },
    ],
    expectedHeaders: {
      "X-Qido-Level": "instance",
      "X-Qido-Parent-Study": STUDY_UID,
      "X-Qido-Parent-Series": SERIES_UID,
    },
    absentHeaders: [],
    capturedScope: { studyUid: STUDY_UID, seriesUid: SERIES_UID },
  },
  {
    name: "instance level empty rows still echo nonce/generation",
    level: "instance",
    run: (provider) => provider.searchInstances(STUDY_UID, SERIES_UID),
    rows: [],
    expectedHeaders: {
      "X-Qido-Level": "instance",
      "X-Qido-Parent-Study": STUDY_UID,
      "X-Qido-Parent-Series": SERIES_UID,
    },
    absentHeaders: [],
    capturedScope: { studyUid: STUDY_UID, seriesUid: SERIES_UID },
  },
  {
    name: "study level empty rows still echo nonce/generation",
    level: "study",
    run: (provider) => provider.searchStudies(),
    rows: [],
    expectedHeaders: { "X-Qido-Level": "study" },
    absentHeaders: ["X-Qido-Parent-Study", "X-Qido-Parent-Series"],
    capturedScope: {},
  },
];

describe("QIDO forwarded", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it.for(POSITIVE_CASES)(
    "forwards $level query through native route: %s",
    async (testCase) => {
      const captured: CapturedCalls = { native: [], upstream: [] };
      const provider = makeProvider(
        async (call) => nativeOkResponse(call, testCase.level, testCase.rows),
        captured,
      );
      const result = await testCase.run(provider);
      const first = captured.native[0];
      if (first === undefined) {
        throw new Error("expected native POST to be recorded");
      }
      expect(captured.upstream).toHaveLength(1);
      expect(result.level).toBe(testCase.level);
      expect(result.capturedScope).toEqual(testCase.capturedScope);
      expect(result.rows).toEqual(testCase.rows);
      expect(result.rawBodySha256).toBe(await sha256Hex(UPSTREAM_BYTES));
      expect(result.sourceNonce).toBe(
        first.headers.get("X-Qido-Source-Nonce") ?? "",
      );
      expect(result.generation).toBe(
        Number(first.headers.get("X-Qido-Generation") ?? "0"),
      );
      expect(new URL(first.url, NATIVE_BASE).pathname).toBe("/dicweb/qido");
      expect(first.method).toBe("POST");
      expect(first.bodyBytes).toEqual(UPSTREAM_BYTES);
      expect(first.headers.get("X-Qido-Source-Nonce")).toMatch(/^[0-9a-f]{32}$/);
      const generationHeader = first.headers.get("X-Qido-Generation");
      expect(generationHeader).toMatch(/^[1-9][0-9]{0,15}$/);
      expect(String(Number(generationHeader))).toBe(generationHeader);
      expect(Number(first.headers.get("X-Qido-Generation"))).toBeGreaterThan(0);
      expect(Number(first.headers.get("X-Qido-Generation"))).toBeLessThanOrEqual(9007199254740991);
      expect(Number.isSafeInteger(Number(first.headers.get("X-Qido-Generation")))).toBe(true);
      for (const [headerName, headerValue] of Object.entries(testCase.expectedHeaders)) {
        expect(first.headers.get(headerName)).toBe(headerValue);
      }
      for (const absentName of testCase.absentHeaders) {
        expect(first.headers.get(absentName)).toBeNull();
      }
    },
  );
});

interface MutationCase {
  readonly name: string;
  readonly mutate: (body: NativeEnvelope) => NativeEnvelope;
}

const MUTATION_CASES: readonly MutationCase[] = [
  {
    name: "wrong level",
    mutate: (body) => ({ ...body, level: "instance" }),
  },
  {
    name: "wrong capturedScope",
    mutate: (body) => ({ ...body, capturedScope: { studyUid: "9.9.9" } }),
  },
  {
    name: "extra root key",
    mutate: (body) => ({ ...body, unexpected: "surprise" }),
  },
  {
    name: "wrong digest",
    mutate: (body) => ({ ...body, rawBodySha256: "deadbeef".repeat(8) }),
  },
];

describe("QIDO forwarded native envelope validation", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it.for(MUTATION_CASES)(
    "rejects native 200 envelope with $name",
    async (mutationCase) => {
      const captured: CapturedCalls = { native: [], upstream: [] };
      const provider = makeProvider(
        async (call) => {
          const valid = await nativeOkEnvelope(call, "study", VALID_STUDY_ROWS);
          return new Response(
            JSON.stringify(mutationCase.mutate(valid)),
            { status: 200, headers: { "Content-Type": "application/json" } },
          );
        },
        captured,
      );
      await expectOcelliCode(provider.searchStudies(), "CommitRefused");
      expect(captured.native).toHaveLength(1);
    },
  );

  it("rejects native 422 InvalidValue envelope", async () => {
    const captured: CapturedCalls = { native: [], upstream: [] };
    const provider = makeProvider(
      async () =>
        new Response(JSON.stringify({ code: "InvalidValue" }), {
          status: 422,
          headers: { "Content-Type": "application/json" },
        }),
      captured,
    );
    await expectOcelliCode(provider.searchStudies(), "InvalidValue");
    expect(captured.native).toHaveLength(1);
  });
});
describe("QIDO streamed cap refusal at 2MiB+1", () => {
  const STREAM_CAP_BYTES = 2097153;
  const CHUNK_BOUND = 65536;
  const UPSTREAM_URL = "https://upstream.example/dicomweb";
  const WADO_URL = "https://upstream.example/wado";
  function makeStreamedUpstreamBody(totalBytes: number): ReadableStream<Uint8Array> {
    let emitted = 0;
    let closed = false;
    return new ReadableStream<Uint8Array>({
      pull(controller) {
        if (closed || emitted >= totalBytes) {
          closed = true;
          controller.close();
          return;
        }
        const size = Math.min(CHUNK_BOUND, totalBytes - emitted);
        const chunk = new Uint8Array(size);
        chunk.fill(0x20);
        if (emitted === 0) {
          chunk[0] = 0x5b;
          chunk[1] = 0x5d;
        }
        controller.enqueue(chunk);
        emitted += size;
      },
      cancel() {
        closed = true;
      },
    });
  }

  function streamFetch(
    captured: { count: number },
    nativeTripwire: (...args: Parameters<typeof fetch>) => unknown,
  ): typeof fetch {
    const injected: typeof fetch = async (input, init) => {
      void input;
      void init;
      captured.count += 1;
      return new Response(
        makeStreamedUpstreamBody(STREAM_CAP_BYTES),
        {
          status: 200,
          headers: { "Content-Type": "application/dicom+json" },
        },
      );
    };
    const native: typeof fetch = async (...args: Parameters<typeof fetch>) => {
      nativeTripwire(...args);
      return new Response(JSON.stringify({ code: "AllocationRefused" }), {
        status: 413,
        headers: { "Content-Type": "application/json" },
      });
    };
    vi.stubGlobal("fetch", native);
    return injected;
  }

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("refuses a 2MiB+1 streamed QIDO body with AllocationRefused and never calls native fetch", async () => {
    const captured = { count: 0 };
    const nativeTripwire = vi.fn();
    const injected = streamFetch(captured, nativeTripwire);
    const provider = new DicomwebProvider({
      studiesServiceUrl: UPSTREAM_URL,
      wadoUriUrl: WADO_URL,
      fetch: injected,
    });
    let caught: unknown;
    try {
      await provider.searchStudies({ limit: 1 });
    } catch (error: unknown) {
      caught = error;
    } finally {
      provider.dispose();
    }
    if (!(caught instanceof OcelliError)) {
      throw new Error("expected OcelliError");
    }
    expect(nativeTripwire).toHaveBeenCalledTimes(0);
    expect(captured.count).toBe(1);
    expect(caught.code).toBe("AllocationRefused");
  });
});
const emptyStudyScope: QidoStudyCapturedScope = {};
type NonemptyStudyScopeRejected = { studyUid: string } extends QidoStudyCapturedScope ? false : true;
const nonemptyStudyScopeRejected: NonemptyStudyScopeRejected = true;
void emptyStudyScope;
void nonemptyStudyScopeRejected;


describe('DicomwebProvider constructor option validation', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it.each([['valid', {}], ['string', { bogus: 'yes' }], ['symbol', { [Symbol('bogus')]: true }]] as const)('refuses unknown option: %s', async (_name, extra) => {
    const upstreamSpy = vi.fn(async () => new Response('[]', { headers: { 'Content-Type': 'application/dicom+json' } }));
    const nativeSpy = vi.fn(async () => new Response('{}', { headers: { 'Content-Type': 'application/json' } }));
    vi.stubGlobal('fetch', nativeSpy);

    const base = {
      studiesServiceUrl: 'https://upstream.example/dicomweb',
      wadoUriUrl: 'https://upstream.example/wado',
      fetch: upstreamSpy,
    };
    const options = Object.assign({}, base, extra);
    let caught: unknown;
    let provider: DicomwebProvider | undefined;
    try {
      provider = new DicomwebProvider(options);
    } catch (error: unknown) {
      caught = error;
    } finally {
      provider?.dispose();
    }

    if (_name === 'valid') {
      expect(caught).toBeUndefined();
      expect(provider).toBeDefined();
    } else {
      if (!(caught instanceof OcelliError)) throw new Error('expected OcelliError');
      expect(caught.code).toBe('UnknownOption');
    }
    expect(upstreamSpy.mock.calls.length).toBe(0);
    expect(nativeSpy.mock.calls.length).toBe(0);
  });
});

import { DicomwebError } from "./dicomweb.js";

describe("DicomwebProvider native boundary", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("accepts study QIDO when the native header overreports Content-Length", async () => {
    const captured: CapturedCalls = { native: [], upstream: [] };
    const provider = makeProvider(async (call) => {
      const response = await nativeOkResponse(call, "study", [{ studyUid: STUDY_UID }]);
      response.headers.set("Content-Length", "16777217");
      return response;
    }, captured);
    try {
      const envelope = await provider.searchStudies();
      expect(envelope.level).toBe("study");
      expect(envelope.rows).toEqual([{ studyUid: STUDY_UID }]);
      expect(envelope.capturedScope).toEqual({});
      expect(envelope.sourceNonce).toMatch(/^[0-9a-f]{32}$/);
      expect(typeof envelope.generation).toBe("number");
      expect(envelope.generation).toBeGreaterThan(0);
      expect(envelope.generation).toBeLessThanOrEqual(9007199254740991);
      expect(envelope.rawBodySha256).toMatch(/^[0-9a-f]{64}$/);
      expect(captured.native).toHaveLength(1);
    } finally {
      provider.dispose();
    }
  });

  it("maps native 500 CommitRefused JSON to DicomwebError HttpStatus with status 500", async () => {
    const captured: CapturedCalls = { native: [], upstream: [] };
    const provider = makeProvider(async () => new Response(JSON.stringify({ code: "CommitRefused" }), { status: 500, headers: { "Content-Type": "application/json" } }), captured);
    try {
      let caught: unknown;
      try {
        await provider.searchStudies();
        expect.fail("expected rejection");
      } catch (error: unknown) {
        caught = error;
      }
      if (!(caught instanceof DicomwebError)) {
        throw new Error("expected DicomwebError");
      }
      expect(caught.code).toBe("HttpStatus");
      expect(caught.status).toBe(500);
      expect(captured.native).toHaveLength(1);
    } finally {
      provider.dispose();
    }
  });
});


import type { QidoSearchOptions } from "./dicomweb.js";

afterEach(vi.unstubAllGlobals);

const sixCases: { level: QidoLevel; extras: boolean; expected: string[] }[] = [
  { level: "study", extras: false, expected: ["0020000D"] },
  { level: "series", extras: false, expected: ["0020000D", "0020000E"] },
  { level: "instance", extras: false, expected: ["0020000D", "0020000E", "00080018"] },
  { level: "study", extras: true, expected: ["0020000D", "00080060", "0020000E", "00080018"] },
  { level: "series", extras: true, expected: ["0020000D", "0020000E", "00080060", "00080018"] },
  { level: "instance", extras: true, expected: ["0020000D", "0020000E", "00080018", "00080060"] },
];

it.each(sixCases)(
  "includes $expected for $level with extras=$extras and never mutates caller arrays",
  async ({ level, extras, expected }) => {
    const captured: CapturedCalls = { native: [], upstream: [] };
    const caller: string[] = ["00080060", "0020000D", "00080060", "0020000E", "00080018"];
    const snapshot: string[] = [...caller];
    const provider = makeProvider((call) => nativeOkResponse(call, level, []), captured);
    try {
      const options: QidoSearchOptions | undefined = extras ? { includeFields: caller } : undefined;
      if (level === "study") {
        await provider.searchStudies(options);
      } else if (level === "series") {
        await provider.searchSeries(STUDY_UID, options);
      } else {
        await provider.searchInstances(STUDY_UID, SERIES_UID, options);
      }
      expect(captured.native).toHaveLength(1);
      expect(captured.upstream).toHaveLength(1);
      const actual = captured.upstream[0];
      if (actual === undefined) {
        throw new Error("upstream call missing");
      }
      expect(new URL(actual.url).searchParams.getAll("includefield")).toEqual(expected);
      expect(caller).toEqual(snapshot);
      caller.push("00080020");
      expect(caller).toEqual([...snapshot, "00080020"]);
      expect(new URL(actual.url).searchParams.getAll("includefield")).toEqual(expected);
    } finally {
      provider.dispose();
    }
  },
);


interface HeldNative {
  readonly call: NativeCall;
  readonly promise: Promise<Response>;
  resolve: (response: Response) => void;
}

function holdNative(call: NativeCall): HeldNative {
  let capturedResolve: ((response: Response) => void) | undefined;
  const promise = new Promise<Response>((resolve) => {
    capturedResolve = resolve;
  });
  if (capturedResolve === undefined) {
    throw new Error("unreachable");
  }
  return { call, promise, resolve: capturedResolve };
}

describe('searchSeries replacement with unresolved prior held at native gate', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });
  it.each([
    { label: 'empty rows', rows: [] as const },
    { label: 'missing parent row', rows: [{ seriesUid: '2.25.9' }] as const },
  ])('resolves the newer intent, aborts the held older intent, and leaves the provider usable ($label)', async ({ rows }) => {
    const latestRows = rows;
    const captured: CapturedCalls = { native: [], upstream: [] };
    const heldOld: HeldNative[] = [];
    const factory = async (call: NativeCall): Promise<Response> => {
      if (heldOld.length === 0) {
        const held = holdNative(call);
        heldOld.push(held);
        return held.promise;
      }
      return nativeOkResponse(call, 'series', latestRows);
    };
    const provider = makeProvider(factory, captured);
    try {
      const older = provider.searchSeries(STUDY_UID);
      const observedOlder = older.catch((error: unknown) => error);
      await vi.waitFor(() => {
        expect(captured.native.length).toBe(1);
        expect(heldOld[0]).toBeDefined();
      });
      const newer = provider.searchSeries(STUDY_UID);
      void newer.catch(() => undefined);
      const newerResult = await newer;
      expect(newerResult.level).toBe('series');
      expect(newerResult.capturedScope).toEqual({ studyUid: STUDY_UID });
      expect(newerResult.rows).toEqual(latestRows);
      expect(captured.native.length).toBe(2);
      const oldHeld = heldOld[0];
      if (oldHeld === undefined) {
        throw new Error('held older native request was not captured');
      }
      const oldCall = oldHeld.call;
      oldHeld.resolve(await nativeOkResponse(oldCall, 'series', [{ seriesUid: '2.25.8' }]));
      const olderResult = await observedOlder;
      if (!(olderResult instanceof OcelliError)) {
        throw new Error('expected OcelliError for the aborted older intent');
      }
      expect(olderResult.code).toBe('Aborted');
      const fresh = provider.searchSeries(STUDY_UID);
      const freshResult = await fresh;
      expect(freshResult.level).toBe('series');
      expect(freshResult.capturedScope).toEqual({ studyUid: STUDY_UID });
      expect(freshResult.rows).toEqual(latestRows);
      expect(captured.native.length).toBe(3);
    } finally {
      provider.dispose();
      for (const held of heldOld) {
        held.resolve(new Response('{"code":"HttpStatus"}', { status: 503, headers: { 'Content-Type': 'application/json' } }));
      }
    }
  });
});


it("replaces latest queued qido without fetching and settles replacement with generated headers", async () => {
  const held: HeldNative[] = [];
  const captured: CapturedCalls = { native: [], upstream: [] };
  const provider = makeProvider(async (call) => {
    const h = holdNative(call);
    held.push(h);
    return h.promise;
  }, captured);
  try {
    const q1 = provider.searchStudies();
    const q1Observed = q1.catch((e: unknown) => e);
    await vi.waitFor(() => expect(held.length).toBe(1));
    const q2 = provider.searchStudies();
    const q2Observed = q2.catch((e: unknown) => e);
    await vi.waitFor(() => expect(held.length).toBe(2));

    const mutableFilter: [string, string] = ['00080060', 'CT'];
    const mutableFilters = [mutableFilter];
    const mutableFields: string[] = ["0020000D"];
    const q3 = provider.searchStudies();
    const q3Observed = q3.catch((e: unknown) => e);
    const q4 = provider.searchStudies({ filters: mutableFilters, includeFields: mutableFields });
    const q4Observed = q4.catch((e: unknown) => e);
    mutableFilter[1] = 'MR';
    mutableFields.push("00080020");

    let q3Settled = false;
    let q3Value: unknown;
    void q3Observed.then((v: unknown) => {
      q3Settled = true;
      q3Value = v;
    });
    await vi.waitFor(() => expect(q3Settled).toBe(true));
    expect(q3Value).toBeInstanceOf(OcelliError);
    if (q3Value instanceof OcelliError) {
      expect(q3Value.code).toBe("Aborted");
    }
    await vi.waitFor(() => expect(captured.native.length).toBe(2));
    await vi.waitFor(() => expect(captured.upstream.length).toBe(2));
    await vi.waitFor(() => expect(held.length).toBe(2));

    if (held[0] === undefined || held[1] === undefined) {
      throw new Error("held operations missing");
    }
    const first = held[0];
    const second = held[1];
    first.resolve(new Response(JSON.stringify({ code: "InvalidRequest" }), {
      status: 400,
      headers: { "Content-Type": "application/json" },
    }));
    const q1Error = await q1Observed;
    if (!(q1Error instanceof OcelliError)) throw new Error();
    expect(q1Error.code).toBe('Aborted');
    await vi.waitFor(() => expect(captured.native.length).toBe(3));
    await vi.waitFor(() => expect(captured.upstream.length).toBe(3));
    await vi.waitFor(() => expect(held.length).toBe(3));

    if (held[2] === undefined) {
      throw new Error("third held operation missing");
    }
    const third = held[2];
    const upstream = captured.upstream[2];
    if (upstream === undefined) {
      throw new Error("third upstream missing");
    }
    const upstreamUrl = new URL(upstream.url);
    expect(upstreamUrl.searchParams.get("00080060")).toBe("CT");
    expect(upstreamUrl.searchParams.getAll("includefield")).toEqual(["0020000D"]);
    const secondGeneration = Number(second.call.headers.get("X-Qido-Generation") ?? "0");
    expect(Number(third.call.headers.get("X-Qido-Generation"))).toBe(secondGeneration + 2);
    expect(third.call.headers.get("X-Qido-Source-Nonce")).not.toBe(
      second.call.headers.get("X-Qido-Source-Nonce"),
    );
    const expectedNonce = third.call.headers.get("X-Qido-Source-Nonce");

    second.resolve(new Response(JSON.stringify({ code: "HttpStatus" }), {
      status: 503,
      headers: { "Content-Type": "application/json" },
    }));
    const q2Error = await q2Observed;
    if (!(q2Error instanceof OcelliError)) throw new Error();
    expect(q2Error.code).toBe('Aborted');

    third.resolve(await nativeOkResponse(third.call, "study", []));
    const result = await q4;
    expect(result.level).toBe("study");
    expect(result.rows).toEqual([]);
    expect(result.capturedScope).toEqual({});
    expect(result.generation).toBe(Number(third.call.headers.get("X-Qido-Generation")));
    expect(result.sourceNonce).toBe(expectedNonce);
    expect(result.rawBodySha256).toBe(await sha256Hex(UPSTREAM_BYTES));
    await expect(q4Observed).resolves.toBe(result);
    await vi.waitFor(() => expect(captured.native.length).toBe(3));
    await vi.waitFor(() => expect(captured.upstream.length).toBe(3));
  } finally {
    provider.dispose();
    for (const h of held) {
      h.resolve(new Response(JSON.stringify({ code: "HttpStatus" }), {
        status: 503,
        headers: { "Content-Type": "application/json" },
      }));
    }
  }
});


const INSTANCE_UID = '2.25.3';

function makeWadoProvider(
  raw: Uint8Array,
  boundary: string,
  nativeFactory: (call: NativeCall) => Promise<Response>,
  captured: CapturedCalls,
): DicomwebProvider {
  const upstreamFetch: typeof fetch = async (input, init) => {
    captured.upstream.push({
      url: observedUrl(input),
      headers: observedHeaders(input, init),
    });
    return new Response(raw.slice(), {
      status: 200,
      headers: {
        "Content-Type": `multipart/related; type="application/dicom"; boundary=${boundary}`,
      },
    });
  };
  const nativeFetch: typeof fetch = async (input, init) => {
    const call: NativeCall = {
      url: observedUrl(input),
      method: observedMethod(input, init),
      headers: observedHeaders(input, init),
      bodyBytes: await readBodyBytes(input, init),
    };
    captured.native.push(call);
    return nativeFactory(call);
  };
  vi.stubGlobal("fetch", nativeFetch);
  return new DicomwebProvider({
    studiesServiceUrl: UPSTREAM_URL,
    wadoUriUrl: WADO_URL,
    fetch: upstreamFetch,
  });
}

async function nativeInstanceEnvelope(call: NativeCall, boundary: string) {
  return {
    identity: {
      studyUid: STUDY_UID,
      seriesUid: SERIES_UID,
      instanceUid: INSTANCE_UID,
    },
    rawBodySha256: await sha256Hex(call.bodyBytes),
    sourceNonce: call.headers.get("X-Wado-Source-Nonce") ?? "",
    generation: Number(call.headers.get("X-Wado-Generation") ?? "0"),
    byteLength: call.bodyBytes.byteLength,
    representation: { field: "wado-rs-instances", boundary },
  };
}

it("stages one WADO instance end to end and hands over exclusive ownership", async () => {
  const raw = new Uint8Array([10, 20, 30, 40]);
  const boundary = "f245-unit-boundary";
  const captured: CapturedCalls = { native: [], upstream: [] };
  const provider = makeWadoProvider(raw, boundary, async (call) => {
    const envelope = await nativeInstanceEnvelope(call, boundary);
    return new Response(JSON.stringify(envelope), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  }, captured);
  try {
    expect(provider.readStagedInfo()).toBeNull();
    const identity: { studyUid: string; seriesUid: string; instanceUid: string } = {
      studyUid: STUDY_UID,
      seriesUid: SERIES_UID,
      instanceUid: INSTANCE_UID,
    };
    const meta = await provider.fetchSelectedInstance(identity);
    expect(captured.native).toHaveLength(1);
    expect(captured.upstream).toHaveLength(1);
    if (captured.native[0] === undefined) throw new Error("no native call");
    if (captured.upstream[0] === undefined) throw new Error("no upstream call");
    const nativeCall = captured.native[0];
    const upstreamUrl = new URL(captured.upstream[0].url);
    expect(upstreamUrl.pathname).toBe("/dicomweb/studies/2.25.1/series/2.25.2/instances/2.25.3");
    const nativeUrl = new URL(nativeCall.url, NATIVE_BASE);
    expect(nativeUrl.pathname).toBe("/dicweb/instance");
    expect(nativeCall.method).toBe("POST");
    expect(new Uint8Array(nativeCall.bodyBytes)).toEqual(raw);
    expect(nativeCall.headers.get("X-Wado-Study-Instance-UID")).toBe("2.25.1");
    expect(nativeCall.headers.get("X-Wado-Series-Instance-UID")).toBe("2.25.2");
    expect(nativeCall.headers.get("X-Wado-SOP-Instance-UID")).toBe("2.25.3");
    expect(nativeCall.headers.get("Content-Type")).toContain(boundary);
    expect(nativeCall.headers.get("Authorization")).toBeNull();
    const expectedMeta = {
      identity: {
        studyUid: "2.25.1",
        seriesUid: "2.25.2",
        instanceUid: "2.25.3",
      },
      rawBodySha256: await sha256Hex(raw),
      byteLength: 4,
      representation: { field: "wado-rs-instances", boundary },
    };
    expect(meta).toMatchObject(expectedMeta);
    const actualNonce = nativeCall.headers.get("X-Wado-Source-Nonce");
    if (actualNonce === null) throw new Error("no nonce");
    expect(meta.sourceNonce).toBe(actualNonce);
    expect(meta.generation).toBe(Number(nativeCall.headers.get("X-Wado-Generation")));
    expect(Object.isFrozen(meta)).toBe(true);
    expect(Object.isFrozen(meta.identity)).toBe(true);
    expect(Object.isFrozen(meta.representation)).toBe(true);
    identity.studyUid = "2.25.999";
    const staged = provider.readStagedInfo();
    if (staged === null) throw new Error("staged lost");
    expect(staged.identity).toEqual({
      studyUid: "2.25.1",
      seriesUid: "2.25.2",
      instanceUid: "2.25.3",
    });
    const owned = provider.takeStagedInstance(meta);
    if (owned === null) throw new Error("take failed");
    expect(owned.body instanceof ArrayBuffer).toBe(true);
    expect(new Uint8Array(owned.body)).toEqual(raw);
    expect(owned.metadata).toEqual(meta);
    expect(provider.readStagedInfo()).toBeNull();
    expect(provider.takeStagedInstance(meta)).toBeNull();
  } finally {
    provider.dispose();
  }
});


it("reports CommitRefused for a wrong native digest then succeeds with a corrected digest", async () => {
  const raw = new Uint8Array([10, 20, 30, 40]);
  const boundary = "f245-unit-boundary";
  const captured: CapturedCalls = { native: [], upstream: [] };
  let badDigest = true;
  const identity = {
    studyUid: "2.25.1",
    seriesUid: "2.25.2",
    instanceUid: "2.25.3",
  };
  const provider = makeWadoProvider(
    raw,
    boundary,
    async (call) => {
      const dto = await nativeInstanceEnvelope(call, boundary);
      if (badDigest) {
        const flipped = dto.rawBodySha256[0] === "0" ? "1" : "0";
        const wrong = flipped + dto.rawBodySha256.slice(1);
        expect(wrong).not.toBe(dto.rawBodySha256);
        return new Response(JSON.stringify({ ...dto, rawBodySha256: wrong }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        });
      }
      return new Response(JSON.stringify(dto), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    },
    captured,
  );
  try {
    await expectOcelliCode(provider.fetchSelectedInstance(identity), "CommitRefused");
    expect(provider.readStagedInfo()).toBeNull();
    expect(captured.upstream.length).toBe(1);
    expect(captured.native.length).toBe(1);
    badDigest = false;
    const meta = await provider.fetchSelectedInstance(identity);
    const first = captured.native[0];
    if (first === undefined) {
      throw new Error("missing first native call");
    }
    expect(meta.rawBodySha256).toBe(await sha256Hex(raw));
    expect(meta.identity).toEqual(identity);
    expect(meta.byteLength).toBe(4);
    expect(meta.representation).toEqual({ field: "wado-rs-instances", boundary });
    expect(provider.readStagedInfo()).not.toBeNull();
    expect(provider.readStagedInfo()).toEqual(meta);
    const owned = provider.takeStagedInstance(meta);
    expect(owned).not.toBeNull();
    if (owned === null) {
      expect.fail("unreachable guard");
    }
    expect(new Uint8Array(owned.body)).toEqual(raw);
    expect(owned.metadata).toEqual(meta);
    expect(provider.readStagedInfo()).toBeNull();
    expect(captured.native.length).toBe(2);
    expect(captured.upstream.length).toBe(2);
    const retry = captured.native[1];
    if (retry === undefined) {
      throw new Error("missing retry native call");
    }
    const firstNonce = first.headers.get("X-Wado-Source-Nonce") ?? "";
    const retryNonce = retry.headers.get("X-Wado-Source-Nonce") ?? "";
    expect(retryNonce).not.toBe(firstNonce);
    expect(meta.sourceNonce).toBe(retryNonce);
    const firstGen = Number(first.headers.get("X-Wado-Generation") ?? "0");
    const retryGen = Number(retry.headers.get("X-Wado-Generation") ?? "0");
    expect(retryGen).toBe(firstGen + 1);
  } finally {
    provider.dispose();
  }
});


const ERROR_CASES: readonly {
  readonly name: string;
  readonly totalBytes: number;
  readonly expected: "InvalidValue" | "AllocationRefused";
}[] = [
  { name: "at cap InvalidValue", totalBytes: 16 * 1024 * 1024, expected: "InvalidValue" },
  { name: "cap plus one AllocationRefused", totalBytes: 16 * 1024 * 1024 + 1, expected: "AllocationRefused" },
];

const DECLARED_LENGTH = String(16 * 1024 * 1024 + 1);
const CHUNK = 64 * 1024;

function streamingErrorBody(totalBytes: number): ReadableStream<Uint8Array> {
  const prefix = new TextEncoder().encode(JSON.stringify({ code: "InvalidValue" }));
  let sent = 0;
  return new ReadableStream<Uint8Array>({
    pull(controller) {
      if (sent >= totalBytes) {
        controller.close();
        return;
      }
      const size = Math.min(CHUNK, totalBytes - sent);
      const chunk = new Uint8Array(size);
      chunk.fill(0x20);
      if (sent === 0) {
        chunk.set(prefix, 0);
      }
      sent += size;
      controller.enqueue(chunk);
    },
  });
}

describe("native 422 bounded error body", () => {
  it.each(ERROR_CASES)("$name", async ({ totalBytes, expected }) => {
    const captured: CapturedCalls = { native: [], upstream: [] };
    const provider = makeProvider(
      async () =>
        new Response(streamingErrorBody(totalBytes), {
          status: 422,
          headers: {
            "Content-Type": "application/json",
            "Content-Length": DECLARED_LENGTH,
          },
        }),
      captured,
    );
    try {
      await expectOcelliCode(provider.searchStudies(), expected);
    } finally {
      provider.dispose();
    }
    expect(captured.native).toHaveLength(1);
    expect(captured.upstream).toHaveLength(1);
    expect(captured.native[0]?.headers.get("Content-Type")).toBe("application/dicom+json");
    expect(captured.native[0]?.headers.get("X-Qido-Level")).toBe("study");
  });
});



it("transfers and detaches the staged buffer via structuredClone with an exact transfer list", async () => {
  const raw = new Uint8Array([10, 20, 30, 40]);
  const boundary = "f245-unit-boundary";
  const captured: CapturedCalls = { native: [], upstream: [] };
  const provider = makeWadoProvider(raw, boundary, async (call) => {
    const envelope = await nativeInstanceEnvelope(call, boundary);
    return new Response(JSON.stringify(envelope), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  }, captured);
  try {
    const meta = await provider.fetchSelectedInstance({
      studyUid: "2.25.1",
      seriesUid: "2.25.2",
      instanceUid: "2.25.3",
    });
    const spy = vi.spyOn(globalThis, "structuredClone");
    try {
      const owned = provider.takeStagedInstance(meta);
      if (owned === null) throw new Error("take failed");
      expect(new Uint8Array(owned.body)).toEqual(raw);
      expect(owned.metadata).toEqual(meta);
      expect(spy).toHaveBeenCalledTimes(1);
      const firstCall = spy.mock.calls[0];
      if (firstCall === undefined) throw new Error("no spy call");
      const originalInput = firstCall[0];
      expect(originalInput instanceof ArrayBuffer).toBe(true);
      if (!(originalInput instanceof ArrayBuffer)) throw new Error("not ArrayBuffer");
      const transferOptions = firstCall[1];
      expect(transferOptions).toBeDefined();
      if (transferOptions === undefined) throw new Error("no options");
      expect(transferOptions.transfer).toEqual([originalInput]);
      expect(() => new Uint8Array(originalInput)).toThrow(TypeError);
      expect(provider.readStagedInfo()).toBeNull();
      expect(provider.takeStagedInstance(meta)).toBeNull();
    } finally {
      spy.mockRestore();
    }
  } finally {
    provider.dispose();
  }
});


const WADO_CHUNK_BYTES = 65536;
const WADO_CHUNK_FILL = 0x65;
const WADO_UNIT_BOUNDARY = "f245-unit-boundary";

interface WadoStreamFixture {
  provider: DicomwebProvider;
  counts: { nativeCalls: number; upstreamCalls: number };
  nativeBodies: Uint8Array[];
}

function streamedWadoProvider(totalBytes: number): WadoStreamFixture {
  const counts = { nativeCalls: 0, upstreamCalls: 0 };
  const nativeBodies: Uint8Array[] = [];
  const upstreamFetch = async (): Promise<Response> => {
    counts.upstreamCalls += 1;
    let sent = 0;
    const stream = new ReadableStream<Uint8Array>({
      pull(controller) {
        if (sent >= totalBytes) {
          controller.close();
          return;
        }
        const size = Math.min(WADO_CHUNK_BYTES, totalBytes - sent);
        const chunk = new Uint8Array(size).fill(WADO_CHUNK_FILL);
        sent += size;
        controller.enqueue(chunk);
      },
    });
    return new Response(stream, {
      status: 200,
      headers: {
        "Content-Type": `multipart/related; type="application/dicom"; boundary=${WADO_UNIT_BOUNDARY}`,
      },
    });
  };
  const nativeFetch: typeof fetch = async (input, init) => {
    counts.nativeCalls += 1;
    if (totalBytes > 32 * 1024 * 1024) {
      return new Response(JSON.stringify({ code: "AllocationRefused" }), {
        status: 413,
      });
    }
    const call: NativeCall = {
      url: observedUrl(input),
      method: observedMethod(input, init),
      headers: observedHeaders(input, init),
      bodyBytes: await readBodyBytes(input, init),
    };
    nativeBodies.push(call.bodyBytes);
    const envelope = await nativeInstanceEnvelope(call, WADO_UNIT_BOUNDARY);
    return new Response(JSON.stringify(envelope), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  };
  vi.stubGlobal("fetch", nativeFetch);
  return {
    provider: new DicomwebProvider({
      studiesServiceUrl: UPSTREAM_URL,
      wadoUriUrl: WADO_URL,
      fetch: upstreamFetch,
    }),
    counts,
    nativeBodies,
  };
}

const WADO_STREAM_CASES: readonly { totalBytes: number; accepted: boolean }[] = [
  { totalBytes: 32 * 1024 * 1024, accepted: true },
  { totalBytes: 32 * 1024 * 1024 + 1, accepted: false },
];

it.each(WADO_STREAM_CASES)(
  "streams exact cap wado upstream body (totalBytes $totalBytes)",
  async ({ totalBytes, accepted }) => {
    const fixture = streamedWadoProvider(totalBytes);
    const identity = {
      studyUid: STUDY_UID,
      seriesUid: SERIES_UID,
      instanceUid: INSTANCE_UID,
    };
    try {
      if (accepted) {
        const meta = await fixture.provider.fetchSelectedInstance(identity);
        expect(meta.byteLength).toBe(32 * 1024 * 1024);
        const posted = fixture.nativeBodies[0];
        expect(posted).toBeDefined();
        if (posted === undefined) {
          expect.fail("missing native posted body");
        }
        expect(await sha256Hex(posted)).toBe(meta.rawBodySha256);
        expect(meta.representation.boundary).toBe(WADO_UNIT_BOUNDARY);
        expect(fixture.counts.nativeCalls).toBe(1);
        expect(meta.identity).toEqual(identity);
        expect(fixture.provider.readStagedInfo()).not.toBeNull();
        fixture.provider.releaseStaged();
        expect(fixture.provider.readStagedInfo()).toBeNull();
      } else {
        await expectOcelliCode(
          fixture.provider.fetchSelectedInstance(identity),
          "AllocationRefused",
        );
        expect(fixture.counts.nativeCalls).toBe(0);
        expect(fixture.provider.readStagedInfo()).toBeNull();
      }
      expect(fixture.counts.upstreamCalls).toBe(1);
    } finally {
      fixture.provider.dispose();
    }
  },
);

describe("QIDO native 422 refusal and explicit retry", () => {
it.each(
  [
    {
      name: "unreadable native 422 body refuses once then recovers on explicit retry",
      refusal: async (): Promise<Response> => {
        const stream = new ReadableStream<Uint8Array>({
          start(controller) {
            controller.error(new Error("synthetic unreadable stream"));
          },
        });
        return new Response(stream, { status: 422 });
      },
    },
    {
      name: "unknown client-only wire code Network refuses once then recovers on explicit retry",
      refusal: async (): Promise<Response> =>
        new Response(JSON.stringify({ code: "Network" }), {
          status: 422,
          headers: { "Content-Type": "application/json" },
        }),
    },
    {
      name: "extra-key 422 body refuses once then recovers on explicit retry",
      refusal: async (): Promise<Response> =>
        new Response(
          JSON.stringify({ code: "InvalidValue", extra: "synthetic" }),
          {
            status: 422,
            headers: { "Content-Type": "application/json" },
          },
        ),
    },
  ] satisfies ReadonlyArray<{
    readonly name: string;
    readonly refusal: () => Promise<Response>;
  }>,
)("$name", async ({ refusal }: { readonly name: string; readonly refusal: () => Promise<Response> }) => {
  let refusing = true;
  const captured: CapturedCalls = { native: [], upstream: [] };
  const provider = makeProvider(async (call: NativeCall) => {
    if (refusing) {
      return refusal();
    }
    return nativeOkResponse(call, "study", []);
  }, captured);
  try {
    await expectOcelliCode(provider.searchStudies(), "CommitRefused");
    const first = captured.native[0];
    if (first === undefined) {
      throw new Error("missing first native call");
    }
    expect(captured.native).toHaveLength(1);
    expect(captured.upstream).toHaveLength(1);
    refusing = false;
    const envelope = await provider.searchStudies();
    expect(envelope.level).toBe("study");
    expect(envelope.capturedScope).toEqual({});
    expect(envelope.rows).toEqual([]);
    expect(envelope.rawBodySha256).toBe(await sha256Hex(UPSTREAM_BYTES));
    const second = captured.native[1];
    if (second === undefined) {
      throw new Error("missing second native call");
    }
    expect(captured.native).toHaveLength(2);
    expect(captured.upstream).toHaveLength(2);
    expect(envelope.sourceNonce).toBe(second.headers.get("X-Qido-Source-Nonce"));
    expect(envelope.generation).toBe(
      Number(second.headers.get("X-Qido-Generation")),
    );
    expect(envelope.generation).toBe(
      Number(first.headers.get("X-Qido-Generation")) + 1,
    );
  } finally {
    provider.dispose();
  }
});
});

describe("QIDO stale oversized native 422", () => {
it("stale oversized native 422 body rejects Aborted, not AllocationRefused", async () => {
  const captured: CapturedCalls = { native: [], upstream: [] };
  const gate: { entered: boolean; open?: () => void } = { entered: false };
  const gatePromise = new Promise<void>((resolve) => {
    gate.open = resolve;
  });
  const inner = streamingErrorBody(16 * 1024 * 1024 + 1).getReader();
  const outerStream = new ReadableStream<Uint8Array>(
    {
      async pull(controller) {
        gate.entered = true;
        await gatePromise;
        const chunk = await inner.read();
        if (!chunk.done) {
          controller.enqueue(chunk.value);
        } else {
          controller.close();
        }
      },
      cancel() {
        void inner.cancel().catch(() => undefined);
      },
    },
    { highWaterMark: 0 },
  );
  const provider = makeProvider(async (call) => {
    if (captured.native.length === 1) {
      return new Response(outerStream, {
        status: 422,
        headers: {
          "Content-Type": "application/json",
          "Content-Length": DECLARED_LENGTH,
        },
      });
    }
    return nativeOkResponse(call, "study", []);
  }, captured);
  const firstSettled = provider.searchStudies();
  const firstObserved = firstSettled.catch((error: unknown) => error);
  try {
    await vi.waitFor(() => {
      expect(gate.entered).toBe(true);
      expect(captured.native.length).toBe(1);
    });
    const second = await provider.searchStudies();
    expect(second.level).toBe("study");
    expect(second.rows).toEqual([]);
    expect(second.capturedScope).toEqual({});
    expect(captured.native.length).toBe(2);
    expect(captured.upstream.length).toBe(2);
    gate.open?.();
    const firstError = await firstObserved;
    expect(firstError instanceof OcelliError).toBe(true);
    if (firstError instanceof OcelliError) {
      expect(firstError.code).toBe("Aborted");
    }
  } finally {
    gate.open?.();
    provider.dispose();
    await inner.cancel().catch(() => undefined);
    await firstObserved;
    inner.releaseLock();
  }
  expect(captured.native[0]?.headers.get("X-Qido-Level")).toBe("study");
  expect(captured.native[1]?.headers.get("X-Qido-Level")).toBe("study");
});
});

describe("WADO native projection single-fault refusal", () => {
type NativeInstanceUnitDto = Awaited<ReturnType<typeof nativeInstanceEnvelope>>;

const cases: ReadonlyArray<{
  readonly name: string;
  readonly mutate: (dto: NativeInstanceUnitDto) => unknown;
}> = [
  {
    name: "wrong study UID",
    mutate: (dto) => ({ ...dto, identity: { ...dto.identity, studyUid: "2.25.999" } }),
  },
  {
    name: "wrong series UID",
    mutate: (dto) => ({ ...dto, identity: { ...dto.identity, seriesUid: "2.25.888" } }),
  },
  {
    name: "wrong instance UID",
    mutate: (dto) => ({ ...dto, identity: { ...dto.identity, instanceUid: "2.25.777" } }),
  },
  {
    name: "same-length wrong study UID",
    mutate: (dto) => ({ ...dto, identity: { ...dto.identity, studyUid: "2.25.9" } }),
  },
  {
    name: "same-length wrong series UID",
    mutate: (dto) => ({ ...dto, identity: { ...dto.identity, seriesUid: "2.25.8" } }),
  },
  {
    name: "same-length wrong instance UID",
    mutate: (dto) => ({ ...dto, identity: { ...dto.identity, instanceUid: "2.25.7" } }),
  },
  {
    name: "identity extra key",
    mutate: (dto) => ({ ...dto, identity: { ...dto.identity, extraKey: "nope" } }),
  },
  {
    name: "outer extra key",
    mutate: (dto) => ({ ...dto, extraKey: 1 }),
  },
  {
    name: "missing byteLength",
    mutate: (dto) => ({
      identity: dto.identity,
      rawBodySha256: dto.rawBodySha256,
      sourceNonce: dto.sourceNonce,
      generation: dto.generation,
      representation: dto.representation,
    }),
  },
  {
    name: "wrong generation",
    mutate: (dto) => ({ ...dto, generation: dto.generation + 1 }),
  },
  {
    name: "generation as string",
    mutate: (dto) => ({ ...dto, generation: String(dto.generation) }),
  },
  {
    name: "wrong byteLength",
    mutate: (dto) => ({ ...dto, byteLength: dto.byteLength + 1 }),
  },
  {
    name: "wrong nonce",
    mutate: (dto) => ({ ...dto, sourceNonce: flipFirstHexChar(dto.sourceNonce) }),
  },
  {
    name: "wrong representation field",
    mutate: (dto) => ({ ...dto, representation: { ...dto.representation, field: "other" } }),
  },
  {
    name: "wrong boundary",
    mutate: (dto) => ({
      ...dto,
      representation: { ...dto.representation, boundary: dto.representation.boundary + "x" },
    }),
  },
  {
    name: "representation extra key",
    mutate: (dto) => ({ ...dto, representation: { ...dto.representation, extraKey: "nope" } }),
  },
];

function flipFirstHexChar(nonce: string): string {
  const first = nonce.charAt(0);
  return (first === "0" ? "1" : "0") + nonce.slice(1);
}

it.each(cases)("$name: projection fault refused then valid refetch recovers", async ({ mutate }) => {
  const raw = new Uint8Array([1, 2, 3, 4]);
  const boundary = "f245-projection-fault-boundary";
  const captured: CapturedCalls = { native: [], upstream: [] };
  let serveValid = false;
  const provider = makeWadoProvider(raw, boundary, async (call) => {
    const dto = await nativeInstanceEnvelope(call, boundary);
    const payload = serveValid ? dto : mutate(dto);
    return new Response(JSON.stringify(payload), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  }, captured);
  try {
    const identity: { studyUid: string; seriesUid: string; instanceUid: string } = {
      studyUid: STUDY_UID,
      seriesUid: SERIES_UID,
      instanceUid: INSTANCE_UID,
    };
    await expect(provider.fetchSelectedInstance(identity)).rejects.toMatchObject({
      code: "CommitRefused",
    });
    expect(provider.readStagedInfo()).toBeNull();
    expect(captured.native).toHaveLength(1);
    expect(captured.upstream).toHaveLength(1);

    serveValid = true;
    const meta = await provider.fetchSelectedInstance(identity);
    expect(captured.native).toHaveLength(2);
    expect(captured.upstream).toHaveLength(2);

    expect(meta.identity).toEqual({
      studyUid: STUDY_UID,
      seriesUid: SERIES_UID,
      instanceUid: INSTANCE_UID,
    });
    expect(meta.byteLength).toBe(raw.byteLength);
    expect(meta.rawBodySha256).toBe(await sha256Hex(raw));
    expect(Object.isFrozen(meta)).toBe(true);
    expect(Object.isFrozen(meta.identity)).toBe(true);
    expect(Object.isFrozen(meta.representation)).toBe(true);
    if (captured.native[1] === undefined) throw new Error("no second native call");
    const secondCall = captured.native[1];
    const actualNonce = secondCall.headers.get("X-Wado-Source-Nonce");
    if (actualNonce === null) throw new Error("no nonce");
    expect(meta.sourceNonce).toBe(actualNonce);
    expect(meta.generation).toBe(Number(secondCall.headers.get("X-Wado-Generation")));
    expect(meta.representation).toEqual({ field: "wado-rs-instances", boundary });

    const staged = provider.readStagedInfo();
    expect(staged).toEqual(meta);
    const owned = provider.takeStagedInstance(meta);
    if (owned === null) throw new Error("take failed");
    expect(new Uint8Array(owned.body)).toEqual(raw);
    expect(owned.metadata).toEqual(meta);
    expect(provider.takeStagedInstance(meta)).toBeNull();
  } finally {
    provider.dispose();
  }
});
});

describe("WADO staged receipt matching", () => {

type Mutator = (meta: StagedMetadata) => StagedMetadata;

const flipFirstHex = (value: string): string =>
  (value[0] === "0" ? "1" : "0") + value.slice(1);

const CASES: readonly {
  readonly name: string;
  readonly mutate: Mutator;
}[] = [
  { name: "wrong study UID", mutate: (m) => ({ ...m, identity: { ...m.identity, studyUid: "2.25.999" } }) },
  { name: "wrong series UID", mutate: (m) => ({ ...m, identity: { ...m.identity, seriesUid: "2.25.999" } }) },
  { name: "wrong instance UID", mutate: (m) => ({ ...m, identity: { ...m.identity, instanceUid: "2.25.999" } }) },
  { name: "same-length wrong study UID", mutate: (m) => ({ ...m, identity: { ...m.identity, studyUid: "2.25.9" } }) },
  { name: "same-length wrong series UID", mutate: (m) => ({ ...m, identity: { ...m.identity, seriesUid: "2.25.8" } }) },
  { name: "same-length wrong instance UID", mutate: (m) => ({ ...m, identity: { ...m.identity, instanceUid: "2.25.7" } }) },
  { name: "flipped digest digit", mutate: (m) => ({ ...m, rawBodySha256: flipFirstHex(m.rawBodySha256) }) },
  { name: "flipped source nonce digit", mutate: (m) => ({ ...m, sourceNonce: flipFirstHex(m.sourceNonce) }) },
  { name: "incremented generation", mutate: (m) => ({ ...m, generation: m.generation + 1 }) },
  { name: "incremented byteLength", mutate: (m) => ({ ...m, byteLength: m.byteLength + 1 }) },
  {
    name: "different representation boundary",
    mutate: (m) => ({ ...m, representation: { field: m.representation.field, boundary: "other-boundary" } }),
  },
  { name: "extra top-level key", mutate: (m) => ({ ...m, extraKey: 1 }) as StagedMetadata },
  { name: "missing top-level key", mutate: (m) => { const copy = { ...m }; delete (copy as { byteLength?: number }).byteLength; return copy as StagedMetadata; } },
  { name: "extra identity key", mutate: (m) => ({ ...m, identity: { ...m.identity, extraKey: "nope" } }) as StagedMetadata },
  { name: "extra representation key", mutate: (m) => ({ ...m, representation: { ...m.representation, extraKey: "nope" } }) as StagedMetadata },
];


describe("takeStagedInstance receipt matching", () => {
  it.each(CASES)("$name is refused while an equivalent clone is accepted", async ({ mutate }) => {
    const raw = new Uint8Array([10, 20, 30, 40]);
    const boundary = "f245-receipt-boundary";
    const captured: CapturedCalls = { native: [], upstream: [] };
    const provider = makeWadoProvider(raw, boundary, async (call) => {
      const envelope = await nativeInstanceEnvelope(call, boundary);
      return new Response(JSON.stringify(envelope), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    }, captured);
    try {
      const identity: { studyUid: string; seriesUid: string; instanceUid: string } = {
        studyUid: STUDY_UID,
        seriesUid: SERIES_UID,
        instanceUid: INSTANCE_UID,
      };
      const meta = await provider.fetchSelectedInstance(identity);
      expect(Object.isFrozen(meta)).toBe(true);
      const badReceipt: StagedMetadata = mutate(meta);
      expect(provider.takeStagedInstance(badReceipt)).toBeNull();
      expect(provider.readStagedInfo()).toEqual(meta);

      const equivalent: StagedMetadata = {
        ...meta,
        identity: { ...meta.identity },
        representation: { ...meta.representation },
      };
      const owned = provider.takeStagedInstance(equivalent);
      if (owned === null) throw new Error("take failed");
      expect(new Uint8Array(owned.body)).toEqual(raw);
      expect(owned.metadata).toEqual(meta);
      expect(provider.readStagedInfo()).toBeNull();
      expect(provider.takeStagedInstance(equivalent)).toBeNull();
    } finally {
      provider.dispose();
      vi.unstubAllGlobals();
    }
  });
});
});

describe("WADO staged transfer failure", () => {
it("refuses a staged transfer on structuredClone failure and recovers by refetching on the same provider", async () => {
  const raw = new Uint8Array([50, 60, 70, 80]);
  const boundary = "f245-transfer-recovery-boundary";
  const captured: CapturedCalls = { native: [], upstream: [] };
  const provider = makeWadoProvider(raw, boundary, async (call) => {
    const envelope = await nativeInstanceEnvelope(call, boundary);
    return new Response(JSON.stringify(envelope), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  }, captured);
  const identity: { studyUid: string; seriesUid: string; instanceUid: string } = {
    studyUid: STUDY_UID,
    seriesUid: SERIES_UID,
    instanceUid: INSTANCE_UID,
  };
  try {
    const meta = await provider.fetchSelectedInstance(identity);
    {
      const firstNativeCall = captured.native[0];
      if (firstNativeCall === undefined) throw new Error("no native call");
      const firstGeneration = firstNativeCall.headers.get("X-Wado-Generation");
      if (firstGeneration === null) throw new Error("no generation header");
      expect(meta.generation).toBe(Number(firstGeneration));
    }
    const cloneSpy = vi.spyOn(globalThis, "structuredClone").mockImplementation(() => {
      throw new DOMException("controlled failure", "DataCloneError");
    });
    try {
      let thrown: unknown = null;
      try {
        provider.takeStagedInstance(meta);
      } catch (error) {
        thrown = error;
      }
      expect(thrown).toBeInstanceOf(OcelliError);
      if (thrown instanceof OcelliError) {
        expect(thrown.code).toBe("CommitRefused");
      } else {
        throw new Error("not an OcelliError");
      }
      expect(cloneSpy.mock.calls).toHaveLength(1);
      expect(provider.readStagedInfo()).toBeNull();
      expect(provider.takeStagedInstance(meta)).toBeNull();
    } finally {
      cloneSpy.mockRestore();
    }
    const refetched = await provider.fetchSelectedInstance(identity);
    expect(refetched.generation).toBe(meta.generation + 1);
    expect(provider.takeStagedInstance(meta)).toBeNull();
    const stillStaged = provider.readStagedInfo();
    if (stillStaged === null) throw new Error("new stage lost");
    expect(stillStaged.generation).toBe(refetched.generation);
    const owned = provider.takeStagedInstance(refetched);
    if (owned === null) throw new Error("take of new receipt failed");
    expect(owned.body instanceof ArrayBuffer).toBe(true);
    expect(new Uint8Array(owned.body)).toEqual(raw);
    expect(owned.metadata).toEqual(refetched);
    expect(owned.metadata.identity).toEqual({
      studyUid: STUDY_UID,
      seriesUid: SERIES_UID,
      instanceUid: INSTANCE_UID,
    });
    expect(owned.metadata.representation).toEqual({ field: "wado-rs-instances", boundary });
    expect(owned.metadata.byteLength).toBe(raw.byteLength);
    expect(owned.metadata.rawBodySha256).toBe(await sha256Hex(raw));
    expect(provider.readStagedInfo()).toBeNull();
  } finally {
    provider.dispose();
  }
});
});

describe("WADO native 422 bounded error body", () => {
it.each(ERROR_CASES)("$name", async ({ totalBytes, expected }) => {
  const raw = new Uint8Array([1, 2, 3, 4]);
  const boundary = "f245-wado-422-boundary";
  const captured: CapturedCalls = { native: [], upstream: [] };
  const provider = makeWadoProvider(
    raw,
    boundary,
    async () =>
      new Response(streamingErrorBody(totalBytes), {
        status: 422,
        headers: {
          "Content-Type": "application/json",
          "Content-Length": DECLARED_LENGTH,
        },
      }),
    captured,
  );
  try {
    await expectOcelliCode(
      provider.fetchSelectedInstance({
        studyUid: STUDY_UID,
        seriesUid: SERIES_UID,
        instanceUid: INSTANCE_UID,
      }),
      expected,
    );
  } finally {
    provider.dispose();
  }
  expect(captured.native).toHaveLength(1);
  expect(captured.upstream).toHaveLength(1);
  if (captured.native[0] === undefined) throw new Error("no native call");
  const nativeCall = captured.native[0];
  const nativeUrl = new URL(nativeCall.url, NATIVE_BASE);
  expect(nativeUrl.pathname).toBe("/dicweb/instance");
  expect(nativeCall.method).toBe("POST");
  expect(nativeCall.headers.get("X-Wado-Study-Instance-UID")).toBe(STUDY_UID);
  expect(nativeCall.headers.get("X-Wado-Series-Instance-UID")).toBe(SERIES_UID);
  expect(nativeCall.headers.get("X-Wado-SOP-Instance-UID")).toBe(INSTANCE_UID);
  expect(nativeCall.headers.get("Content-Type")).toContain(boundary);
  expect(new Uint8Array(nativeCall.bodyBytes)).toEqual(raw);
  expect(provider.readStagedInfo()).toBeNull();
  expect(provider.takeStagedInstance({
    identity: { studyUid: STUDY_UID, seriesUid: SERIES_UID, instanceUid: INSTANCE_UID },
    rawBodySha256: await sha256Hex(raw),
    sourceNonce: nativeCall.headers.get("X-Wado-Source-Nonce") ?? "",
    generation: Number(nativeCall.headers.get("X-Wado-Generation") ?? "0"),
    byteLength: raw.byteLength,
    representation: { field: "wado-rs-instances", boundary },
  })).toBeNull();
});
});

describe("WADO native 422 refusal and explicit retry", () => {
it.each(
  [
    {
      name: "unreadable native 422 body refuses WADO once then recovers on explicit retry",
      refusal: async (): Promise<Response> => {
        const stream = new ReadableStream<Uint8Array>({
          start(controller) {
            controller.error(new Error("synthetic unreadable stream"));
          },
        });
        return new Response(stream, { status: 422 });
      },
    },
    {
      name: "unknown client-only wire code Network refuses WADO once then recovers on explicit retry",
      refusal: async (): Promise<Response> =>
        new Response(JSON.stringify({ code: "Network" }), {
          status: 422,
          headers: { "Content-Type": "application/json" },
        }),
    },
    {
      name: "extra-key 422 body refuses WADO once then recovers on explicit retry",
      refusal: async (): Promise<Response> =>
        new Response(
          JSON.stringify({ code: "InvalidValue", extra: "synthetic" }),
          {
            status: 422,
            headers: { "Content-Type": "application/json" },
          },
        ),
    },
  ] satisfies ReadonlyArray<{
    readonly name: string;
    readonly refusal: () => Promise<Response>;
  }>,
)("$name", async ({ refusal }: { readonly name: string; readonly refusal: () => Promise<Response> }) => {
  let refusing = true;
  const raw = new Uint8Array([1, 2, 3, 4]);
  const boundary = "f245-wado-recovery-boundary";
  const captured: CapturedCalls = { native: [], upstream: [] };
  const provider = makeWadoProvider(raw, boundary, async (call: NativeCall) => {
    if (refusing) {
      return refusal();
    }
    const envelope = await nativeInstanceEnvelope(call, boundary);
    return new Response(JSON.stringify(envelope), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  }, captured);
  try {
    const identity: InstanceIdentity = {
      studyUid: STUDY_UID,
      seriesUid: SERIES_UID,
      instanceUid: INSTANCE_UID,
    };
    await expectOcelliCode(provider.fetchSelectedInstance(identity), "CommitRefused");
    expect(provider.readStagedInfo()).toBeNull();
    expect(captured.native).toHaveLength(1);
    expect(captured.upstream).toHaveLength(1);
    const first = captured.native[0];
    if (first === undefined) {
      throw new Error("missing first native call");
    }
    refusing = false;
    const meta = await provider.fetchSelectedInstance(identity);
    expect(meta.identity).toEqual({
      studyUid: STUDY_UID,
      seriesUid: SERIES_UID,
      instanceUid: INSTANCE_UID,
    });
    expect(meta.byteLength).toBe(4);
    expect(meta.rawBodySha256).toBe(await sha256Hex(raw));
    expect(meta.representation).toEqual({ field: "wado-rs-instances", boundary });
    expect(captured.native).toHaveLength(2);
    expect(captured.upstream).toHaveLength(2);
    const second = captured.native[1];
    if (second === undefined) {
      throw new Error("missing second native call");
    }
    expect(meta.sourceNonce).toBe(second.headers.get("X-Wado-Source-Nonce"));
    expect(meta.generation).toBe(Number(second.headers.get("X-Wado-Generation")));
    expect(meta.generation).toBe(
      Number(first.headers.get("X-Wado-Generation")) + 1,
    );
    const owned = provider.takeStagedInstance(meta);
    if (owned === null) {
      throw new Error("take failed");
    }
    expect(new Uint8Array(owned.body)).toEqual(raw);
    expect(owned.metadata).toEqual(meta);
    expect(provider.readStagedInfo()).toBeNull();
  } finally {
    provider.dispose();
  }
});
});

describe("WADO stale oversized native 422", () => {
it("stale oversized native 422 body rejects Aborted, not AllocationRefused, without clobbering a newer WADO stage", async () => {
  const captured: CapturedCalls = { native: [], upstream: [] };
  const gate: { entered: boolean; open?: () => void } = { entered: false };
  const gatePromise = new Promise<void>((resolve) => {
    gate.open = resolve;
  });
  const inner = streamingErrorBody(16 * 1024 * 1024 + 1).getReader();
  const outerStream = new ReadableStream<Uint8Array>(
    {
      async pull(controller) {
        gate.entered = true;
        await gatePromise;
        const chunk = await inner.read();
        if (!chunk.done) {
          controller.enqueue(chunk.value);
        } else {
          controller.close();
        }
      },
      cancel() {
        void inner.cancel().catch(() => undefined);
      },
    },
    { highWaterMark: 0 },
  );
  const boundary = "f245-stale-boundary";
  const raw = new Uint8Array([1, 2, 3, 4, 5]);
  const provider = makeWadoProvider(raw, boundary, async (call) => {
    if (captured.native.length === 1) {
      return new Response(outerStream, {
        status: 422,
        headers: {
          "Content-Type": "application/json",
          "Content-Length": DECLARED_LENGTH,
        },
      });
    }
    const envelope = await nativeInstanceEnvelope(call, boundary);
    return new Response(JSON.stringify(envelope), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  }, captured);
  const identity: { studyUid: string; seriesUid: string; instanceUid: string } = {
    studyUid: STUDY_UID,
    seriesUid: SERIES_UID,
    instanceUid: INSTANCE_UID,
  };
  const firstSettled = provider.fetchSelectedInstance(identity);
  const firstObserved = firstSettled.catch((error: unknown) => error);
  let second: StagedMetadata;
  try {
    await vi.waitFor(() => {
      expect(gate.entered).toBe(true);
      expect(captured.native.length).toBe(1);
    });
    second = await provider.fetchSelectedInstance(identity);
    expect(captured.native.length).toBe(2);
    if (captured.native[1] === undefined) throw new Error("no second native call");
    const secondCall = captured.native[1];
    const staged = provider.readStagedInfo();
    if (staged === null) throw new Error("staged lost while old body blocked");
    expect(staged).toBe(second);
    expect(staged.identity).toEqual({
      studyUid: STUDY_UID,
      seriesUid: SERIES_UID,
      instanceUid: INSTANCE_UID,
    });
    expect(staged.sourceNonce).toBe(secondCall.headers.get("X-Wado-Source-Nonce") ?? "");
    expect(staged.generation).toBe(Number(secondCall.headers.get("X-Wado-Generation") ?? "0"));
    expect(staged.rawBodySha256).toBe(await sha256Hex(secondCall.bodyBytes));
    expect(staged.byteLength).toBe(secondCall.bodyBytes.byteLength);
    gate.open?.();
    const firstError = await firstObserved;
    expect(firstError instanceof OcelliError).toBe(true);
    if (firstError instanceof OcelliError) {
      expect(firstError.code).toBe("Aborted");
    }
    const afterError = provider.readStagedInfo();
    expect(afterError).toEqual(second);
    const owned = provider.takeStagedInstance(second);
    if (owned === null) throw new Error("take failed");
    expect(owned.body instanceof ArrayBuffer).toBe(true);
    expect(new Uint8Array(owned.body)).toEqual(new Uint8Array(secondCall.bodyBytes));
    expect(owned.metadata).toEqual(second);
  } finally {
    gate.open?.();
    provider.dispose();
    await inner.cancel().catch(() => undefined);
    await firstObserved;
    inner.releaseLock();
  }
  expect(captured.native[0]?.headers.get("X-Wado-SOP-Instance-UID")).toBe(INSTANCE_UID);
  expect(captured.native[1]?.headers.get("X-Wado-SOP-Instance-UID")).toBe(INSTANCE_UID);
});
});

describe("DicomwebProvider pre-aborted QIDO signal", () => {
it("refuses a pre-aborted searchStudies signal before any state change", async () => {
  const raw = new Uint8Array([1, 2, 3, 4]);
  const boundary = "f245-pre-abort-boundary";
  const captured: CapturedCalls = { native: [], upstream: [] };
  const provider = makeWadoProvider(raw, boundary, async (call) => {
    const envelope = await nativeInstanceEnvelope(call, boundary);
    return new Response(JSON.stringify(envelope), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  }, captured);
  try {
    const meta = await provider.fetchSelectedInstance({
      studyUid: STUDY_UID,
      seriesUid: SERIES_UID,
      instanceUid: INSTANCE_UID,
    });
    expect(provider.readStagedInfo()).toEqual(meta);
    await expectOcelliCode(
      provider.searchStudies({ signal: AbortSignal.abort() }),
      "Aborted",
    );
    await expect(provider.searchStudies({ signal: AbortSignal.abort() })).rejects.toSatisfy(
      (error: unknown) =>
        error instanceof OcelliError &&
        error.code === "Aborted" &&
        error.message === "operation was aborted before admission",
    );
    expect(captured.upstream).toHaveLength(1);
    expect(captured.native).toHaveLength(1);
    expect(provider.readStagedInfo()).toEqual(meta);
    const owned = provider.takeStagedInstance(meta);
    expect(owned).not.toBeNull();
    if (owned === null) {
      expect.fail("unreachable guard");
    }
    expect(new Uint8Array(owned.body)).toEqual(raw);
    expect(owned.metadata).toEqual(meta);
  } finally {
    provider.dispose();
  }
});
});
