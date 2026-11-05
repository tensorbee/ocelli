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
 * stays false until `loadCore` resolves true. F-198 is that load.
 * F-078 publishes arrow, label, key image and ETDRS display-list marks.
 */

export {
  acceptTransferredFrame,
  exportDecodedFrame,
  holdSessionBytes,
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
export { OcelliError } from "./ocelli-error.js";
export { postWadoInstance } from "./instance-bytes.js";
export type { WadoInstanceRequest } from "./instance-bytes.js";
export type {
  DicomwebClientOptions,
  DicomwebErrorCode,
  QidoSearchOptions,
} from "./dicomweb.js";
export type { OcelliErrorCode } from "./ocelli-error.js";

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

export {
  POINTER_BUTTONS,
  POINTER_MODIFIERS,
  TOOL_MODES,
  TOOL_REFUSALS,
  createToolRegistry,
} from "./tools.js";
export type {
  ModeChange,
  PointerBinding,
  PointerButton,
  PointerModifier,
  RegisteredTool,
  ToolGroupView,
  ToolMode,
  ToolOutcome,
  ToolRefusal,
  ToolRegistry,
} from "./tools.js";

export { DISPATCH_REASONS, createPointerDispatch } from "./pointer.js";
export type {
  DispatchFrame,
  DispatchReason,
  GestureBasis,
  PointerDispatch,
  PointerDispatchOptions,
  PointerDomEvent,
  PointerTarget,
  ShellGestureEvent,
} from "./pointer.js";

export {
  MANIPULATION_TOOLS,
  stackDrag,
  windowDrag,
  windowRegion,
} from "./manipulation.js";

export { ROTATION_TOOLS, orientationQuadrant, rotationDrag } from "./orientation.js";

export { magnifyCircle, orientationMarker, scaleLength } from "./magnify.js";

export {
  overlayGrid,
  segmentationIntersection,
  sliceIntersection,
} from "./intersection.js";

export { cropDrag, cropPlanes } from "./crop.js";

export { eraseNearest, mipIndex, sculptVertex } from "./sculpt.js";

export { contentTree, provenanceItems } from "./sr.js";

export {
  annotationsForFrame,
  createAnnotationState,
  putAnnotation,
  redoAnnotation,
  undoAnnotation,
} from "./annotation-state.js";

export { loadMeasurements, persistMeasurements } from "./measurement-persist.js";

export { readTid1500, writeTid1500 } from "./tid1500.js";
export type { TidSource } from "./tid1500.js";

export { readKeyObject, writeKeyObject } from "./tid2010.js";
export type { KeyObjectDocument, KeyObjectRequest } from "./tid2010.js";

export { readSecondaryCapture, writeSecondaryCapture } from "./secondary-capture.js";
export type { SecondaryCaptureImage, SecondaryCaptureRequest } from "./secondary-capture.js";

export { disableElement, dispatchOnElement, enableElement } from "./seam.js";
export type { ElementSeam, SeamEvent } from "./seam.js";

export { attachSideBySide } from "./alongside.js";
export type { SideBySide } from "./alongside.js";

export { enableFlagged, kindEnabled, setKindEnabled } from "./flag.js";
export type { ViewportKind } from "./flag.js";

export { shadowDivergence } from "./shadow.js";

export {
  DEFAULT_DECODE_WORKERS,
  MAX_DECODE_WORKERS,
  openDecodePool,
  threadingEscalation,
} from "./pool.js";
export type { DecodePool, WorkerHandle } from "./pool.js";

export { certifyRow } from "./matrix.js";
export type { MatrixInput, MatrixObservation, MatrixStatus, MatrixTier } from "./matrix.js";

export {
  startupArtifact,
  summariseColdStart,
  summariseFirstFrame,
} from "./startup.js";
export type { ColdPhases, StartupRole } from "./startup.js";

export { keyAction } from "./keys.js";
export type { KeyAction } from "./keys.js";

export { nextMigrationKind } from "./playbook.js";

export { mayClaimStable, removalMinorsRemaining } from "./release.js";
export type { StableClaim } from "./release.js";

export { classifyProvenance } from "./provenance.js";

export { rewriteListenerNames } from "./migrate.js";

export {
  COMMAND_CURSORS,
  CURSOR_PHASES,
  createCursorController,
  createToolState,
} from "./cursor.js";
export type {
  CursorController,
  CursorControllerOptions,
  CursorPair,
  CursorPhase,
  CursorStyle,
  ToolState,
  ToolStateEvent,
} from "./cursor.js";

export { ANNOTATION_REFUSALS, createAnnotationLayer } from "./annotation.js";
export type {
  AnnotationLayer,
  AnnotationLayerEvent,
  AnnotationMark,
  AnnotationPoint,
  AnnotationRefusal,
  AnnotationSyncResult,
} from "./annotation.js";

export { crosshairLines, referenceCursor, referenceLines } from "./crosshair.js";
export type { Segment } from "./crosshair.js";

export {
  MARK_REFUSALS,
  arrowMark,
  etdrsMarks,
  etdrsRegion,
  keyImageMark,
  labelMark,
} from "./marks.js";
export type { EtdrsRegion, MarkRefusal, MarkResult, MarksResult } from "./marks.js";

export {
  SNAPSHOT_ERROR,
  SNAPSHOT_MAGIC,
  SNAPSHOT_VERSION,
  parseSnapshot,
  restoreSessionSnapshot,
  sessionSnapshot,
  sessionSnapshotError,
} from "./snapshot.js";
export {
  CORNERSTONE_EVENT_COUNT,
  CORNERSTONE_EVENT_MAP,
  EVENT_DELIVERY,
  OCELLI_EVENTS,
  UNMAPPED_EVENTS,
  decodeNamedEvent,
} from "./events.js";
export type {
  CameraSnapshot,
  DisplayAreaSnapshot,
  EvidenceSnapshot,
  ModalitySnapshot,
  ParsedSnapshot,
  PhotometricSnapshot,
  PresentationSnapshot,
  SessionSnapshotExports,
  ViewportSnapshot,
  VoiSnapshot,
} from "./snapshot.js";
export type {
  CacheBudgetChangedPayload,
  CacheKeyPayload,
  CachePressurePayload,
  CornerstoneEventRow,
  EventDelivery,
  FramePresentedPayload,
  OcelliEventName,
  OcelliNamedEvent,
  UnmappedEvent,
} from "./events.js";

/** Package version, kept in step with the crate versions by `/release`. */
export const VERSION = "0.1.0";

export { coreAvailable, loadCore } from "./load.js";
