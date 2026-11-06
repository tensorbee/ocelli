/**
 * The built wasm session, called through the public helpers.
 *
 * `npm run test` does not build the module and does not collect this file.
 * Build it, then run this file alone:
 *
 *     wasm-pack build crates/ocelli-wasm --dev --target web \
 *       --out-dir target/boundary-wasm
 *     npx vitest run --config vitest.boundary.config.ts
 *
 * A missing module is a failure of this file, not a skip. A dev build is not
 * a release-size measurement and not a browser-worker observation.
 */
import { env } from "node:process";
import { readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { describe, expect, it } from "vitest";

import {
  acceptTransferredFrame,
  COLORMAP_BYTE,
  COMMAND_ID,
  contentId,
  decodeNamedEvent,
  drainRing,
  EVENT_HEADER_BYTES,
  EVENT_STRIDE,
  FLAG_INVERT,
  FLAG_LOADING,
  frameResident,
  INVERT_BYTE,
  releaseSeries,
  RING_EVENT_KIND,
  sessionBulkSink,
  sessionCommandSink,
  setFrameBudget,
  showResident,
  restoreSessionSnapshot,
  sessionSnapshot,
  sessionSnapshotError,
  SNAPSHOT_ERROR,
  SNAPSHOT_MAGIC,
  packCommands,
  parseSnapshot,
  PRESET_BYTE,
  readViewportState,
  submitCommands,
  VOI_FUNCTION_BYTE,
  writeFrame,
  writeCommands,
  writeStack,
} from "./index.js";
import type { CacheReleasedPayload, PackedCommand } from "./index.js";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const MODULE = env.OCELLI_BOUNDARY_MODULE ?? "crates/ocelli-wasm/target/boundary-wasm";
const GLUE = join(ROOT, MODULE, "ocelli_wasm.js");
const WASM = join(ROOT, MODULE, "ocelli_wasm_bg.wasm");
const MAX_BULK_BYTES = 67_108_864;

interface BuiltExports {
  memory: WebAssembly.Memory;
  alloc(len: number): number;
  bulk_ticket(): number;
  release_bulk(ticket: number): number;
  apply_commands(ticket: number, len: number): number;
  configure_stack(ticket: number, len: number): number;
  commit_selected_frame(ticket: number, len: number, viewport: number, columns: number, rows: number, index: number, epoch: number): number;
  copy_state(): Uint8Array;
  drain_events(): Uint8Array;
  snapshot(): Uint8Array;
  snapshot_error(): number;
  restore_snapshot(bytes: Uint8Array): number;
  bindCpuCanvas(
    canvas: unknown,
    cssWidth: number,
    cssHeight: number,
    dpr: number,
  ): void;
  publishCpuCanvas(): void;
  publishCpuInteraction(): void;
  commit_frame(
    ticket: number,
    len: number,
    viewport: number,
    columns: number,
    rows: number,
    generation: number,
  ): number;
  set_frame_budget(bytes: number): number;
  commit_cached_frame(
    ticket: number,
    len: number,
    viewport: number,
    columns: number,
    rows: number,
    index: number,
    epoch: number,
    content: bigint,
  ): number;
  frame_resident(
    index: number,
    epoch: number,
    content: bigint,
    columns: number,
    rows: number,
  ): number;
  show_resident(
    index: number,
    epoch: number,
    content: bigint,
    columns: number,
    rows: number,
  ): number;
  release_series(series: bigint): number;
  request_frame_pressure(): number;
  content_id(ticket: number, len: number, uidLen: number, tsLen: number): bigint | undefined;
  [name: string]: unknown;
}

// The wasm session is `StackViewport::try_identity(1)`.
const META = { viewport: 1, columns: 1, rows: 1, generation: 1 };

/**
 * Node has no `OffscreenCanvas` or `ImageData`. `publishCpuCanvas` needs both
 * to retain and blit a frame, so this test supplies the smallest objects the
 * wasm side reads: a canvas with width, height and a 2d context, and an
 * `ImageData` with a clamped array. `putImageData` is a no-op because the
 * observation is the event, not the pixels. The globals are installed once
 * for the file and do not affect the other tests, which never call the CPU
 * canvas exports.
 */
class StubImageData {
  readonly data: Uint8ClampedArray;

  constructor(width: number, height: number) {
    this.data = new Uint8ClampedArray(width * height * 4);
  }
}

class StubContext {
  putImageData(...args: unknown[]): void {
    void args;
  }
}

class StubCanvas {
  width = 0;
  height = 0;
  private readonly context = new StubContext();

  getContext(kind: string): StubContext | null {
    return kind === "2d" ? this.context : null;
  }
}

function installCanvasStubs(): void {
  const globals = globalThis as unknown as Record<string, unknown>;
  globals.ImageData = StubImageData;
  globals.OffscreenCanvas = StubCanvas;
  globals.OffscreenCanvasRenderingContext2D = StubContext;
}

let instance = 0;
async function loadModule(): Promise<BuiltExports> {
  const glue = await import(`${pathToFileURL(GLUE).href}?instance=${instance++}`);
  const bytes = await readFile(WASM);
  const exports = await glue.default({ module_or_path: bytes });
  return {
    ...exports,
    copy_state: glue.copy_state,
    drain_events: glue.drain_events,
    snapshot: glue.snapshot,
    snapshot_error: glue.snapshot_error,
    restore_snapshot: glue.restore_snapshot,
    // The F-231 exports are re-taken from the glue like the ones above:
    // the raw export of an Option<u64> is a flag and value pair, and the
    // wrapped name is the one the public surface answers with.
    set_frame_budget: glue.set_frame_budget,
    commit_cached_frame: glue.commit_cached_frame,
    frame_resident: glue.frame_resident,
    show_resident: glue.show_resident,
    release_series: glue.release_series,
    request_frame_pressure: glue.request_frame_pressure,
    content_id: glue.content_id,
  } as BuiltExports;
}

function fill(memory: WebAssembly.Memory, ptr: number, bytes: Uint8Array): void {
  new Uint8Array(memory.buffer, ptr, bytes.byteLength).set(bytes);
}

function read(memory: WebAssembly.Memory, ptr: number, len: number): number[] {
  return Array.from(new Uint8Array(memory.buffer, ptr, len));
}

function submit(wasm: BuiltExports, bytes: Uint8Array): number {
  return submitCommands(
    {
      write(ptr, payload) {
        new Uint8Array(wasm.memory.buffer, ptr, payload.byteLength).set(payload);
      },
    },
    sessionCommandSink(wasm),
    bytes,
  );
}

function commandBytes(kind: number, payload: readonly number[]): Uint8Array {
  const bytes = new Uint8Array(8 + payload.length);
  const view = new DataView(bytes.buffer);
  view.setUint16(0, kind, true);
  view.setUint16(2, 1, true);
  view.setUint32(4, payload.length, true);
  payload.forEach((byte, index) => view.setUint8(8 + index, byte));
  return bytes;
}

function f64(value: number): number[] {
  const buffer = new ArrayBuffer(8);
  new DataView(buffer).setFloat64(0, value, true);
  return [...new Uint8Array(buffer)];
}

const EVERY_KIND: readonly PackedCommand[] = [
  { kind: "Fit", width: 800, height: 600, columns: 512, rows: 512 },
  { kind: "Pan", x: 1, y: 2 },
  { kind: "Zoom", factor: 2 },
  { kind: "Rotate", radians: 0 },
  { kind: "FlipHorizontal", enabled: true },
  { kind: "FlipVertical", enabled: true },
  { kind: "SetWindow", centre: 40, width: 400, function: "linear" },
  { kind: "ApplyPreset", preset: "soft_tissue" },
  { kind: "SetInvert", invert: "inverse" },
  { kind: "SetColormap", colormap: "stops" },
  { kind: "Scroll", step: 1 },
];

describe("wasm session boundary", () => {
  it("installs ordered keys and refuses stale selection through the public bulk adapter", async () => {
    const wasm = await loadModule();
    const keys = [{ series: 1n, instance: 1, frame: 7 }, { series: 1n, instance: 1, frame: 3 }];
    const epoch = writeStack(wasm, keys);
    expect(epoch).toBeGreaterThan(0);
    const state = () => {
      const bytes = wasm.copy_state();
      return new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    };
    expect(state().getUint32(76, true)).toBe(2);
    expect(state().getUint32(72, true)).toBe(0);
    const sink = sessionBulkSink(wasm);
    const frame = new Uint8Array(new Float32Array([40]).buffer);
    expect(() => writeFrame(wasm, sink, frame, { ...META, stackEpoch: epoch })).toThrow(/stackIndex/);
    expect(wasm.bulk_ticket()).toBe(0);
    writeFrame(wasm, sink, frame, { ...META, stackIndex: 0, stackEpoch: epoch });
    const commands = sessionCommandSink(wasm);
    expect(writeCommands(wasm, commands, packCommands(1, [{ kind: "Scroll", step: 1 }]))).toBe(1);
    expect(state().getUint32(72, true)).toBe(1);
    expect(state().getUint32(80, true) & 2).toBe(2);
    expect(() => writeFrame(wasm, sink, frame, { ...META, stackIndex: 0, stackEpoch: epoch })).toThrow(/commit refused/);
    expect(wasm.bulk_ticket()).toBe(0);
    writeFrame(wasm, sink, frame, { ...META, stackIndex: 1, stackEpoch: epoch });
    expect(state().getUint32(80, true) & 2).toBe(0);
    const next = writeStack(wasm, keys);
    expect(next).not.toBe(epoch);
    expect(() => writeFrame(wasm, sink, frame, { ...META, stackIndex: 0, stackEpoch: epoch })).toThrow(/commit refused/);
    expect(() => writeFrame(wasm, sink, frame, META)).toThrow(/commit refused/);
    writeFrame(wasm, sink, frame, { ...META, stackIndex: 0, stackEpoch: next });
  });
  it("refuses address 0, reuses an address only with a new ticket, and commits through writeFrame", async () => {
    const wasm = await loadModule();
    const before = read(wasm.memory, 0, 8);

    expect(wasm.alloc(MAX_BULK_BYTES + 1)).toBe(0);
    expect(read(wasm.memory, 0, 8)).toEqual(before);
    expect(wasm.bulk_ticket()).toBe(0);

    const held = wasm.alloc(8);
    const heldTicket = wasm.bulk_ticket();
    expect(held).not.toBe(0);
    expect(heldTicket).not.toBe(0);
    expect(heldTicket).not.toBe(held);
    fill(wasm.memory, held, new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8]));
    const occupied = read(wasm.memory, held, 8);

    expect(wasm.alloc(8)).toBe(0);
    expect(read(wasm.memory, 0, 8)).toEqual(before);
    expect(read(wasm.memory, held, 8)).toEqual(occupied);
    expect(wasm.bulk_ticket()).toBe(heldTicket);

    const sink = sessionBulkSink(wasm);
    expect(() => writeFrame({ memory: wasm.memory }, sink, new Uint8Array(4), META)).toThrow(
      "allocation refused",
    );
    expect(read(wasm.memory, 0, 8)).toEqual(before);
    expect(wasm.bulk_ticket()).toBe(heldTicket);
    expect(wasm.commit_frame(0, 8, 1, 1, 1, 1)).toBe(0);
    expect(wasm.bulk_ticket()).toBe(heldTicket);
    expect(wasm.commit_frame(heldTicket, 8, 1, 1, 1, 1)).toBe(1);
    expect(wasm.bulk_ticket()).toBe(0);

    const emptyA = wasm.alloc(0);
    const ticketA = wasm.bulk_ticket();
    expect(wasm.commit_frame(ticketA, 0, 1, 1, 1, 1)).toBe(1);
    const emptyB = wasm.alloc(0);
    const ticketB = wasm.bulk_ticket();
    expect(emptyA).toBe(emptyB);
    expect(ticketA).not.toBe(ticketB);
    expect(wasm.commit_frame(ticketA, 0, 1, 1, 1, 1)).toBe(0);
    expect(wasm.bulk_ticket()).toBe(ticketB);
    expect(wasm.commit_frame(ticketB, 0, 1, 1, 1, 1)).toBe(1);

    const first = wasm.alloc(4);
    const firstTicket = wasm.bulk_ticket();
    fill(wasm.memory, first, new Uint8Array([9, 8, 7, 6]));
    expect(wasm.commit_frame(firstTicket, 4, 1, 1, 1, 1)).toBe(1);
    const second = wasm.alloc(4);
    const secondTicket = wasm.bulk_ticket();
    expect(second).toBe(first);
    expect(secondTicket).not.toBe(firstTicket);
    fill(wasm.memory, second, new Uint8Array([4, 3, 2, 1]));
    expect(wasm.commit_frame(firstTicket, 4, 1, 1, 1, 1)).toBe(0);
    expect(read(wasm.memory, second, 4)).toEqual([4, 3, 2, 1]);
    expect(wasm.commit_frame(secondTicket, 4, 1, 1, 1, 1)).toBe(1);

    const accepted = new Uint8Array([11, 12, 13, 14]);
    writeFrame({ memory: wasm.memory }, sessionBulkSink(wasm), accepted, META);
    expect(wasm.bulk_ticket()).toBe(0);

    const grownBefore = wasm.memory.buffer.byteLength;
    const grownLen = Math.min(grownBefore, 1_048_576);
    writeFrame(
      { memory: wasm.memory },
      sessionBulkSink(wasm),
      new Uint8Array(grownLen),
      META,
    );
    expect(wasm.memory.buffer.byteLength).toBeGreaterThan(grownBefore);
    writeFrame({ memory: wasm.memory }, sessionBulkSink(wasm), accepted, META);

    const inclusive = wasm.alloc(MAX_BULK_BYTES);
    expect(inclusive).not.toBe(0);
    expect(wasm.commit_frame(wasm.bulk_ticket(), MAX_BULK_BYTES, 1, 1, 1, 1)).toBe(1);
  });

  it("keeps a copied state and ring readable across a later memory growth", async () => {
    const wasm = await loadModule();
    writeFrame(
      { memory: wasm.memory },
      sessionBulkSink(wasm),
      new Uint8Array(new Float32Array([40]).buffer),
      META,
    );
    const state = wasm.copy_state();
    const stateBefore = Array.from(state);
    const ring = wasm.drain_events();
    const ringBefore = Array.from(ring);
    const memoryBefore = wasm.memory.buffer;
    const grownBefore = memoryBefore.byteLength;
    const grownLen = Math.min(grownBefore, 1_048_576);
    writeFrame(
      { memory: wasm.memory },
      sessionBulkSink(wasm),
      new Uint8Array(grownLen),
      META,
    );
    expect(wasm.memory.buffer.byteLength).toBeGreaterThan(grownBefore);
    // A view over wasm memory would carry the pre-growth buffer and detach on
    // growth. A copy carries its own buffer, so it is neither the pre-growth
    // buffer nor the post-growth one.
    expect(state.buffer).not.toBe(memoryBefore);
    expect(state.buffer).not.toBe(wasm.memory.buffer);
    expect(state.byteLength).toBe(96);
    expect(Array.from(state)).toEqual(stateBefore);
    expect(ring.buffer).not.toBe(memoryBefore);
    expect(ring.buffer).not.toBe(wasm.memory.buffer);
    expect(ring.byteLength).toBeGreaterThanOrEqual(16);
    expect(Array.from(ring)).toEqual(ringBefore);
  });

  it("applies a command through the public sink and rejects a stale ticket", async () => {
    const wasm = await loadModule();
    const packed = packCommands(1, [{ kind: "Zoom", factor: 2 }]);
    let writes = 0;
    const applied = submitCommands(
      {
        write(ptr, bytes) {
          writes += 1;
          new Uint8Array(wasm.memory.buffer, ptr, bytes.byteLength).set(bytes);
        },
      },
      sessionCommandSink(wasm),
      packed,
    );
    expect(writes).toBe(1);
    expect(applied).toBe(1);

    const held = wasm.alloc(packed.byteLength);
    const heldTicket = wasm.bulk_ticket();
    new Uint8Array(wasm.memory.buffer, held, packed.byteLength).set(packed);
    writes = 0;
    const refused = submitCommands(
      {
        write() {
          writes += 1;
        },
      },
      sessionCommandSink(wasm),
      packed,
    );
    expect(refused).toBe(0);
    expect(writes).toBe(0);
    expect(wasm.bulk_ticket()).toBe(heldTicket);
    expect(wasm.apply_commands(heldTicket, packed.byteLength)).toBe(1);

    const again = wasm.alloc(packed.byteLength);
    const againTicket = wasm.bulk_ticket();
    expect(again).toBe(held);
    expect(againTicket).not.toBe(heldTicket);
    new Uint8Array(wasm.memory.buffer, again, packed.byteLength).set(packed);
    expect(wasm.apply_commands(heldTicket, packed.byteLength)).toBe(0);
    expect(wasm.apply_commands(againTicket, packed.byteLength)).toBe(1);
  });

  it("keeps a reused address from authorizing the newest public ticket", async () => {
    const wasm = await loadModule();
    const sink = sessionBulkSink(wasm);
    const first = sink.alloc(4);
    const firstTicket = sink.allocationTicket?.() ?? 0;
    expect(first).not.toBe(0);
    expect(firstTicket).not.toBe(0);
    fill(wasm.memory, first, new Uint8Array([9, 8, 7, 6]));
    expect(sink.commit_frame(first, 4, META)).toBe(0);
    expect(wasm.bulk_ticket()).toBe(firstTicket);
    expect(sink.commitAllocated?.(firstTicket, 4, META)).toBe(1);
    expect(wasm.bulk_ticket()).toBe(0);

    const second = sink.alloc(4);
    const secondTicket = sink.allocationTicket?.() ?? 0;
    expect(second).toBe(first);
    expect(secondTicket).not.toBe(firstTicket);
    fill(wasm.memory, second, new Uint8Array([4, 3, 2, 1]));
    expect(sink.commit_frame(first, 4, META)).toBe(0);
    expect(sink.commitAllocated?.(firstTicket, 4, META)).toBe(0);
    expect(wasm.bulk_ticket()).toBe(secondTicket);
    expect(read(wasm.memory, second, 4)).toEqual([4, 3, 2, 1]);
    expect(sink.commitAllocated?.(secondTicket, 4, META)).toBe(1);

    const packed = packCommands(1, [{ kind: "Zoom", factor: 2 }]);
    const commands = sessionCommandSink(wasm);
    const held = commands.alloc(packed.byteLength);
    const heldTicket = commands.allocationTicket?.() ?? 0;
    fill(wasm.memory, held, packed);
    expect(commands.apply_commands(held, packed.byteLength)).toBe(0);
    expect(wasm.bulk_ticket()).toBe(heldTicket);
    expect(commands.applyAllocated?.(heldTicket, packed.byteLength)).toBe(1);

    const again = commands.alloc(packed.byteLength);
    const againTicket = commands.allocationTicket?.() ?? 0;
    expect(again).toBe(held);
    expect(againTicket).not.toBe(heldTicket);
    fill(wasm.memory, again, packed);
    expect(commands.apply_commands(again, packed.byteLength)).toBe(0);
    expect(commands.applyAllocated?.(heldTicket, packed.byteLength)).toBe(0);
    expect(wasm.bulk_ticket()).toBe(againTicket);
    expect(commands.applyAllocated?.(againTicket, packed.byteLength)).toBe(1);
  });

  it("recovers from bad metadata and a write failure on one live module", async () => {
    const wasm = await loadModule();
    const sink = sessionBulkSink(wasm);
    expect(() =>
      writeFrame({ memory: wasm.memory }, sink, new Uint8Array(4), {
        viewport: "bad",
        columns: 1,
        rows: 1,
        generation: 1,
      }),
    ).toThrow("viewport is not a u32");
    expect(wasm.bulk_ticket()).toBe(0);

    const held = sink.alloc(4);
    const heldTicket = sink.allocationTicket?.() ?? 0;
    expect(held).not.toBe(0);
    sink.releaseAllocated?.(0);
    sink.releaseAllocated?.(heldTicket + 1);
    expect(wasm.bulk_ticket()).toBe(heldTicket);
    expect(wasm.release_bulk(heldTicket + 1)).toBe(0);
    expect(wasm.bulk_ticket()).toBe(heldTicket);
    sink.releaseAllocated?.(heldTicket);
    expect(wasm.bulk_ticket()).toBe(0);

    writeFrame({ memory: wasm.memory }, sink, new Uint8Array([1, 2, 3, 4]), META);
    expect(wasm.bulk_ticket()).toBe(0);

    const buffer = new ArrayBuffer(4);
    new Uint8Array(buffer).set([5, 6, 7, 8]);
    acceptTransferredFrame(
      { memory: wasm.memory },
      sink,
      buffer,
      { series: 1, instance: 1, frame: 0, bytes: 4 },
      META,
    );
    expect(wasm.bulk_ticket()).toBe(0);

    const packed = packCommands(1, [{ kind: "SetInvert", invert: "inverse" }]);
    const commands = sessionCommandSink(wasm);
    expect(() =>
      submitCommands(
        {
          write() {
            throw new Error("write failed");
          },
        },
        commands,
        packed,
      ),
    ).toThrow("write failed");
    expect(wasm.bulk_ticket()).toBe(0);
    const applied = submitCommands(
      {
        write(ptr, bytes) {
          new Uint8Array(wasm.memory.buffer, ptr, bytes.byteLength).set(bytes);
        },
      },
      commands,
      packed,
    );
    expect(applied).toBe(1);
    expect(wasm.bulk_ticket()).toBe(0);
  });

  it("applies one command of every kind packed by packCommands", async () => {
    const wasm = await loadModule();
    const packed = packCommands(1, EVERY_KIND);
    expect(submit(wasm, packed)).toBe(EVERY_KIND.length);
  });

  it("reads the built state through readViewportState at the generated offsets", async () => {
    const wasm = await loadModule();
    const packed = packCommands(1, [
      { kind: "Fit", width: 800, height: 600, columns: 512, rows: 512 },
      { kind: "SetWindow", centre: 40, width: 400, function: "linear" },
    ]);
    expect(submit(wasm, packed)).toBe(2);
    const readback = readViewportState(wasm.copy_state());
    expect(readback).not.toBeNull();
    expect(readback?.voiCenter).toBe(40);
    expect(readback?.voiWidth).toBe(400);
    expect(readback?.sliceIndex).toBe(0);
    expect(readback?.numSlices).toBe(0);
    expect(readback?.flags).toBe(0);
    expect(readback?.camera[0]).toBeCloseTo(512 / 600, 5);
    expect(readback?.camera[12]).toBeCloseTo(-256 / 3, 4);
  });

  it("accepts every generated enum byte and refuses one outside the table", async () => {
    const wasm = await loadModule();
    for (const byte of Object.values(VOI_FUNCTION_BYTE)) {
      const bytes = commandBytes(COMMAND_ID.SetWindow, [
        ...f64(40),
        ...f64(400),
        byte,
      ]);
      expect(submit(wasm, bytes)).toBe(1);
    }
    for (const byte of Object.values(PRESET_BYTE)) {
      expect(submit(wasm, commandBytes(COMMAND_ID.ApplyPreset, [byte]))).toBe(1);
    }
    for (const byte of Object.values(INVERT_BYTE)) {
      expect(submit(wasm, commandBytes(COMMAND_ID.SetInvert, [byte]))).toBe(1);
    }
    for (const byte of Object.values(COLORMAP_BYTE)) {
      expect(submit(wasm, commandBytes(COMMAND_ID.SetColormap, [byte]))).toBe(1);
    }
    expect(
      submit(
        wasm,
        commandBytes(COMMAND_ID.SetWindow, [...f64(40), ...f64(400), 9]),
      ),
    ).toBe(0);
    expect(submit(wasm, commandBytes(COMMAND_ID.ApplyPreset, [9]))).toBe(0);
    expect(submit(wasm, commandBytes(COMMAND_ID.SetInvert, [2]))).toBe(0);
    expect(submit(wasm, commandBytes(COMMAND_ID.SetColormap, [2]))).toBe(0);
  });

  it("publishes a presented event and drains it at the generated stride", async () => {
    installCanvasStubs();
    const wasm = await loadModule();
    writeFrame(
      { memory: wasm.memory },
      sessionBulkSink(wasm),
      new Uint8Array(new Float32Array([40]).buffer),
      META,
    );
    expect(
      submit(wasm, packCommands(1, [{ kind: "SetInvert", invert: "inverse" }])),
    ).toBe(1);
    wasm.bindCpuCanvas(new StubCanvas(), 80, 40, 1);
    wasm.publishCpuCanvas();
    const ring = wasm.drain_events();
    expect(ring.byteLength).toBe(EVENT_HEADER_BYTES + EVENT_STRIDE);
    const drained = drainRing(ring);
    expect(drained.events).toHaveLength(1);
    expect(drained.dropped).toBe(0);
    const event = drained.events[0];
    expect(event?.kind).toBe(RING_EVENT_KIND.presented);
    expect(event?.viewport).toBe(1);
    expect(event?.seq).toBe(0n);
    const payload = event?.payload ?? new Uint8Array();
    const view = new DataView(
      payload.buffer,
      payload.byteOffset,
      payload.byteLength,
    );
    expect(view.getUint32(4, true)).toBe(80);
    expect(view.getUint32(8, true)).toBe(40);
    expect(view.getUint32(12, true)).toBe(FLAG_INVERT);
  });

  it("drains a full ring at the generated capacity and stride", async () => {
    installCanvasStubs();
    const wasm = await loadModule();
    writeFrame(
      { memory: wasm.memory },
      sessionBulkSink(wasm),
      new Uint8Array(new Float32Array([40]).buffer),
      META,
    );
    wasm.bindCpuCanvas(new StubCanvas(), 64, 64, 1);
    for (let index = 0; index < 300; index += 1) {
      wasm.publishCpuInteraction();
    }
    const ring = wasm.drain_events();
    expect(ring.byteLength).toBe(EVENT_HEADER_BYTES + 256 * EVENT_STRIDE);
    const drained = drainRing(ring);
    expect(drained.events).toHaveLength(256);
    expect(drained.dropped).toBe(44);
    expect(drained.events[255]?.seq).toBe(255n);
  });

  it("round-trips a snapshot through the public helpers", async () => {
    const wasm = await loadModule();
    expect(
      submit(
        wasm,
        packCommands(1, [
          { kind: "Fit", width: 800, height: 600, columns: 512, rows: 512 },
          { kind: "Pan", x: 12.5, y: -4 },
          { kind: "Zoom", factor: 2 },
          { kind: "SetWindow", centre: 40, width: 400, function: "linear" },
          { kind: "SetInvert", invert: "inverse" },
        ]),
      ),
    ).toBe(5);
    const saved = sessionSnapshot(wasm);
    expect(sessionSnapshotError(wasm)).toBe(0);
    expect(saved.byteLength).toBeGreaterThan(0);
    const parsed = parseSnapshot(saved);
    expect(parsed.ok).toBe(true);
    if (!parsed.ok) {
      return;
    }
    expect(parsed.snapshot.magic).toBe(SNAPSHOT_MAGIC);
    expect(parsed.snapshot.camera.panX).toBe(12.5);
    expect(parsed.snapshot.camera.zoom).toBe(2);
    expect(parsed.snapshot.presentation.evidence).toEqual({
      kind: "shape",
      shape: "inverse",
    });
    expect(new TextDecoder().decode(saved)).not.toContain("matrix");
    const before = wasm.copy_state();
    expect(
      submit(wasm, packCommands(1, [{ kind: "Pan", x: 80, y: 80 }])),
    ).toBe(1);
    expect(restoreSessionSnapshot(wasm, saved)).toBe(0);
    expect(sessionSnapshotError(wasm)).toBe(0);
    expect(Array.from(wasm.copy_state())).toEqual(Array.from(before));
    const refused = restoreSessionSnapshot(wasm, new TextEncoder().encode("{"));
    expect(refused).toBe(SNAPSHOT_ERROR.Json);
    expect(Array.from(wasm.copy_state())).toEqual(Array.from(before));
    const version = JSON.parse(new TextDecoder().decode(saved)) as {
      version: number;
    };
    version.version = 2;
    expect(
      restoreSessionSnapshot(wasm, new TextEncoder().encode(JSON.stringify(version))),
    ).toBe(SNAPSHOT_ERROR.UnknownVersion);
    expect(Array.from(wasm.copy_state())).toEqual(Array.from(before));
  });

  it("exports the cpu canvas blit and no pixel getter", async () => {
    const glue = await import(pathToFileURL(GLUE).href);
    const names = Object.keys(glue);
    expect(names).toContain("bindCpuCanvas");
    expect(names).toContain("publishCpuCanvas");
    expect(names).toContain("resizeCpuCanvas");
    expect(names).toContain("resolvedTier");
    for (const banned of ["cpu_frame", "stored_samples", "pixels", "getPixels", "readPixels"]) {
      expect(names).not.toContain(banned);
    }
  });

  it("reports the tier resolve kept as A, B or C", async () => {
    const glue = await import(pathToFileURL(GLUE).href);
    const bytes = await readFile(WASM);
    await glue.default({ module_or_path: bytes });
    const tier = await glue.resolvedTier();
    expect(["A", "B", "C"]).toContain(tier);
  });

  it("hashes a content id in the core, drops the slot, and answers the pinned FNV value", async () => {
    // F-231 AC12. The value is F-224 AC2's, recomputed by hand from the
    // FNV-1a definition with the length prefixes over "1.2.3",
    // "1.2.840.10008.1.2.1" and [0, 1, 2, 3]. A TypeScript re-implementation
    // of the hash, or a slot the export left outstanding, goes red here.
    const wasm = await loadModule();
    const id = contentId(wasm, "1.2.3", "1.2.840.10008.1.2.1", new Uint8Array([0, 1, 2, 3]));
    expect(id).toBe(0x49e8_b650_ff1c_d3f1n);
    expect(wasm.bulk_ticket()).toBe(0);
    // The slot is gone, so the next allocation is accepted.
    expect(wasm.alloc(4)).not.toBe(0);
    wasm.release_bulk(wasm.bulk_ticket());
    expect(wasm.bulk_ticket()).toBe(0);
  });

  it("caches, shows, reports and releases committed frames with bigint identities", async () => {
    // F-231 AC13, the built module over the public helpers. The series and
    // the contents carry bits above the 32-bit half, so a u64 argument
    // truncated to u32 at any export collides these with their low halves
    // and fails a step below.
    const wasm = await loadModule();
    const series = 0x1_0000_0005n;
    const first = 0x1_0000_0001n;
    const second = 0x1_0000_0002n;
    const third = 0x1_0000_0003n;
    const fourth = 0x1_0000_0004n;
    const contents = [first, second, third, fourth];
    const keys = [0, 1, 2, 3].map((instance) => ({ series, instance, frame: 0 }));
    const epoch = writeStack(wasm, keys);
    expect(epoch).toBeGreaterThan(0);
    setFrameBudget(wasm, 3_145_728);
    // The budget change is setup, not the observation.
    drainRing(wasm.drain_events());

    const sink = sessionBulkSink(wasm);
    const frame = new Uint8Array(1_048_576);
    for (let index = 0; index < 4; index += 1) {
      if (index > 0) {
        expect(
          writeCommands(wasm, sessionCommandSink(wasm), packCommands(1, [{ kind: "Scroll", step: 1 }])),
        ).toBe(1);
      }
      writeFrame(wasm, sink, frame, {
        viewport: 1,
        columns: 512,
        rows: 512,
        generation: 0,
        stackIndex: index,
        stackEpoch: epoch,
        content: contents[index],
      });
    }

    // AC2's order on the built module: pressure, pressure, evicted, with
    // the evicted payload naming the full-width series, index 0's key and
    // 1,048,576 bytes. The fourth commit is what evicted that entry.
    const drained = drainRing(wasm.drain_events());
    expect(drained.dropped).toBe(0);
    const named = drained.events.map((raw) => decodeNamedEvent(raw));
    expect(named.map((row) => row?.name)).toEqual([
      "cache.pressure",
      "cache.pressure",
      "cache.evicted",
    ]);
    const evicted = named[2]?.payload;
    if (evicted === undefined || !("instance" in evicted)) {
      throw new Error("the evicted payload was missing");
    }
    expect(evicted.series).toBe(series);
    expect(evicted.instance).toBe(0);
    expect(evicted.bytes).toBe(1_048_576n);

    // AC4: residency of a surviving entry's matching triple, and not of a
    // content that differs only above the 32-bit half, nor of another
    // extent. The queries keep the entry.
    const request = { index: 3, epoch, content: fourth, columns: 512, rows: 512 };
    expect(frameResident(wasm, request)).toBe(true);
    expect(frameResident(wasm, { ...request, content: 0x2_0000_0004n })).toBe(false);
    expect(frameResident(wasm, { ...request, columns: 511 })).toBe(false);
    expect(frameResident(wasm, { ...request, rows: 513 })).toBe(false);
    expect(frameResident(wasm, request)).toBe(true);

    // D1's reproduction on the built module. Index 1 is cached and the
    // selection is index 3. Scrolled back to slice 0, a show of index 1 is
    // refused and leaves the state the scroll left, loading flag included.
    // Before the fix it returned true, adopted index 1's samples and
    // cleared the loading flag while copy_state still reported slice 0.
    expect(
      writeCommands(wasm, sessionCommandSink(wasm), packCommands(1, [{ kind: "Scroll", step: -3 }])),
    ).toBe(1);
    expect(
      showResident(wasm, { index: 1, epoch, content: second, columns: 512, rows: 512 }),
    ).toBe(false);
    const refused = readViewportState(wasm.copy_state());
    expect(refused?.sliceIndex).toBe(0);
    expect(refused !== null && (refused.flags & FLAG_LOADING) !== 0).toBe(true);
    // The refusal is not a stale removal, so the entry is kept.
    expect(
      frameResident(wasm, { index: 1, epoch, content: second, columns: 512, rows: 512 }),
    ).toBe(true);

    // AC5: the selection is index 1 now. A matching show answers, a
    // mismatch is refused and removes.
    expect(
      writeCommands(wasm, sessionCommandSink(wasm), packCommands(1, [{ kind: "Scroll", step: 1 }])),
    ).toBe(1);
    expect(
      showResident(wasm, { index: 1, epoch, content: second, columns: 512, rows: 512 }),
    ).toBe(true);
    expect(
      showResident(wasm, { index: 1, epoch, content: 0x3_0000_0002n, columns: 512, rows: 512 }),
    ).toBe(false);
    expect(
      frameResident(wasm, { index: 1, epoch, content: second, columns: 512, rows: 512 }),
    ).toBe(false);

    // Put the removed entry back so the release below has three frames of
    // the series again, and drain the crossings along the way. The
    // selection is already index 1.
    writeFrame(wasm, sink, frame, {
      viewport: 1,
      columns: 512,
      rows: 512,
      generation: 0,
      stackIndex: 1,
      stackEpoch: epoch,
      content: second,
    });
    drainRing(wasm.drain_events());

    // AC7: the release returns the series' three frames and emits one
    // released event naming the full-width series, then a pressure reading.
    expect(releaseSeries(wasm, series)).toBe(3);
    const released = drainRing(wasm.drain_events());
    const releasedNamed = released.events.map((raw) => decodeNamedEvent(raw));
    expect(releasedNamed.map((row) => row?.name)).toEqual(["cache.released", "cache.pressure"]);
    const payload = releasedNamed[0]?.payload as CacheReleasedPayload | undefined;
    expect(payload?.series).toBe(series);
    expect(payload?.frames).toBe(3);
    expect(payload?.bytes).toBe(3_145_728n);
  });
});
