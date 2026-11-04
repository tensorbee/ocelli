import { useState } from "react";

import { COLORMAPS, WINDOW_PRESETS } from "@ocelli/core";

import {
  PAGE_MANIPULATION,
  isColormap,
  isWindowPreset,
  manipulationReport,
  type ManipulationReport,
} from "./manipulation-report.js";

function word(value: number | string | null): string {
  return value === null ? "refused" : String(value);
}

/** Presets, window, rotate, flip, marker, magnifier, colour map, scale, crop and the error sentence. */
export function ManipulationPanel() {
  const [preset, setPreset] = useState(PAGE_MANIPULATION.preset);
  const [colormap, setColormap] = useState(PAGE_MANIPULATION.colormap);
  const [report, setReport] = useState<ManipulationReport | null>(null);

  return (
    <section>
      <h2>Manipulation</h2>
      <p>One to one unavailable</p>
      <p>Histogram unavailable</p>
      <p>Four corners unavailable</p>
      <p>Overlay planes and display shutters stay on the overlay panel.</p>
      <p>
        <label>
          Preset
          <select
            aria-label="Preset"
            value={preset}
            onChange={(event) => {
              if (isWindowPreset(event.target.value)) {
                setPreset(event.target.value);
              }
            }}
          >
            {WINDOW_PRESETS.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </label>
        <label>
          Colour map
          <select
            aria-label="Colour map"
            value={colormap}
            onChange={(event) => {
              if (isColormap(event.target.value)) {
                setColormap(event.target.value);
              }
            }}
          >
            {COLORMAPS.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </label>
      </p>
      <p>
        <button
          type="button"
          onClick={() => {
            setReport(manipulationReport({ ...PAGE_MANIPULATION, preset, colormap }));
          }}
        >
          Show manipulation
        </button>
      </p>
      {report !== null && (
        <div>
          <p>Preset {report.preset}</p>
          <p>Preset bytes {word(report.presetBytes)}</p>
          <p>Window centre {word(report.centre)}</p>
          <p>Window width {word(report.width)}</p>
          <p>Region centre {word(report.regionCentre)}</p>
          <p>Region width {word(report.regionWidth)}</p>
          <p>Rotate radians {word(report.radians)}</p>
          <p>Quadrant {word(report.quadrant)}</p>
          <p>Flip bytes {word(report.flipBytes)}</p>
          <p>Marker {word(report.marker)}</p>
          <p>Magnify scale {word(report.magnifyScale)}</p>
          <p>Colour map {report.colormap}</p>
          <p>Colour map bytes {word(report.colormapBytes)}</p>
          <p>Scale pixels {word(report.scalePixels)}</p>
          <p>Crop planes {word(report.cropCount)}</p>
          <p>Warning {report.warning}</p>
        </div>
      )}
    </section>
  );
}
