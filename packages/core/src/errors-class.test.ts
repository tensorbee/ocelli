import { describe, expect, it } from "vitest";

import {
  COMMAND_PAYLOAD_BYTES,
  DicomwebClient,
  DicomwebError,
  acceptTransferredFrame,
  openDecodePool,
  packCommands,
  postWadoInstance,
  sessionBulkSink,
  submitCommands,
  writeDicomwebResponse,
  writeFrame,
  writeStack,
  type BulkSink,
  type FrameCommitMeta,
  type SessionBulkExports,
  type SessionStackExports,
  type WasmMemory,
} from "./index.js";
import { OcelliError, type OcelliErrorCode } from "./ocelli-error.js";

const SHELL_CODES = [
  "InvalidRequest",
  "Network",
  "Aborted",
  "HttpStatus",
  "ContentType",
  "UnknownOption",
  "InvalidOptions",
  "AllocationRefused",
  "CommitRefused",
  "LengthMismatch",
  "InvalidValue",
  "ExportUnavailable",
  "CoreLoadFailed",
] as const satisfies readonly OcelliErrorCode[];

type MissingShellCode = Exclude<OcelliErrorCode, (typeof SHELL_CODES)[number]>;
const shellCodesComplete: [MissingShellCode] extends [never] ? true : never = true;

const META = { viewport: 1, columns: 1, rows: 1, generation: 1 };
const STUDIES = "https://example.test/dicomweb/";
const WADO = "https://example.test/wado";

function thrown(run: () => unknown): unknown {
  try {
    run();
  } catch (error) {
    return error;
  }
  throw new Error("expected a throw");
}

function expectCode(error: unknown, code: OcelliErrorCode, message: string): void {
  expect(error).toBeInstanceOf(OcelliError);
  if (!(error instanceof OcelliError)) {
    throw new Error("expected OcelliError");
  }
  expect(error.name).toBe("OcelliError");
  expect(error.code).toBe(code);
  expect(error.message).toBe(message);
}

function memory(): WasmMemory {
  return { memory: new WebAssembly.Memory({ initial: 1 }) };
}

function refusingSink(code: number): BulkSink {
  return {
    alloc: () => 0,
    commit_frame: () => code,
  };
}

