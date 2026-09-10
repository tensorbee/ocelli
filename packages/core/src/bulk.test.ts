import { describe as group, expect, it } from "vitest";

import {
  writeDicomwebResponse,
  writeFrame,
  type BulkSink,
  type DicomwebResponseKind,
  type DicomwebResponseSink,
  type WasmMemory,
} from "./bulk.js";

/**
 * HLD section 17.2, the bulk channel and its trap.
 *
 * The specification states the order and states the failure:
 *
 *   "// packages/core/src/bulk.ts -- the ONLY correct order
 *    const ptr = session.alloc(bytes.byteLength);
 *    // Build the view AFTER the allocation. Use it immediately. Let it go.
 *    new Uint8Array(wasm.memory.buffer, ptr, bytes.byteLength).set(bytes);
 *    session.commit_frame(ptr, bytes.byteLength, meta);"
 *
 *   "NEVER CACHE THE VIEW. A module-level const HEAP =
 *    new Uint8Array(wasm.memory.buffer) is the classic failure. Any wasm
 *    memory growth relocates the ArrayBuffer and detaches every outstanding
 *    view; the next write silently targets a detached buffer or throws far
 *    from the cause."
 *
 * **The ordering was unguarded until this file existed.** `eslint.config.js`
 * puts `bulk.ts` in `ALLOWED_TO_DISABLE`, correctly, because it is one of the
 * two files permitted to build the view at all, so the lint that states the
 * rule for the whole tree is the one thing that cannot state it here. Nothing
 * else looked: `writeFrame` is exported from `@ocelli/core`'s published index
 * and had no test. Measured in the S03 sprint review's ninth pass, by
 * rewriting the body into the shape the file's own header quotes as the
 * classic failure and watching `npx eslint .` and `npx vitest run` both exit
 * 0.
 *
 * **What makes the order assertable is an `alloc` that grows the memory**,
 * which is exactly what the real `Session::alloc` is free to do and what the
 * hazard is about. A view hoisted above the `alloc` is a view over the
 * pre-growth ArrayBuffer, and growth detaches that buffer, so the write
 * either throws or lands somewhere nothing will ever read. A view built after
 * the `alloc` is a view over the buffer the core is now using.
 *
 * **Seven of the eight tests below therefore run against a sink that grows,
 * and the eighth deliberately does not.** Rewriting the body into
 *
 *   const heap = new Uint8Array(wasm.memory.buffer);
 *   const ptr = session.alloc(bytes.byteLength);
 *   heap.set(bytes, ptr);
 *
 * turns those seven red and leaves `writes only inside the allocation` green,
 * measured: `npx vitest run packages/core/src/bulk.test.ts` reports
 * `7 failed | 1 passed (8)` and exits 1. That eighth test is about bounds
 * rather than about ordering, and bounds need a memory whose surrounding
 * bytes survive the call so they can be read back. A growing `alloc` returns
 * a pointer into a fresh page, where there are no neighbouring bytes to
 * disturb and nothing to assert. So it fixes `alloc` at 4096 inside an
 * already-allocated two-page memory and checks the eight bytes either side of
 * the write. It passes under both shapes and is evidence of nothing about
 * ordering, which is exactly why the other seven do not follow its pattern.
 */

const PAGE_BYTES = 65536;

/** What one call to the stand-in sink observed, so ordering is assertable. */
interface Commit {
  readonly ptr: number;
  readonly len: number;
  readonly meta: unknown;
  /**
   * A copy of linear memory at `[ptr, ptr + len)` taken INSIDE
   * `commit_frame`. The core takes ownership at this point, so this is what
   * the core would actually read. It is the payload of the whole test: under
   * a hoisted view it is zeroes, under the specified order it is the bytes.
   */
  readonly seen: Uint8Array;
}

/**
 * A `BulkSink` whose `alloc` grows linear memory, and the `WasmMemory` it
 * shares.
 *
 * `WebAssembly.Memory.prototype.grow` detaches the previous `ArrayBuffer`
 * and installs a new one, which is the platform behaviour HLD 17.2's warning
 * is about. Growing by a whole page and returning the first byte of the new
 * page also means the returned pointer does not exist in the old buffer at
 * all, so a hoisted view cannot accidentally be long enough to absorb the
 * write.
 */
