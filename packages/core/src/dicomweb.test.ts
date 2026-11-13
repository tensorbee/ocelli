import { describe, expect, it } from "vitest";

import {
  DicomwebClient,
  DicomwebError,
  OcelliError,
  type DicomwebClientOptions,
  type DicomwebResponseKind,
  type DicomwebResponseSink,
  type WasmMemory,
} from "./index.js";

interface ResponseCommit {
  readonly bytes: Uint8Array;
  readonly kind: DicomwebResponseKind;
}

function responseSink(): {
  readonly wasm: WasmMemory;
  readonly sink: DicomwebResponseSink;
  readonly commits: ResponseCommit[];
} {
  const memory = new WebAssembly.Memory({ initial: 1, maximum: 8 });
  const commits: ResponseCommit[] = [];
  return {
    wasm: { memory },
    sink: {
      alloc(len: number): number {
        const ptr = memory.buffer.byteLength;
        if (len > 65536) {
          throw new Error("synthetic response is larger than one page");
        }
        memory.grow(1);
        return ptr;
      },
      commit_dicomweb_response(
        ptr: number,
        len: number,
        kind: DicomwebResponseKind,
      ): void {
        commits.push({
          bytes: new Uint8Array(memory.buffer.slice(ptr, ptr + len)),
          kind,
        });
      },
    },
    commits,
  };
}

function oneCommit(commits: readonly ResponseCommit[]): ResponseCommit {
  const commit = commits[0];
  if (commit === undefined) {
    throw new Error("expected one synthetic response commit");
  }
  return commit;
}

function makeClient(
  fetcher: typeof fetch,
  fixture = responseSink(),
): { readonly client: DicomwebClient; readonly fixture: ReturnType<typeof responseSink> } {
  return {
    client: new DicomwebClient({
      studiesServiceUrl: "https://example.test/dicomweb/",
      wadoUriUrl: "https://example.test/wado",
      fetch: fetcher,
      wasm: fixture.wasm,
      sink: fixture.sink,
    }),
    fixture,
  };
}

function ok(body: BodyInit, contentType: string): Response {
  return new Response(body, {
    status: 200,
    headers: { "Content-Type": contentType },
  });
}

/** One valid response per request kind, so every kind reaches its commit. */
function okForAnyKind(input: RequestInfo | URL): Response {
  const path = new URL(String(input)).pathname;
  if (path.endsWith("/frames/1")) {
    return ok(
      "frames",
      'multipart/related; type="application/octet-stream"; boundary=frame-edge',
    );
  }
  if (path.endsWith("/2.25.3")) {
    return ok(
      "instance",
      'multipart/related; type="application/dicom"; boundary="instance-edge"',
    );
  }
  if (path === "/wado") {
    return ok("part10", "application/dicom");
  }
  return ok("[]", "application/dicom+json");
}

/**
 * Options with one key the public type rejects. The own property is still
 * there at run time, which is what the constructor reads.
 */
function optionsWith(
  fetcher: typeof fetch,
  key: string,
  value: unknown,
): DicomwebClientOptions {
  const fixture = responseSink();
  return {
    studiesServiceUrl: "https://example.test/dicomweb/",
    wadoUriUrl: "https://example.test/wado",
    fetch: fetcher,
    wasm: fixture.wasm,
    sink: fixture.sink,
    [key]: value,
  } as DicomwebClientOptions;
}

