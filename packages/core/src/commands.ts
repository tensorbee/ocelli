/**
 * Packed stack commands. HLD section 17.1, ids from the F-101 contract.
 *
 * Kind 0 is not a command. Ids follow `STACK_COMMANDS` and start at 1.
 * Payloads are little-endian. This file writes bytes. It does not keep a
 * camera and it does not view wasm memory.
 */

import {
  COLORMAP_BYTE,
  COMMAND_ID,
  COMMAND_PAYLOAD_BYTES,
  INVERT_BYTE,
  PRESET_BYTE,
  VOI_FUNCTION_BYTE,
} from "./generated.js";
import type { Colormap, Invert, VoiFunction, WindowPreset } from "./viewport.js";

export { COMMAND_ID };

/** One packed command. Names match `STACK_COMMANDS`. */
export type PackedCommand =
  | {
      readonly kind: "Fit";
      readonly width: number;
      readonly height: number;
      readonly columns: number;
      readonly rows: number;
    }
  | { readonly kind: "Pan"; readonly x: number; readonly y: number }
  | { readonly kind: "Zoom"; readonly factor: number }
  | { readonly kind: "Rotate"; readonly radians: number }
  | { readonly kind: "FlipHorizontal"; readonly enabled: boolean }
  | { readonly kind: "FlipVertical"; readonly enabled: boolean }
  | {
      readonly kind: "SetWindow";
      readonly centre: number;
      readonly width: number;
      readonly function: VoiFunction;
    }
  | { readonly kind: "ApplyPreset"; readonly preset: WindowPreset }
  | { readonly kind: "SetInvert"; readonly invert: Invert }
  | { readonly kind: "SetColormap"; readonly colormap: Colormap }
  | { readonly kind: "Scroll"; readonly step: number };

/** A sink that accepts one command buffer per call. */
export interface CommandSink {
  alloc(len: number): number;
  apply_commands(ptr: number, len: number): number;
  /** Ticket captured by the latest successful `alloc` on this adapter. */
  allocationTicket?(): number;
  /** Apply the slot whose ticket was captured at `alloc`. */
  applyAllocated?(ticket: number, len: number): number;
  /** Drop that same ticket. A foreign ticket does not drop the current slot. */
  releaseAllocated?(ticket: number): void;
}

/** The generated command exports. `apply_commands` takes a ticket. */
export interface SessionCommandExports {
  alloc(len: number): number;
  bulk_ticket(): number;
  release_bulk(ticket: number): number;
  apply_commands(ticket: number, len: number): number;
}

/** Writes command bytes after `alloc` and before `apply_commands`. */
export interface CommandWrite {
  write(ptr: number, bytes: Uint8Array): void;
}

/**
 * Pack `commands` for `viewportId`.
 *
 * An empty list is an empty buffer, which the core accepts as zero commands.
 */
export function packCommands(
  viewportId: number,
  commands: readonly PackedCommand[],
): Uint8Array {
  u16(viewportId, "viewport");
  const out: number[] = [];
  for (const command of commands) {
    append(out, viewportId, command);
  }
  return Uint8Array.from(out);
}

/**
 * Pack one pan per pointer sample into one buffer.
 *
 * Several samples are one batch. They are not one call each.
 */
export function packPointerFrame(
  viewportId: number,
  samples: readonly { readonly x: number; readonly y: number }[],
): Uint8Array {
  return packCommands(
    viewportId,
    samples.map((sample) => ({ kind: "Pan", x: sample.x, y: sample.y })),
  );
}

/**
 * Allocate, write, then apply once.
 *
 * The write callback runs after `alloc`. This function does not build a view
 * over wasm memory itself.
 */
export function submitCommands(
  write: CommandWrite,
  sink: CommandSink,
  bytes: Uint8Array,
): number {
  const ptr = sink.alloc(bytes.byteLength);
  if (ptr === 0) {
    return 0;
  }
  const ticket = sink.allocationTicket?.() ?? 0;
  try {
    write.write(ptr, bytes);
    if (sink.applyAllocated !== undefined) {
      return sink.applyAllocated(ticket, bytes.byteLength);
    }
    return sink.apply_commands(ptr, bytes.byteLength);
  } catch (error) {
    sink.releaseAllocated?.(ticket);
    throw error;
  }
}

