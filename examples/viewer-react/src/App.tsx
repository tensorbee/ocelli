import {
  readKeyObject,
  readSecondaryCapture,
  VERSION,
  viewportId,
  writeKeyObject,
  writeSecondaryCapture,
} from "@ocelli/core";
import { OcelliViewport } from "@ocelli/react";
import type { OcelliSession } from "@ocelli/react";
import { useState } from "react";

import { CorePanel } from "./core-panel.js";
import { LayoutPanel } from "./layout-panel.js";

/**
 * The example viewer.
 *
 * It starts as the smallest thing that proves the boundary and grows one
 * panel per landed viewport or tool story. It is also the manual smoke test
 * `/verify` points a human at, so it must run from a clean clone.
 *
 * A clean clone has no built wasm module, so the honest state is "core not
 * built" rather than a crash. F-002 (`bin/ocelli.sh wasm`) produces the
 * module. The wasm core section loads it in one worker when the package
 * directory is present. This factory still returns null. The stack viewport
 * stays absent until a caller supplies a session.
 */
function sessionFromModule(): OcelliSession | null {
  return null;
}

interface FrameOpenView {
  display: number | null;
  paddingCount: number;
  colour: string;
  lossy: string;
  burnedIn: string;
  windowLabel: string;
  creators: string;
}

function readView(value: unknown): FrameOpenView | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const record = value as Record<string, unknown>;
  if (typeof record.padding_count !== "number") {
    return null;
  }
  if (typeof record.colour !== "string") {
    return null;
  }
  if (typeof record.lossy !== "string") {
    return null;
  }
  if (typeof record.burned_in !== "string") {
    return null;
  }
  if (typeof record.window !== "string") {
    return null;
  }
  const display = record.display;
  if (display !== undefined && typeof display !== "number") {
    return null;
  }
  return {
    display: typeof display === "number" ? display : null,
    paddingCount: record.padding_count,
    colour: record.colour,
    lossy: record.lossy,
    burnedIn: record.burned_in,
    windowLabel: record.window,
    creators: creatorLine(record.creators),
  };
}

function creatorLine(value: unknown): string {
  if (!Array.isArray(value)) {
    return "";
  }
  const lines: string[] = [];
  for (const item of value) {
    if (typeof item !== "object" || item === null) {
      continue;
    }
    const record = item as Record<string, unknown>;
    if (
      typeof record.group !== "number" ||
      typeof record.slot !== "number" ||
      typeof record.creator !== "string"
    ) {
      continue;
    }
    lines.push(
      `${record.group.toString(16)}:${record.slot.toString(16)} ${record.creator}`,
    );
  }
  return lines.join(", ");
}

function problemSentence(text: string): string {
  try {
    const parsed: unknown = JSON.parse(text);
    if (typeof parsed === "object" && parsed !== null && "error" in parsed) {
      const code = (parsed as { error?: unknown }).error;
      if (typeof code === "string") {
        return `The frame could not be opened (${code}).`;
      }
    }
  } catch {
    return "The frame could not be opened.";
  }
  return "The frame could not be opened.";
}

interface SeriesPlace {
  input: number;
  frame: number;
}

interface SeriesGroupView {
  order: string;
  frames: number;
  slots: SeriesPlace[][];
  timing: string;
  milliseconds: number | null;
  framesPerSecond: number | null;
  playback: string;
  sheared: boolean;
  nonUniform: boolean;
}

interface SeriesView {
  groups: SeriesGroupView[];
}

function readSeries(value: unknown): SeriesView | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const record = value as Record<string, unknown>;
  if (!Array.isArray(record.groups)) {
    return null;
  }
  const groups: SeriesGroupView[] = [];
  for (const item of record.groups) {
    const group = readSeriesGroup(item);
    if (group === null) {
      return null;
    }
    groups.push(group);
  }
  return { groups };
}