describe("DicomwebClient, DICOM PS3.18 2026c", () => {
  it("constructs QIDO levels with unique filters, UID lists, and repeated includefield", async () => {
    const urls: string[] = [];
    const fetcher: typeof fetch = async (input) => {
      urls.push(String(input));
      return ok("[]", "application/dicom+json");
    };
    const { client, fixture } = makeClient(fetcher);
    const options = {
      filters: [
        ["00080060", "CT"],
        ["0020000D", "2.25.1,2.25.2"],
      ] as const,
      includeFields: ["00080018", "0020000D"],
      limit: 5,
      offset: 10,
      fuzzyMatching: true,
    };

    await client.searchStudies(options);
    await client.searchSeries("2.25.1", options);
    await client.searchInstances("2.25.1", "2.25.2", options);

    expect(urls.map((value) => new URL(value).pathname)).toEqual([
      "/dicomweb/studies",
      "/dicomweb/studies/2.25.1/series",
      "/dicomweb/studies/2.25.1/series/2.25.2/instances",
    ]);
    for (const value of urls) {
      const query = new URL(value).searchParams;
      expect(query.getAll("00080060")).toEqual(["CT"]);
      expect(query.get("0020000D")).toBe("2.25.1,2.25.2");
      expect(query.getAll("includefield")).toEqual(["00080018", "0020000D"]);
      expect(query.get("limit")).toBe("5");
      expect(query.get("offset")).toBe("10");
      expect(query.get("fuzzymatching")).toBe("true");
    }
    expect(fixture.commits).toHaveLength(3);
    expect(fixture.commits.map((commit) => commit.kind)).toEqual([
      { type: "qido-json" },
      { type: "qido-json" },
      { type: "qido-json" },
    ]);
  });

  it("refuses invalid pagination and repeated matching attributes before fetch", async () => {
    let calls = 0;
    const { client } = makeClient(async () => {
      calls += 1;
      return ok("[]", "application/dicom+json");
    });
    for (const limit of [-1, 1.5, Number.NaN, Number.POSITIVE_INFINITY]) {
      await expect(client.searchStudies({ limit })).rejects.toMatchObject({
        code: "InvalidRequest",
      });
    }
    for (const offset of [-1, 1.5, Number.NaN, Number.POSITIVE_INFINITY]) {
      await expect(client.searchStudies({ offset })).rejects.toMatchObject({
        code: "InvalidRequest",
      });
    }
    await expect(
      client.searchStudies({
        filters: [
          ["00080060", "CT"],
          ["00080060", "MR"],
        ],
      }),
    ).rejects.toMatchObject({ code: "InvalidRequest" });
    for (const reserved of [
      "limit",
      "offset",
      "fuzzymatching",
      "includefield",
      "emptyvaluematching",
      "multiplevaluematching",
    ]) {
      await expect(
        client.searchStudies({ filters: [[reserved, "1"]] }),
      ).rejects.toMatchObject({ code: "InvalidRequest" });
    }
    expect(calls).toBe(0);
  });

  it("refuses invalid DICOM UIDs before fetch and accepts the 64-character boundary", async () => {
    let calls = 0;
    const fetcher: typeof fetch = async () => {
      calls += 1;
      return ok("[]", "application/dicom+json");
    };
    const { client } = makeClient(fetcher);
    const valid = "2.25.1";
    const invalid = [
      "",
      ".",
      "..",
      ".1",
      "1.",
      "1..2",
      "1.two.3",
      "1.-2.3",
      "01.2.3",
      "1.02.3",
      `1.${"2".repeat(63)}`,
    ];

    for (const uid of invalid) {
      const operations = [
        () => client.searchSeries(uid),
        () => client.searchInstances(uid, valid),
        () => client.searchInstances(valid, uid),
        () => client.retrieveInstance(uid, valid, valid),
        () => client.retrieveInstance(valid, uid, valid),
        () => client.retrieveInstance(valid, valid, uid),
        () =>
          client.retrieveFrames(
            uid,
            valid,
            valid,
            [1],
            "application/octet-stream",
          ),
        () =>
          client.retrieveFrames(
            valid,
            uid,
            valid,
            [1],
            "application/octet-stream",
          ),
        () =>
          client.retrieveFrames(
            valid,
            valid,
            uid,
            [1],
            "application/octet-stream",
          ),
        () => client.retrieveWadoUri(uid, valid, valid),
        () => client.retrieveWadoUri(valid, uid, valid),
        () => client.retrieveWadoUri(valid, valid, uid),
      ];
      for (const operation of operations) {
        await expect(operation()).rejects.toMatchObject({ code: "InvalidRequest" });
      }
    }
    expect(calls).toBe(0);

    const maximum = `1.${"2".repeat(62)}`;
    expect(maximum).toHaveLength(64);
    await client.searchSeries(maximum);
    expect(calls).toBe(1);
  });

  it("requests bounded WADO-RS instance and frame resources", async () => {
    const calls: Array<{ readonly url: string; readonly init?: RequestInit }> = [];
    const fetcher: typeof fetch = async (input, init) => {
      calls.push({ url: String(input), ...(init === undefined ? {} : { init }) });
      if (calls.length === 1) {
        return ok(
          "instance",
          'multipart/related; type="application/dicom"; boundary="instance-edge"',
        );
      }
      return ok(
        "frames",
        'multipart/related; type="application/octet-stream"; boundary=frame-edge',
      );
    };
    const { client, fixture } = makeClient(fetcher);

    await client.retrieveInstance("2.25.1", "2.25.2", "2.25.3");
    await client.retrieveFrames(
      "2.25.1",
      "2.25.2",
      "2.25.3",
      [1, 3],
      "application/octet-stream",
    );

    expect(new URL(calls[0]?.url ?? "").pathname).toBe(
      "/dicomweb/studies/2.25.1/series/2.25.2/instances/2.25.3",
    );
    expect(new Headers(calls[0]?.init?.headers).get("Accept")).toBe(
      'multipart/related; type="application/dicom"; transfer-syntax=*',
    );
    expect(new URL(calls[1]?.url ?? "").pathname).toBe(
      "/dicomweb/studies/2.25.1/series/2.25.2/instances/2.25.3/frames/1,3",
    );
    expect(new Headers(calls[1]?.init?.headers).get("Accept")).toBe(
      'multipart/related; type="application/octet-stream"',
    );
    expect(fixture.commits.map((commit) => commit.kind)).toEqual([
      { type: "wado-rs-instances", boundary: "instance-edge" },
      {
        type: "wado-rs-frames",
        boundary: "frame-edge",
        mediaType: "application/octet-stream",
      },
    ]);
  });

  it("supplies every mandatory WADO-URI parameter and requests DICOM", async () => {
    let request: { readonly url: string; readonly init?: RequestInit } | undefined;
    const fetcher: typeof fetch = async (input, init) => {
      request = { url: String(input), ...(init === undefined ? {} : { init }) };
      return ok("part10", "application/dicom");
    };
    const { client, fixture } = makeClient(fetcher);

    await client.retrieveWadoUri("2.25.1", "2.25.2", "2.25.3");

    const url = new URL(request?.url ?? "");
    expect(url.searchParams.get("requestType")).toBe("WADO");
    expect(url.searchParams.get("studyUID")).toBe("2.25.1");
    expect(url.searchParams.get("seriesUID")).toBe("2.25.2");
    expect(url.searchParams.get("objectUID")).toBe("2.25.3");
    expect(url.searchParams.get("contentType")).toBe("application/dicom");
    expect(new Headers(request?.init?.headers).get("Accept")).toBe(
      "application/dicom",
    );
    expect(oneCommit(fixture.commits).kind).toEqual({ type: "wado-uri-part10" });
  });

  it("lets the injected fetch own authentication and forwards cancellation", async () => {
    const controller = new AbortController();
    let seenSignal: AbortSignal | null | undefined;
    let injectedAuthorization: string | null = null;
    const authenticated: typeof fetch = async (input, init) => {
      const headers = new Headers(init?.headers);
      headers.set("Authorization", "Bearer synthetic-token");
      injectedAuthorization = headers.get("Authorization");
      seenSignal = init?.signal;
      return ok("[]", "application/dicom+json");
    };
    const { client } = makeClient(authenticated);

    await client.searchStudies({ signal: controller.signal });

    expect(injectedAuthorization).toBe("Bearer synthetic-token");
    expect(seenSignal).toBe(controller.signal);
  });

  it("keeps HTTP, content type, abort, and network failures distinct and safe", async () => {
    const marker = "SYNTHETIC_SECRET_MARKER";
    const statusBody = new Response(marker, { status: 503 });
    Object.defineProperty(statusBody, "arrayBuffer", {
      value: () => {
        throw new Error("error response body was read");
      },
    });
    const cases: Array<{
      readonly fetcher: typeof fetch;
      readonly code: string;
      readonly status?: number;
    }> = [
      { fetcher: async () => statusBody, code: "HttpStatus", status: 503 },
      {
        fetcher: async () => ok(marker, "text/plain"),
        code: "ContentType",
      },
      {
        fetcher: async () => {
          throw new DOMException(marker, "AbortError");
        },
        code: "Aborted",
      },
      {
        fetcher: async () => {
          throw new Error(marker);
        },
        code: "Network",
      },
    ];

    for (const testCase of cases) {
      const { client, fixture } = makeClient(testCase.fetcher);
      const promise = client.searchStudies();
      await expect(promise).rejects.toMatchObject({
        code: testCase.code,
        ...(testCase.status === undefined ? {} : { status: testCase.status }),
      });
      const error = await promise.catch((caught: unknown) => caught);
      expect(error).toBeInstanceOf(DicomwebError);
      expect(String(error)).not.toContain(marker);
      expect(fixture.commits).toHaveLength(0);
    }
  });

  it("does not retry a rejected fetch", async () => {
    let calls = 0;
    const fetcher: typeof fetch = async () => {
      calls += 1;
      throw new Error("synthetic network refusal");
    };
    const { client } = makeClient(fetcher);

    await expect(client.searchStudies()).rejects.toMatchObject({ code: "Network" });
    expect(calls).toBe(1);
  });

  it("sanitizes configured URL and response-body transport failures", async () => {
    const marker = "SYNTHETIC_SECRET_MARKER";
    // `commits` is the fixture's observation list. It is not a client option.
    const fixture = responseSink();
    expect(() =>
      new DicomwebClient({
        studiesServiceUrl: marker,
        wadoUriUrl: "https://example.test/wado",
        fetch: async () => ok("[]", "application/dicom+json"),
        wasm: fixture.wasm,
        sink: fixture.sink,
      }),
    ).toThrowError(DicomwebError);

    for (const failure of [
      new DOMException(marker, "AbortError"),
      new Error(marker),
    ]) {
      const response = ok("unused", "application/dicom+json");
      Object.defineProperty(response, "arrayBuffer", {
        value: async () => Promise.reject(failure),
      });
      const { client, fixture } = makeClient(async () => response);
      const error = await client.searchStudies().catch((caught: unknown) => caught);
      expect(error).toBeInstanceOf(DicomwebError);
      expect(error).toMatchObject({
        code: failure instanceof DOMException ? "Aborted" : "Network",
      });
      expect(String(error)).not.toContain(marker);
      expect(fixture.commits).toHaveLength(0);
    }
  });

  it("validates boundary grammar and quoted parameterized frame types before commit", async () => {
    const mediaType =
      "image/jls; transfer-syntax=1.2.840.10008.1.2.4.80; synthetic=one";
    const valid = makeClient(async () =>
      ok(
        "frame",
        'multipart/related; type="image/jls; synthetic=one; transfer-syntax=1.2.840.10008.1.2.4.80"; boundary=frame-edge',
      ),
    );
    await valid.client.retrieveFrames("2.25.1", "2.25.2", "2.25.3", [1], mediaType);
    expect(oneCommit(valid.fixture.commits).kind).toEqual({
      type: "wado-rs-frames",
      boundary: "frame-edge",
      mediaType,
    });

    const invalid = makeClient(async () =>
      ok(
        "frame",
        `multipart/related; type="${mediaType}"; boundary="bad boundary "`,
      ),
    );
    await expect(
      invalid.client.retrieveFrames("2.25.1", "2.25.2", "2.25.3", [1], mediaType),
    ).rejects.toMatchObject({ code: "ContentType" });
    expect(invalid.fixture.commits).toHaveLength(0);
  });

  it("refuses empty, descending, and repeated frame lists before fetch", async () => {
    let calls = 0;
    const fetcher: typeof fetch = async () => {
      calls += 1;
      return ok("unused", "application/octet-stream");
    };
    const { client } = makeClient(fetcher);

    for (const frames of [[], [2, 1], [1, 1]]) {
      await expect(
        client.retrieveFrames(
          "2.25.1",
          "2.25.2",
          "2.25.3",
          frames,
          "application/octet-stream",
        ),
      ).rejects.toMatchObject({ code: "InvalidRequest" });
    }
    expect(calls).toBe(0);
  });

  it("sends the cache mode on every request", async () => {
    const inits: Array<RequestInit | undefined> = [];
    const fetcher: typeof fetch = async (input, init) => {
      inits.push(init);
      return okForAnyKind(input);
    };
    const everyKind = async (client: DicomwebClient): Promise<void> => {
      await client.searchStudies();
      await client.searchSeries("2.25.1");
      await client.searchInstances("2.25.1", "2.25.2");
      await client.retrieveInstance("2.25.1", "2.25.2", "2.25.3");
      await client.retrieveFrames(
        "2.25.1",
        "2.25.2",
        "2.25.3",
        [1],
        "application/octet-stream",
      );
      await client.retrieveWadoUri("2.25.1", "2.25.2", "2.25.3");
    };

    // No cache option. Every request asks the server before a stored
    // response is used, which is the "no-cache" mode and not the Fetch
    // default that RFC 9111's heuristic freshness can serve stale.
    const { client } = makeClient(fetcher);
    await everyKind(client);
    expect(inits).toHaveLength(6);
    for (const init of inits) {
      expect(init?.cache, "without the option").toBe("no-cache");
    }

    // Each accepted value is sent unchanged, on every kind.
    for (const cache of ["no-cache", "no-store", "reload", "default"] as const) {
      inits.length = 0;
      const fixture = responseSink();
      const explicit = new DicomwebClient({
        studiesServiceUrl: "https://example.test/dicomweb/",
        wadoUriUrl: "https://example.test/wado",
        fetch: fetcher,
        wasm: fixture.wasm,
        sink: fixture.sink,
        cache,
      });
      await everyKind(explicit);
      for (const init of inits) {
        expect(init?.cache, cache).toBe(cache);
      }
    }
  });

  it("refuses a cache mode that can serve a stale response", () => {
    let calls = 0;
    const fetcher: typeof fetch = async () => {
      calls += 1;
      return ok("[]", "application/dicom+json");
    };
    for (const cache of ["force-cache", "only-if-cached", "nocache"]) {
      let caught: unknown;
      try {
        new DicomwebClient(optionsWith(fetcher, "cache", cache));
      } catch (error: unknown) {
        caught = error;
      }
      expect(caught, cache).toBeInstanceOf(DicomwebError);
      if (!(caught instanceof DicomwebError)) {
        throw new Error(`expected DicomwebError for ${cache}`);
      }
      expect(caught.code, cache).toBe("InvalidRequest");
    }

    // A misspelled key is F-230's own-key refusal, which names the key, and
    // not a transport class.
    let typo: unknown;
    try {
      new DicomwebClient(optionsWith(fetcher, "cahce", "no-store"));
    } catch (error: unknown) {
      typo = error;
    }
    expect(typo).toBeInstanceOf(OcelliError);
    if (!(typo instanceof OcelliError)) {
      throw new Error("expected OcelliError for the misspelled key");
    }
    expect(typo.code).toBe("UnknownOption");
    expect(typo.option).toBe("cahce");
    expect(calls).toBe(0);
  });
});

