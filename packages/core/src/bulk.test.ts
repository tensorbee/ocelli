import { describe as group, expect, it } from "vitest";

import {
  acceptTransferredFrame,
  exportDecodedFrame,
  transferredFrameMessage,
  writeDicomwebResponse,
  writeFrame,
  writeCommands,
  writeStack,
  type BulkSink,
  type DicomwebResponseKind,
  type DicomwebResponseSink,
  type HandoffMeta,
  type WasmMemory,
} from "./bulk.js";

it("command writes acquire memory after allocation growth and never write on refusal", () => {
  const memory = new WebAssembly.Memory({ initial: 1 });
  const bytes = new Uint8Array([1, 2, 3]);
  let applied = 0;
  const sink = {
    alloc() { memory.grow(1); return 65536; },
    apply_commands(ptr: number, len: number) {
      expect(ptr).toBe(65536);
      expect(Array.from(new Uint8Array(memory.buffer, ptr, len))).toEqual([1, 2, 3]);
      applied += 1;
      return 1;
    },
  };
  expect(writeCommands({ memory }, sink, bytes)).toBe(1);
  expect(applied).toBe(1);
  expect(writeCommands({ memory }, { ...sink, alloc: () => 0 }, bytes)).toBe(0);
  expect(applied).toBe(1);
});

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
 * **Seven of the eight `writeFrame` tests run against a sink that grows,
 * and `writes only inside the allocation` deliberately does not.** Rewriting
 * that function into
 *
 *   const heap = new Uint8Array(wasm.memory.buffer);
 *   const ptr = session.alloc(bytes.byteLength);
 *   heap.set(bytes, ptr);
 *
 * turns those seven red and leaves the bounds test green. Measured when the
 * `writeFrame` group was the whole file: `npx vitest run
 * packages/core/src/bulk.test.ts` reported `7 failed | 1 passed (8)` and
 * exited 1. The bounds test needs a memory whose surrounding bytes survive
 * the call so they can be read back. A growing `alloc` returns a pointer
 * into a fresh page, where there are no neighbouring bytes to disturb and
 * nothing to assert. So it fixes `alloc` at 4096 inside an already-allocated
 * two-page memory and checks the eight bytes either side of the write. It
 * passes under both shapes and is evidence of nothing about ordering, which
 * is exactly why the other seven do not follow its pattern.
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
  it("validates ticket-sink metadata before allocation", () => {
    const memory = new WebAssembly.Memory({ initial: 1, maximum: 2 });
    let allocs = 0;
    let released = 0;
    const sink: BulkSink = {
      alloc(): number {
        allocs += 1;
        return 8;
      },
      allocationTicket(): number {
        return 7;
      },
      commitAllocated(): number {
        return 1;
      },
      commit_frame(): number {
        return 1;
      },
      releaseAllocated(ticket: number): void {
        released = ticket;
      },
    };

    expect(() =>
      writeFrame({ memory }, sink, pattern(4), {
        viewport: "bad",
        columns: 1,
        rows: 1,
        generation: 1,
      }),
    ).toThrow("viewport is not a u32");
    expect(allocs).toBe(0);
    expect(released).toBe(0);
  });

  it("releases the captured ticket when the copy throws", () => {
    const memory = new WebAssembly.Memory({ initial: 1, maximum: 2 });
    const released: number[] = [];
    const sink: BulkSink = {
      alloc(): number {
        return memory.buffer.byteLength + 8;
      },
      allocationTicket(): number {
        return 11;
      },
      commitAllocated(): number {
        return 1;
      },
      commit_frame(): number {
        return 1;
      },
      releaseAllocated(ticket: number): void {
        released.push(ticket);
      },
    };

    expect(() =>
      writeFrame({ memory }, sink, pattern(4), {
        viewport: 1,
        columns: 1,
        rows: 1,
        generation: 1,
      }),
    ).toThrow(RangeError);
    expect(released).toEqual([11]);
  });

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
  it("refuses a zero allocation before any view or commit", () => {
    const memory = new WebAssembly.Memory({ initial: 1, maximum: 2 });
    new Uint8Array(memory.buffer, 0, 8).fill(0x5a);
    let commits = 0;
    const sink: BulkSink = {
      alloc(): number {
        return 0;
      },
      commit_frame(): void {
        commits += 1;
      },
    };

    expect(() => writeFrame({ memory }, sink, pattern(8), null)).toThrow(
      "allocation refused",
    );
    expect(commits).toBe(0);
    expect(Array.from(new Uint8Array(memory.buffer, 0, 8))).toEqual(
      Array(8).fill(0x5a),
    );
  });

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

  it("refuses a zero allocation before the response commit", () => {
    const memory = new WebAssembly.Memory({ initial: 1, maximum: 2 });
    new Uint8Array(memory.buffer, 0, 4).fill(0x11);
    let commits = 0;
    const sink: DicomwebResponseSink = {
      alloc(): number {
        return 0;
      },
      commit_dicomweb_response(): void {
        commits += 1;
      },
    };

    expect(() =>
      writeDicomwebResponse({ memory }, sink, new Uint8Array([1, 2, 3, 4]), {
        type: "qido-json",
      }),
    ).toThrow("allocation refused");
    expect(commits).toBe(0);
    expect(Array.from(new Uint8Array(memory.buffer, 0, 4))).toEqual(
      Array(4).fill(0x11),
    );
  });
});

