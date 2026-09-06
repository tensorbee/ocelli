import { describe as group, expect, it } from "vitest";

import { describeError, ERROR_CODE } from "./errors.js";
import {
  PANIC_HEADER_BYTES,
  PANIC_MAGIC,
  PANIC_MESSAGE_CAPACITY,
  PANIC_RECORD_BYTES,
  PANIC_RECORD_VERSION,
  readPanicRecord,
  type PanicMemory,
} from "./panic.js";

/**
 * A real `WebAssembly.Memory`, so this exercises the same object shape the
 * shell sees rather than a stand-in for one. Nothing is instantiated: the
 * record is bytes at a fixed address and reading it needs no module, which is
 * the whole property F-005 depends on.
 */
function memoryWith(
  ptr: number,
  fill: (view: DataView, bytes: Uint8Array) => void,
): PanicMemory {
  const memory = new WebAssembly.Memory({ initial: 1 });
  const view = new DataView(memory.buffer, ptr, PANIC_RECORD_BYTES);
  const bytes = new Uint8Array(memory.buffer, ptr, PANIC_RECORD_BYTES);
  fill(view, bytes);
  return { memory };
}

/**
 * Build a record by hand, from the layout written out in `panic.ts` and in
 * `crates/ocelli-wasm/src/panic.rs`. The offsets are typed in here rather than
 * imported, so a change to the reader alone goes red.
 */
function writeRecord(
  view: DataView,
  bytes: Uint8Array,
  options: {
    magic?: number;
    version?: number;
    code?: number;
    message: string;
    length?: number;
  },
): void {
  const encoded = new TextEncoder().encode(options.message);
  view.setUint32(0, options.magic ?? PANIC_MAGIC, true);
  view.setUint32(4, options.version ?? PANIC_RECORD_VERSION, true);
  view.setUint32(8, options.code ?? 1, true);
  view.setUint32(12, options.length ?? encoded.byteLength, true);
  bytes.set(encoded.subarray(0, PANIC_MESSAGE_CAPACITY), PANIC_HEADER_BYTES);
}

const PTR = 1024;

