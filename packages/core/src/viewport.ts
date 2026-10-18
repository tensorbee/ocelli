/**
 * Viewport, scene and tool types for the public API.
 *
 * Decision D1, HLD section 2.1: a new API, not a cornerstone-shaped shim.
 * F-101 packs commands and copies state across the boundary. This module
 * names the values and does not call into wasm.
 */

/** Eleven stack commands, in the order the public API publishes them. */
export const STACK_COMMANDS = [
  "Fit",
  "Pan",
  "Zoom",
  "Rotate",
  "FlipHorizontal",
  "FlipVertical",
  "SetWindow",
  "ApplyPreset",
  "SetInvert",
  "SetColormap",
  "Scroll",
] as const;

/** One command a tool binding may name. */
export type StackCommand = (typeof STACK_COMMANDS)[number];

/**
 * Drawing availability for the resolved tier.
 *
 * The three states are the contract in `docs/lld/feature-availability.md`.
 * The plan word `full` is published as `available`. Tier C drawing stays
 * `unavailable` until F-X003. `degraded` is a declared fallback, and this
 * story declares none.
 */
export const TIER_AVAILABILITY = ["available", "degraded", "unavailable"] as const;

/** One of `available`, `degraded` or `unavailable`. */
export type TierAvailability = (typeof TIER_AVAILABILITY)[number];

/** The documented tier C drawing answer. */
export const TIER_C_DRAWING: TierAvailability = "unavailable";

/** How many numbers HLD section 17.4 stores in `camera`. */
export const CAMERA_COMPONENTS = 16;

/**
 * Section 17.4's camera, column-major, sixteen numbers.
 *
 * F-050 owns any camera description beyond this list.
 */
export type Camera = readonly [
  number, number, number, number,
  number, number, number, number,
  number, number, number, number,
  number, number, number, number,
];

/**
 * Section 17.4's readback fields, in the order the struct declares them.
 *
 * F-101 copies the bytes. This list is the order a reader checks against.
 */
export const VIEWPORT_STATE_FIELDS = [
  "camera",
  "voiCenter",
  "voiWidth",
  "sliceIndex",
  "numSlices",
  "flags",
] as const;

declare const viewportIdBrand: unique symbol;

/**
 * A viewport identity.
 *
 * A plain number stays a slice index. An `{ id: number }` value stays some
 * other id. Only `viewportId` mints this type.
 */
export type ViewportId = number & {
  readonly [viewportIdBrand]: "ViewportId";
};

/** Mint a viewport id. The brand is erased at runtime. */
export function viewportId(id: number): ViewportId {
  return id as ViewportId;
}

/**
 * Ordered viewport ids.
 *
 * The order is the order of addition. The scene holds no pixels and no device.
 */
export interface Scene {
  /** Viewport ids, earliest first. */
  readonly viewports: readonly ViewportId[];
}

/**
 * The three VOI functions, in DICOM order.
 *
 * `linear` is `VoiFunction::Linear`. `linear_exact` is
 * `VoiFunction::LinearExact`. `sigmoid` is `VoiFunction::Sigmoid`.
 */
export const VOI_FUNCTIONS = ["linear", "linear_exact", "sigmoid"] as const;

/** One VOI function a window can name. */
export type VoiFunction = (typeof VOI_FUNCTIONS)[number];

/** The three window presets F-046 defines. No clinical-standard claim. */
export const WINDOW_PRESETS = ["soft_tissue", "lung", "bone"] as const;

/** One named window preset. */
export type WindowPreset = (typeof WINDOW_PRESETS)[number];

/** The two colour tables F-044 already uploads. */
export const COLORMAPS = ["grey", "stops"] as const;

/** One colour table. */
export type Colormap = (typeof COLORMAPS)[number];

/**
 * An explicit presentation shape.
 *
 * `inverse` asks for inversion. `identity` asks for none. Neither value
 * XORs with photometric interpretation.
 */
export const INVERT = ["identity", "inverse"] as const;

/** The presentation shape `SetInvert` carries. */
export type Invert = (typeof INVERT)[number];

/**
 * Documented stack viewport state.
 *
 * It does not draw. The fields are the view inputs, the presentation inputs,
 * and the slice index and length.
 */
export interface StackViewport {
  /** Identity inside a scene. */
  readonly id: ViewportId;
  /** Drawing availability for the resolved tier. */
  readonly drawing: TierAvailability;
  /** Section 17.4's sixteen camera numbers. */
  readonly camera: Camera;
  /** VOI window centre. */
  readonly voiCenter: number;
  /** VOI window width. */
  readonly voiWidth: number;
  /** `linear`, `linear_exact` or `sigmoid`. */
  readonly voiFunction: VoiFunction;
  /** Selected frame, stored order. */
  readonly sliceIndex: number;
  /** Length of the stored sequence. */
  readonly numSlices: number;
  /** `grey` or `stops`. */
  readonly colormap: Colormap;
  /** Selected window preset, when one is applied. */
  readonly preset: WindowPreset | null;
  /** Explicit presentation shape. Flag bit 0 is the packed form of `inverse`. */
  readonly invert: Invert;
}

/**
 * A tool name mapped to one stack command.
 *
 * The gesture that selects the tool stays in the shell. This value does not
 * run it. Hit-testing stays in `ocelli-geom`.
 */
export interface ToolBinding {
  /** Caller-chosen tool name. Not one of the sixty-three cornerstone classes. */
  readonly tool: string;
  /** The command the tool issues. */
  readonly command: StackCommand;
}