function readSeriesGroup(value: unknown): SeriesGroupView | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const record = value as Record<string, unknown>;
  const slots = readSeriesSlots(record.slots);
  if (slots === null) {
    return null;
  }
  if (
    typeof record.order !== "string" ||
    typeof record.frames !== "number" ||
    typeof record.timing !== "string" ||
    typeof record.playback !== "string" ||
    typeof record.sheared !== "boolean" ||
    typeof record.non_uniform_step !== "boolean"
  ) {
    return null;
  }
  const milliseconds = record.milliseconds;
  const framesPerSecond = record.frames_per_second;
  if (milliseconds !== undefined && typeof milliseconds !== "number") {
    return null;
  }
  if (framesPerSecond !== undefined && typeof framesPerSecond !== "number") {
    return null;
  }
  return {
    order: record.order,
    frames: record.frames,
    slots,
    timing: record.timing,
    milliseconds: typeof milliseconds === "number" ? milliseconds : null,
    framesPerSecond: typeof framesPerSecond === "number" ? framesPerSecond : null,
    playback: record.playback,
    sheared: record.sheared,
    nonUniform: record.non_uniform_step,
  };
}

function readSeriesSlots(value: unknown): SeriesPlace[][] | null {
  if (!Array.isArray(value)) {
    return null;
  }
  const slots: SeriesPlace[][] = [];
  for (const slot of value) {
    if (!Array.isArray(slot)) {
      return null;
    }
    const places: SeriesPlace[] = [];
    for (const place of slot) {
      if (typeof place !== "object" || place === null) {
        return null;
      }
      const record = place as Record<string, unknown>;
      if (typeof record.input !== "number" || typeof record.frame !== "number") {
        return null;
      }
      places.push({ input: record.input, frame: record.frame });
    }
    slots.push(places);
  }
  return slots;
}

function seriesProblem(text: string): string {
  try {
    const parsed: unknown = JSON.parse(text);
    if (typeof parsed === "object" && parsed !== null && "error" in parsed) {
      const record = parsed as { error?: unknown; part?: unknown; frame?: unknown };
      if (typeof record.error === "string") {
        if (typeof record.part === "number") {
          return `The series could not be opened (${record.error} at part ${record.part.toString()}).`;
        }
        if (typeof record.frame === "number") {
          return `The series could not be opened (${record.error} at frame ${record.frame.toString()}).`;
        }
        return `The series could not be opened (${record.error}).`;
      }
    }
  } catch {
    return "The series could not be opened.";
  }
  return "The series could not be opened.";
}

function orderLabel(order: string): string {
  if (order === "projected") {
    return "Projected";
  }
  if (order === "instance_number") {
    return "Instance number";
  }
  return order;
}

function timingLabel(timing: string): string {
  if (timing === "frame_time") {
    return "Frame time";
  }
  if (timing === "frame_time_vector") {
    return "Frame time vector";
  }
  if (timing === "recommended_display_frame_rate") {
    return "Recommended display frame rate";
  }
  if (timing === "cine_rate") {
    return "Cine rate";
  }
  if (timing === "unavailable") {
    return "Unavailable";
  }
  return timing;
}

function intervalLabel(group: SeriesGroupView): string {
  if (group.milliseconds !== null) {
    return `${group.milliseconds.toString()} ms`;
  }
  if (group.framesPerSecond !== null) {
    return `${group.framesPerSecond.toString()} fps`;
  }
  if (group.timing === "frame_time_vector") {
    return "per frame";
  }
  return "none";
}

function placeLabel(place: SeriesPlace): string {
  return `${place.input.toString()}.${place.frame.toString()}`;
}

function frameLine(group: SeriesGroupView): string {
  const labels: string[] = [];
  for (const slot of group.slots) {
    for (const place of slot) {
      labels.push(placeLabel(place));
    }
  }
  if (labels.length === 0) {
    return "none";
  }
  return labels.join(" then ");
}

function slotLengths(group: SeriesGroupView): string {
  if (group.slots.length === 0) {
    return "none";
  }
  return group.slots.map((slot) => slot.length.toString()).join(", ");
}

