/**
 * Ocelli's event catalogue. F-103, HLD section 17.3 and decision D4.
 *
 * One table. The ring is drained once per frame. This module names those
 * events and decodes their payloads. It does not register a listener.
 *
 * Pinned cornerstone3D 5.8.2.
 * core Events.js sha256 5d27e4da38d427221021ebd57ab31aaf4ea04512ba2fe8a1e5ce1bc479a5e8dc
 * tools Events.js sha256 daa7d58d2e1538c5b0872705f06549e4b4bc1c5b40bce6fdb00cd8dc9b02ea26
 * Measured 50 core and 53 tool. Appendix B says 50 and 53.
 */

import { RING_EVENT_KIND } from "./generated.js";
import type { OcelliEvent } from "./ring.js";

export const EVENT_DELIVERY = ["ring", "shell", "unavailable"] as const;
export type EventDelivery = (typeof EVENT_DELIVERY)[number];

export const OCELLI_EVENTS = [
  "cache.evicted",
  "cache.displaced",
  "cache.refused",
  "cache.pressure",
  "cache.budgetChanged",
  "frame.presented",
  "viewport.mounted",
  "viewport.unmounted",
  "viewport.resized",
  "pointer.down",
  "pointer.up",
  "pointer.move",
  "pointer.drag",
  "pointer.wheel",
  "pointer.click",
  "pointer.doubleClick",
  "key.down",
  "key.up",
  "touch.start",
  "touch.drag",
  "touch.end",
  "touch.tap",
  "touch.swipe",
  "tool.activated",
  "tool.modeChanged",
  "annotation.added",
  "annotation.completed",
  "annotation.modified",
  "annotation.removed",
  "annotation.selectionChanged",
  "annotation.locked",
  "annotation.visibilityChanged",
  "annotation.rendered",
] as const;
export type OcelliEventName = (typeof OCELLI_EVENTS)[number];

export interface CacheKeyPayload {
  readonly series: bigint;
  readonly instance: number;
  readonly frame: number;
  readonly bytes: bigint;
}

export interface CachePressurePayload {
  readonly tier: 0 | 1 | 2;
  readonly used: bigint;
  readonly budget: bigint;
  readonly entries: number;
  readonly evicted: number;
  readonly refused: number;
}

export interface CacheBudgetChangedPayload {
  readonly tier: 0 | 1 | 2;
  readonly budget: bigint;
  readonly evictedBytes: bigint;
  readonly evicted: number;
  readonly dropped: number;
}

export interface FramePresentedPayload {
  readonly generation: number;
  readonly drawnWidth: number;
  readonly drawnHeight: number;
  readonly flags: number;
}

export type OcelliNamedEvent =
  | {
      readonly name: "cache.evicted";
      readonly viewport: number;
      readonly seq: bigint;
      readonly payload: CacheKeyPayload;
    }
  | {
      readonly name: "cache.displaced";
      readonly viewport: number;
      readonly seq: bigint;
      readonly payload: CacheKeyPayload;
    }
  | {
      readonly name: "cache.refused";
      readonly viewport: number;
      readonly seq: bigint;
      readonly payload: CacheKeyPayload;
    }
  | {
      readonly name: "cache.pressure";
      readonly viewport: number;
      readonly seq: bigint;
      readonly payload: CachePressurePayload;
    }
  | {
      readonly name: "cache.budgetChanged";
      readonly viewport: number;
      readonly seq: bigint;
      readonly payload: CacheBudgetChangedPayload;
    }
  | {
      readonly name: "frame.presented";
      readonly viewport: number;
      readonly seq: bigint;
      readonly payload: FramePresentedPayload;
    };

export interface CornerstoneEventRow {
  readonly cornerstone: string;
  readonly source: "core" | "tool";
  readonly ocelli: OcelliEventName | null;
  readonly delivery: EventDelivery;
  readonly producer: string | null;
  readonly reason: string | null;
}

/** An Ocelli event 5.8.2 does not name. It still has one owner. */
export interface UnmappedEvent {
  readonly name: OcelliEventName;
  readonly delivery: EventDelivery;
  readonly producer: string;
  readonly reason: string;
}

