/**
 * The wasm panic probe. HLD section 23, and the one test in F-005 that matters.
 *
 * Everything else in this story is checked on the host, where
 * `panic = "unwind"` and a test can catch a panic. **None of that says
 * anything about wasm32.** `rustc --print cfg --target wasm32-unknown-unknown`
 * reports `panic="abort"`, which is the target's own default and therefore
 * holds in every profile, so on the target that ships there is no unwind and
 * nothing catches. What that leaves behind is what the design rests on, and it
 * has to be measured rather than reasoned about.
 *
 * Four properties, and the design fails at its first step without them.
 *
 *   1. The panic hook RUNS under `panic = "abort"` on wasm32.
 *   2. The call that entered the module throws, so the shell learns something
 *      went wrong without being told.
 *   3. Linear memory is still readable from JavaScript after the trap.
 *   4. The record can be read with NO further export call, which is section
 *      23's "must not be reused" honoured literally. Asking a poisoned
 *      instance why it was poisoned is reusing it.
 *
 * ## Why node and a raw instantiate
 *
 * No new dependency and no browser in the CI floor. The module is instantiated
 * with `WebAssembly.instantiate` directly rather than through the wasm-bindgen
 * glue, because the glue installs its own error handling around a trap and the
 * probe would then be measuring the glue.
 *
 * The design plan flagged a risk here: that bindgen placeholder imports would
 * have to be satisfied with stubs, and that a probe whose stubs get called is
 * measuring the stubs. Measured on wasm-bindgen 0.2.127 with `--target web`,
 * **the release module declares no imports at all**. The stub machinery below
 * is kept anyway, and it THROWS if a stub is ever called, so the day an import
 * appears the probe says so instead of quietly measuring something else.
 *
 * `wasm-bindgen-test` is not used, and that is deliberate: its harness installs
 * its own panic hook and treats a panic as a test failure, so testing panic
 * behaviour under it fights the tool.
 *
 * Run through the gate, which builds the module it needs:
 *
 *     bin/ocelli.sh gate panic
 */

import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const MODULE_PATH = path.join(
  ROOT,
  "crates",
  "ocelli-wasm",
  "target",
  "panic-probe",
  "ocelli_wasm_bg.wasm",
);

/**
 * The panic record's layout, written out here rather than imported.
 *
 * `crates/ocelli-wasm/src/panic.rs` asserts the same numbers, and
 * `packages/core/src/panic.ts` asserts them a third time. Three statements of
 * one wire contract, none derived from another, is the arrangement that makes
 * a disagreement visible. Deriving these from the Rust would make this file
 * agree with whatever the Rust says.
 */
const LAYOUT = {
  /** "OCP1" little-endian: O=0x4F, C=0x43, P=0x50, 1=0x31. */
  magic: 0x3150434f,
  version: 1,
  bytes: 528,
  /** ErrorCode::Panicked in ci/error-codes.json. */
  panickedCode: 1,
  offsets: { magic: 0, version: 4, code: 8, msgLen: 12, msg: 16 },
};

const failures = [];

function check(condition, description) {
  if (condition) {
    console.log(`  ok   ${description}`);
  } else {
    console.log(`  FAIL ${description}`);
    failures.push(description);
  }
}

/**
 * Build an import object for whatever the module asks for, where every stub
 * throws. A probe whose stubs are called is measuring the stubs.
 */
function stubbedImports(module) {
  const imports = {};
  const requested = WebAssembly.Module.imports(module);
  for (const entry of requested) {
    imports[entry.module] ??= {};
    if (entry.kind === "function") {
      imports[entry.module][entry.name] = () => {
        throw new Error(
          `the probe's stub for ${entry.module}.${entry.name} was called, so ` +
            `this run would be measuring the stub rather than the module`,
        );
      };
    } else if (entry.kind === "memory") {
      imports[entry.module][entry.name] = new WebAssembly.Memory({
        initial: 17,
      });
    } else if (entry.kind === "global") {
      imports[entry.module][entry.name] = new WebAssembly.Global(
        { value: "i32", mutable: true },
        0,
      );
    } else if (entry.kind === "table") {
      imports[entry.module][entry.name] = new WebAssembly.Table({
        element: "anyfunc",
        initial: 0,
      });
    }
  }
  return { imports, requested };
}