describe("F245 exact cap streamed delivery", () => {
  it("F245 exact cap accepts streamed bytes without arrayBuffer", async () => {
    let received: Uint8Array | undefined;
    let kindSeen: DicomwebResponseKind | undefined;
    let receiveCount = 0;
    const client = new DicomwebClient({
      studiesServiceUrl: "https://example.invalid/studies",
      wadoUriUrl: "https://example.invalid/wado",
      fetch: async (): Promise<Response> => {
        const stream = new ReadableStream<Uint8Array>({
          start(controller) {
            controller.enqueue(new Uint8Array([0x01, 0x02]));
            controller.enqueue(new Uint8Array([0x03, 0x04]));
            controller.close();
          },
        });
        const response = new Response(stream, {
          status: 200,
          headers: { "Content-Type": "application/dicom+json" },
        });
        Object.defineProperty(response, "arrayBuffer", {
          value: () => Promise.reject(new Error("arrayBuffer must not be called")),
        });
        return response;
      },
      receive: async (raw: Uint8Array, kind: DicomwebResponseKind): Promise<void> => {
        receiveCount += 1;
        received = raw;
        kindSeen = kind;
      },
      maxResponseBytes: 4,
    });
    await client.searchStudies();
    expect(receiveCount).toBe(1);
    expect(received).toEqual(new Uint8Array([0x01, 0x02, 0x03, 0x04]));
    expect(kindSeen).toEqual({ type: "qido-json" });
  });

  it("F245 excess streamed bytes cancels before receive", async () => {
    let cancelled = false;
    let receivedCount = 0;
    const client = new DicomwebClient({
      studiesServiceUrl: "https://example.invalid/studies",
      wadoUriUrl: "https://example.invalid/wado",
      fetch: async (): Promise<Response> => {
        let served = 0;
        const stream = new ReadableStream<Uint8Array>({
          pull(controller) {
            if (served < 4) {
              controller.enqueue(new Uint8Array([0xaa, 0xbb]));
              served += 2;
            }
          },
          cancel() {
            cancelled = true;
          },
        });
        const response = new Response(stream, {
          status: 200,
          headers: { "Content-Type": "application/dicom+json", "Content-Length": "2" },
        });
        Object.defineProperty(response, "arrayBuffer", {
          value: () => Promise.reject(new Error("arrayBuffer must not be called")),
        });
        return response;
      },
      receive: async (): Promise<void> => {
        receivedCount += 1;
      },
      maxResponseBytes: 3,
    });
    let caught: unknown;
    try {
      await client.searchStudies();
    } catch (error) {
      caught = error;
    }
    expect(caught).toBeInstanceOf(OcelliError);
    if (caught instanceof OcelliError) {
      expect(caught.code).toBe("AllocationRefused");
    }
    expect(receivedCount).toBe(0);
    expect(cancelled).toBe(true);
  });
});