describe("OcelliError", () => {
  it("pins the shell code union", () => {
    expect(shellCodesComplete).toBe(true);
    expect([...SHELL_CODES]).toEqual([
      "InvalidRequest",
      "Network",
      "Aborted",
      "HttpStatus",
      "ContentType",
      "UnknownOption",
      "InvalidOptions",
      "AllocationRefused",
      "CommitRefused",
      "LengthMismatch",
      "InvalidValue",
      "ExportUnavailable",
      "CoreLoadFailed",
    ]);
  });

  it("maps allocation refusals", () => {
    const wasm = memory();
    const bytes = new Uint8Array([1, 2, 3, 4]);
    expectCode(
      thrown(() => writeFrame(wasm, refusingSink(0), bytes, META)),
      "AllocationRefused",
      "allocation refused",
    );
    expectCode(
      thrown(() =>
        writeDicomwebResponse(wasm, { alloc: () => 0, commit_dicomweb_response() {} }, bytes, {
          type: "qido-json",
        }),
      ),
      "AllocationRefused",
      "allocation refused",
    );
    expectCode(
      thrown(() =>
        acceptTransferredFrame(wasm, refusingSink(0), new ArrayBuffer(4), {
          series: 1,
          instance: 1,
          frame: 1,
          bytes: 4,
        }, META),
      ),
      "AllocationRefused",
      "allocation refused",
    );
    const stack: SessionStackExports = {
      ...wasm,
      alloc: () => 0,
      bulk_ticket: () => 1,
      release_bulk: () => 1,
      configure_stack: () => 1,
    };
    expectCode(
      thrown(() => writeStack(stack, [{ series: 1n, instance: 1, frame: 1 }])),
      "AllocationRefused",
      "stack allocation refused",
    );
  });

  it("maps commit refusals", () => {
    const wasm = memory();
    const bytes = new Uint8Array([1, 2, 3, 4]);
    const sink: BulkSink = {
      alloc: () => 16,
      commit_frame: () => 0,
    };
    expectCode(
      thrown(() => writeFrame(wasm, sink, bytes, META)),
      "CommitRefused",
      "commit refused",
    );
    expectCode(
      thrown(() =>
        acceptTransferredFrame(wasm, sink, new ArrayBuffer(4), {
          series: 1,
          instance: 1,
          frame: 1,
          bytes: 4,
        }, META),
      ),
      "CommitRefused",
      "commit refused",
    );
    let released = 0;
    const stack: SessionStackExports = {
      ...wasm,
      alloc: () => 32,
      bulk_ticket: () => 9,
      release_bulk(ticket) {
        released = ticket;
        return 1;
      },
      configure_stack: () => 0,
    };
    expectCode(
      thrown(() => writeStack(stack, [{ series: 1n, instance: 1, frame: 1 }])),
      "CommitRefused",
      "stack configuration refused",
    );
    expect(released).toBe(9);
  });

  it("maps length mismatches", () => {
    const wasm = memory();
    expectCode(
      thrown(() =>
        acceptTransferredFrame(wasm, refusingSink(1), new ArrayBuffer(8), {
          series: 1,
          instance: 1,
          frame: 1,
          bytes: 7,
        }, META),
      ),
      "LengthMismatch",
      "handoff length 8 disagrees with meta.bytes 7",
    );
    const stack: SessionStackExports = {
      ...wasm,
      alloc: () => 0,
      bulk_ticket: () => 1,
      release_bulk: () => 1,
      configure_stack: () => 1,
    };
    expectCode(
      thrown(() => writeStack(stack, [])),
      "LengthMismatch",
      "stack length is outside the bulk limit",
    );
  });

  it("maps invalid values and a non-object frame meta", () => {
    const wasm = memory();
    const stack: SessionStackExports = {
      ...wasm,
      alloc: () => 0,
      bulk_ticket: () => 1,
      release_bulk: () => 1,
      configure_stack: () => 1,
    };
    expectCode(
      thrown(() => writeStack(stack, [{ series: -1n, instance: 1, frame: 1 }])),
      "InvalidValue",
      "series is not a u64",
    );
    const sink: BulkSink = {
      alloc: () => 16,
      allocationTicket: () => 1,
      commitAllocated: () => 1,
      commit_frame: () => 1,
    };
    expectCode(
      thrown(() => writeFrame(wasm, sink, new Uint8Array(4), { ...META, viewport: -1 })),
      "InvalidValue",
      "viewport is not a u32",
    );
    const session = sessionBulkSink({
      memory: wasm.memory,
      alloc: () => 16,
      bulk_ticket: () => 4,
      release_bulk: () => 1,
      commit_frame: () => 1,
    });
    expect(session.alloc(4)).toBe(16);
    expectCode(
      thrown(() => session.commitAllocated?.(4, 4, null)),
      "InvalidOptions",
      "frame meta is not an object",
    );
  });

  it("keeps the non-object frame meta message at the public entry points", () => {
    const wasm = memory();
    let allocs = 0;
    const ticketed: BulkSink = {
      alloc(): number {
        allocs += 1;
        return 16;
      },
      allocationTicket: () => 1,
      commitAllocated: () => 1,
      commit_frame: () => 1,
      releaseAllocated: () => {},
    };
    const handoff = { series: 1, instance: 1, frame: 1, bytes: 4 };
    const nonObjects: unknown[] = [null, undefined, 7, "meta", true, Symbol("meta"), () => {}];
    for (const value of nonObjects) {
      const meta = value as unknown as FrameCommitMeta;
      expectCode(
        thrown(() => writeFrame(wasm, ticketed, new Uint8Array(4), meta)),
        "InvalidOptions",
        "frame meta is not an object",
      );
      expectCode(
        thrown(() =>
          acceptTransferredFrame(
            wasm,
            ticketed,
            new ArrayBuffer(4),
            handoff,
            meta,
          ),
        ),
        "InvalidOptions",
        "frame meta is not an object",
      );
    }
    const array = [] as unknown as FrameCommitMeta;
    expectCode(
      thrown(() => writeFrame(wasm, ticketed, new Uint8Array(4), array)),
      "InvalidOptions",
      "FrameCommitMeta is not an options object",
    );
    expectCode(
      thrown(() => acceptTransferredFrame(wasm, ticketed, new ArrayBuffer(4), handoff, array)),
      "InvalidOptions",
      "FrameCommitMeta is not an options object",
    );
    expect(allocs).toBe(0);
  });

  it("maps a missing selected-frame export", () => {
    const wasm = memory();
    const session: SessionBulkExports = {
      memory: wasm.memory,
      alloc: () => 32,
      bulk_ticket: () => 6,
      release_bulk: () => 1,
      commit_frame: () => 1,
    };
    expectCode(
      thrown(() =>
        writeFrame(wasm, sessionBulkSink(session), new Uint8Array(4), {
          ...META,
          stackIndex: 2,
          stackEpoch: 3,
        }),
      ),
      "ExportUnavailable",
      "selected-frame export is unavailable",
    );
  });

  it("maps command value errors", () => {
    expectCode(
      thrown(() => packCommands(70_000, [])),
      "InvalidValue",
      "viewport 70000 is not a u16",
    );
    expectCode(
      thrown(() =>
        packCommands(1, [{ kind: "Fit", width: 1, height: 1, columns: -1, rows: 1 }]),
      ),
      "InvalidValue",
      "columns -1 is not a u32",
    );
    expectCode(
      thrown(() => packCommands(1, [{ kind: "Scroll", step: 2_147_483_648 }])),
      "InvalidValue",
      "step 2147483648 is not an i32",
    );
    // The generated table is a mutable object. Zoom is written to 3 so the
    // defensive length check can run, then restored to 8.
    const table = COMMAND_PAYLOAD_BYTES as { Zoom: number };
    const saved = table.Zoom;
    table.Zoom = 3;
    try {
      expectCode(
        thrown(() => packCommands(1, [{ kind: "Zoom", factor: 2 }])),
        "InvalidValue",
        "Zoom payload is 8 bytes and the Rust table declares 3",
      );
    } finally {
      table.Zoom = saved;
    }
    expect(COMMAND_PAYLOAD_BYTES.Zoom).toBe(8);
  });

  it("maps an instance body that was not committed", async () => {
    const original = DicomwebClient.prototype.retrieveInstance;
    // The stub resolves without writing the sink, so the refusal after
    // retrieve is the production throw under test.
    DicomwebClient.prototype.retrieveInstance = (() => Promise.resolve()) as typeof original;
    try {
      let caught: unknown;
      try {
        await postWadoInstance({
          studiesServiceUrl: STUDIES,
          wadoUriUrl: WADO,
          fetch: async () => new Response(null, { status: 200 }),
          studyUid: "1.2.3",
          seriesUid: "1.2.4",
          instanceUid: "1.2.5",
          post(): void {},
        });
      } catch (error) {
        caught = error;
      }
      expectCode(caught, "CommitRefused", "the instance body was not committed");
    } finally {
      DicomwebClient.prototype.retrieveInstance = original;
    }
  });

  it("keeps DicomwebError as an OcelliError without a cause", async () => {
    const cases: readonly [DicomwebError, string][] = [
      [new DicomwebError("InvalidRequest"), "DICOMweb request parameters are invalid"],
      [new DicomwebError("Network"), "DICOMweb request failed"],
      [new DicomwebError("Aborted"), "DICOMweb request was cancelled"],
      [new DicomwebError("HttpStatus", 503), "DICOMweb server returned an unsuccessful status"],
      [new DicomwebError("ContentType"), "DICOMweb response content type is incompatible"],
    ];
    for (const [error, message] of cases) {
      expect(error).toBeInstanceOf(OcelliError);
      expect(error).toBeInstanceOf(Error);
      expect(error).toBeInstanceOf(DicomwebError);
      expect(error.name).toBe("DicomwebError");
      expect(error.message).toBe(message);
      expect(error.cause).toBeUndefined();
    }
    expect(cases[3]?.[0].status).toBe(503);
    expect(cases[0]?.[0].status).toBeUndefined();

    const marker = new Error("synthetic network refusal");
    const client = new DicomwebClient({
      studiesServiceUrl: STUDIES,
      wadoUriUrl: WADO,
      fetch: async () => {
        throw marker;
      },
      wasm: memory(),
      sink: { alloc: () => 0, commit_dicomweb_response() {} },
    });
    const caught = await client.searchStudies().then(
      () => undefined,
      (error: unknown) => error,
    );
    expect(caught).toBeInstanceOf(DicomwebError);
    if (!(caught instanceof DicomwebError)) {
      throw new Error("expected DicomwebError");
    }
    expect(caught).toBeInstanceOf(OcelliError);
    expect(caught.code).toBe("Network");
    expect(caught.cause).toBeUndefined();
  });

  it("caller errors are rethrown unchanged", () => {
    const writeError = new Error("write failed");
    let released = 0;
    let caught: unknown;
    try {
      submitCommands(
        {
          write() {
            throw writeError;
          },
        },
        {
          alloc: () => 40,
          allocationTicket: () => 9,
          apply_commands: () => 1,
          releaseAllocated(ticket) {
            released = ticket;
          },
        },
        new Uint8Array([1]),
      );
    } catch (error) {
      caught = error;
    }
    expect(caught).toBe(writeError);
    expect(released).toBe(9);

    const startError = new Error("start failed");
    const owner = {};
    caught = undefined;
    try {
      openDecodePool(owner, 1, () => {
        throw startError;
      });
    } catch (error) {
      caught = error;
    }
    expect(caught).toBe(startError);

    const configureError = new Error("configure failed");
    const wasm = memory();
    released = 0;
    const stack: SessionStackExports = {
      ...wasm,
      alloc: () => 32,
      bulk_ticket: () => 8,
      release_bulk(ticket) {
        released = ticket;
        return 1;
      },
      configure_stack() {
        throw configureError;
      },
    };
    caught = undefined;
    try {
      writeStack(stack, [{ series: 1n, instance: 1, frame: 1 }]);
    } catch (error) {
      caught = error;
    }
    expect(caught).toBe(configureError);
    expect(released).toBe(8);
  });
});
