import { describe, expect, it } from "vitest";

import {
  DicomwebClient,
  DicomwebError,
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
    expect(() =>
      new DicomwebClient({
        studiesServiceUrl: marker,
        wadoUriUrl: "https://example.test/wado",
        fetch: async () => ok("[]", "application/dicom+json"),
        ...responseSink(),
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
});
