/**
 * The bulk channel. HLD section 17.2.
 *
 * THIS IS ONE OF TWO FILES PERMITTED TO BUILD A VIEW OVER WASM LINEAR MEMORY.
 * `eslint.config.js` turns `no-restricted-syntax` off here and in `panic.ts`,
 * and in no other production file. It was the only one until F-005 landed
 * `panic.ts` during S03, which is why an earlier version of this header said
 * so. A separate block covers `packages/core/src/*.test.ts`, for a different
 * reason that block states. `ALLOWED_TO_DISABLE` in `eslint.config.js` is the
 * list, and a third production entry is a design-plan decision.
 *
 * The trap, in the HLD's words:
 *
 *   "A module-level `const HEAP = new Uint8Array(wasm.memory.buffer)` is the
 *    classic failure. Any wasm memory growth relocates the ArrayBuffer and
 *    detaches every outstanding view; the next write silently targets a
 *    detached buffer or throws far from the cause."
 *
 * So the order below is not a style preference, it is the whole contract:
 * allocate first, build the view after the allocation, use it immediately,
 * let it go. Never hoist the view, never store it on `this`, never return it.
 *
 * F-035 adds the section 24 handoff in this file, not in a third one.
 * `exportDecodedFrame` copies one range out into a fresh `ArrayBuffer`.
 * `acceptTransferredFrame` copies a transferred buffer back with the order
 * below. `transferredFrameMessage` builds the message and the transfer list
 * and does not call `postMessage`. A view is still built inside the call and
 * is never stored, returned or hoisted.
 *
 * F-101 (E16.2) built the boundary this talks to. The adapters are
 * `sessionBulkSink` and `sessionCommandSink` in `index.ts`.
 */

/** The subset of a wasm module's exports this file needs. */
import { submitCommands, type CommandSink } from "./commands.js";

export interface WasmMemory {
  readonly memory: WebAssembly.Memory;
}

/**
 * Identity and sample length for one section 24 frame transfer.
 *
 * `bytes` is the length `buffer.byteLength` must match. The other fields are
 * the frame identity and are not lengths.
 */
export interface HandoffMeta {
  readonly series: number;
  readonly instance: number;
  readonly frame: number;
  readonly bytes: number;
}

/** Section 24's decode-to-render message. The buffer is transferred, not copied. */
export interface FrameMessage {
  readonly kind: "frame";
  readonly frameId: number;
  readonly buffer: ArrayBuffer;
  readonly meta: HandoffMeta;
}

/** The subset of the Rust `Session` this file needs. */
export interface BulkSink {
  /** Reserve `len` bytes and return a pointer into linear memory. */
  alloc(len: number): number;
  /**
   * Hand ownership back. `ptr` must be the value returned by `alloc`.
   *
   * A numeric `0` is a refusal. `void` is a stand-in sink that has no code.
   * A session adapter refuses this call. The address is not a lifetime.
   */
  commit_frame(ptr: number, len: number, meta: unknown): number | void;
  /**
   * Ticket captured by the latest successful `alloc` on this adapter.
   * Absent on a stand-in sink that still commits by address.
   */
  allocationTicket?(): number;
  /**
   * Commit the slot whose ticket was captured at `alloc`. A stale ticket
   * returns 0 and leaves the current slot.
   */
  commitAllocated?(ticket: number, len: number, meta: unknown): number;
  /** Drop that same ticket. A foreign ticket does not drop the current slot. */
  releaseAllocated?(ticket: number): void;
}

/** The four integers the numeric `commit_frame` export stores. */
export interface FrameCommitMeta {
  readonly viewport: number;
  readonly columns: number;
  readonly rows: number;
  readonly generation: number;
  readonly stackIndex?: number;
  readonly stackEpoch?: number;
}

/**
 * The generated session exports. `commit_frame` takes a ticket and four
 * integers. It does not take the public metadata object.
 */
export interface SessionBulkExports extends WasmMemory {
  alloc(len: number): number;
  bulk_ticket(): number;
  release_bulk(ticket: number): number;
  commit_frame(
    ticket: number,
    len: number,
    viewport: number,
    columns: number,
    rows: number,
    generation: number,
  ): number;
  commit_selected_frame?(ticket: number, len: number, viewport: number, columns: number, rows: number, index: number, epoch: number): number;
}

/** An immutable frame identity in stored stack order. */
export interface StackFrameKey {
  readonly series: bigint;
  readonly instance: number;
  readonly frame: number;
}

/** Stack initialization uses the existing ticketed bulk channel. */
export interface SessionStackExports extends WasmMemory {
  alloc(len: number): number;
  bulk_ticket(): number;
  release_bulk(ticket: number): number;
  configure_stack(ticket: number, len: number): number;
}

