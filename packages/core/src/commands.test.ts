import { describe, expect, it } from "vitest";

import {
  COMMAND_ID,
  packCommands,
  packPointerFrame,
  submitCommands,
} from "./commands.js";
import type { PackedCommand } from "./commands.js";
import {
  COLORMAP_BYTE,
  COMMAND_PAYLOAD_BYTES,
  INVERT_BYTE,
  PRESET_BYTE,
  VOI_FUNCTION_BYTE,
} from "./generated.js";
import { drainRing } from "./ring.js";
import {
  COLORMAPS,
  INVERT,
  STACK_COMMANDS,
  VOI_FUNCTIONS,
  WINDOW_PRESETS,
} from "./viewport.js";

const F64_8 = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x40];
const F64_3 = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x40];
const F64_NEG_4 = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0xc0];
const F64_2 = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40];
const F64_1 = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf0, 0x3f];
const F64_40 = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x44, 0x40];
const F64_400 = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x79, 0x40];

const SAMPLES: readonly PackedCommand[] = [
  { kind: "Fit", width: 8, height: 8, columns: 2, rows: 4 },
  { kind: "Pan", x: 3, y: -4 },
  { kind: "Zoom", factor: 2 },
  { kind: "Rotate", radians: 1 },
  { kind: "FlipHorizontal", enabled: true },
  { kind: "FlipVertical", enabled: false },
  { kind: "SetWindow", centre: 40, width: 400, function: "linear_exact" },
  { kind: "ApplyPreset", preset: "lung" },
  { kind: "SetInvert", invert: "inverse" },
  { kind: "SetColormap", colormap: "stops" },
  { kind: "Scroll", step: -1 },
];

