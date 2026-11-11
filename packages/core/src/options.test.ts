import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import {
  DicomwebClient,
  acceptTransferredFrame,
  createCursorController,
  createPointerDispatch,
  createToolRegistry,
  postWadoInstance,
  viewportId,
  writeFrame,
  writeKeyObject,
  writeSecondaryCapture,
  type BulkSink,
  type FrameCommitMeta,
  type KeyObjectRequest,
  type PointerDispatchOptions,
  type QidoSearchOptions,
  type SecondaryCaptureRequest,
  type WadoInstanceRequest,
} from "./index.js";
import { OcelliError } from "./ocelli-error.js";
import { OPTION_KEY_LISTS, OPTION_LISTS_COMPLETE } from "./options.js";

const SRC = dirname(fileURLToPath(import.meta.url));
const STUDIES = "https://example.test/dicomweb/";
const WADO = "https://example.test/wado";

/**
 * The cast keeps a key the public type rejects. The own property is still
 * there at run time, which is what the refusal reads.
 */
function withKey<T extends object>(value: T, key: string | symbol, extra: unknown): T {
  return { ...value, [key]: extra } as T;
}

async function raised(run: () => unknown): Promise<unknown> {
  try {
    await run();
  } catch (error) {
    return error;
  }
  throw new Error("expected a refusal");
}

function expectUnknown(error: unknown, owner: string, option = "zzUnknown"): void {
  expect(error).toBeInstanceOf(OcelliError);
  if (!(error instanceof OcelliError)) {
    throw new Error("expected OcelliError");
  }
  expect(error.code).toBe("UnknownOption");
  expect(error.option).toBe(option);
  expect(error.message).toBe(`${owner} has no option ${JSON.stringify(option)}`);
}

function clientOptions(fetches: { count: number }) {
  const memory = new WebAssembly.Memory({ initial: 1, maximum: 2 });
  return {
    studiesServiceUrl: STUDIES,
    wadoUriUrl: WADO,
    fetch: (async () => {
      fetches.count += 1;
      return new Response("[]", {
        status: 200,
        headers: { "Content-Type": "application/dicom+json" },
      });
    }) as typeof fetch,
    wasm: { memory },
    sink: {
      alloc(): number {
        return 0;
      },
      commit_dicomweb_response(): void {},
    },
  };
}

function pointerBase(): PointerDispatchOptions {
  const tools = createToolRegistry();
  expect(tools.createGroup("g").ok).toBe(true);
  return {
    registry: tools,
    groupId: "g",
    viewport: viewportId(4),
  };
}

function cursorBase() {
  const tools = createToolRegistry();
  expect(tools.createGroup("g").ok).toBe(true);
  return {
    registry: tools,
    groupId: "g",
    style: { cursor: "" },
  };
}

const keyRequest: KeyObjectRequest = {
  title: "Key image",
  study: "2.25.11",
  series: "2.25.12",
  instance: "2.25.10",
  referencedClass: "1.2.840.10008.5.1.4.1.1.2",
  referencedInstance: "2.25.13",
};

const captureRequest: SecondaryCaptureRequest = {
  study: "2.25.21",
  series: "2.25.22",
  instance: "2.25.20",
  columns: 2,
  rows: 2,
  pixels: new Uint8Array([0, 1, 2, 3]),
};

const frameMeta: FrameCommitMeta = {
  viewport: 1,
  columns: 1,
  rows: 1,
  generation: 1,
};

function optionTypeNames(): string[] {
  const found = new Set<string>(["FrameCommitMeta"]);
  const pattern = /export (?:interface|type) ([A-Za-z]*(?:Options|Request))\b/g;
  for (const name of readdirSync(SRC)) {
    if (!name.endsWith(".ts") || name.endsWith(".test.ts")) {
      continue;
    }
    const text = readFileSync(join(SRC, name), "utf8");
    for (const match of text.matchAll(pattern)) {
      const typeName = match[1];
      if (typeName !== undefined) {
        found.add(typeName);
      }
    }
  }
  return [...found].sort();
}

