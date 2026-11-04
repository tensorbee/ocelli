import {
  COLORMAPS,
  ERROR_CODE,
  WINDOW_PRESETS,
  cropPlanes,
  describeError,
  magnifyCircle,
  orientationMarker,
  orientationQuadrant,
  packCommands,
  rotationDrag,
  scaleLength,
  windowDrag,
  windowRegion,
} from "@ocelli/core";
import type { Colormap, WindowPreset } from "@ocelli/core";

/** Caller choices for one manipulation report. */
export interface ManipulationInput {
  readonly preset: WindowPreset;
  readonly colormap: Colormap;
  readonly centre: number;
  readonly width: number;
  readonly originX: number;
  readonly originY: number;
  readonly lastX: number;
  readonly lastY: number;
  readonly rotateLastX: number;
  readonly samples: readonly number[];
  readonly radians: number;
  readonly markerQuadrant: number;
  readonly magnifyScale: number | null;
  readonly millimetres: number;
  readonly pixelsPerMillimetre: number;
  readonly code: number;
}

/** Names and numbers from the existing manipulation exports. */
export interface ManipulationReport {
  readonly preset: string;
  readonly presetBytes: number;
  readonly presetPacked: Uint8Array;
  readonly centre: number | null;
  readonly width: number | null;
  readonly regionCentre: number | null;
  readonly regionWidth: number | null;
  readonly radians: number | null;
  readonly quadrant: 0 | 1 | 2 | 3 | null;
  readonly flipBytes: number;
  readonly flipPacked: Uint8Array;
  readonly marker: string | null;
  readonly magnifyScale: number | null;
  readonly colormap: string;
  readonly colormapBytes: number;
  readonly colormapPacked: Uint8Array;
  readonly scalePixels: number | null;
  readonly cropCount: number | null;
  readonly warning: string;
}

/** The fixed inputs the example button uses. */
export const PAGE_MANIPULATION: ManipulationInput = {
  preset: "lung",
  colormap: "stops",
  centre: 40,
  width: 400,
  originX: 0,
  originY: 0,
  lastX: 256,
  lastY: -256,
  rotateLastX: 512,
  samples: [0, 10],
  radians: 0,
  markerQuadrant: 1,
  magnifyScale: null,
  millimetres: 10,
  pixelsPerMillimetre: 2,
  code: ERROR_CODE.Unavailable,
};

export function isWindowPreset(value: string): value is WindowPreset {
  for (const preset of WINDOW_PRESETS) {
    if (preset === value) {
      return true;
    }
  }
  return false;
}

export function isColormap(value: string): value is Colormap {
  for (const colormap of COLORMAPS) {
    if (colormap === value) {
      return true;
    }
  }
  return false;
}

/** Preset, window, rotate, flip, marker, magnify, colour map, scale, crop, warning. */
export function manipulationReport(input: ManipulationInput): ManipulationReport {
  const presetPacked = packCommands(1, [{ kind: "ApplyPreset", preset: input.preset }]);
  const flipPacked = packCommands(1, [
    { kind: "FlipHorizontal", enabled: true },
    { kind: "FlipVertical", enabled: false },
  ]);
  const colormapPacked = packCommands(1, [{ kind: "SetColormap", colormap: input.colormap }]);
  const dragged = windowDrag(
    { centre: input.centre, width: input.width },
    input.originX,
    input.originY,
    input.lastX,
    input.lastY,
  );
  const region = windowRegion(input.samples);
  const turned = rotationDrag(input.radians, input.originX, input.rotateLastX);
  const marked = orientationMarker(["R", "A", "L", "P"], input.markerQuadrant);
  const glass = magnifyCircle(4, 5, 8, input.magnifyScale);
  const planes = cropPlanes([
    { origin: [0, 0, 0], normal: [0, 0, 1] },
  ]);
  return {
    preset: input.preset,
    presetBytes: presetPacked.byteLength,
    presetPacked,
    centre: dragged === null ? null : dragged.centre,
    width: dragged === null ? null : dragged.width,
    regionCentre: region === null ? null : region.centre,
    regionWidth: region === null ? null : region.width,
    radians: turned,
    quadrant: turned === null ? null : orientationQuadrant(turned),
    flipBytes: flipPacked.byteLength,
    flipPacked,
    marker: marked === null ? null : marked.join(" "),
    magnifyScale: glass === null ? null : glass.scale,
    colormap: input.colormap,
    colormapBytes: colormapPacked.byteLength,
    colormapPacked,
    scalePixels: scaleLength(input.millimetres, input.pixelsPerMillimetre),
    cropCount: planes === null ? null : planes.length,
    warning: describeError(input.code),
  };
}