/** Install nonempty ordered keys and return the new stack epoch. */
export function writeStack(wasm: SessionStackExports, keys: readonly StackFrameKey[]): number {
  if (keys.length < 1 || keys.length > 4194304) throw new RangeError("stack length is outside the bulk limit");
  const bytes = new Uint8Array(keys.length * 16);
  const view = new DataView(bytes.buffer);
  keys.forEach((key, i) => {
    if (key.series < 0n || key.series > 0xffff_ffff_ffff_ffffn) throw new RangeError("series is not a u64");
    view.setBigUint64(i * 16, key.series, true);
    view.setUint32(i * 16 + 8, u32(key.instance, "instance"), true);
    view.setUint32(i * 16 + 12, u32(key.frame, "frame"), true);
  });
  const ptr = wasm.alloc(bytes.length);
  if (!ptr) throw new Error("stack allocation refused");
  const ticket = wasm.bulk_ticket();
  try {
    new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
    const epoch = wasm.configure_stack(ticket, bytes.length);
    if (!epoch) throw new Error("stack configuration refused");
    return epoch;
  } catch (error) { wasm.release_bulk(ticket); throw error; }
}

/** The Rust response parser selected for one validated HTTP response. */
export type DicomwebResponseKind =
  | { readonly type: "qido-json" }
  | { readonly type: "wado-uri-part10" }
  | { readonly type: "wado-rs-instances"; readonly boundary: string }
  | {
      readonly type: "wado-rs-frames";
      readonly boundary: string;
      readonly mediaType: string;
    };

/** The future F-101 boundary subset needed to commit a DICOMweb response. */
export interface DicomwebResponseSink {
  /** Reserve `len` bytes and return a pointer into linear memory. */
  alloc(len: number): number;
  /** Hand ownership to the response parser selected by `kind`. */
  commit_dicomweb_response(
    ptr: number,
    len: number,
    kind: DicomwebResponseKind,
  ): void;
}

/**
 * Copy `bytes` into the core and hand ownership over.
 *
 * The view is constructed between `alloc` and `set` and is unreachable
 * afterwards, which is the only arrangement that is safe across a memory
 * growth. Do not refactor the view out of this function.
 */
export function writeFrame(
  wasm: WasmMemory,
  session: BulkSink,
  bytes: Uint8Array,
  meta: unknown,
): void {
  if (session.commitAllocated !== undefined) {
    frameCommitMeta(meta);
  }
  const ptr = session.alloc(bytes.byteLength);
  if (ptr === 0) {
    throw new Error("allocation refused");
  }
  const ticket = session.allocationTicket?.() ?? 0;

  try {
    // Build the view AFTER the allocation. Use it immediately. Let it go.
    new Uint8Array(wasm.memory.buffer, ptr, bytes.byteLength).set(bytes);
    const code = commitOwned(session, ptr, bytes.byteLength, meta, ticket);
    if (code === 0) {
      throw new Error("commit refused");
    }
  } catch (error) {
    session.releaseAllocated?.(ticket);
    throw error;
  }
}

/** Write one packed batch through the same allocation/ticket discipline as frames. */
export function writeCommands(
  wasm: WasmMemory,
  sink: CommandSink,
  bytes: Uint8Array,
): number {
  return submitCommands({
    write(ptr, data) {
      // Allocation happened inside submitCommands. Memory growth cannot leave
      // this immediate view pointing at the old allocation.
      new Uint8Array(wasm.memory.buffer, ptr, data.byteLength).set(data);
    },
  }, sink, bytes);
}

/**
 * Adapt the numeric session exports to {@link BulkSink}.
 *
 * `alloc` returns the write address and captures the lifetime ticket.
 * `commitAllocated` accepts only the ticket captured for that alloc.
 * `commit_frame` refuses, including when a later alloc reused the address.
 * A refused `alloc` leaves the previous ticket in place and writes nothing.
 */
