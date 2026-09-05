import { afterEach, describe, expect, it, vi } from "vitest";

import {
  SIMD128_PROBE_MODULE,
  moduleValidates,
  sharedMemoryAvailable,
  wasmSimd128Supported,
} from "./capabilities.js";

describe("wasmSimd128Supported", () => {
  /**
   * The committed module is the one `wat2wasm` produced, and it is asserted by
   * shape rather than only by length. A 29-byte blob that happened to validate
   * would pass a length check while proving nothing about SIMD.
   *
   * Byte 26 is the `0xFD` SIMD opcode prefix and byte 27 is `0x0F`, which is
   * `i8x16.splat`. Replacing the module with a valid non-SIMD one goes red
   * here.
   */
  it("carries the assembled module, with its SIMD opcode", () => {
    expect(SIMD128_PROBE_MODULE).toHaveLength(29);
    expect(SIMD128_PROBE_MODULE.slice(0, 4)).toEqual([0x00, 0x61, 0x73, 0x6d]);
    expect(SIMD128_PROBE_MODULE[26]).toBe(0xfd);
    expect(SIMD128_PROBE_MODULE[27]).toBe(0x0f);
  });

  /**
   * Every runtime this project's tests run on supports SIMD128, so the probe
   * says yes here. That on its own proves very little, which is what the next
   * test is for.
   */
  it("accepts the committed module", () => {
    expect(wasmSimd128Supported()).toBe(true);
  });

  /**
   * **The test that makes the previous one mean something.** Break the SIMD
   * opcode prefix and validation must fail. Without this, a probe that always
   * returned `true`, or a constant that was quietly replaced by a valid
   * non-SIMD module, would pass every other assertion here.
   *
   * Byte 26 and not byte 27, and that is not arbitrary: incrementing byte 27
   * turns `i8x16.splat` into `i16x8.splat`, which is still a valid SIMD
   * instruction, so the module still validates. Byte 26 is the prefix, and
   * there is no opcode `0xFE` in its place.
   */
  it("rejects the same module with one byte of its SIMD opcode broken", () => {
    const corrupted = [...SIMD128_PROBE_MODULE];
    corrupted[26] = 0xfe;
    expect(moduleValidates(corrupted)).toBe(false);
  });

  /** Anything that is not a module at all is refused rather than thrown. */
  it("refuses a truncated module without throwing", () => {
    expect(moduleValidates([0x00, 0x61])).toBe(false);
    expect(moduleValidates([])).toBe(false);
  });
});

describe("sharedMemoryAvailable", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  /**
   * Both conditions are read, and neither on its own is enough.
   * `SharedArrayBuffer` exists as a constructor in plenty of contexts that are
   * not cross-origin isolated, and a page that claims isolation without the
   * constructor has nothing to share.
   */
  it("needs both SharedArrayBuffer and cross-origin isolation", () => {
    vi.stubGlobal("SharedArrayBuffer", ArrayBuffer);
    vi.stubGlobal("crossOriginIsolated", true);
    expect(sharedMemoryAvailable()).toBe(true);

    vi.stubGlobal("crossOriginIsolated", false);
    expect(sharedMemoryAvailable()).toBe(false);

    vi.stubGlobal("crossOriginIsolated", true);
    vi.stubGlobal("SharedArrayBuffer", undefined);
    expect(sharedMemoryAvailable()).toBe(false);
  });

  /**
   * A host that reports neither is the ordinary case, and it is an answer
   * rather than a failure. Decision D5 means nothing branches on it either
   * way.
   */
  it("is false where nothing is isolated", () => {
    vi.stubGlobal("SharedArrayBuffer", undefined);
    vi.stubGlobal("crossOriginIsolated", undefined);
    expect(sharedMemoryAvailable()).toBe(false);
  });
});
