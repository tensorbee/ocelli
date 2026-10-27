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
 * `WasmMemory`, `HandoffMeta`, `FrameMessage`, `DrainResult`, `OcelliEvent`,
 * `OcelliRecord`, `RecordOperands`, `PanicMemory`, `PanicRecord`,
 * `CoreStatus`, `Camera`, `Colormap`, `Invert`, `Scene`, `StackCommand`,
 * `StackViewport`, `TierAvailability`, `ToolBinding`, `ViewportId`,
 * `VoiFunction`, `WindowPreset`) are erased
 * before this runs, and `npx tsc` is what watches those.
 *
 * Adding an export is a deliberate act, so adding a line here is the cost of
 * publishing one.
 */
const PUBLISHED: readonly string[] = [
  // ./bulk.js, HLD 17.2 and section 24
  "acceptTransferredFrame",
  "exportDecodedFrame",
  "transferredFrameMessage",
  "sessionBulkSink",
  "writeDicomwebResponse",
  "writeFrame",
  "writeCommands",
  "writeStack",
  // ./dicomweb.js, PS3.18 sections 9.1.2, 10.4, and 10.6
  "DicomwebClient",
  "DicomwebError",
  // ./ring.js, HLD 17.3
  "drainRing",
  "readEvent",
  "HEADER_BYTES",
  // ./generated.js, F-104, from crates/ocelli-viewport/src/boundary.rs
  "COLORMAP_BYTE",
  "COMMAND_ID",
  "COMMAND_PAYLOAD_BYTES",
  "EVENT_HEADER_BYTES",
  "EVENT_KIND_OFFSET",
  "EVENT_PAYLOAD_BYTES",
  "EVENT_PAYLOAD_OFFSET",
  "EVENT_RING_CAPACITY_OFFSET",
  "EVENT_RING_DROPPED_OFFSET",
  "EVENT_RING_PENDING_OFFSET",
  "EVENT_RING_TAIL_OFFSET",
  "EVENT_SEQ_OFFSET",
  "EVENT_STRIDE",
  "EVENT_VIEWPORT_OFFSET",
  "FLAG_ERROR",
  "FLAG_INVERT",
  "FLAG_LOADING",
  "FLAG_REFINING",
  "INVERT_BYTE",
  "PRESET_BYTE",
  "RING_EVENT_KIND",
  "STACK_COMMANDS",
  "STATE_CAMERA_COMPONENT_BYTES",
  "STATE_CAMERA_COMPONENTS",
  "STATE_CAMERA_OFFSET",
  "STATE_FLAGS_OFFSET",
  "STATE_NUM_SLICES_OFFSET",
  "STATE_PADDING_OFFSET",
  "STATE_SLICE_INDEX_OFFSET",
  "STATE_VOI_CENTER_OFFSET",
  "STATE_VOI_WIDTH_OFFSET",
  "VIEWPORT_STATE_BYTES",
  "VOI_FUNCTION_BYTE",
  // ./commands.js, F-101
  "packCommands",
  "packPointerFrame",
  "sessionCommandSink",
  "submitCommands",
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
  // ./viewport.js, HLD 17.4 and the stack command set
  "CAMERA_COMPONENTS",
  "COLORMAPS",
  "INVERT",
  "TIER_AVAILABILITY",
  "TIER_C_DRAWING",
  "VIEWPORT_STATE_FIELDS",
  "VOI_FUNCTIONS",
  "WINDOW_PRESETS",
  "readViewportState",
  "viewportId",
  // ./snapshot.js, F-050
  "SNAPSHOT_ERROR",
  "SNAPSHOT_MAGIC",
  "SNAPSHOT_VERSION",
  "parseSnapshot",
  "restoreSessionSnapshot",
  "sessionSnapshot",
  "sessionSnapshotError",
  // ./events.js, F-103
  "CORNERSTONE_EVENT_COUNT",
  "CORNERSTONE_EVENT_MAP",
  "EVENT_DELIVERY",
  "OCELLI_EVENTS",
  "UNMAPPED_EVENTS",
  "decodeNamedEvent",
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
    expect(typeof core.acceptTransferredFrame).toBe("function");
    expect(typeof core.exportDecodedFrame).toBe("function");
    expect(typeof core.transferredFrameMessage).toBe("function");
    expect(typeof core.writeDicomwebResponse).toBe("function");
    expect(typeof core.writeFrame).toBe("function");
    expect(typeof core.DicomwebClient).toBe("function");
    expect(typeof core.DicomwebError).toBe("function");
    expect(typeof core.readEvent).toBe("function");
  });
});