function SeriesOpenPanel() {
  const [view, setView] = useState<SeriesView | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [playback, setPlayback] = useState<string[]>([]);

  async function run(body: BodyInit): Promise<void> {
    setPending(true);
    setProblem(null);
    try {
      const response = await fetch("/series-open", { method: "POST", body });
      const text = await response.text();
      if (!response.ok) {
        setProblem(seriesProblem(text));
        return;
      }
      const parsed: unknown = JSON.parse(text);
      const next = readSeries(parsed);
      if (next === null) {
        setProblem("The series-open response could not be read.");
        return;
      }
      setView(next);
      setPlayback(next.groups.map((group) => group.playback));
    } catch {
      setProblem("The series-open helper is not running.");
    } finally {
      setPending(false);
    }
  }

  return (
    <section>
      <h2>Series open</h2>
      <p>
        The synthetic button sends three built-in objects. Two are axial slices
        at different positions and one is sagittal. A file you choose is posted
        to the helper on this machine and is not saved. A shown position is the
        frame list index and the frame inside that object.
      </p>
      <p>
        <button
          type="button"
          disabled={pending}
          onClick={() => {
            void run("synthetic");
          }}
        >
          Synthetic series
        </button>{" "}
        <input
          type="file"
          multiple
          disabled={pending}
          aria-label="Choose Part 10 files"
          onChange={(event) => {
            const files = event.currentTarget.files;
            if (files === null || files.length === 0) {
              return;
            }
            const form = new FormData();
            for (const file of files) {
              form.append("file", file);
            }
            void run(form);
          }}
        />
      </p>
      {problem !== null && <p>{problem}</p>}
      {view !== null && (
        <div>
          <p>{view.groups.length.toString()} groups</p>
          {view.groups.map((group, index) => (
            <div key={index}>
              <p>
                Group {(index + 1).toString()}: {orderLabel(group.order)}. Frames{" "}
                {frameLine(group)}. Slot lengths {slotLengths(group)}.
              </p>
              <p>
                Timing {timingLabel(group.timing)}. Interval {intervalLabel(group)}.
              </p>
              <p>
                Sheared {group.sheared ? "yes" : "no"}. Non-uniform step{" "}
                {group.nonUniform ? "yes" : "no"}.
              </p>
              <label>
                Playback{" "}
                <select
                  aria-label={`Playback for group ${(index + 1).toString()}`}
                  value={playback[index] ?? group.playback}
                  onChange={(event) => {
                    const value = event.currentTarget.value;
                    setPlayback((current) => {
                      const next = current.slice();
                      next[index] = value;
                      return next;
                    });
                  }}
                >
                  <option value="loop">Loop</option>
                  <option value="sweep">Bounce</option>
                </select>
              </label>
            </div>
          ))}
          <p>The control starts from the file. Playing images stays on the folder page.</p>
        </div>
      )}
    </section>
  );
}

interface OverlayView {
  rows: number;
  columns: number;
  hit: boolean;
  hidden: boolean;
  outside: number;
  presentation: number | null;
  mask: number[] | null;
}

function readOverlay(value: unknown): OverlayView | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const record = value as Record<string, unknown>;
  if (typeof record.rows !== "number" || typeof record.columns !== "number") {
    return null;
  }
  if (typeof record.hit !== "boolean" || typeof record.hidden !== "boolean") {
    return null;
  }
  if (typeof record.outside !== "number") {
    return null;
  }
  const presentation = record.presentation;
  if (presentation !== null && typeof presentation !== "number") {
    return null;
  }
  const mask = readMask(record.mask);
  if (mask === undefined) {
    return null;
  }
  return {
    rows: record.rows,
    columns: record.columns,
    hit: record.hit,
    hidden: record.hidden,
    outside: record.outside,
    presentation: typeof presentation === "number" ? presentation : null,
    mask,
  };
}

function readMask(value: unknown): number[] | null | undefined {
  if (value === null) {
    return null;
  }
  if (!Array.isArray(value)) {
    return undefined;
  }
  const bits: number[] = [];
  for (const item of value) {
    if (item !== 0 && item !== 1) {
      return undefined;
    }
    bits.push(item);
  }
  return bits;
}

