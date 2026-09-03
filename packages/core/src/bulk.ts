/**
 * The bulk channel. HLD section 17.2.
 *
 * THIS IS THE ONLY FILE PERMITTED TO BUILD A VIEW OVER WASM LINEAR MEMORY.
 * `eslint.config.js` turns `no-restricted-syntax` off here and nowhere else.
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
 * Scaffold. F-096 (E16.2) implements the boundary this talks to.
 */

/** The subset of a wasm module's exports this file needs. */
export interface WasmMemory {
  readonly memory: WebAssembly.Memory;
}

/** The subset of the Rust `Session` this file needs. */
export interface BulkSink {
  /** Reserve `len` bytes and return a pointer into linear memory. */
  alloc(len: number): number;
  /** Hand ownership back. `ptr` must be the value returned by `alloc`. */
  commit_frame(ptr: number, len: number, meta: unknown): void;
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
