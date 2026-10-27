/**
 * F-102 pump. Expectations come from HLD 5.1, 5.3 and 17.1, and from the
 * F-050 record, not from reading the pump and restating it.
 */
import { describe, expect, it } from "vitest";

import {
  COMMAND_PAYLOAD_BYTES,
  EVENT_HEADER_BYTES,
  EVENT_KIND_OFFSET,
  EVENT_RING_DROPPED_OFFSET,
  EVENT_RING_PENDING_OFFSET,
  EVENT_SEQ_OFFSET,
  EVENT_STRIDE,
  EVENT_VIEWPORT_OFFSET,
  OCELLI_EVENTS,
  RING_EVENT_KIND,
  STATE_CAMERA_OFFSET,
  STATE_FLAGS_OFFSET,
  STATE_NUM_SLICES_OFFSET,
  STATE_SLICE_INDEX_OFFSET,
  STATE_VOI_CENTER_OFFSET,
  STATE_VOI_WIDTH_OFFSET,
  VIEWPORT_STATE_BYTES,
  packCommands,
  parseSnapshot,
  viewportId,
} from "@ocelli/core";

import { createViewportPump } from "./pump.js";

/** HLD section 17.1. kind, viewport and payload length. */
const COMMAND_HEADER_BYTES = 8;
const VIEWPORT = viewportId(7);

const RECORD = {
  magic: "OCS1",
  version: 1,
  viewportId: 7,
  displayArea: {
    cssWidth: 800,
    cssHeight: 600,
    columns: 512,
    rows: 512,
    columnMillimetres: 1,
    rowMillimetres: 1,
    shiftColumn: 0,
    shiftRow: 0,
    fitted: true,
    anchorX: 400,
    anchorY: 300,
    fitScale: 1,
  },
  camera: {
    panX: 12.5,
    panY: -4,
    zoom: 2,
    radians: 0,
    flipHorizontal: false,
    flipVertical: false,
  },
  presentation: {
    modality: { kind: "rescale", slope: 1, intercept: -1024 },
    voi: {
      kind: "window",
      center: 40,
      width: 400,
      function: "linear",
      ymin: 0,
      ymax: 255,
    },
    photometric: "monochrome2",
    evidence: { kind: "shape", shape: "inverse" },
    colormap: "grey",
  },
};

function recordBytes(): Uint8Array {
  return new TextEncoder().encode(JSON.stringify(RECORD));
}

function stateBytes(slice: number, centre: number): Uint8Array {
  const bytes = new Uint8Array(VIEWPORT_STATE_BYTES);
  const view = new DataView(bytes.buffer);
  view.setFloat32(STATE_CAMERA_OFFSET, 1, true);
  view.setFloat32(STATE_VOI_CENTER_OFFSET, centre, true);
  view.setFloat32(STATE_VOI_WIDTH_OFFSET, 400, true);
  view.setUint32(STATE_SLICE_INDEX_OFFSET, slice, true);
  view.setUint32(STATE_NUM_SLICES_OFFSET, 8, true);
  view.setUint32(STATE_FLAGS_OFFSET, 0, true);
  return bytes;
}

function ringBytes(kinds: readonly number[], dropped: number): Uint8Array {
  const bytes = new Uint8Array(EVENT_HEADER_BYTES + kinds.length * EVENT_STRIDE);
  const view = new DataView(bytes.buffer);
  view.setUint32(EVENT_RING_PENDING_OFFSET, kinds.length, true);
  view.setUint32(EVENT_RING_DROPPED_OFFSET, dropped, true);
  kinds.forEach((kind, index) => {
    const at = EVENT_HEADER_BYTES + index * EVENT_STRIDE;
    view.setUint32(at + EVENT_KIND_OFFSET, kind, true);
    view.setUint32(at + EVENT_VIEWPORT_OFFSET, VIEWPORT, true);
    view.setBigUint64(at + EVENT_SEQ_OFFSET, BigInt(index + 1), true);
  });
  return bytes;
}

interface Applied {
  readonly ticket: number;
  readonly len: number;
  readonly returned: number;
}