function overlayProblem(text: string): string {
  try {
    const parsed: unknown = JSON.parse(text);
    if (typeof parsed === "object" && parsed !== null && "error" in parsed) {
      const code = (parsed as { error?: unknown }).error;
      if (typeof code === "string") {
        return `The overlay could not be read (${code}).`;
      }
    }
  } catch {
    return "The overlay could not be read.";
  }
  return "The overlay could not be read.";
}

function OverlayPanel() {
  const [view, setView] = useState<OverlayView | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  async function run(body: BodyInit): Promise<void> {
    setPending(true);
    setProblem(null);
    try {
      const response = await fetch("/overlay", { method: "POST", body });
      const text = await response.text();
      if (!response.ok) {
        setProblem(overlayProblem(text));
        return;
      }
      const parsed: unknown = JSON.parse(text);
      const next = readOverlay(parsed);
      if (next === null) {
        setProblem("The overlay response could not be read.");
        return;
      }
      setView(next);
    } catch {
      setProblem("The overlay helper is not running.");
    } finally {
      setPending(false);
    }
  }

  return (
    <section>
      <h2>Overlay</h2>
      <p>
        The synthetic button posts a built-in 4 by 4 object. One overlay bit is
        set at image (1, 1) and the rectangular shutter hides that pixel. A
        file you choose is posted to the helper on this machine and is not
        saved.
      </p>
      <p>
        <button
          type="button"
          disabled={pending}
          onClick={() => {
            void run("synthetic");
          }}
        >
          Synthetic overlay
        </button>{" "}
        <input
          type="file"
          disabled={pending}
          aria-label="Choose a Part 10 file for overlay"
          onChange={(event) => {
            const file = event.currentTarget.files?.item(0);
            if (file !== undefined && file !== null) {
              void run(file);
            }
          }}
        />
      </p>
      {problem !== null && <p>{problem}</p>}
      {view !== null && (
        <div>
          <p>
            {view.hit
              ? "Overlay hit at (1, 1)."
              : "Overlay misses (1, 1)."}
          </p>
          <p>
            {view.hidden
              ? "Shutter hides (1, 1)."
              : "Shutter leaves (1, 1) open."}
          </p>
          <p>Outside {view.outside.toString()}.</p>
          {view.presentation === null ? (
            <p>The neutral value is absent.</p>
          ) : (
            <p>Neutral value {view.presentation.toString()}.</p>
          )}
          {view.mask !== null && <p>Mask {view.mask.join(" ")}</p>}
        </div>
      )}
    </section>
  );
}

interface TierRow {
  tier: string;
  nearest: string;
  linear: string;
}

interface InterpolateView {
  nearest: number;
  linear: number;
  tiers: TierRow[];
}

function readInterpolate(value: unknown): InterpolateView | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const record = value as Record<string, unknown>;
  if (typeof record.nearest !== "number" || typeof record.linear !== "number") {
    return null;
  }
  if (!Array.isArray(record.tiers) || record.tiers.length !== 3) {
    return null;
  }
  const tiers: TierRow[] = [];
  for (const item of record.tiers) {
    if (typeof item !== "object" || item === null) {
      return null;
    }
    const row = item as Record<string, unknown>;
    if (
      typeof row.tier !== "string" ||
      typeof row.nearest !== "string" ||
      typeof row.linear !== "string"
    ) {
      return null;
    }
    tiers.push({
      tier: row.tier,
      nearest: row.nearest,
      linear: row.linear,
    });
  }
  return {
    nearest: record.nearest,
    linear: record.linear,
    tiers,
  };
}

