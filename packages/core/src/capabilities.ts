/**
 * Runtime capability probes that belong in the shell rather than in the core.
 *
 * The Rust side resolves the rendering tier
 * (`ocelli_render::probe::resolve`). These two answer questions that only the
 * host page can, and both are asked BEFORE a wasm artefact is fetched, which
 * is precisely why they cannot live in Rust.
 *
 * See `docs/lld/tier-resolution.md`.
 */

/**
 * A 29-byte WebAssembly module whose only function uses one SIMD instruction.
 *
 * Assembled with `wat2wasm` (wabt 1.0.36) from this source, never typed by
 * hand:
 *
 * ```wat
 * (module
 *   (func (result v128) i32.const 0 i8x16.splat))
 * ```
 *
 * Byte 26 is `0xFD`, the SIMD opcode prefix, and byte 27 is `0x0F`, which
 * makes the pair `i8x16.splat`. A runtime without SIMD128 rejects the module
 * at validation, which is what makes this a probe rather than a formality.
 *
 * **Why this exists at all.** Deviation D-07 promotes wasm SIMD128 from a
 * detection detail to a requirement, and a module compiled with `simd128` will
 * not instantiate on a runtime without it. So by the time Ocelli's Rust is
 * running, the answer is always yes and there is nothing left to ask. The
 * question has to be asked here, before the fetch, or it cannot be asked.
 */
export const SIMD128_PROBE_MODULE: readonly number[] = [
  0x00, 0x61, 0x73, 0x6d, // "\0asm"
  0x01, 0x00, 0x00, 0x00, // version 1
  0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7b, // type: () -> v128
  0x03, 0x02, 0x01, 0x00, // function: one, of type 0
  0x0a, 0x08, 0x01, 0x06, 0x00, // code: one body, six bytes
  0x41, 0x00, // i32.const 0
  0xfd, 0x0f, // i8x16.splat
  0x0b, // end
];

/**
 * Whether the host accepts a module as valid WebAssembly.
 *
 * Exported so a test can feed it a deliberately corrupted copy of
 * {@link SIMD128_PROBE_MODULE}. A probe that answers `true` for a module with
 * no SIMD instruction in it is not a probe, and the only way to show it is not
 * one is to break the instruction and watch the answer change.
 *
 * `WebAssembly.validate` throws on a value it cannot read as a buffer, so the
 * call is guarded. A throw means "not supported", never a broken page.
 */
export function moduleValidates(bytes: readonly number[]): boolean {
  try {
    return WebAssembly.validate(Uint8Array.from(bytes));
  } catch {
    return false;
  }
}

/**
 * Whether this host can run a WebAssembly module built with SIMD128.
 *
 * The honest runtime probe. Ocelli's own Rust cannot answer this, because a
 * module that needs SIMD128 fails to instantiate rather than reporting the
 * absence, so the shell has to ask before it fetches.
 */
export function wasmSimd128Supported(): boolean {
  return moduleValidates(SIMD128_PROBE_MODULE);
}

/**
 * Whether shared memory is usable in this document.
 *
 * **A diagnostic, and deliberately nothing more.** Decision D5 keeps Ocelli
 * single-threaded with one wasm instance per worker, and deviation D-07
 * restates that for a second reason: on a shared host, spending more cores per
 * session reduces sessions per host, which is that deployment model's whole
 * economics. So nothing branches on this. It exists because a support ticket
 * that says "the viewer is slow" is answered from the environment it ran in,
 * and because F-006's benchmark harness has to record the conditions its
 * numbers were taken under.
 *
 * **There is no threads field anywhere on the Rust side**, in `Caps`,
 * `TierSignals` or `TierEvidence`. The strongest way to hold a decision is to
 * leave nowhere to branch on it.
 *
 * Both conditions are needed. `SharedArrayBuffer` can exist as a constructor
 * while the document is not cross-origin isolated, in which case constructing
 * one is not useful, and the COOP/COEP headers that isolation needs are the
 * permanent build tax decision D5 declines to pay.
 */
export function sharedMemoryAvailable(): boolean {
  return (
    typeof SharedArrayBuffer !== "undefined" &&
    globalThis.crossOriginIsolated === true
  );
}