describe("F245 constructor option validation", () => {
  const baseOptions = () => ({
    studiesServiceUrl: "https://example.invalid/studies",
    wadoUriUrl: "https://example.invalid/wado",
    maxResponseBytes: 4,
    fetch: async (): Promise<Response> => {
      throw new Error("fetch must not run");
    },
    receive: async (): Promise<void> => {},
  });

  function catchCode(run: () => unknown): string {
    let caught: unknown;
    try {
      run();
    } catch (error) {
      caught = error;
    }
    if (!(caught instanceof OcelliError)) {
      throw new Error("Expected OcelliError to be thrown");
    }
    expect(caught.code).toBeTypeOf("string");
    return caught.code;
  }

  it("F245 invalid numeric caps are refused synchronously", () => {
    const caps = [0, -1, 1.5, NaN, Infinity, Number.MAX_SAFE_INTEGER + 1];
    for (const cap of caps) {
      const options = baseOptions();
      Reflect.set(options, "maxResponseBytes", cap);
      const code = catchCode(() => new DicomwebClient(options));
      expect(code).toBe("InvalidOptions");
    }
  });

  it("F245 own cap without receiver is refused", () => {
    const options = baseOptions();
    Reflect.set(options, "maxResponseBytes", 4);
    Reflect.deleteProperty(options, "receive");
    const code = catchCode(() => new DicomwebClient(options));
    expect(code).toBe("InvalidOptions");
  });

  it("F245 own receiver without cap is refused", () => {
    const options = baseOptions();
    Reflect.deleteProperty(options, "maxResponseBytes");
    Reflect.set(options, "receive", async (): Promise<void> => {});
    const code = catchCode(() => new DicomwebClient(options));
    expect(code).toBe("InvalidOptions");
  });

  it("F245 own undefined on either key is refused", () => {
    const capUndefined = baseOptions();
    Reflect.set(capUndefined, "maxResponseBytes", undefined);
    expect(catchCode(() => new DicomwebClient(capUndefined))).toBe("InvalidOptions");

    const receiveUndefined = baseOptions();
    Reflect.set(receiveUndefined, "receive", undefined);
    expect(catchCode(() => new DicomwebClient(receiveUndefined))).toBe("InvalidOptions");
  });

  it("F245 mixed own wasm or sink keys are refused", () => {
    const mixedSink = baseOptions();
    Reflect.set(mixedSink, "sink", undefined);
    expect(catchCode(() => new DicomwebClient(mixedSink))).toBe("InvalidOptions");

    const mixedWasm = baseOptions();
    Reflect.set(mixedWasm, "wasm", undefined);
    expect(catchCode(() => new DicomwebClient(mixedWasm))).toBe("InvalidOptions");
  });

  it("F245 unknown own option key is refused", () => {
    const options = baseOptions();
    Reflect.set(options, "bogusOption", 1);
    const code = catchCode(() => new DicomwebClient(options));
    expect(code).toBe("UnknownOption");
  });
  it("F245 cap and callback are captured at construction", async () => {
    let countA = 0;
    let countB = 0;
    let bytesA: Uint8Array | undefined;
    const options = {
      studiesServiceUrl: "https://example.invalid/studies",
      wadoUriUrl: "https://example.invalid/wado",
      fetch: async (): Promise<Response> => {
        const stream = new ReadableStream<Uint8Array>({
          start(controller) {
            controller.enqueue(new Uint8Array([0x01, 0x02, 0x03, 0x04]));
            controller.close();
          },
        });
        return new Response(stream, {
          status: 200,
          headers: { "Content-Type": "application/dicom+json" },
        });
      },
      receive: async (raw: Uint8Array): Promise<void> => {
        countA += 1;
        bytesA = raw;
      },
      maxResponseBytes: 4,
    };
    const client = new DicomwebClient(options);
    options.maxResponseBytes = 1;
    options.receive = async (): Promise<void> => {
      countB += 1;
    };
    await client.searchStudies();
    expect(countA).toBe(1);
    expect(countB).toBe(0);
    expect(bytesA).toEqual(new Uint8Array([0x01, 0x02, 0x03, 0x04]));
  });

  it("F245 legacy options deliver to sink with inherited receive ignored", async () => {
    const fixture = responseSink();
    let inheritedCount = 0;
    const options = {
      studiesServiceUrl: "https://example.invalid/studies",
      wadoUriUrl: "https://example.invalid/wado",
      fetch: async (): Promise<Response> =>
        new Response("[]", {
          status: 200,
          headers: { "Content-Type": "application/dicom+json" },
        }),
      wasm: fixture.wasm,
      sink: fixture.sink,
    };
    Object.setPrototypeOf(options, {
      receive: async (): Promise<void> => {
        inheritedCount += 1;
      },
    });
    const client = new DicomwebClient(options);
    await client.searchStudies();
    expect(fixture.commits).toHaveLength(1);
    const commit = oneCommit(fixture.commits);
    expect(commit.bytes).toEqual(new Uint8Array([0x5b, 0x5d]));
    expect(commit.kind).toEqual({ type: "qido-json" });
    expect(inheritedCount).toBe(0);
  });
  it.each([
    "studiesServiceUrl",
    "wadoUriUrl",
  ])("rejects constructor when %s is invalid", (field) => {
    const options = baseOptions();
    Reflect.set(options, field, "synthetic-URL-marker");

    let caught: unknown;
    try {
      new DicomwebClient(options);
    } catch (error) {
      caught = error;
    }

    expect(caught).toBeInstanceOf(OcelliError);
    expect(caught instanceof OcelliError).toBe(true);
    if (caught instanceof OcelliError) {
      expect(caught.code).toBe("InvalidOptions");
      expect(caught.message).not.toContain("synthetic-URL-marker");
    }
  });
});