function growingSink(): {
  wasm: WasmMemory;
  sink: BulkSink;
  commits: Commit[];
  grows: number;
} {
  const memory = new WebAssembly.Memory({ initial: 1, maximum: 8 });
  const wasm: WasmMemory = { memory };
  const commits: Commit[] = [];
  const state = { grows: 0 };

  const sink: BulkSink = {
    alloc(len: number): number {
      if (len > PAGE_BYTES) {
        throw new Error("fixture allocates one page, ask for less");
      }
      const ptr = memory.buffer.byteLength;
      memory.grow(1);
      state.grows += 1;
      return ptr;
    },
    commit_frame(ptr: number, len: number, meta: unknown): void {
      commits.push({
        ptr,
        len,
        meta,
        seen: new Uint8Array(memory.buffer.slice(ptr, ptr + len)),
      });
    },
  };

  return {
    wasm,
    sink,
    commits,
    get grows(): number {
      return state.grows;
    },
  };
}

/**
 * The commit at `index`, or a failure naming what was missing.
 *
 * `noUncheckedIndexedAccess` is on, so an index read is `Commit | undefined`.
 * Narrowing it with a throw rather than with `!` or an `as` keeps the failure
 * mode honest: a test that expected a commit and got none says so, instead of
 * reporting an unrelated property access on `undefined`.
 */
function at(commits: readonly Commit[], index: number): Commit {
  const commit = commits[index];
  if (commit === undefined) {
    throw new Error(
      `expected a commit at index ${index}, saw ${commits.length}`,
    );
  }
  return commit;
}

/** A recognisable pattern, never all zero, so a missed write is visible. */
function pattern(len: number): Uint8Array {
  const bytes = new Uint8Array(len);
  for (let i = 0; i < len; i += 1) {
    bytes[i] = (i * 7 + 3) & 0xff;
  }
  return bytes;
}