/** An `ArrayBuffer` of exactly `bytes.byteLength`, not a view onto `bytes`. */
function exactBuffer(bytes: Uint8Array): ArrayBuffer {
  const buffer = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(buffer).set(bytes);
  return buffer;
}

/**
 * HLD section 24's copy out of the decode instance, and section 17.2's trap.
 *
 * The first call reads bytes already in linear memory and must return a fresh
 * `ArrayBuffer`. `memory.grow` then detaches that backing buffer. The second
 * call must return the bytes written after the growth. A view taken on the
 * first call and kept, including a module-level binding filled on first use,
 * is detached by the growth, so the second read throws or copies nothing this
 * test accepts.
 *
 * Measured mutation, reverted. `exportDecodedFrame` kept
 * `retainedView ??= new Uint8Array(wasm.memory.buffer)` and copied from that
 * view. `npx vitest run packages/core/src/bulk.test.ts` reported
 * `1 failed | 12 passed (13)` and exited 1. The red test was this one, with
 * `TypeError: Cannot perform Construct on a detached ArrayBuffer`.
 */
group("exportDecodedFrame, HLD 17.2 and section 24", () => {
  it("returns the second call's bytes after wasm memory grows", () => {
    const memory = new WebAssembly.Memory({ initial: 1, maximum: 8 });
    const wasm: WasmMemory = { memory };
    const first = pattern(64);
    new Uint8Array(memory.buffer, 0, first.byteLength).set(first);

    const firstOut = exportDecodedFrame(wasm, 0, first.byteLength);

    expect(firstOut).toBeInstanceOf(ArrayBuffer);
    expect(firstOut).not.toBe(memory.buffer);
    expect(firstOut.byteLength).toBe(first.byteLength);
    expect(Array.from(new Uint8Array(firstOut))).toEqual(Array.from(first));

    const grown = memory.grow(1);
    expect(grown).toBe(1);

    const second = pattern(48).map((b) => b ^ 0xff);
    const ptr = PAGE_BYTES;
    new Uint8Array(memory.buffer, ptr, second.byteLength).set(second);

    const secondOut = exportDecodedFrame(wasm, ptr, second.byteLength);

    expect(secondOut).toBeInstanceOf(ArrayBuffer);
    expect(secondOut).not.toBe(memory.buffer);
    expect(secondOut).not.toBe(firstOut);
    expect(secondOut.byteLength).toBe(second.byteLength);
    expect(Array.from(new Uint8Array(secondOut))).toEqual(Array.from(second));
    expect(Array.from(new Uint8Array(firstOut))).toEqual(Array.from(first));
  });
});

/**
 * The render-worker copy. Section 17.2's order is `alloc`, then a view, then
 * `set`, then `commit_frame`. The length check is the plan's refusal and it
 * happens before that copy. `bytes` is the length. The other identity fields
 * are deliberately equal to the buffer length in the refusal, and deliberately
 * not equal to it in the copy, so the wrong field cannot satisfy both.
 *
 * Measured mutation, reverted. The copy built
 * `new Uint8Array(wasm.memory.buffer)` before `alloc` and wrote through that
 * view. The same vitest command reported `1 failed | 12 passed (13)` and
 * exited 1. The red test was `finishes the copy before commit_frame`, with
 * `TypeError: Cannot perform %TypedArray%.prototype.set on a detached ArrayBuffer`.
 */
