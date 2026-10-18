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
 * Scaffold. F-101 (E16.2) implements the boundary this talks to.
 */

/** The subset of a wasm module's exports this file needs. */
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
  /** Hand ownership back. `ptr` must be the value returned by `alloc`. */
  commit_frame(ptr: number, len: number, meta: unknown): void;
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
  const ptr = session.alloc(bytes.byteLength);

  // Build the view AFTER the allocation. Use it immediately. Let it go.
  new Uint8Array(wasm.memory.buffer, ptr, bytes.byteLength).set(bytes);

  session.commit_frame(ptr, bytes.byteLength, meta);
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
): void {
  if (buffer.byteLength !== meta.bytes) {
    throw new Error(
      `handoff length ${buffer.byteLength} disagrees with meta.bytes ${meta.bytes}`,
    );
  }

  const len = buffer.byteLength;
  const ptr = session.alloc(len);

  // Build the view AFTER the allocation. Use it immediately. Let it go.
  new Uint8Array(wasm.memory.buffer, ptr, len).set(new Uint8Array(buffer));

  session.commit_frame(ptr, len, meta);
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