describe("packed commands", () => {
  it("numbers the eleven names from 1", () => {
    expect(STACK_COMMANDS.map((name) => COMMAND_ID[name])).toEqual([
      1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
    ]);
  });

  it("declares the generated payload length for every kind", () => {
    for (const command of SAMPLES) {
      const packed = packCommands(7, [command]);
      const view = new DataView(packed.buffer);
      expect(view.getUint32(4, true)).toBe(COMMAND_PAYLOAD_BYTES[command.kind]);
      expect(packed.byteLength).toBe(8 + COMMAND_PAYLOAD_BYTES[command.kind]);
    }
  });

  it("agrees with the generated enum bytes and the public names", () => {
    expect(Object.keys(VOI_FUNCTION_BYTE)).toEqual([...VOI_FUNCTIONS]);
    expect(Object.keys(PRESET_BYTE)).toEqual([...WINDOW_PRESETS]);
    expect(Object.keys(COLORMAP_BYTE)).toEqual([...COLORMAPS]);
    expect(Object.keys(INVERT_BYTE)).toEqual([...INVERT]);
    expect(Object.values(VOI_FUNCTION_BYTE)).toEqual([0, 1, 2]);
    expect(Object.values(PRESET_BYTE)).toEqual([0, 1, 2]);
    expect(Object.values(COLORMAP_BYTE)).toEqual([0, 1]);
    expect(Object.values(INVERT_BYTE)).toEqual([0, 1]);
  });

  it("writes every id in little-endian against hand-built bytes", () => {
    const packed = packCommands(7, [
      { kind: "Fit", width: 8, height: 8, columns: 2, rows: 4 },
      { kind: "Pan", x: 3, y: -4 },
      { kind: "Zoom", factor: 2 },
      { kind: "Rotate", radians: 1 },
      { kind: "FlipHorizontal", enabled: true },
      { kind: "FlipVertical", enabled: false },
      { kind: "SetWindow", centre: 40, width: 400, function: "linear_exact" },
      { kind: "ApplyPreset", preset: "lung" },
      { kind: "SetInvert", invert: "inverse" },
      { kind: "SetColormap", colormap: "stops" },
      { kind: "Scroll", step: -1 },
    ]);
    expect(Array.from(packed)).toEqual([
      1, 0, 7, 0, 24, 0, 0, 0,
      ...F64_8,
      ...F64_8,
      2, 0, 0, 0,
      4, 0, 0, 0,
      2, 0, 7, 0, 16, 0, 0, 0,
      ...F64_3,
      ...F64_NEG_4,
      3, 0, 7, 0, 8, 0, 0, 0,
      ...F64_2,
      4, 0, 7, 0, 8, 0, 0, 0,
      ...F64_1,
      5, 0, 7, 0, 1, 0, 0, 0, 1,
      6, 0, 7, 0, 1, 0, 0, 0, 0,
      7, 0, 7, 0, 17, 0, 0, 0,
      ...F64_40,
      ...F64_400,
      1,
      8, 0, 7, 0, 1, 0, 0, 0, 1,
      9, 0, 7, 0, 1, 0, 0, 0, 1,
      10, 0, 7, 0, 1, 0, 0, 0, 1,
      11, 0, 7, 0, 4, 0, 0, 0, 0xff, 0xff, 0xff, 0xff,
    ]);
  });

  it("packs every pointer sample into one buffer and submits it once", () => {
    const packed = packPointerFrame(7, [
      { x: 3, y: -4 },
      { x: 3, y: -4 },
    ]);
    expect(packed.byteLength).toBe(2 * (8 + 16));
    const calls: number[] = [];
    let allocated = false;
    const result = submitCommands(
      {
        write(ptr, bytes) {
          expect(allocated).toBe(true);
          expect(ptr).toBe(100);
          expect(Array.from(bytes)).toEqual(Array.from(packed));
        },
      },
      {
        alloc(len) {
          allocated = true;
          expect(len).toBe(packed.byteLength);
          return 100;
        },
        apply_commands(ptr, len) {
          calls.push(ptr, len);
          return 2;
        },
      },
      packed,
    );
    expect(calls).toEqual([100, packed.byteLength]);
    expect(result).toBe(2);
  });

  it("releases the captured ticket when the write throws", () => {
    const released: number[] = [];
    let applied = 0;
    expect(() =>
      submitCommands(
        {
          write() {
            throw new Error("write failed");
          },
        },
        {
          alloc() {
            return 40;
          },
          allocationTicket() {
            return 9;
          },
          apply_commands() {
            applied += 1;
            return 1;
          },
          applyAllocated() {
            applied += 1;
            return 1;
          },
          releaseAllocated(ticket) {
            released.push(ticket);
          },
        },
        packCommands(7, [{ kind: "Zoom", factor: 2 }]),
      ),
    ).toThrow("write failed");
    expect(released).toEqual([9]);
    expect(applied).toBe(0);
  });

  it("does not write or apply when alloc returns 0", () => {
    let writes = 0;
    let applies = 0;
    const result = submitCommands(
      {
        write() {
          writes += 1;
        },
      },
      {
        alloc() {
          return 0;
        },
        apply_commands() {
          applies += 1;
          return 1;
        },
      },
      packCommands(7, [{ kind: "Zoom", factor: 2 }]),
    );
    expect(writes).toBe(0);
    expect(applies).toBe(0);
    expect(result).toBe(0);
  });

  it("refuses a viewport id that does not fit in u16", () => {
    expect(() => packCommands(70_000, [])).toThrow(RangeError);
    expect(() =>
      packCommands(70_000, [{ kind: "Scroll", step: 1 }]),
    ).toThrow(RangeError);
  });
});

describe("drainRing", () => {
  it("reads kind 6 from a copied buffer and ignores a short one", () => {
    const bytes = new Uint8Array(16 + 48);
    const view = new DataView(bytes.buffer);
    view.setUint32(0, 1, true);
    view.setUint32(8, 256, true);
    view.setUint32(16, 6, true);
    view.setUint32(20, 7, true);
    view.setBigUint64(24, 4n, true);
    view.setUint32(32, 9, true);
    view.setUint32(36, 20, true);
    view.setUint32(40, 10, true);
    view.setUint32(44, 8, true);
    const drained = drainRing(bytes);
    expect(drained.dropped).toBe(0);
    expect(drained.events).toHaveLength(1);
    expect(drained.events[0]?.kind).toBe(6);
    expect(drained.events[0]?.viewport).toBe(7);
    expect(drained.events[0]?.seq).toBe(4n);
    const payload = drained.events[0]?.payload ?? new Uint8Array();
    const words = new DataView(payload.buffer, payload.byteOffset, payload.byteLength);
    expect(words.getUint32(0, true)).toBe(9);
    expect(words.getUint32(4, true)).toBe(20);
    expect(words.getUint32(8, true)).toBe(10);
    expect(words.getUint32(12, true)).toBe(8);

    const short = bytes.slice(0, 20);
    const missed = new DataView(short.buffer);
    missed.setUint32(0, 1, true);
    expect(drainRing(short).events).toEqual([]);
  });
});