const CORE_ROWS: readonly CornerstoneEventRow[] = [
  { cornerstone: "ERROR_EVENT", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "errors use the section 23 record, not this ring" },
  { cornerstone: "CACHE_SIZE_EXCEEDED", source: "core", ocelli: "cache.pressure", delivery: "ring", producer: "ocelli-cache TelemetryEvent::Pressure", reason: "a full cache is the 5.8.2 event closest to a pressure crossing" },
  { cornerstone: "IMAGE_LOAD_ERROR", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "CAMERA_MODIFIED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "camera inputs are viewport state and the snapshot" },
  { cornerstone: "CAMERA_RESET", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "camera inputs are viewport state and the snapshot" },
  { cornerstone: "VOI_MODIFIED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "the window is viewport state" },
  { cornerstone: "PRESET_MODIFIED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "a preset is a command, not a delivered event" },
  { cornerstone: "DISPLAY_AREA_MODIFIED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "display area is the snapshot" },
  { cornerstone: "ELEMENT_DISABLED", source: "core", ocelli: "viewport.unmounted", delivery: "shell", producer: "shell viewport lifecycle", reason: null },
  { cornerstone: "ELEMENT_ENABLED", source: "core", ocelli: "viewport.mounted", delivery: "shell", producer: "shell viewport lifecycle", reason: null },
  { cornerstone: "IMAGE_RENDERED", source: "core", ocelli: "frame.presented", delivery: "ring", producer: "ocelli-viewport push_presented", reason: "the presented frame" },
  { cornerstone: "IMAGE_VOLUME_MODIFIED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli volume event yet" },
  { cornerstone: "IMAGE_VOLUME_LOADING_COMPLETED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli volume event yet" },
  { cornerstone: "IMAGE_LOADED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "a loaded frame arrives through the bulk handoff" },
  { cornerstone: "IMAGE_RETRIEVAL_STAGE", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "IMAGE_LOAD_FAILED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "VOLUME_VIEWPORT_NEW_VOLUME", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli volume event yet" },
  { cornerstone: "VOLUME_LOADED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli volume event yet" },
  { cornerstone: "VOLUME_LOADED_FAILED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli volume event yet" },
  { cornerstone: "IMAGE_CACHE_IMAGE_ADDED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "an admission is not a cache ring kind" },
  { cornerstone: "IMAGE_CACHE_IMAGE_REMOVED", source: "core", ocelli: "cache.evicted", delivery: "ring", producer: "ocelli-cache CacheEvent::Evicted", reason: "removal is the 5.8.2 event closest to frame-cache eviction" },
  { cornerstone: "VOLUME_CACHE_VOLUME_ADDED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "the volume cache is not the frame cache" },
  { cornerstone: "VOLUME_CACHE_VOLUME_REMOVED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "volume removal is not frame-cache eviction" },
  { cornerstone: "STACK_NEW_IMAGE", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "the selected slice is viewport state" },
  { cornerstone: "VOLUME_NEW_IMAGE", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli volume event yet" },
  { cornerstone: "PRE_STACK_NEW_IMAGE", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "the selected slice is viewport state" },
  { cornerstone: "IMAGE_SPACING_CALIBRATED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "spacing is geometry state" },
  { cornerstone: "VIEWPORT_NEW_IMAGE_SET", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "STACK_VIEWPORT_SCROLL", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "scroll position is viewport state" },
  { cornerstone: "STACK_SCROLL_OUT_OF_BOUNDS", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "scroll position is viewport state" },
  { cornerstone: "GEOMETRY_CACHE_GEOMETRY_ADDED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "GEOMETRY_CACHE_GEOMETRY_REMOVED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "VOLUME_VIEWPORT_SCROLL_OUT_OF_BOUNDS", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli volume event yet" },
  { cornerstone: "VOLUME_VIEWPORT_SCROLL", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli volume event yet" },
  { cornerstone: "CLIPPING_PLANES_UPDATED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "WEB_WORKER_PROGRESS", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "worker progress is not a ring kind" },
  { cornerstone: "COLORMAP_MODIFIED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "the colormap is viewport state" },
  { cornerstone: "DYNAMIC_VOLUME_DIMENSION_GROUP_CHANGED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli volume event yet" },
  { cornerstone: "DYNAMIC_VOLUME_TIME_POINT_INDEX_CHANGED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli volume event yet" },
  { cornerstone: "DYNAMIC_VOLUME_DIMENSION_GROUP_LOADED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli volume event yet" },
  { cornerstone: "DYNAMIC_VOLUME_TIME_POINT_LOADED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli volume event yet" },
  { cornerstone: "GEOMETRY_LOADED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "GEOMETRY_LOAD_PROGRESS", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "GEOMETRY_LOADED_FAILED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "ACTORS_CHANGED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "RENDER_BACKEND_CHANGED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "tier resolution is not an event" },
  { cornerstone: "WEBGL_CONTEXT_LOST", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "device loss is renderer state, not this catalogue" },
  { cornerstone: "WEBGL_CONTEXT_RESTORED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "device loss is renderer state, not this catalogue" },
  { cornerstone: "RENDER_PATH_ERROR", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "RENDERING_PIPELINE_CHANGED", source: "core", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
];

const TOOL_ROWS: readonly CornerstoneEventRow[] = [
  { cornerstone: "TOOL_ACTIVATED", source: "tool", ocelli: "tool.activated", delivery: "shell", producer: "shell tool state", reason: null },
  { cornerstone: "TOOLGROUP_VIEWPORT_ADDED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "TOOLGROUP_VIEWPORT_REMOVED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "TOOL_MODE_CHANGED", source: "tool", ocelli: "tool.modeChanged", delivery: "shell", producer: "shell tool state", reason: null },
  { cornerstone: "CROSSHAIR_TOOL_CENTER_CHANGED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "WORLD_CROSSHAIR_POINT_CHANGED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "WORLD_CROSSHAIR_POINT_CLEARED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "WORLD_CROSSHAIR_JUMPED_TO_POINT", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "SLICE_INTERSECTION_LINE_SELECTED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "SLICE_INTERSECTION_MANIPULATION_STARTED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "SLICE_INTERSECTION_MANIPULATION_ENDED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "SLICE_INTERSECTION_SLAB_THICKNESS_CHANGED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "VOLUMECROPPINGCONTROL_TOOL_CHANGED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "VOLUMECROPPING_TOOL_CHANGED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "STACK_PREFETCH_COMPLETE", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "ANNOTATION_ADDED", source: "tool", ocelli: "annotation.added", delivery: "shell", producer: "shell annotation layer", reason: null },
  { cornerstone: "ANNOTATION_COMPLETED", source: "tool", ocelli: "annotation.completed", delivery: "shell", producer: "shell annotation layer", reason: null },
  { cornerstone: "ANNOTATION_MODIFIED", source: "tool", ocelli: "annotation.modified", delivery: "shell", producer: "shell annotation layer", reason: null },
  { cornerstone: "ANNOTATION_REMOVED", source: "tool", ocelli: "annotation.removed", delivery: "shell", producer: "shell annotation layer", reason: null },
  { cornerstone: "ANNOTATION_SELECTION_CHANGE", source: "tool", ocelli: "annotation.selectionChanged", delivery: "shell", producer: "shell annotation layer", reason: null },
  { cornerstone: "ANNOTATION_LOCK_CHANGE", source: "tool", ocelli: "annotation.locked", delivery: "shell", producer: "shell annotation layer", reason: null },
  { cornerstone: "ANNOTATION_VISIBILITY_CHANGE", source: "tool", ocelli: "annotation.visibilityChanged", delivery: "shell", producer: "shell annotation layer", reason: null },
  { cornerstone: "ANNOTATION_RENDERED", source: "tool", ocelli: "annotation.rendered", delivery: "shell", producer: "shell annotation layer", reason: null },
  { cornerstone: "ANNOTATION_CUT_MERGE_PROCESS_COMPLETED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "ANNOTATION_INTERPOLATION_PROCESS_COMPLETED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "INTERPOLATED_ANNOTATIONS_REMOVED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "SEGMENTATION_MODIFIED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "SEGMENTATION_RENDERED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "SEGMENTATION_REPRESENTATION_ADDED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "SEGMENTATION_ADDED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "SEGMENTATION_REPRESENTATION_MODIFIED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "SEGMENTATION_REMOVED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "SEGMENTATION_REPRESENTATION_REMOVED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "SEGMENTATION_DATA_MODIFIED", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "HISTORY_UNDO", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "HISTORY_REDO", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "no Ocelli surface yet" },
  { cornerstone: "KEY_DOWN", source: "tool", ocelli: "key.down", delivery: "shell", producer: "shell keyboard", reason: null },
  { cornerstone: "KEY_UP", source: "tool", ocelli: "key.up", delivery: "shell", producer: "shell keyboard", reason: null },
  { cornerstone: "MOUSE_DOWN", source: "tool", ocelli: "pointer.down", delivery: "shell", producer: "shell pointer", reason: null },
  { cornerstone: "MOUSE_UP", source: "tool", ocelli: "pointer.up", delivery: "shell", producer: "shell pointer", reason: null },
  { cornerstone: "MOUSE_DOWN_ACTIVATE", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "a tool gesture is not pointer.down" },
  { cornerstone: "MOUSE_DRAG", source: "tool", ocelli: "pointer.drag", delivery: "shell", producer: "shell pointer", reason: null },
  { cornerstone: "MOUSE_MOVE", source: "tool", ocelli: "pointer.move", delivery: "shell", producer: "shell pointer", reason: null },
  { cornerstone: "MOUSE_CLICK", source: "tool", ocelli: "pointer.click", delivery: "shell", producer: "shell pointer", reason: null },
  { cornerstone: "MOUSE_DOUBLE_CLICK", source: "tool", ocelli: "pointer.doubleClick", delivery: "shell", producer: "shell pointer", reason: null },
  { cornerstone: "MOUSE_WHEEL", source: "tool", ocelli: "pointer.wheel", delivery: "shell", producer: "shell pointer", reason: null },
  { cornerstone: "TOUCH_START", source: "tool", ocelli: "touch.start", delivery: "shell", producer: "shell touch", reason: null },
  { cornerstone: "TOUCH_START_ACTIVATE", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "a tool gesture is not touch.start" },
  { cornerstone: "TOUCH_PRESS", source: "tool", ocelli: null, delivery: "unavailable", producer: null, reason: "a press is not touch.tap" },
  { cornerstone: "TOUCH_DRAG", source: "tool", ocelli: "touch.drag", delivery: "shell", producer: "shell touch", reason: null },
  { cornerstone: "TOUCH_END", source: "tool", ocelli: "touch.end", delivery: "shell", producer: "shell touch", reason: null },
  { cornerstone: "TOUCH_TAP", source: "tool", ocelli: "touch.tap", delivery: "shell", producer: "shell touch", reason: null },
  { cornerstone: "TOUCH_SWIPE", source: "tool", ocelli: "touch.swipe", delivery: "shell", producer: "shell touch", reason: null },
];

export const CORNERSTONE_EVENT_MAP: readonly CornerstoneEventRow[] = [
  ...CORE_ROWS,
  ...TOOL_ROWS,
];

/** Installed 5.8.2 events. Appendix B's 50 plus 53 names the same set. */
export const CORNERSTONE_EVENT_COUNT = 103;

export const UNMAPPED_EVENTS: readonly UnmappedEvent[] = [
  {
    name: "cache.displaced",
    delivery: "ring",
    producer: "ocelli-cache CacheEvent::Displaced",
    reason: "5.8.2 has no displacement event",
  },
  {
    name: "cache.refused",
    delivery: "ring",
    producer: "ocelli-cache CacheEvent::Refused",
    reason: "5.8.2 has no refusal event",
  },
  {
    name: "cache.budgetChanged",
    delivery: "ring",
    producer: "ocelli-cache TelemetryEvent::BudgetChanged",
    reason: "5.8.2 has no budget-changed event",
  },
  {
    name: "viewport.resized",
    delivery: "shell",
    producer: "shell ResizeObserver",
    reason: "5.8.2 has no element-resize event",
  },
];

const CACHE_KEY_BYTES = 24;
const PRESSURE_BYTES = 32;
const BUDGET_BYTES = 28;
const PRESENTED_BYTES = 16;

function payloadView(payload: Uint8Array): DataView {
  return new DataView(payload.buffer, payload.byteOffset, payload.byteLength);
}

function readTier(view: DataView): 0 | 1 | 2 | null {
  const value = view.getUint8(0);
  if (value === 0) {
    return 0;
  }
  if (value === 1) {
    return 1;
  }
  if (value === 2) {
    return 2;
  }
  return null;
}

/**
 * Cache key payload. `ocelli-cache` `CacheEvent::encode` writes series at 0,
 * instance at 8, frame at 12 and bytes at 16, all little-endian.
 */
function readCacheKey(payload: Uint8Array): CacheKeyPayload | null {
  if (payload.byteLength < CACHE_KEY_BYTES) {
    return null;
  }
  const view = payloadView(payload);
  return {
    series: view.getBigUint64(0, true),
    instance: view.getUint32(8, true),
    frame: view.getUint32(12, true),
    bytes: view.getBigUint64(16, true),
  };
}

/**
 * Pressure payload. `TelemetryEvent::encode` writes the tier at 0, used at 4,
 * budget at 12, entries at 20, evicted at 24 and refused at 28.
 */
function readPressure(payload: Uint8Array): CachePressurePayload | null {
  if (payload.byteLength < PRESSURE_BYTES) {
    return null;
  }
  const view = payloadView(payload);
  const tier = readTier(view);
  if (tier === null) {
    return null;
  }
  return {
    tier,
    used: view.getBigUint64(4, true),
    budget: view.getBigUint64(12, true),
    entries: view.getUint32(20, true),
    evicted: view.getUint32(24, true),
    refused: view.getUint32(28, true),
  };
}

/**
 * Budget payload. The same encoder writes the tier at 0, budget at 4,
 * evicted bytes at 12, evicted entries at 20 and dropped at 24.
 */
function readBudget(payload: Uint8Array): CacheBudgetChangedPayload | null {
  if (payload.byteLength < BUDGET_BYTES) {
    return null;
  }
  const view = payloadView(payload);
  const tier = readTier(view);
  if (tier === null) {
    return null;
  }
  return {
    tier,
    budget: view.getBigUint64(4, true),
    evictedBytes: view.getBigUint64(12, true),
    evicted: view.getUint32(20, true),
    dropped: view.getUint32(24, true),
  };
}

/**
 * Presented payload. `push_presented` writes generation, drawn width, drawn
 * height and flags, each a little-endian `u32`.
 */
function readPresented(payload: Uint8Array): FramePresentedPayload | null {
  if (payload.byteLength < PRESENTED_BYTES) {
    return null;
  }
  const view = payloadView(payload);
  return {
    generation: view.getUint32(0, true),
    drawnWidth: view.getUint32(4, true),
    drawnHeight: view.getUint32(8, true),
    flags: view.getUint32(12, true),
  };
}

function named(
  name: "cache.evicted" | "cache.displaced" | "cache.refused",
  event: OcelliEvent,
  payload: CacheKeyPayload,
): OcelliNamedEvent {
  return { name, viewport: event.viewport, seq: event.seq, payload };
}

/**
 * Decode one drained event. An unknown kind, a short payload or a tier byte
 * outside 0, 1 and 2 returns null. Nothing is guessed.
 */
export function decodeNamedEvent(event: OcelliEvent): OcelliNamedEvent | null {
  if (event.kind === RING_EVENT_KIND.evicted) {
    const payload = readCacheKey(event.payload);
    return payload === null ? null : named("cache.evicted", event, payload);
  }
  if (event.kind === RING_EVENT_KIND.displaced) {
    const payload = readCacheKey(event.payload);
    return payload === null ? null : named("cache.displaced", event, payload);
  }
  if (event.kind === RING_EVENT_KIND.refused) {
    const payload = readCacheKey(event.payload);
    return payload === null ? null : named("cache.refused", event, payload);
  }
  if (event.kind === RING_EVENT_KIND.pressure) {
    const payload = readPressure(event.payload);
    return payload === null
      ? null
      : { name: "cache.pressure", viewport: event.viewport, seq: event.seq, payload };
  }
  if (event.kind === RING_EVENT_KIND.budget_changed) {
    const payload = readBudget(event.payload);
    return payload === null
      ? null
      : {
          name: "cache.budgetChanged",
          viewport: event.viewport,
          seq: event.seq,
          payload,
        };
  }
  if (event.kind === RING_EVENT_KIND.presented) {
    const payload = readPresented(event.payload);
    return payload === null
      ? null
      : { name: "frame.presented", viewport: event.viewport, seq: event.seq, payload };
  }
  return null;
}