function harness() {
  const memory = new WebAssembly.Memory({ initial: 1 });
  let pointer = 0;
  let ticket = 4;
  let appliedReturn = 1;
  let state = stateBytes(0, 40);
  let ring = ringBytes([], 0);
  let snapshot = recordBytes();
  let snapshotError = 0;
  let restoreCode = 0;
  let drains = 0;
  let copies = 0;
  let allocs = 0;
  let cpuFrames = 0;
  let presentedFrames = 0;
  let snapshotErrorReads = 0;
  const applied: Applied[] = [];
  const restored: Uint8Array[] = [];
  const session = {
    memory,
    alloc(len: number): number {
      void len;
      allocs += 1;
      pointer = 64;
      return pointer;
    },
    bulk_ticket(): number {
      ticket += 1;
      return ticket;
    },
    release_bulk(): number {
      return 0;
    },
    apply_commands(presented: number, len: number): number {
      applied.push({ ticket: presented, len, returned: appliedReturn });
      return appliedReturn;
    },
    drain_events(): Uint8Array {
      drains += 1;
      return ring;
    },
    copy_state(): Uint8Array {
      copies += 1;
      return state;
    },
    snapshot(): Uint8Array {
      return snapshot;
    },
    snapshot_error(): number {
      snapshotErrorReads += 1;
      return snapshotError;
    },
    restore_snapshot(bytes: Uint8Array): number {
      restored.push(bytes);
      return restoreCode;
    },
    cpu_frame(): void {
      cpuFrames += 1;
    },
    presented_frame(): void {
      presentedFrames += 1;
    },
  };
  return {
    session,
    memory,
    applied,
    restored,
    counts() {
      return {
        drains,
        copies,
        allocs,
        cpuFrames,
        presentedFrames,
        snapshotErrorReads,
      };
    },
    written(len: number): Uint8Array {
      return new Uint8Array(memory.buffer.slice(pointer, pointer + len));
    },
    setApplied(count: number): void {
      appliedReturn = count;
    },
    setState(next: Uint8Array): void {
      state = next;
    },
    setRing(next: Uint8Array): void {
      ring = next;
    },
    setSnapshot(next: Uint8Array, error: number): void {
      snapshot = next;
      snapshotError = error;
    },
    setRestore(code: number): void {
      restoreCode = code;
    },
  };
}

function scheduler() {
  let next = 0;
  let pending: FrameRequestCallback | null = null;
  let pendingHandle = 0;
  const cancelled: number[] = [];
  return {
    request(callback: FrameRequestCallback): number {
      next += 1;
      pending = callback;
      pendingHandle = next;
      return next;
    },
    cancel(handle: number): void {
      cancelled.push(handle);
      if (handle === pendingHandle) {
        pending = null;
      }
    },
    fire(): void {
      const callback = pending;
      pending = null;
      callback?.(0);
    },
    requests(): number {
      return next;
    },
    cancelled,
  };
}

function commandCount(bytes: Uint8Array): { count: number; payload: number } {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let offset = 0;
  let count = 0;
  let payload = 0;
  while (offset < bytes.byteLength) {
    const len = view.getUint32(offset + 4, true);
    payload += len;
    count += 1;
    offset += COMMAND_HEADER_BYTES + len;
  }
  return { count, payload };
}

