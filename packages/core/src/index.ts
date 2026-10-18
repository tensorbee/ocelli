/**
 * `@ocelli/core`, the TypeScript shell.
 *
 * The shell owns everything the DOM touches. The core owns everything a pixel
 * touches. See `docs/hld/03-architecture-and-crates.md`.
 *
 * What lives here, and it is the correct home rather than a compromise
 * (`docs/hld/07-concurrency-and-typescript.md`):
 *
 * - DOM, pointer, touch and wheel events; canvas lifecycle; ResizeObserver
 * - The SVG annotation drawing layer
 * - Tool interaction state machines
 * - Framework bindings, DICOMweb fetch and authentication
 * - ONNX and SAM-backed AI tools, and the dcmjs bridge for TID 1500
 *
 * What does NOT live here: anything a pixel touches.
 *
 * F-021 publishes the bounded DICOMweb client and response sink API below.
 * F-035 publishes the section 24 handoff from `bulk.ts`.
 * F-100 publishes the viewport, scene and tool types from `viewport.ts`.
 * F-101 still owns the live wasm command channel and event ring.
 */

export {
  acceptTransferredFrame,
  exportDecodedFrame,
  transferredFrameMessage,
  writeDicomwebResponse,
  writeFrame,
} from "./bulk.js";
export type {
  BulkSink,
  DicomwebResponseKind,
  DicomwebResponseSink,
  FrameMessage,
  HandoffMeta,
  WasmMemory,
} from "./bulk.js";

export { DicomwebClient, DicomwebError } from "./dicomweb.js";
export type {
  DicomwebClientOptions,
  DicomwebErrorCode,
  QidoSearchOptions,
} from "./dicomweb.js";

export { readEvent, EVENT_STRIDE, HEADER_BYTES } from "./ring.js";
export type { DrainResult, OcelliEvent } from "./ring.js";

export {
  decodeRecord,
  describeError,
  ERROR_CODE,
  LOG_LEVEL,
  RECORD_BYTES,
  SEVERITY,
} from "./errors.js";
export type { OcelliRecord, RecordOperands } from "./errors.js";

export {
  PANIC_FALLBACK_CODE,
  PANIC_HEADER_BYTES,
  PANIC_MAGIC,
  PANIC_MESSAGE_CAPACITY,
  PANIC_RECORD_BYTES,
  PANIC_RECORD_VERSION,
  readPanicRecord,
} from "./panic.js";
export type { PanicMemory, PanicRecord } from "./panic.js";

export { CORE_OK, fatalFromPanic, isUsable, nextStatus } from "./fatal.js";
export type { CoreStatus } from "./fatal.js";

export {
  SIMD128_PROBE_MODULE,
  moduleValidates,
  sharedMemoryAvailable,
  wasmSimd128Supported,
} from "./capabilities.js";

export {
  CAMERA_COMPONENTS,
  COLORMAPS,
  INVERT,
  STACK_COMMANDS,
  TIER_AVAILABILITY,
  TIER_C_DRAWING,
  VIEWPORT_STATE_FIELDS,
  VOI_FUNCTIONS,
  WINDOW_PRESETS,
  viewportId,
} from "./viewport.js";
export type {
  Camera,
  Colormap,
  Invert,
  Scene,
  StackCommand,
  StackViewport,
  TierAvailability,
  ToolBinding,
  ViewportId,
  VoiFunction,
  WindowPreset,
} from "./viewport.js";

/** Package version, kept in step with the crate versions by `/release`. */
export const VERSION = "0.1.0";

/**
 * Whether a built core is present.
 *
 * The wasm module is produced by `bin/ocelli.sh wasm` into
 * `crates/ocelli-wasm/pkg` and is not committed, so a clean clone has no core.
 * The example viewer uses this to render an honest "core not built" state
 * instead of failing to start.
 */
export function coreAvailable(): boolean {
  return false;
}