it('preserves the exact rejected OcelliError object identity from the awaited receive callback and rejects searchStudies only after the receiver gate is released', async () => {
  const refusal = new OcelliError('CommitRefused', 'synthetic coded refusal');
  type Resolver = { resolver?: () => void };
  const readyHolder: Resolver = {};
  const ready = new Promise<void>((resolve) => {
    readyHolder.resolver = resolve;
  });
  const gateHolder: Resolver = {};
  const gate = new Promise<void>((resolve) => {
    gateHolder.resolver = resolve;
  });
  if (readyHolder.resolver === undefined || gateHolder.resolver === undefined) {
    throw new Error('synthetic latch resolvers must exist before use');
  }
  const stableReadyResolver = readyHolder.resolver;
  const client = new DicomwebClient({
    studiesServiceUrl: 'https://example.invalid/studies',
    wadoUriUrl: 'https://example.invalid/wado',
    fetch: async () => new Response('[]', { headers: { 'Content-Type': 'application/dicom+json' } }),
    receive: async (): Promise<void> => {
      stableReadyResolver();
      await gate;
      throw refusal;
    },
    maxResponseBytes: 4,
  });
  type Outcome = { kind: 'resolved' } | { kind: 'rejected'; error: unknown };
  let outcome: Outcome | undefined;
  let outcomeError: unknown;
  let settled = false;
  const searchPromise = client.searchStudies({});
  searchPromise.then(
    () => {
      outcome = { kind: 'resolved' };
      settled = true;
    },
    (error: unknown) => {
      outcome = { kind: 'rejected', error };
      outcomeError = error;
      settled = true;
    }
  );
  try {
    if (readyHolder.resolver === undefined) {
      throw new Error('ready resolver missing');
    }
    await ready;
    await new Promise<void>((resolve) => {
      setTimeout(resolve, 0);
    });
    expect(outcome).toBeUndefined();
    expect(settled).toBe(false);
  } finally {
    gateHolder.resolver();
  }
  await searchPromise.then(
    () => {
      outcome = { kind: 'resolved' };
    },
    (error: unknown) => {
      outcome = { kind: 'rejected', error };
    }
  );
  if (outcome === undefined || outcome.kind !== 'rejected') {
    expect.fail('searchStudies should reject with the receive callback error');
  }
  expect(outcomeError).toBe(refusal);
  expect(outcome.error).toBe(refusal);
});

  describe("abort handling", () => {
    it("aborts a pending stream read and cancels the underlying source", async () => {
      const controller = new AbortController();
      const ready: { resolve: (() => void) | undefined } = {
        resolve: undefined,
      };
      let cancelCount = 0;
      let receiveCalls = 0;
      const stream = new ReadableStream<Uint8Array>(
        {
          pull() {
            if (ready.resolve !== undefined) {
              ready.resolve();
              ready.resolve = undefined;
            }
          },
          cancel() {
            cancelCount += 1;
          },
        },
        { highWaterMark: 0 },
        );
      const pullReady = new Promise<void>((resolve) => {
        ready.resolve = resolve;
      });
      const fetchFn = async (): Promise<Response> =>
        new Response(stream, {
          status: 200,
          headers: { "content-type": "application/dicom+json" },
        });
      const client = new DicomwebClient({
        studiesServiceUrl: "https://example.invalid/dicomweb",
        wadoUriUrl: "https://example.invalid/wado",
        fetch: fetchFn,
        maxResponseBytes: 1024,
        receive: async () => {
          receiveCalls += 1;
        },
      });
      try {
        const search = client
          .searchStudies({ signal: controller.signal })
          .catch((error: unknown) => error);
        await pullReady;
        controller.abort();
        const error = await search;
        expect(error).toBeInstanceOf(DicomwebError);
        expect(error).toMatchObject({ code: "Aborted" });
        expect(receiveCalls).toBe(0);
        expect(cancelCount).toBeGreaterThanOrEqual(1);
        expect(stream.locked).toBe(false);
      } finally {
        controller.abort();
      }
    });

    it("honors a pre-aborted signal before applying the declared length cap", async () => {
      const fetchFn = async (): Promise<Response> =>
        new Response(null, {
          status: 200,
          headers: {
            "content-type": "application/dicom+json",
            "content-length": "100",
          },
        });
      const receiveCalls = { count: 0 };
      const client = new DicomwebClient({
        studiesServiceUrl: "https://example.invalid/dicomweb",
        wadoUriUrl: "https://example.invalid/wado",
        fetch: fetchFn,
        maxResponseBytes: 4,
        receive: async () => {
          receiveCalls.count += 1;
        },
      });
      const controller = new AbortController();
      try {
        controller.abort();
        const error = await client
          .searchStudies({ signal: controller.signal })
          .catch((caught: unknown) => caught);
        expect(error).toBeInstanceOf(DicomwebError);
        expect(error).toMatchObject({ code: "Aborted" });
        expect(receiveCalls.count).toBe(0);
      } finally {
        controller.abort();
      }
    });
  });


