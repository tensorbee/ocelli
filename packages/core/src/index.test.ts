import { describe, expect, it } from "vitest";

import * as core from "./index.js";
import { coreAvailable, VERSION } from "./index.js";

/**
 * The runtime surface `@ocelli/core` publishes, typed in by hand.
 *
 * **The list is here and not derived from `index.ts`, which is the whole
 * point.** A list built from the module under test moves with it and asserts
 * nothing. Measured in the S03 sprint review's tenth pass, before this list
 * existed: deleting the `writeFrame` or the `readEvent` export from `index.ts`
 * left `npx eslint .`, `npx tsc` and `npx vitest run` all at 0, and
 * `scripts/package_check.py`'s out-of-workspace consumer exercises only
 * `VERSION`, `coreAvailable` and the component export, so nothing anywhere
 * noticed. `bulk.test.ts` and `ring.test.ts` both give "it ships from the
 * published index" as the reason they matter, and that fact was held by no
 * check.
 *
 * Only value exports are observable here. The type-only exports (`BulkSink`,
 * `WasmMemory`, `DrainResult`, `OcelliEvent`, `OcelliRecord`,
 * `RecordOperands`, `PanicMemory`, `PanicRecord`, `CoreStatus`) are erased
 * before this runs, and `npx tsc` is what watches those.
 *
 * Adding an export is a deliberate act, so adding a line here is the cost of
 * publishing one.
 */
const PUBLISHED: readonly string[] = [
  // ./bulk.js, HLD 17.2
  "writeDicomwebResponse",
  "writeFrame",
  // ./dicomweb.js, PS3.18 sections 9.1.2, 10.4, and 10.6
  "DicomwebClient",
  "DicomwebError",
  // ./ring.js, HLD 17.3
  "readEvent",
  "EVENT_STRIDE",
  "HEADER_BYTES",
  // ./errors.js, HLD 23
  "decodeRecord",
  "describeError",
  "ERROR_CODE",
  "LOG_LEVEL",
  "RECORD_BYTES",
  "SEVERITY",
  // ./panic.js, HLD 23
  "PANIC_FALLBACK_CODE",
  "PANIC_HEADER_BYTES",
  "PANIC_MAGIC",
  "PANIC_MESSAGE_CAPACITY",
  "PANIC_RECORD_BYTES",
  "PANIC_RECORD_VERSION",
  "readPanicRecord",
  // ./fatal.js, HLD 23
  "CORE_OK",
  "fatalFromPanic",
  "isUsable",
  "nextStatus",
  // ./capabilities.js, HLD 7 and deviation D-07
  "SIMD128_PROBE_MODULE",
  "moduleValidates",
  "sharedMemoryAvailable",
  "wasmSimd128Supported",
  // this file
  "VERSION",
  "coreAvailable",
];

describe("@ocelli/core", () => {
  /**
   * The package version and the Rust workspace version are one number.
   * `docs/RELEASE.md` says the crates and the packages version together and
   * that a skew between them is not a supported configuration.
   *
   * This asserts the literal rather than reading `package.json`, for the same
   * reason `ocelli_version()`'s test in `crates/ocelli-wasm` asserts a
   * literal: comparing the constant against the file it was copied from
   * restates it, and passes whatever either says.
   *
   * `scripts/package_check.py` is what compares the two files. This is what
   * catches the constant drifting away from its own manifest.
   */
  it("declares the workspace version", () => {
    expect(VERSION).toBe("0.1.0");
  });

  /**
   * A clean clone has no built wasm core, so this is `false` and that is the
   * honest answer rather than a failure to start. The example viewer uses it
   * to render a "core not built" state.
   *
   * **When F-101 makes this detect a real core, this test has to change**, and
   * that is the point of asserting it now. The change becomes visible in a
   * diff instead of happening quietly.
   */
  it("reports no core until one is built", () => {
    expect(coreAvailable()).toBe(false);
  });

  /**
   * The published surface, both directions. A deleted export goes red because
   * the name is missing, and an added one goes red because it is not on the
   * list, so publishing something by accident is as visible as unpublishing
   * something by accident.
   */
  it("publishes exactly the runtime surface named above", () => {
    expect(Object.keys(core).sort()).toEqual([...PUBLISHED].sort());
  });

  /**
   * The two the boundary tests are about, named separately from the set.
   *
   * `bulk.test.ts` and `ring.test.ts` both cover HLD 17.2 and 17.3 on the
   * grounds that these ship from this index. The set assertion above already
   * covers the names, and this covers what they are: a `writeFrame` that
   * became a re-exported constant would satisfy the set and not this.
   */
  it("publishes bulk, DICOMweb, and event entry points as callables", () => {
    expect(typeof core.writeDicomwebResponse).toBe("function");
    expect(typeof core.writeFrame).toBe("function");
    expect(typeof core.DicomwebClient).toBe("function");
    expect(typeof core.DicomwebError).toBe("function");
    expect(typeof core.readEvent).toBe("function");
  });
});