group("readPanicRecord", () => {
  /**
   * The ordinary case for an instance that has not panicked. It is also the
   * correct answer for a torn record, because the core writes the magic LAST:
   * a hook that panicked part way through leaves zero here, and "absent" is a
   * safer reading than "present with garbage in it".
   */
  it("returns null for an all-zero region", () => {
    const wasm = memoryWith(PTR, () => {
      /* leave it zeroed */
    });
    expect(readPanicRecord(wasm, PTR, PANIC_RECORD_BYTES)).toBeNull();
  });

  it("returns null when the magic is anything else", () => {
    const wasm = memoryWith(PTR, (view, bytes) => {
      writeRecord(view, bytes, { magic: 0xdeadbeef, message: "torn" });
    });
    expect(readPanicRecord(wasm, PTR, PANIC_RECORD_BYTES)).toBeNull();
  });

  /**
   * `crates/ocelli-wasm/src/panic.rs` documents `record_ptr()` returning 0 as
   * the ordinary answer on a 64-bit host, so 0 is the sentinel for "no record"
   * and never an address to read. The record is written AT zero here, so the
   * `ptr <= 0` guard is the only thing that can return null: without it the
   * magic check would find a valid record and hand it back.
   */
  it("returns null for a zero pointer even when the bytes there are a record", () => {
    const wasm = memoryWith(0, (view, bytes) => {
      writeRecord(view, bytes, { message: "at address zero" });
    });
    expect(readPanicRecord(wasm, 0, PANIC_RECORD_BYTES)).toBeNull();
  });

  /**
   * A negative pointer throws a `RangeError` out of the `DataView`
   * constructor, so the guard is the only thing between a bad cached integer
   * and an exception raised while the shell is already handling a dead
   * instance. `toBeNull` would not catch a throw, so the throw is asserted
   * against directly.
   */
  it("returns null for a negative pointer rather than throwing", () => {
    const wasm = memoryWith(PTR, (view, bytes) => {
      writeRecord(view, bytes, { message: "ignored" });
    });
    expect(() => readPanicRecord(wasm, -16, PANIC_RECORD_BYTES)).not.toThrow();
    expect(readPanicRecord(wasm, -16, PANIC_RECORD_BYTES)).toBeNull();
  });

  it("returns null when the length is shorter than the layout", () => {
    const wasm = memoryWith(PTR, (view, bytes) => {
      writeRecord(view, bytes, { message: "ignored" });
    });
    expect(readPanicRecord(wasm, PTR, PANIC_RECORD_BYTES - 1)).toBeNull();
  });

  it("returns null rather than reading past the end of memory", () => {
    const memory = new WebAssembly.Memory({ initial: 1 });
    const beyond = memory.buffer.byteLength - 8;
    expect(
      readPanicRecord({ memory }, beyond, PANIC_RECORD_BYTES),
    ).toBeNull();
  });

  it("decodes a hand-built record", () => {
    const wasm = memoryWith(PTR, (view, bytes) => {
      writeRecord(view, bytes, {
        code: 1,
        message: "ocelli panic probe, F-005 at crates/ocelli-wasm/src/lib.rs:110:5",
      });
    });
    const record = readPanicRecord(wasm, PTR, PANIC_RECORD_BYTES);
    expect(record).not.toBeNull();
    expect(record?.version).toBe(PANIC_RECORD_VERSION);
    expect(record?.code).toBe(1);
    expect(record?.message).toContain("ocelli panic probe, F-005");
    expect(record?.location).toBe("crates/ocelli-wasm/src/lib.rs:110:5");
    expect(record?.truncated).toBe(false);
    expect(record?.unknownVersion).toBe(false);
  });

  it("reports no location when the message carries none", () => {
    const wasm = memoryWith(PTR, (view, bytes) => {
      writeRecord(view, bytes, { message: "no location here" });
    });
    expect(readPanicRecord(wasm, PTR, PANIC_RECORD_BYTES)?.location).toBeNull();
  });

  /**
   * **The location is the TRAILING one**, which is what `LOCATION`'s `$`
   * anchor is for and what nothing asserted. Rust's panic hook appends
   * ` at file:line:column` to the end of the message, so a `file:line:column`
   * appearing earlier is part of what the panic said and not where it
   * happened. A message quoting one path while panicking in another is the
   * ordinary shape of an assertion failure that names a file.
   *
   * Without the anchor the regex takes the FIRST match, so this test is the
   * only thing separating the two. It is diagnostic rather than arithmetic,
   * which is why it is one case: a wrong location sends a reader to the wrong
   * file, it does not produce a wrong pixel.
   */
  it("takes the trailing location and not an earlier one", () => {
    const wasm = memoryWith(PTR, (view, bytes) => {
      writeRecord(view, bytes, {
        message:
          "assertion failed at crates/a/src/one.rs:1:2 at crates/b/src/two.rs:33:4",
      });
    });
    expect(readPanicRecord(wasm, PTR, PANIC_RECORD_BYTES)?.location).toBe(
      "crates/b/src/two.rs:33:4",
    );
  });

  /**
   * A message that filled the buffer is reported as truncated. The core clamps
   * the length to capacity, and a length larger than capacity, which a newer
   * or damaged core could write, is clamped again here rather than trusted.
   */
  it("reports a full buffer as truncated and clamps an over-long length", () => {
    const wasm = memoryWith(PTR, (view, bytes) => {
      writeRecord(view, bytes, {
        message: "z".repeat(PANIC_MESSAGE_CAPACITY + 64),
        length: PANIC_MESSAGE_CAPACITY + 64,
      });
    });
    const record = readPanicRecord(wasm, PTR, PANIC_RECORD_BYTES);
    expect(record?.truncated).toBe(true);
    expect(record?.message.length).toBe(PANIC_MESSAGE_CAPACITY);
  });

  /**
   * A newer core is a real thing, and refusing its record would turn "I do not
   * understand this" into "there was no panic", which is section 23's silent
   * blank canvas.
   */
  it("reports an unknown layout version rather than refusing the record", () => {
    const wasm = memoryWith(PTR, (view, bytes) => {
      writeRecord(view, bytes, { version: 99, message: "from the future" });
    });
    const record = readPanicRecord(wasm, PTR, PANIC_RECORD_BYTES);
    expect(record?.unknownVersion).toBe(true);
    expect(record?.version).toBe(99);
  });

  /**
   * **The version is at offset 4 and the code is at offset 8, and each record
   * here carries a different number in the two words.**
   *
   * Every other case in this file leaves both at 1, because `writeRecord`
   * defaults `version` to `PANIC_RECORD_VERSION` and `code` to `1`, and the one
   * case that varies the version asserts `version` and `unknownVersion` and
   * never `code`. So the reader could take the code out of the VERSION word and
   * this suite stayed green.
   *
   * It is latent only while the two numbers agree.
   * `crates/ocelli-wasm/src/panic.rs` says the version is "Bumped when the
   * field order below changes", so the first bump makes every panic report code
   * 2, `describeError(2)` misses the table, and the user is told the build does
   * not recognise error code 2 instead of getting HLD section 23's sentence,
   * at the one moment that sentence is load bearing.
   *
   * `ERROR_CODE.Workgroup` rather than an invented number, because the field
   * holds an `ErrorCode` and 701 is a registered one.
   */
  it("reads the code from its own word and not from the version's", () => {
    const wasm = memoryWith(PTR, (view, bytes) => {
      writeRecord(view, bytes, {
        version: PANIC_RECORD_VERSION,
        code: ERROR_CODE.Workgroup,
        message: "a code that is not the version",
      });
    });
    const record = readPanicRecord(wasm, PTR, PANIC_RECORD_BYTES);
    expect(record?.code).toBe(ERROR_CODE.Workgroup);
    expect(record?.version).toBe(PANIC_RECORD_VERSION);
    expect(record?.unknownVersion).toBe(false);
  });

  /**
   * The bump itself, written out. A core with layout version 2 that panicked
   * still reports `Panicked`, and the shell still has the sentence for it.
   */
  it("keeps the code when a newer core bumps the layout version", () => {
    const wasm = memoryWith(PTR, (view, bytes) => {
      writeRecord(view, bytes, {
        version: PANIC_RECORD_VERSION + 1,
        code: ERROR_CODE.Panicked,
        message: "written by a core one version ahead",
      });
    });
    const record = readPanicRecord(wasm, PTR, PANIC_RECORD_BYTES);
    expect(record?.code).toBe(ERROR_CODE.Panicked);
    expect(record?.version).toBe(PANIC_RECORD_VERSION + 1);
    expect(record?.unknownVersion).toBe(true);
    expect(describeError(record?.code ?? 0)).not.toContain("does not recognise");
  });

  /**
   * A truncation can land inside a UTF-8 sequence. Decoding leniently turns
   * that into one replacement character instead of an exception thrown while
   * the shell is already handling a dead instance.
   */
  it("decodes a message cut inside a UTF-8 sequence without throwing", () => {
    const wasm = memoryWith(PTR, (view, bytes) => {
      // "e" then the first two bytes of a three-byte code point.
      bytes.set([0x65, 0xe2, 0x82], PANIC_HEADER_BYTES);
      view.setUint32(0, PANIC_MAGIC, true);
      view.setUint32(4, PANIC_RECORD_VERSION, true);
      view.setUint32(8, 1, true);
      view.setUint32(12, 3, true);
    });
    const record = readPanicRecord(wasm, PTR, PANIC_RECORD_BYTES);
    expect(record?.message.startsWith("e")).toBe(true);
    expect(record?.message.length).toBeGreaterThan(0);
  });

  /**
   * The layout numbers are a wire contract with
   * `crates/ocelli-wasm/src/panic.rs`, which asserts the same literals, and
   * with `scripts/panic_probe.mjs`, which asserts them a third time.
   */
  it("states the layout the core writes", () => {
    expect(PANIC_HEADER_BYTES).toBe(16);
    expect(PANIC_MESSAGE_CAPACITY).toBe(512);
    expect(PANIC_RECORD_BYTES).toBe(528);
    expect(PANIC_MAGIC).toBe(0x3150434f);
    expect(PANIC_RECORD_VERSION).toBe(1);
  });
});
