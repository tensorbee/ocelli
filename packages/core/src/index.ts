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
 * F-101 publishes command packing and the copied ring drain. `coreAvailable()`
 * stays false until a built module is loaded.
 */

export {
  acceptTransferredFrame,
  exportDecodedFrame,
  sessionBulkSink,
  transferredFrameMessage,
  writeDicomwebResponse,
  writeFrame,
  writeCommands,
  writeStack,
} from "./bulk.js";
export type {
  BulkSink,
  FrameCommitMeta,
  DicomwebResponseKind,
  DicomwebResponseSink,
  FrameMessage,
  HandoffMeta,
  SessionBulkExports,
  WasmMemory,
  StackFrameKey,
  SessionStackExports,
} from "./bulk.js";

export { DicomwebClient, DicomwebError } from "./dicomweb.js";
export type {
  DicomwebClientOptions,
  DicomwebErrorCode,
  QidoSearchOptions,
} from "./dicomweb.js";

export { drainRing, readEvent, HEADER_BYTES } from "./ring.js";
export type { DrainResult, OcelliEvent } from "./ring.js";

export {
  packCommands,
  packPointerFrame,
  sessionCommandSink,
  submitCommands,
} from "./commands.js";
export type {
  CommandSink,
  CommandWrite,
  PackedCommand,
  SessionCommandExports,
} from "./commands.js";

export {
  COLORMAP_BYTE,
  COMMAND_ID,
  COMMAND_PAYLOAD_BYTES,
  EVENT_HEADER_BYTES,
  EVENT_KIND_OFFSET,
  EVENT_PAYLOAD_BYTES,
  EVENT_PAYLOAD_OFFSET,
  EVENT_RING_CAPACITY_OFFSET,
  EVENT_RING_DROPPED_OFFSET,
  EVENT_RING_PENDING_OFFSET,
  EVENT_RING_TAIL_OFFSET,
  EVENT_SEQ_OFFSET,
  EVENT_STRIDE,
  EVENT_VIEWPORT_OFFSET,
  FLAG_ERROR,
  FLAG_INVERT,
  FLAG_LOADING,
  FLAG_REFINING,
  INVERT_BYTE,
  PRESET_BYTE,
  RING_EVENT_KIND,
  STACK_COMMANDS,
  STATE_CAMERA_COMPONENT_BYTES,
  STATE_CAMERA_COMPONENTS,
  STATE_CAMERA_OFFSET,
  STATE_FLAGS_OFFSET,
  STATE_NUM_SLICES_OFFSET,
  STATE_PADDING_OFFSET,
  STATE_SLICE_INDEX_OFFSET,
  STATE_VOI_CENTER_OFFSET,
  STATE_VOI_WIDTH_OFFSET,
  VIEWPORT_STATE_BYTES,
  VOI_FUNCTION_BYTE,
} from "./generated.js";
export type { StackCommand } from "./generated.js";

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
  TIER_AVAILABILITY,
  TIER_C_DRAWING,
  VIEWPORT_STATE_FIELDS,
  VOI_FUNCTIONS,
  WINDOW_PRESETS,
  readViewportState,
  viewportId,
} from "./viewport.js";
export type {
  Camera,
  Colormap,
  Invert,
  Scene,
  StackViewport,
  TierAvailability,
  ToolBinding,
  ViewportId,
  ViewportReadback,
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