async function main() {
  let bytes;
  try {
    bytes = await readFile(MODULE_PATH);
  } catch {
    console.error(`FAIL: no probe module at ${MODULE_PATH}`);
    console.error(
      "Build it with: wasm-pack build crates/ocelli-wasm --target web " +
        "--out-dir target/panic-probe -- --features panic-probe",
    );
    console.error("Or run `bin/ocelli.sh gate panic`, which does that first.");
    return 1;
  }

  const module = await WebAssembly.compile(bytes);
  const { imports, requested } = stubbedImports(module);
  console.log(
    `  note module declares ${String(requested.length)} import(s)` +
      (requested.length === 0 ? ", so no stub can be reached" : ""),
  );

  const instance = await WebAssembly.instantiate(module, imports);
  const wasm = instance.exports;

  // Everything that calls into the module happens HERE, before the trap.
  wasm.install_panic_hook();
  const ptr = wasm.panic_record_ptr();
  const len = wasm.panic_record_len();
  const { memory } = wasm;

  check(ptr > 0, "panic_record_ptr returns a real linear-memory address");
  check(
    len === LAYOUT.bytes,
    `panic_record_len is ${String(LAYOUT.bytes)}, got ${String(len)}`,
  );

  // Before the panic the record must read as absent, or "absent" would not
  // distinguish anything.
  const before = new DataView(memory.buffer, ptr, len);
  check(
    before.getUint32(LAYOUT.offsets.magic, true) === 0,
    "the magic is zero before any panic, so an unpanicked instance reads clean",
  );

  // ---------------------------------------------------------------------
  // The trap. NOTHING below this line calls an export.
  // ---------------------------------------------------------------------
  let threw = false;
  let thrown = null;
  try {
    wasm.panic_probe_trigger();
  } catch (error) {
    threw = true;
    thrown = error;
  }

  check(threw, "the call that entered the module throws on a panic");
  // Specifically a RuntimeError, not merely "something". `instanceof Object`
  // would be true of almost anything and would be an assertion that cannot go
  // red, which is the defect class a green suite cannot report on. Measured on
  // node 24: the abort compiles to `unreachable`, so the trap arrives as
  // `WebAssembly.RuntimeError: unreachable`.
  check(
    thrown instanceof WebAssembly.RuntimeError,
    "what it throws is a WebAssembly.RuntimeError, so the module trapped",
  );

  let readable = true;
  let view = null;
  try {
    // Built fresh, after the trap, and never cached. Memory growth detaches a
    // view, and although no wasm code can run now, the discipline is the point
    // (HLD section 17.2). `packages/core/src/panic.ts` does the same.
    view = new DataView(memory.buffer, ptr, len);
  } catch {
    readable = false;
  }
  check(readable, "linear memory is still readable after the trap");
  if (!readable || view === null) {
    return report(thrown);
  }

  const magic = view.getUint32(LAYOUT.offsets.magic, true);
  const version = view.getUint32(LAYOUT.offsets.version, true);
  const code = view.getUint32(LAYOUT.offsets.code, true);
  const msgLen = view.getUint32(LAYOUT.offsets.msgLen, true);

  check(
    magic === LAYOUT.magic,
    "THE PANIC HOOK RAN under panic = \"abort\" on wasm32, " +
      `magic is 0x${magic.toString(16)}`,
  );
  check(version === LAYOUT.version, "the record carries its layout version");
  check(
    code === LAYOUT.panickedCode,
    `the record carries ErrorCode::Panicked (${String(LAYOUT.panickedCode)})`,
  );
  check(msgLen > 0, "the record carries a message");
  check(msgLen <= LAYOUT.bytes - LAYOUT.offsets.msg, "the length is in range");

  const message = new TextDecoder("utf-8", { fatal: false }).decode(
    new Uint8Array(memory.buffer, ptr + LAYOUT.offsets.msg, msgLen),
  );
  console.log(`  note recorded message: ${JSON.stringify(message)}`);

  check(
    message.includes("ocelli panic probe, F-005"),
    "the payload of the panic is in the message",
  );
  check(
    message.includes("crates/ocelli-wasm/src/lib.rs"),
    "the panic's FILE is in the message",
  );
  check(
    /:\d+:\d+/.test(message),
    "the panic's LINE and COLUMN are in the message",
  );

  return report(thrown);
}

function report(thrown) {
  console.log();
  if (failures.length === 0) {
    console.log(
      "OK: the panic hook runs under panic = \"abort\" on wasm32, the call " +
        "throws, and the record is readable from linear memory with no " +
        "further export call",
    );
    return 0;
  }
  console.log(`FAIL: ${String(failures.length)} panic-probe assertion(s)`);
  for (const failure of failures) {
    console.log(`  ${failure}`);
  }
  console.log();
  console.log(
    "This is the property F-005's whole design rests on. Do not soften the " +
      "probe. See docs/lld/errors.md and .claude/plans/F-005-design.md.",
  );
  if (thrown !== null) {
    console.log(`  what the module threw: ${String(thrown)}`);
  }
  return 1;
}

process.exit(await main());