function InterpolatePanel() {
  const [view, setView] = useState<InterpolateView | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [mode, setMode] = useState<"nearest" | "linear">("nearest");

  async function run(): Promise<void> {
    setPending(true);
    setProblem(null);
    try {
      const response = await fetch("/interpolate", {
        method: "POST",
        body: "synthetic",
      });
      const text = await response.text();
      if (!response.ok) {
        setProblem("The interpolation request was refused.");
        return;
      }
      const parsed: unknown = JSON.parse(text);
      const next = readInterpolate(parsed);
      if (next === null) {
        setProblem("The interpolation response could not be read.");
        return;
      }
      setView(next);
    } catch {
      setProblem("The interpolation helper is not running.");
    } finally {
      setPending(false);
    }
  }

  const sentence =
    view === null
      ? null
      : mode === "linear"
        ? `Linear ${view.linear.toString()}.`
        : `Nearest ${view.nearest.toString()}.`;

  return (
    <section>
      <h2>Interpolation</h2>
      <p>
        The synthetic button samples a 2 by 2 display image at the centre of
        the four texels. Nearest stays on one corner. Linear is the mean of
        the four values.
      </p>
      <p>
        <button
          type="button"
          disabled={pending}
          onClick={() => {
            void run();
          }}
        >
          Synthetic sample
        </button>
      </p>
      {problem !== null && <p>{problem}</p>}
      {view !== null && sentence !== null && (
        <div>
          <label>
            Mode{" "}
            <select
              value={mode}
              onChange={(event) => {
                setMode(
                  event.currentTarget.value === "linear" ? "linear" : "nearest",
                );
              }}
            >
              <option value="nearest">Nearest</option>
              <option value="linear">Linear</option>
            </select>
          </label>
          <p>{sentence}</p>
          {view.tiers.map((row) => (
            <p key={row.tier}>
              Tier {row.tier} nearest {row.nearest}. Tier {row.tier} linear{" "}
              {row.linear}.
            </p>
          ))}
        </div>
      )}
    </section>
  );
}

interface CalibrationView {
  source: string;
  rowMm: number;
  columnMm: number;
  suvLbm: number;
  suvBsa: number;
  lengthRatio: number;
  contourVolume: number;
}

function readCalibration(value: unknown): CalibrationView | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const record = value as Record<string, unknown>;
  if (
    typeof record.source !== "string" ||
    typeof record.rowMm !== "number" ||
    typeof record.columnMm !== "number" ||
    typeof record.suvLbm !== "number" ||
    typeof record.suvBsa !== "number" ||
    typeof record.lengthRatio !== "number" ||
    typeof record.contourVolume !== "number"
  ) {
    return null;
  }
  return {
    source: record.source,
    rowMm: record.rowMm,
    columnMm: record.columnMm,
    suvLbm: record.suvLbm,
    suvBsa: record.suvBsa,
    lengthRatio: record.lengthRatio,
    contourVolume: record.contourVolume,
  };
}

function CalibrationPanel() {
  const [view, setView] = useState<CalibrationView | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  async function run(): Promise<void> {
    setPending(true);
    setProblem(null);
    try {
      const response = await fetch("/calibration", {
        method: "POST",
        body: "synthetic",
      });
      const text = await response.text();
      if (!response.ok) {
        setProblem("The calibration request was refused.");
        return;
      }
      const parsed: unknown = JSON.parse(text);
      const next = readCalibration(parsed);
      if (next === null) {
        setProblem("The calibration response could not be read.");
        return;
      }
      setView(next);
    } catch {
      setProblem("The calibration helper is not running.");
    } finally {
      setPending(false);
    }
  }

  return (
    <section>
      <h2>Calibration</h2>
      <p>
        The synthetic button uses imager spacing 0.2 mm and a magnification
        factor of 2. Patient spacing is the imager spacing divided by that
        factor. SUV by lean body mass and by body surface area use the same
        decay as body-weight SUV. The length ratio is 4 over 2. The contour
        volume is spacing 5 mm times areas 10 and 20.
      </p>
      <p>
        <button
          type="button"
          disabled={pending}
          onClick={() => {
            void run();
          }}
        >
          Synthetic calibration
        </button>
      </p>
      {problem !== null && <p>{problem}</p>}
      {view !== null && (
        <div>
          <p>Source {view.source}</p>
          <p>
            Spacing {view.rowMm.toString()} mm by {view.columnMm.toString()} mm
          </p>
          <p>SUVlbm {view.suvLbm.toString()}</p>
          <p>SUVbsa {view.suvBsa.toString()}</p>
          <p>Length ratio {view.lengthRatio.toString()}</p>
          <p>Contour volume {view.contourVolume.toString()}</p>
        </div>
      )}
    </section>
  );
}