group("writeFrame, HLD 17.2", () => {
  /**
   * The rule itself. The allocation grows linear memory, so the bytes reach
   * the core only if the view was built after the allocation returned.
   *
   * This is the assertion that goes red against the classic failure. Under
   *
   *   const heap = new Uint8Array(wasm.memory.buffer);
   *   const ptr = session.alloc(bytes.byteLength);
   *   heap.set(bytes, ptr);
   *
   * `heap` is detached by the growth inside `alloc` and `set` throws, so this
   * test fails at the call rather than at the expectation. Either way it is
   * red, and that is the point: today the failure has somewhere to be seen.
   */
  it("copies the bytes into memory the allocation returned", () => {
    const { wasm, sink, commits } = growingSink();
    const bytes = pattern(64);

    writeFrame(wasm, sink, bytes, { frame: 0 });

    expect(commits).toHaveLength(1);
    const commit = at(commits, 0);
    const landed = new Uint8Array(wasm.memory.buffer, commit.ptr, 64);
    expect(Array.from(landed)).toEqual(Array.from(bytes));
  });

  /**
   * The copy must be complete BEFORE `commit_frame`, because `commit_frame`
   * is where ownership passes to the core. `Commit.seen` is taken inside the
   * sink, so this asserts the state of memory at the moment of the handover
   * rather than after `writeFrame` returned.
   *
   * A shape that committed first and copied afterwards would satisfy the test
   * above and fail this one.
   */
  it("finishes the copy before ownership passes to the core", () => {
    const { wasm, sink, commits } = growingSink();
    const bytes = pattern(128);

    writeFrame(wasm, sink, bytes, "meta");

    expect(Array.from(at(commits, 0).seen)).toEqual(Array.from(bytes));
  });

  /** The handover carries the allocation's own pointer, length and meta. */
  it("commits the pointer, the length and the meta it was given", () => {
    const { wasm, sink, commits } = growingSink();
    const meta = { rows: 512, columns: 512 };
    const bytes = pattern(96);

    writeFrame(wasm, sink, bytes, meta);

    const commit = at(commits, 0);
    expect(commit.len).toBe(96);
    expect(commit.meta).toBe(meta);
    expect(commit.ptr).toBe(PAGE_BYTES);
    expect(commit.ptr + commit.len).toBeLessThanOrEqual(
      wasm.memory.buffer.byteLength,
    );
  });

  /**
   * The length asked for is the source's byte length, not its element count
   * and not a page. A frame is asked for exactly, because `alloc` is a
   * reservation the core will hand back at `commit_frame`.
   */
  it("allocates exactly the source byte length", () => {
    const memory = new WebAssembly.Memory({ initial: 1, maximum: 8 });
    const asked: number[] = [];
    const sink: BulkSink = {
      alloc(len: number): number {
        asked.push(len);
        const ptr = memory.buffer.byteLength;
        memory.grow(1);
        return ptr;
      },
      commit_frame(): void {},
    };

    writeFrame({ memory }, sink, pattern(37), null);

    expect(asked).toEqual([37]);
  });

  /**
   * **The one that catches a cached view surviving the call.** Two frames
   * through the same sink, each allocation growing the memory again, so the
   * buffer the first frame was written through is detached by the time the
   * second runs.
   *
   * A view retained on a module-level binding, on `this`, or returned to a
   * caller fails here even if it were somehow built in the right order the
   * first time. HLD 17.2: "Never hoist the view, never store it on `this`,
   * never return it."
   */
  it("survives a second frame across a second growth", () => {
    const fixture = growingSink();
    const first = pattern(80);
    const second = pattern(48).map((b) => b ^ 0xff);

    writeFrame(fixture.wasm, fixture.sink, first, "a");
    writeFrame(fixture.wasm, fixture.sink, second, "b");

    expect(fixture.grows).toBe(2);
    expect(fixture.commits).toHaveLength(2);
    expect(Array.from(at(fixture.commits, 0).seen)).toEqual(
      Array.from(first),
    );
    expect(Array.from(at(fixture.commits, 1).seen)).toEqual(
      Array.from(second),
    );
    expect(at(fixture.commits, 1).ptr).toBeGreaterThan(
      at(fixture.commits, 0).ptr,
    );
  });

  /**
   * The write is bounded by the allocation. Bytes either side of `[ptr, ptr +
   * len)` stay as the core left them, so a length taken from the wrong array
   * or an offset applied twice is visible.
   */
  it("writes only inside the allocation", () => {
    const memory = new WebAssembly.Memory({ initial: 2, maximum: 8 });
    const ptr = 4096;
    const len = 64;
    new Uint8Array(memory.buffer).fill(0xaa);
    const sink: BulkSink = {
      alloc(): number {
        return ptr;
      },
      commit_frame(): void {},
    };

    writeFrame({ memory }, sink, pattern(len), null);

    const around = new Uint8Array(memory.buffer, ptr - 8, len + 16);
    expect(Array.from(around.subarray(0, 8))).toEqual(Array(8).fill(0xaa));
    expect(Array.from(around.subarray(8 + len))).toEqual(Array(8).fill(0xaa));
  });

  /** An empty frame is a legal frame and must not be a special case. */
  it("handles a zero-length frame", () => {
    const { wasm, sink, commits } = growingSink();

    writeFrame(wasm, sink, new Uint8Array(0), null);

    expect(at(commits, 0).len).toBe(0);
    expect(at(commits, 0).seen).toHaveLength(0);
  });

  /**
   * The source is the caller's, and the caller keeps it. A copy into linear
   * memory must not disturb it.
   */
  it("leaves the caller's bytes alone", () => {
    const { wasm, sink } = growingSink();
    const bytes = pattern(64);
    const before = Array.from(bytes);

    writeFrame(wasm, sink, bytes, null);

    expect(Array.from(bytes)).toEqual(before);
  });
});

group("writeDicomwebResponse, HLD 17.2 and PS3.18", () => {
  it("writes once through a fresh post-allocation view and commits once", () => {
    const memory = new WebAssembly.Memory({ initial: 1, maximum: 4 });
    const commits: Array<{
      readonly bytes: Uint8Array;
      readonly kind: DicomwebResponseKind;
    }> = [];
    const sink: DicomwebResponseSink = {
      alloc(): number {
        const ptr = memory.buffer.byteLength;
        memory.grow(1);
        return ptr;
      },
      commit_dicomweb_response(ptr, len, kind): void {
        commits.push({
          bytes: new Uint8Array(memory.buffer.slice(ptr, ptr + len)),
          kind,
        });
      },
    };
    const bytes = new Uint8Array([3, 1, 4, 1, 5]);
    const kind: DicomwebResponseKind = { type: "qido-json" };

    writeDicomwebResponse({ memory }, sink, bytes, kind);

    expect(commits).toHaveLength(1);
    expect(Array.from(commits[0]?.bytes ?? [])).toEqual(Array.from(bytes));
    expect(commits[0]?.kind).toBe(kind);
  });
});