describe("F-102 viewport pump", () => {
  it("batches_one_frame", () => {
    const box = harness();
    const clock = scheduler();
    const pump = createViewportPump(box.session, {
      viewport: VIEWPORT,
      requestFrame: clock.request,
      cancelFrame: clock.cancel,
    });
    const pans = Array.from({ length: 10 }, (_, index) => ({
      kind: "Pan" as const,
      x: index,
      y: -index,
    }));
    for (const pan of pans) {
      pump.enqueue(pan);
    }
    expect(box.counts().allocs).toBe(0);
    box.setApplied(10);
    pump.flushNow();
    expect(box.applied).toHaveLength(1);
    expect(box.applied[0]?.returned).toBe(10);
    const expected = packCommands(VIEWPORT, pans);
    expect(Array.from(box.written(expected.byteLength))).toEqual(Array.from(expected));
    expect(box.counts().drains).toBe(1);
    expect(box.counts().copies).toBe(1);
  });

  it("drains_once_per_frame", () => {
    const box = harness();
    box.setRing(ringBytes([RING_EVENT_KIND.presented, RING_EVENT_KIND.evicted], 0));
    const pump = createViewportPump(box.session, { viewport: VIEWPORT });
    pump.enqueue({ kind: "Zoom", factor: 2 });
    pump.enqueue({ kind: "Zoom", factor: 3 });
    box.setApplied(2);
    pump.flushNow();
    expect(box.counts().drains).toBe(1);
    expect(box.counts().copies).toBe(1);
    expect(box.applied).toHaveLength(1);
  });

  it("surfaces_dropped", () => {
    const box = harness();
    box.setRing(ringBytes([RING_EVENT_KIND.presented, RING_EVENT_KIND.evicted], 4));
    const seen: string[] = [];
    let dropped = -1;
    const pump = createViewportPump(box.session, {
      viewport: VIEWPORT,
      onEvents(events, overflow) {
        dropped = overflow;
        for (const event of events) {
          seen.push(event.name);
        }
      },
    });
    pump.flushNow();
    expect(dropped).toBe(4);
    expect(box.applied).toHaveLength(0);
    expect(box.counts().allocs).toBe(0);
    expect(seen).toEqual(["frame.presented", "cache.evicted"]);
    for (const name of seen) {
      expect(OCELLI_EVENTS).toContain(name);
    }
  });

  it("state_only_on_change", () => {
    const box = harness();
    box.setState(stateBytes(2, 40));
    const centres: number[] = [];
    const pump = createViewportPump(box.session, {
      viewport: VIEWPORT,
      onState(state) {
        centres.push(state.voiCenter);
      },
    });
    pump.flushNow();
    pump.flushNow();
    box.setState(stateBytes(2, 80));
    pump.flushNow();
    expect(centres).toEqual([40, 80]);
  });

  it("stop_is_idempotent", () => {
    const box = harness();
    const clock = scheduler();
    const pump = createViewportPump(box.session, {
      viewport: VIEWPORT,
      requestFrame: clock.request,
      cancelFrame: clock.cancel,
    });
    pump.start();
    pump.start();
    expect(clock.requests()).toBe(1);
    clock.fire();
    expect(clock.requests()).toBe(2);
    expect(box.counts().drains).toBe(1);
    pump.stop();
    pump.stop();
    expect(clock.cancelled).toEqual([2]);
    clock.fire();
    expect(box.counts().drains).toBe(1);
  });

  it("restart_inside_the_frame_keeps_one_loop", () => {
    const box = harness();
    const clock = scheduler();
    const pump = createViewportPump(box.session, {
      viewport: VIEWPORT,
      requestFrame: clock.request,
      cancelFrame: clock.cancel,
      onEvents() {
        pump.stop();
        pump.start();
      },
    });
    pump.start();
    clock.fire();
    expect(clock.requests()).toBe(2);
    clock.fire();
    expect(box.counts().drains).toBe(2);
  });

  it("refusal_callback_does_not_reenter", () => {
    const box = harness();
    let calls = 0;
    const pump = createViewportPump(box.session, {
      viewport: VIEWPORT,
      onRefused() {
        calls += 1;
        pump.flushNow();
      },
    });
    pump.enqueue({ kind: "Pan", x: 1, y: 2 });
    box.setApplied(0);
    pump.flushNow();
    expect(calls).toBe(1);
    expect(box.applied).toHaveLength(1);
  });

  it("refusal_is_reported_once", () => {
    const box = harness();
    const refused: number[] = [];
    const pump = createViewportPump(box.session, {
      viewport: VIEWPORT,
      onRefused(commands) {
        refused.push(commands.length);
      },
    });
    pump.enqueue({ kind: "Pan", x: 1, y: 2 });
    box.setApplied(0);
    pump.flushNow();
    pump.flushNow();
    expect(refused).toEqual([1]);
    expect(box.applied).toHaveLength(1);
  });

  it("never_reads_pixels", () => {
    const box = harness();
    const pump = createViewportPump(box.session, { viewport: VIEWPORT });
    pump.enqueue({ kind: "Pan", x: 1, y: 1 });
    box.setApplied(1);
    pump.flushNow();
    pump.flushNow();
    expect(box.counts().cpuFrames).toBe(0);
    expect(box.counts().presentedFrames).toBe(0);
  });

  it("imports_the_catalogue", () => {
    const box = harness();
    box.setRing(ringBytes([
      RING_EVENT_KIND.evicted,
      RING_EVENT_KIND.displaced,
      RING_EVENT_KIND.refused,
      RING_EVENT_KIND.pressure,
      RING_EVENT_KIND.budget_changed,
      RING_EVENT_KIND.presented,
    ], 0));
    const names: string[] = [];
    const pump = createViewportPump(box.session, {
      viewport: VIEWPORT,
      onEvents(events) {
        for (const event of events) {
          names.push(event.name);
        }
      },
    });
    pump.flushNow();
    expect(names).toEqual([
      "cache.evicted",
      "cache.displaced",
      "cache.refused",
      "cache.pressure",
      "cache.budgetChanged",
      "frame.presented",
    ]);
    expect(new Set(names).size).toBe(names.length);
  });

  it("checkpoint_round_trip", () => {
    const box = harness();
    const saved = recordBytes();
    expect(parseSnapshot(saved).ok).toBe(true);
    box.setSnapshot(saved, 0);
    box.setState(stateBytes(3, 40));
    const pump = createViewportPump(box.session, { viewport: VIEWPORT });
    pump.flushNow();
    const checkpoint = pump.checkpoint();
    expect(checkpoint).not.toBeNull();
    expect(checkpoint?.sliceIndex).toBe(3);
    expect(Array.from(checkpoint?.snapshot ?? [])).toEqual(Array.from(saved));
    box.setApplied(1);
    box.setRestore(0);
    expect(pump.restore(checkpoint ?? { snapshot: saved, sliceIndex: 3 })).toBe(true);
    const scroll = packCommands(VIEWPORT, [{ kind: "Scroll", step: 3 }]);
    expect(Array.from(box.written(scroll.byteLength))).toEqual(Array.from(scroll));
    expect(box.restored).toHaveLength(1);
  });

  it("refused_checkpoint_is_null", () => {
    const box = harness();
    box.setSnapshot(new Uint8Array(), 2);
    const pump = createViewportPump(box.session, { viewport: VIEWPORT });
    expect(pump.checkpoint()).toBeNull();
    expect(box.counts().snapshotErrorReads).toBe(1);
    expect(box.session.snapshot_error()).toBe(2);
    box.setRestore(4);
    const before = box.applied.length;
    expect(pump.restore({ snapshot: recordBytes(), sliceIndex: 1 })).toBe(false);
    pump.flushNow();
    expect(box.applied.length).toBe(before);
  });

  it("packs_a_family_to_one_buffer", () => {
    for (const count of [1, 2, 4, 7]) {
      const box = harness();
      const commands = Array.from({ length: count }, (_, index) => ({
        kind: "Pan" as const,
        x: index + 0.5,
        y: -index,
      }));
      const pump = createViewportPump(box.session, { viewport: VIEWPORT });
      for (const command of commands) {
        pump.enqueue(command);
      }
      box.setApplied(count);
      pump.flushNow();
      const packed = packCommands(VIEWPORT, commands);
      expect(box.applied).toHaveLength(1);
      expect(Array.from(box.written(packed.byteLength))).toEqual(Array.from(packed));
      const walked = commandCount(packed);
      expect(walked.count).toBe(count);
      expect(walked.payload).toBe(count * COMMAND_PAYLOAD_BYTES.Pan);
      expect(packed.byteLength).toBe(walked.payload + count * COMMAND_HEADER_BYTES);
    }
  });
});