/**
 * Adapt the numeric session exports to {@link CommandSink}.
 *
 * `alloc` returns the write address and captures the lifetime ticket.
 * `applyAllocated` accepts only that ticket. `apply_commands` refuses.
 * A refused `alloc` does not replace the outstanding ticket.
 */
export function sessionCommandSink(wasm: SessionCommandExports): CommandSink {
  let ticket = 0;
  return {
    alloc(len: number): number {
      const ptr = wasm.alloc(len);
      if (ptr === 0) {
        return 0;
      }
      ticket = wasm.bulk_ticket();
      return ptr;
    },
    allocationTicket(): number {
      return ticket;
    },
    /**
     * The address is not an authorization. This call always refuses and
     * leaves the current slot in place.
     */
    apply_commands(ptr: number, len: number): number {
      void ptr;
      void len;
      return 0;
    },
    applyAllocated(presented: number, len: number): number {
      if (presented === 0 || presented !== ticket) {
        return 0;
      }
      const applied = wasm.apply_commands(presented, len);
      ticket = 0;
      return applied;
    },
    releaseAllocated(presented: number): void {
      if (presented === 0 || presented !== ticket) {
        return;
      }
      wasm.release_bulk(presented);
      ticket = 0;
    },
  };
}

function append(out: number[], viewportId: number, command: PackedCommand): void {
  const payload = payloadOf(command);
  const expected = COMMAND_PAYLOAD_BYTES[command.kind];
  if (payload.length !== expected) {
    throw new RangeError(
      `${command.kind} payload is ${payload.length} bytes and the Rust table declares ${expected}`,
    );
  }
  pushU16(out, COMMAND_ID[command.kind]);
  pushU16(out, u16(viewportId, "viewport"));
  pushU32(out, expected);
  out.push(...payload);
}

function payloadOf(command: PackedCommand): number[] {
  switch (command.kind) {
    case "Fit":
      return [
        ...f64(command.width),
        ...f64(command.height),
        ...u32(command.columns, "columns"),
        ...u32(command.rows, "rows"),
      ];
    case "Pan":
      return [...f64(command.x), ...f64(command.y)];
    case "Zoom":
      return f64(command.factor);
    case "Rotate":
      return f64(command.radians);
    case "FlipHorizontal":
    case "FlipVertical":
      return [command.enabled ? 1 : 0];
    case "SetWindow":
      return [
        ...f64(command.centre),
        ...f64(command.width),
        VOI_FUNCTION_BYTE[command.function],
      ];
    case "ApplyPreset":
      return [PRESET_BYTE[command.preset]];
    case "SetInvert":
      return [INVERT_BYTE[command.invert]];
    case "SetColormap":
      return [COLORMAP_BYTE[command.colormap]];
    case "Scroll":
      return i32(command.step);
    default: {
      const never: never = command;
      return never;
    }
  }
}

function u16(value: number, name: string): number {
  if (!Number.isInteger(value) || value < 0 || value > 0xffff) {
    throw new RangeError(`${name} ${value} is not a u16`);
  }
  return value;
}

function pushU16(out: number[], value: number): void {
  out.push(value & 0xff, (value >> 8) & 0xff);
}

function pushU32(out: number[], value: number): void {
  out.push(
    value & 0xff,
    (value >> 8) & 0xff,
    (value >> 16) & 0xff,
    (value >> 24) & 0xff,
  );
}

function u32(value: number, name: string): number[] {
  if (!Number.isInteger(value) || value < 0 || value > 0xffff_ffff) {
    throw new RangeError(`${name} ${value} is not a u32`);
  }
  return [value & 0xff, (value >> 8) & 0xff, (value >> 16) & 0xff, (value >> 24) & 0xff];
}

function i32(value: number): number[] {
  if (!Number.isInteger(value) || value < -0x8000_0000 || value > 0x7fff_ffff) {
    throw new RangeError(`step ${value} is not an i32`);
  }
  const bits = value < 0 ? value + 0x1_0000_0000 : value;
  return [bits & 0xff, (bits >> 8) & 0xff, (bits >> 16) & 0xff, (bits >> 24) & 0xff];
}

function f64(value: number): number[] {
  const buffer = new ArrayBuffer(8);
  new DataView(buffer).setFloat64(0, value, true);
  return [...new Uint8Array(buffer)];
}