describe("public options", () => {
  it("refuses an unknown key at every entry point", async () => {
    const fetches = { count: 0 };
    const listeners: string[] = [];
    const frameAllocs = { count: 0 };
    const handoffAllocs = { count: 0 };
    const base = clientOptions(fetches);
    const memory = new WebAssembly.Memory({ initial: 1 });
    const bytes = new Uint8Array([1, 2, 3, 4]);

    expectUnknown(
      await raised(() => new DicomwebClient(withKey(base, "zzUnknown", 1))),
      "DicomwebClientOptions",
    );
    expectUnknown(
      await raised(() => new DicomwebClient(withKey(base, "zzUnknown", 1))),
      "DicomwebClientOptions",
    );

    const client = new DicomwebClient(base);
    expectUnknown(
      await raised(() => client.searchStudies(withKey({}, "zzUnknown", 1))),
      "QidoSearchOptions",
    );
    expectUnknown(
      await raised(() => client.searchSeries("1.2.3", withKey({}, "zzUnknown", 1))),
      "QidoSearchOptions",
    );
    expectUnknown(
      await raised(() => client.searchInstances("1.2.3", "1.2.4", withKey({}, "zzUnknown", 1))),
      "QidoSearchOptions",
    );

    const target = {
      addEventListener(type: string): void {
        listeners.push(type);
      },
      removeEventListener(): void {},
    };
    expectUnknown(
      await raised(() =>
        createPointerDispatch(withKey({ ...pointerBase(), target }, "zzUnknown", 1)),
      ),
      "PointerDispatchOptions",
    );

    expectUnknown(
      await raised(() => createCursorController(withKey(cursorBase(), "zzUnknown", 1))),
      "CursorControllerOptions",
    );

    const request: WadoInstanceRequest = {
      studiesServiceUrl: STUDIES,
      wadoUriUrl: WADO,
      fetch: base.fetch,
      studyUid: "1.2.3",
      seriesUid: "1.2.4",
      instanceUid: "1.2.5",
      post(): void {},
    };
    expectUnknown(
      await raised(() => postWadoInstance(withKey(request, "zzUnknown", 1))),
      "WadoInstanceRequest",
    );

    expectUnknown(
      await raised(() => writeKeyObject(withKey(keyRequest, "zzUnknown", 1))),
      "KeyObjectRequest",
    );
    expectUnknown(
      await raised(() => writeSecondaryCapture(withKey(captureRequest, "zzUnknown", 1))),
      "SecondaryCaptureRequest",
    );

    const ticketed: BulkSink = {
      alloc(): number {
        frameAllocs.count += 1;
        return 16;
      },
      allocationTicket(): number {
        return 1;
      },
      commitAllocated(): number {
        return 1;
      },
      commit_frame(): number {
        return 1;
      },
      releaseAllocated(): void {},
    };
    expectUnknown(
      await raised(() => writeFrame({ memory }, ticketed, bytes, withKey(frameMeta, "zzUnknown", 1))),
      "FrameCommitMeta",
    );

    const handoff: BulkSink = {
      alloc(): number {
        handoffAllocs.count += 1;
        return 16;
      },
      commit_frame(): number {
        return 1;
      },
    };
    expectUnknown(
      await raised(() =>
        acceptTransferredFrame(
          { memory },
          handoff,
          new ArrayBuffer(4),
          { series: 1, instance: 1, frame: 1, bytes: 4 },
          withKey(frameMeta, "zzUnknown", 1),
        ),
      ),
      "FrameCommitMeta",
    );

    expect(fetches.count).toBe(0);
    expect(listeners).toEqual([]);
    expect(frameAllocs.count).toBe(0);
    expect(handoffAllocs.count).toBe(0);
  });

  it("rejects an unknown search key as a promise", async () => {
    const fetches = { count: 0 };
    const client = new DicomwebClient(clientOptions(fetches));
    const bad = withKey({}, "zzUnknown", 1);
    const calls = [
      ["searchStudies", () => client.searchStudies(bad)],
      ["searchSeries", () => client.searchSeries("1.2.3", bad)],
      ["searchInstances", () => client.searchInstances("1.2.3", "1.2.4", bad)],
    ] as const;
    for (const [name, call] of calls) {
      let returned: unknown;
      try {
        returned = call();
      } catch (error) {
        returned = error;
      }
      expect(returned, name).toBeInstanceOf(Promise);
      const reason: unknown = await (returned as Promise<unknown>).catch(
        (caught: unknown) => caught,
      );
      expect(reason, name).toBeInstanceOf(OcelliError);
      if (!(reason instanceof OcelliError)) {
        throw new Error(`expected OcelliError from ${name}`);
      }
      expect(reason.code, name).toBe("UnknownOption");
    }
    expect(fetches.count).toBe(0);
  });

  it("edge keys", async () => {
    const fetches = { count: 0 };
    const base = clientOptions(fetches);
    const client = new DicomwebClient({
      ...base,
      sink: {
        alloc: () => 16,
        commit_dicomweb_response() {},
      },
    });
    // exactOptionalPropertyTypes rejects a declared optional set to undefined.
    // A direct cast does not overlap, so this goes through unknown. The own
    // key is limit and its value is undefined, which the check accepts.
    await client.searchStudies({ limit: undefined } as unknown as QidoSearchOptions);
    expect(fetches.count).toBe(1);

    const dispatch = createPointerDispatch(withKey(pointerBase(), "now", undefined));
    expect(dispatch.takeFrame().reason).toBe("empty");

    const memory = new WebAssembly.Memory({ initial: 1 });
    writeFrame(
      { memory },
      {
        alloc: () => 16,
        allocationTicket: () => 1,
        commitAllocated: () => 1,
        commit_frame: () => 1,
        releaseAllocated: () => {},
      },
      new Uint8Array(4),
      withKey(frameMeta, "stackIndex", undefined),
    );

    const symbol = Symbol("clock");
    expectUnknown(
      await raised(() => createPointerDispatch(withKey(pointerBase(), symbol, 1))),
      "PointerDispatchOptions",
      String(symbol),
    );

    const longKey = "k".repeat(100);
    const shown = longKey.slice(0, 64);
    expectUnknown(
      await raised(() => new DicomwebClient(withKey(base, longKey, 1))),
      "DicomwebClientOptions",
      shown,
    );
    const exact = "m".repeat(64);
    expectUnknown(
      await raised(() => new DicomwebClient(withKey(base, exact, 1))),
      "DicomwebClientOptions",
      exact,
    );

    for (const value of [null, ["studiesServiceUrl"], 7]) {
      // The parameter type is DicomwebClientOptions. These values are the
      // non-objects the runtime must refuse, so the cast erases that type.
      const error = await raised(() => new DicomwebClient(value as never));
      expect(error).toBeInstanceOf(OcelliError);
      if (!(error instanceof OcelliError)) {
        throw new Error("expected OcelliError");
      }
      expect(error.code).toBe("InvalidOptions");
      expect(error.option).toBeUndefined();
      expect(error.message).toBe("DicomwebClientOptions is not an options object");
    }

    const inherited = Object.assign(Object.create({ zzInherited: 1 }), base);
    expect(() => new DicomwebClient(inherited)).not.toThrow();
  });

  it("every options type has a key list", () => {
    expect(OPTION_LISTS_COMPLETE).toBe(true);
    expect(Object.keys(OPTION_KEY_LISTS).sort()).toEqual(optionTypeNames());
  });
});