group("acceptTransferredFrame, HLD 17.2", () => {
  it("finishes the copy before commit_frame", () => {
    const { wasm, sink, commits } = growingSink();
    const bytes = pattern(128);
    const buffer = exactBuffer(bytes);
    const meta: HandoffMeta = {
      series: 7,
      instance: 3,
      frame: 2,
      bytes: bytes.byteLength,
    };

    const target = { viewport: 1, columns: 16, rows: 8, generation: 3 };
    acceptTransferredFrame(wasm, sink, buffer, meta, target);

    const commit = at(commits, 0);
    expect(Array.from(commit.seen)).toEqual(Array.from(bytes));
    expect(commit.len).toBe(128);
    expect(commit.ptr).toBe(PAGE_BYTES);
    expect(commit.meta).toEqual({ handoff: meta, ...target });
    expect(
      (commit.meta as { readonly handoff: HandoffMeta }).handoff,
    ).toBe(meta);
    expect(Array.from(new Uint8Array(buffer))).toEqual(Array.from(bytes));
  });

  it("throws before any copy when the length disagrees", () => {
    const memory = new WebAssembly.Memory({ initial: 1, maximum: 4 });
    const asked: number[] = [];
    let commits = 0;
    const sink: BulkSink = {
      alloc(len: number): number {
        asked.push(len);
        return 0;
      },
      commit_frame(): void {
        commits += 1;
      },
    };
    const buffer = new ArrayBuffer(8);
    const meta: HandoffMeta = {
      series: 8,
      instance: 8,
      frame: 8,
      bytes: 7,
    };

    expect(() =>
      acceptTransferredFrame(
        { memory },
        sink,
        buffer,
        meta,
        { viewport: 1, columns: 1, rows: 1, generation: 1 },
      ),
    ).toThrow("handoff length 8 disagrees with meta.bytes 7");
    expect(asked).toEqual([]);
    expect(commits).toBe(0);
  });

  it("refuses a zero allocation before the transferred copy", () => {
    const memory = new WebAssembly.Memory({ initial: 1, maximum: 2 });
    new Uint8Array(memory.buffer, 0, 4).fill(0x22);
    let commits = 0;
    const sink: BulkSink = {
      alloc(): number {
        return 0;
      },
      commit_frame(): void {
        commits += 1;
      },
    };
    const buffer = exactBuffer(pattern(4));
    const meta: HandoffMeta = {
      series: 1,
      instance: 1,
      frame: 1,
      bytes: 4,
    };

    expect(() =>
      acceptTransferredFrame(
        { memory },
        sink,
        buffer,
        meta,
        { viewport: 1, columns: 1, rows: 1, generation: 1 },
      ),
    ).toThrow("allocation refused");
    expect(commits).toBe(0);
    expect(Array.from(new Uint8Array(memory.buffer, 0, 4))).toEqual(
      Array(4).fill(0x22),
    );
  });

  it("refuses a target without a u32 viewport before allocation", () => {
    const memory = new WebAssembly.Memory({ initial: 1, maximum: 2 });
    let allocs = 0;
    const sink: BulkSink = {
      alloc(): number {
        allocs += 1;
        return 8;
      },
      commit_frame(): void {},
    };
    const buffer = exactBuffer(pattern(4));
    const meta: HandoffMeta = {
      series: 1,
      instance: 1,
      frame: 0,
      bytes: 4,
    };

    expect(() =>
      acceptTransferredFrame({ memory }, sink, buffer, meta, {
        viewport: "bad" as unknown as number,
        columns: 1,
        rows: 1,
        generation: 1,
      }),
    ).toThrow("viewport is not a u32");
    expect(allocs).toBe(0);
  });
});

/**
 * Section 24, decode to render. The message is `{ kind:'frame', frameId,
 * buffer, meta }` and the transfer list is `[buffer]`. This package does not
 * call `postMessage`.
 */
group("transferredFrameMessage, HLD section 24", () => {
  it("returns kind frame and lists the same buffer for transfer", () => {
    const buffer = exactBuffer(pattern(4));
    const meta: HandoffMeta = {
      series: 9,
      instance: 8,
      frame: 7,
      bytes: 4,
    };

    const handed = transferredFrameMessage(11, buffer, meta);
    const listed = handed.transfer[0];
    if (listed === undefined) {
      throw new Error("transfer list was empty");
    }

    expect(handed.message.kind).toBe("frame");
    expect(handed.message.frameId).toBe(11);
    expect(handed.message.buffer).toBe(buffer);
    expect(handed.message.meta).toBe(meta);
    expect(handed.transfer).toHaveLength(1);
    expect(listed).toBe(buffer);
  });
});

it("writes each stack key as little-endian series, instance and frame", () => {
  const memory = new WebAssembly.Memory({ initial: 2 });
  const at = 65536;
  let seen: number[] = [];
  const wasm = {
    memory,
    alloc() { return at; },
    bulk_ticket() { return 77; },
    release_bulk(ticket: number) {
      expect(ticket).toBe(77);
      return 1;
    },
    configure_stack(ticket: number, len: number) {
      expect(ticket).toBe(77);
      expect(len).toBe(32);
      seen = Array.from(new Uint8Array(memory.buffer, at, len));
      return 5;
    },
  };
  const epoch = writeStack(wasm, [
    { series: 0x0102_0304_0506_0708n, instance: 9, frame: 4 },
    { series: 1n, instance: 2, frame: 3 },
  ]);
  expect(epoch).toBe(5);
  expect(seen).toEqual([
    0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0x09, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00,
  ]);
});