it('reports Aborted when receiver aborts after resolving', async () => {
  const controller = new AbortController();
  let calls = 0;
  const client = new DicomwebClient({
    studiesServiceUrl: 'https://example.invalid/studies',
    wadoUriUrl: 'https://example.invalid/wado',
    maxResponseBytes: 4,
    fetch: async () => new Response('[]', { headers: { 'Content-Type': 'application/dicom+json' } }),
    receive: async () => {
      calls += 1;
      controller.abort();
    },
  });
  const observed = await client.searchStudies({ signal: controller.signal }).catch((e: unknown) => e);
  expect(calls).toBe(1);
  expect(observed).toBeInstanceOf(DicomwebError);
  expect(observed).toMatchObject({ code: 'Aborted' });
});

it('Aborted takes precedence over coded callback error', async () => {
  const controller = new AbortController();
  let calls = 0;
  const client = new DicomwebClient({
    studiesServiceUrl: 'https://example.invalid/studies',
    wadoUriUrl: 'https://example.invalid/wado',
    maxResponseBytes: 4,
    fetch: async () => new Response('[]', { headers: { 'Content-Type': 'application/dicom+json' } }),
    receive: async () => {
      calls += 1;
      controller.abort();
      throw new OcelliError('CommitRefused', 'synthetic coded refusal');
    },
  });
  const observed = await client.searchStudies({ signal: controller.signal }).catch((e: unknown) => e);
  expect(calls).toBe(1);
  expect(observed).toBeInstanceOf(DicomwebError);
  expect(observed).toMatchObject({ code: 'Aborted' });
});