function FrameOpenPanel() {
  const [view, setView] = useState<FrameOpenView | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  async function run(body: BodyInit): Promise<void> {
    setPending(true);
    setProblem(null);
    try {
      const response = await fetch("/frame-open", { method: "POST", body });
      const text = await response.text();
      if (!response.ok) {
        setProblem(problemSentence(text));
        return;
      }
      const parsed: unknown = JSON.parse(text);
      const next = readView(parsed);
      if (next === null) {
        setProblem("The frame-open response could not be read.");
        return;
      }
      setView(next);
    } catch {
      setProblem("The frame-open helper is not running.");
    } finally {
      setPending(false);
    }
  }

  return (
    <section>
      <h2>Frame open</h2>
      <p>
        The synthetic button opens a built-in monochrome sample. Stored 50,
        rescale slope 1, intercept 0, LINEAR_EXACT centre 100 and width 200.
        A file you choose is posted to the helper on this machine and is not
        saved.
      </p>
      <p>
        <button
          type="button"
          disabled={pending}
          onClick={() => {
            void run("synthetic");
          }}
        >
          Synthetic frame
        </button>{" "}
        <input
          type="file"
          disabled={pending}
          aria-label="Choose a Part 10 file"
          onChange={(event) => {
            const file = event.currentTarget.files?.item(0);
            if (file !== undefined && file !== null) {
              void run(file);
            }
          }}
        />
      </p>
      {problem !== null && <p>{problem}</p>}
      {view !== null && (
        <div>
          {view.display !== null && <p>Display {view.display}</p>}
          {view.display !== null ? (
            <p>Padding count {view.paddingCount}</p>
          ) : (
            <p>Declared padding ranges {view.paddingCount}</p>
          )}
          <p>Colour {view.colour}</p>
          <p>Window {view.windowLabel}</p>
          <p>Lossy {view.lossy}</p>
          <p>Burned in {view.burnedIn}</p>
          {view.creators !== "" && <p>Creators {view.creators}</p>}
        </div>
      )}
    </section>
  );
}

interface DocumentView {
  title: string;
  referencedClass: string;
  referencedInstance: string;
  pixels: string;
}

function DocumentPanel() {
  const [view, setView] = useState<DocumentView | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  function run(): void {
    setProblem(null);
    const keyBytes = writeKeyObject({
      title: "Key image",
      study: "2.25.1961",
      series: "2.25.1962",
      instance: "2.25.1963",
      referencedClass: "1.2.840.10008.5.1.4.1.1.2",
      referencedInstance: "2.25.1964",
    });
    const captureBytes = writeSecondaryCapture({
      study: "2.25.1965",
      series: "2.25.1966",
      instance: "2.25.1967",
      columns: 2,
      rows: 2,
      pixels: new Uint8Array([0, 1, 2, 3]),
    });
    if (keyBytes === null || captureBytes === null) {
      setView(null);
      setProblem("The document was refused.");
      return;
    }
    const key = readKeyObject(keyBytes);
    const capture = readSecondaryCapture(captureBytes);
    if (key === null || capture === null) {
      setView(null);
      setProblem("The document could not be read.");
      return;
    }
    setView({
      title: key.title,
      referencedClass: key.referencedClass,
      referencedInstance: key.referencedInstance,
      pixels: Array.from(capture.pixels).join(", "),
    });
  }

  return (
    <section>
      <h2>Key object and secondary capture</h2>
      <p>
        The synthetic button writes a TID 2010 key object titled Key image and
        a 2 by 2 secondary capture of bytes 0, 1, 2 and 3. The study, series
        and instance identifiers are synthetic 2.25 values. The capture is
        8-bit MONOCHROME2 and has no window and no patient module.
      </p>
      <p>
        <button
          type="button"
          onClick={() => {
            run();
          }}
        >
          Synthetic documents
        </button>
      </p>
      {problem !== null && <p>{problem}</p>}
      {view !== null && (
        <div>
          <p>Title {view.title}</p>
          <p>Referenced class {view.referencedClass}</p>
          <p>Referenced instance {view.referencedInstance}</p>
          <p>Pixels {view.pixels}</p>
        </div>
      )}
    </section>
  );
}