export function sessionBulkSink(wasm: SessionBulkExports): BulkSink {
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
     * The address is not an authorization. Heap reuse must not select the
     * newest ticket, so this call always refuses and leaves the slot.
     */
    commit_frame(ptr: number, len: number, meta: unknown): number {
      void ptr;
      void len;
      void meta;
      return 0;
    },
    commitAllocated(presented: number, len: number, meta: unknown): number {
      if (presented === 0 || presented !== ticket) {
        return 0;
      }
      const fields = frameCommitMeta(meta);
      let code;
      if (fields.stackIndex !== undefined && fields.stackEpoch !== undefined) {
        if (!wasm.commit_selected_frame) throw new Error("selected-frame export is unavailable");
        code = wasm.commit_selected_frame(presented, len, fields.viewport, fields.columns, fields.rows, fields.stackIndex, fields.stackEpoch);
      } else code = wasm.commit_frame(
        presented,
        len,
        fields.viewport,
        fields.columns,
        fields.rows,
        fields.generation,
      );
      ticket = 0;
      return code;
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

/**
 * Copy one complete validated DICOMweb response into the future core sink.
 *
 * F-101 supplies the live Session method. Keeping this operation against a
 * narrow sink proves the allocation and one-write discipline without
 * inventing that command boundary early.
 */
export function writeDicomwebResponse(
  wasm: WasmMemory,
  sink: DicomwebResponseSink,
  bytes: Uint8Array,
  kind: DicomwebResponseKind,
): void {
  const ptr = sink.alloc(bytes.byteLength);
  if (ptr === 0) {
    throw new Error("allocation refused");
  }

  // Build the view AFTER the allocation. Use it immediately. Let it go.
  new Uint8Array(wasm.memory.buffer, ptr, bytes.byteLength).set(bytes);

  sink.commit_dicomweb_response(ptr, bytes.byteLength, kind);
}

/**
 * Copy `[ptr, ptr + len)` out of linear memory into a fresh `ArrayBuffer`.
 *
 * The view is built for this copy and then unreachable. The returned buffer
 * is the section 24 transfer, not a view over wasm memory.
 */
export function exportDecodedFrame(
  wasm: WasmMemory,
  ptr: number,
  len: number,
): ArrayBuffer {
  const copy = new ArrayBuffer(len);

  // Build the view at this read. Use it immediately. Let it go.
  new Uint8Array(copy).set(new Uint8Array(wasm.memory.buffer, ptr, len));

  return copy;
}

/**
 * Copy `buffer` into the core and hand ownership over.
 *
 * A length that disagrees with `meta.bytes` is refused before `alloc`, so a
 * mismatch does not grow linear memory and does not copy. The view is built
 * after `alloc` and is unreachable after `set`.
 */
export function acceptTransferredFrame(
  wasm: WasmMemory,
  session: BulkSink,
  buffer: ArrayBuffer,
  meta: HandoffMeta,
  target: FrameCommitMeta,
): void {
  if (buffer.byteLength !== meta.bytes) {
    throw new Error(
      `handoff length ${buffer.byteLength} disagrees with meta.bytes ${meta.bytes}`,
    );
  }
  const fields = frameCommitMeta(target);
  const committed = {
    handoff: meta,
    viewport: fields.viewport,
    columns: fields.columns,
    rows: fields.rows,
    generation: fields.generation,
  };

  const len = buffer.byteLength;
  const ptr = session.alloc(len);
  if (ptr === 0) {
    throw new Error("allocation refused");
  }
  const ticket = session.allocationTicket?.() ?? 0;

  try {
    // Build the view AFTER the allocation. Use it immediately. Let it go.
    new Uint8Array(wasm.memory.buffer, ptr, len).set(new Uint8Array(buffer));
    const code = commitOwned(session, ptr, len, committed, ticket);
    if (code === 0) {
      throw new Error("commit refused");
    }
  } catch (error) {
    session.releaseAllocated?.(ticket);
    throw error;
  }
}

function commitOwned(
  session: BulkSink,
  ptr: number,
  len: number,
  meta: unknown,
  ticket: number,
): number | void {
  if (session.commitAllocated !== undefined) {
    return session.commitAllocated(ticket, len, meta);
  }
  return session.commit_frame(ptr, len, meta);
}

function frameCommitMeta(meta: unknown): FrameCommitMeta {
  if (typeof meta !== "object" || meta === null) {
    throw new TypeError("frame meta is not an object");
  }
  const record = meta as {
    viewport?: unknown;
    columns?: unknown;
    rows?: unknown;
    generation?: unknown;
    stackIndex?: unknown;
    stackEpoch?: unknown;
  };
  const selection = record.stackIndex !== undefined || record.stackEpoch !== undefined;
  return {
    viewport: u32(record.viewport, "viewport"),
    columns: u32(record.columns, "columns"),
    rows: u32(record.rows, "rows"),
    generation: u32(record.generation, "generation"),
    ...(selection ? { stackIndex: u32(record.stackIndex, "stackIndex"), stackEpoch: u32(record.stackEpoch, "stackEpoch") } : {}),
  };
}

function u32(value: unknown, name: string): number {
  if (
    typeof value !== "number" ||
    !Number.isInteger(value) ||
    value < 0 ||
    value > 0xffff_ffff
  ) {
    throw new TypeError(`${name} is not a u32`);
  }
  return value;
}

/**
 * Section 24's decode-to-render message and the list `postMessage` transfers.
 *
 * The buffer in the message and the buffer in the list are the same object.
 * This function does not call `postMessage` and does not copy `buffer`.
 */
export function transferredFrameMessage(
  frameId: number,
  buffer: ArrayBuffer,
  meta: HandoffMeta,
): { readonly message: FrameMessage; readonly transfer: ArrayBuffer[] } {
  return {
    message: { kind: "frame", frameId, buffer, meta },
    transfer: [buffer],
  };
}