it('normalizes foreign receiver errors with safe fixed message', async () => {
  const foreignError = new Error('synthetic-private-marker');
  const controller = new AbortController();
  let calls = 0;
  const client = new DicomwebClient({
    studiesServiceUrl: 'https://example.invalid/studies',
    wadoUriUrl: 'https://example.invalid/wado',
    maxResponseBytes: 4,
    fetch: async () => new Response('[]', { headers: { 'Content-Type': 'application/dicom+json' } }),
    receive: async () => {
      calls += 1;
      throw foreignError;
    },
  });
  const observed = await client.searchStudies({ signal: controller.signal }).catch((e: unknown) => e);
  expect(calls).toBe(1);
  expect(observed).toBeInstanceOf(OcelliError);
  expect(observed).not.toBe(foreignError);
  expect(observed).toMatchObject({ code: 'CommitRefused', message: 'Delivery callback refused the response' });
});

it('F245: Content-Length header overreport must not refuse when actual bytes fit maxResponseBytes', async () => {
  const captured: number[][] = [];
  const stream = new ReadableStream<Uint8Array>({
    start(controller) {
      controller.enqueue(new Uint8Array([91, 93]));
      controller.close();
    },
  });
  const client = new DicomwebClient({
    studiesServiceUrl: 'https://example.invalid/studies',
    wadoUriUrl: 'https://example.invalid/wado',
    fetch: async () => new Response(stream, {
      headers: {
        'Content-Type': 'application/dicom+json',
        'Content-Length': '100',
      },
    }),
    maxResponseBytes: 2,
    receive: async (bytes: Uint8Array) => {
      captured.push(Array.from(bytes));
    },
  });
  await client.searchStudies();
  expect(captured).toEqual([[91, 93]]);
  expect(stream.locked).toBe(false);
})

it.each(["studiesServiceUrl", "wadoUriUrl"] as const)(
  "rejects a %s whose value is not a syntactically valid URL",
  (selectedUrlProperty) => {
    const fixture = responseSink();
    const options = { studiesServiceUrl: "https://example.invalid/", wadoUriUrl: "https://example.invalid/", fetch: async (): Promise<Response> => { throw new Error("fetch must not be called"); }, wasm: fixture.wasm, sink: fixture.sink };
    Reflect.set(options, selectedUrlProperty, "synthetic-URL-marker");
    let caught: unknown;
    try {
      new DicomwebClient(options);
    } catch (error) {
      caught = error;
    }
    expect(caught).toBeInstanceOf(DicomwebError);
    if (!(caught instanceof DicomwebError)) {
      throw new Error("unreachable: caught is not a DicomwebError");
    }
    expect(caught.code).toBe("InvalidRequest");
    expect(caught.message).not.toContain("synthetic-URL-marker");
  },
);