interface TiltView {
  orthogonal: string;
  spacingMm: number;
  lateralMm: number;
}

function readTilt(value: unknown): TiltView | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const record = value as Record<string, unknown>;
  if (
    record.orthogonal !== "sheared" ||
    typeof record.spacingMm !== "number" ||
    typeof record.lateralMm !== "number"
  ) {
    return null;
  }
  return {
    orthogonal: record.orthogonal,
    spacingMm: record.spacingMm,
    lateralMm: record.lateralMm,
  };
}

function TiltPanel() {
  const [view, setView] = useState<TiltView | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  async function run(): Promise<void> {
    setPending(true);
    setProblem(null);
    try {
      const response = await fetch("/tilt", {
        method: "POST",
        body: "synthetic",
      });
      const text = await response.text();
      if (!response.ok) {
        setProblem("The tilt request was refused.");
        return;
      }
      const parsed: unknown = JSON.parse(text);
      const next = readTilt(parsed);
      if (next === null) {
        setProblem("The tilt response could not be read.");
        return;
      }
      setView(next);
    } catch {
      setProblem("The tilt helper is not running.");
    } finally {
      setPending(false);
    }
  }

  return (
    <section>
      <h2>Tilted volume</h2>
      <p>
        The synthetic button sends a stack that steps 3 mm along the normal
        and 3 mm across the row axis. Orthogonal assembly refuses that stack.
        Tilted assembly keeps the projected spacing and the lateral length.
        The pixels stay in the slices they arrived in.
      </p>
      <p>
        <button
          type="button"
          disabled={pending}
          onClick={() => {
            void run();
          }}
        >
          Synthetic tilt
        </button>
      </p>
      {problem !== null && <p>{problem}</p>}
      {view !== null && (
        <div>
          <p>Orthogonal {view.orthogonal}</p>
          <p>Spacing {view.spacingMm.toString()} mm</p>
          <p>Lateral {view.lateralMm.toString()} mm</p>
        </div>
      )}
    </section>
  );
}

export function App() {
  const [ready, setReady] = useState(false);
  const session = sessionFromModule();

  return (
    <main>
      <header>
        <h1>Ocelli example viewer</h1>
        <p>
          <code>@ocelli/core</code> {VERSION} &middot;{" "}
          <span>{ready ? "core ready" : "core not built"}</span>
        </p>
      </header>

      {!ready && (
        <section>
          <h2>No core in this clone</h2>
          <p>
            The wasm module is built, not committed. Run{" "}
            <code>bin/ocelli.sh wasm</code> to produce{" "}
            <code>crates/ocelli-wasm/pkg</code>, then reload.
          </p>
          <p>
            The viewport below mounts its host element and reports the absent
            state. It starts a frame pump only when a session is supplied.
          </p>
        </section>
      )}

      <FrameOpenPanel />
      <SeriesOpenPanel />
      <OverlayPanel />
      <InterpolatePanel />
      <CalibrationPanel />
      <DocumentPanel />
      <TiltPanel />
      <LayoutPanel />
      <CorePanel onAvailable={setReady} />

      <section>
        <h2>Stack viewport</h2>
        <OcelliViewport
          id="stack-1"
          className="viewport"
          viewport={viewportId(1)}
          session={session}
        />
      </section>
    </main>
  );
}