describe("bounded abort precedence", () => {
  type Case = {
    name: string;
    status: number;
    contentType: string;
    run: (client: DicomwebClient, signal: AbortSignal) => Promise<unknown>;
  };

  it.each<Case>([
    {
      name: "QIDO 503 after abort yields Aborted",
      status: 503,
      contentType: "application/dicom+json",
      run: (client, signal) => client.searchStudies({ signal }),
    },
    {
      name: "QIDO 200 text/plain after abort yields Aborted",
      status: 200,
      contentType: "text/plain",
      run: (client, signal) => client.searchStudies({ signal }),
    },
    {
      name: "WADO instance 200 text/plain after abort yields Aborted",
      status: 200,
      contentType: "text/plain",
      run: (client, signal) =>
        client.retrieveInstance("2.25.1", "2.25.2", "2.25.3", signal),
    },
  ])("$name", async ({ status, contentType, run }) => {
    const controller = new AbortController();
    const fetch = async (): Promise<Response> => {
      controller.abort();
      return new Response("[]", {
        status,
        headers: { "Content-Type": contentType },
      });
    };
    let received = 0;
    const client = new DicomwebClient({
      studiesServiceUrl: "https://example.invalid/",
      wadoUriUrl: "https://example.invalid/",
      fetch,
      maxResponseBytes: 4,
      receive: async (): Promise<void> => {
        received += 1;
      },
    });

    const result = run(client, controller.signal);
    await expect(result).rejects.toThrow(DicomwebError);
    await expect(result).rejects.toMatchObject({
      code: "Aborted",
    });
    expect(received).toBe(0);
  });
});

it('delivers exact synthetic bytes and callback kind for bounded public retrieveInstance', async () => {
  const captures: { bytes: number[]; kind: DicomwebResponseKind }[] = [];
  const client = new DicomwebClient({
    studiesServiceUrl: 'https://example.invalid/',
    wadoUriUrl: 'https://example.invalid/',
    fetch: async () => new Response(new Uint8Array([1, 2]), { headers: { 'Content-Type': 'multipart/related; type="application/dicom"; boundary=f245' } }),
    maxResponseBytes: 2,
    receive: async (bytes: Uint8Array, kind: DicomwebResponseKind): Promise<void> => {
      captures.push({ bytes: Array.from(bytes), kind });
    },
  });
  await client.retrieveInstance('2.25.1', '2.25.2', '2.25.3');
  expect(captures).toEqual([{ bytes: [1, 2], kind: { type: 'wado-rs-instances', boundary: 'f245' } }]);
});

it('delivers exact synthetic bytes and callback kind for bounded public retrieveFrames', async () => {
  const captures: { bytes: number[]; kind: DicomwebResponseKind }[] = [];
  const client = new DicomwebClient({
    studiesServiceUrl: 'https://example.invalid/',
    wadoUriUrl: 'https://example.invalid/',
    fetch: async () => new Response(new Uint8Array([1, 2]), { headers: { 'Content-Type': 'multipart/related; type="application/octet-stream"; boundary=f245' } }),
    maxResponseBytes: 2,
    receive: async (bytes: Uint8Array, kind: DicomwebResponseKind): Promise<void> => {
      captures.push({ bytes: Array.from(bytes), kind });
    },
  });
  await client.retrieveFrames('2.25.1', '2.25.2', '2.25.3', [1], 'application/octet-stream');
  expect(captures).toEqual([{ bytes: [1, 2], kind: { type: 'wado-rs-frames', boundary: 'f245', mediaType: 'application/octet-stream' } }]);
});

it('delivers exact synthetic bytes and callback kind for bounded public retrieveWadoUri', async () => {
  const captures: { bytes: number[]; kind: DicomwebResponseKind }[] = [];
  const client = new DicomwebClient({
    studiesServiceUrl: 'https://example.invalid/',
    wadoUriUrl: 'https://example.invalid/',
    fetch: async () => new Response(new Uint8Array([1, 2]), { headers: { 'Content-Type': 'application/dicom' } }),
    maxResponseBytes: 2,
    receive: async (bytes: Uint8Array, kind: DicomwebResponseKind): Promise<void> => {
      captures.push({ bytes: Array.from(bytes), kind });
    },
  });
  await client.retrieveWadoUri('2.25.1', '2.25.2', '2.25.3');
  expect(captures).toEqual([{ bytes: [1, 2], kind: { type: 'wado-uri-part10' } }]);
});

it("invokes the receiver unbound with an undefined this, so a second study search still refuses allocation", async () => {
  let requests = 0;
  const fetchImpl = async (): Promise<Response> => {
    requests += 1;
    if (requests === 1) {
      return new Response("[]", { headers: { "content-type": "application/dicom+json" } });
    }
    return new Response("[{}]", { headers: { "content-type": "application/dicom+json" } });
  };
  let replacementCalls = 0;
  const replacement = async (): Promise<void> => {
    replacementCalls += 1;
  };
  let originalCalls = 0;
  let observedThisWasUndefined: boolean | undefined = undefined;
  const receive = async function (this: unknown): Promise<void> {
    originalCalls += 1;
    observedThisWasUndefined = this === undefined;
    if (this !== null && typeof this === "object") {
      Reflect.set(this, "maxResponseBytes", 100);
      Reflect.set(this, "receive", replacement);
    }
  };
  const client = new DicomwebClient({
    studiesServiceUrl: "https://example.invalid/",
    wadoUriUrl: "https://example.invalid/",
    fetch: fetchImpl,
    maxResponseBytes: 2,
    receive,
  });
  await client.searchStudies();
  const second = await client.searchStudies().catch((error: unknown) => error);
  const code = second instanceof OcelliError ? second.code : undefined;
  expect({ originalCalls, replacementCalls, requests, thisWasUndefined: observedThisWasUndefined, code }).toEqual({ originalCalls: 1, replacementCalls: 0, requests: 2, thisWasUndefined: true, code: "AllocationRefused" });
});
