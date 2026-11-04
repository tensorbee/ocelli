import {
  overlayGrid,
  referenceCursor,
  referenceLines,
  stackDrag,
} from "@ocelli/core";

/** Caller numbers for one layout report. They are not a patient study. */
export interface LayoutInput {
  readonly columns: number;
  readonly rows: number;
  readonly width: number;
  readonly height: number;
  readonly x: number;
  readonly y: number;
  readonly originY: number;
  readonly lastY: number;
}

/** The four calls this section is allowed to show. */
export interface LayoutReport {
  readonly gridLines: number | null;
  readonly dragStep: number | null;
  readonly horizontal: string | null;
  readonly vertical: string | null;
  readonly cursor: string | null;
}

/** The fixed inputs the example button uses. */
export const PAGE_LAYOUT: LayoutInput = {
  columns: 2,
  rows: 2,
  width: 100,
  height: 80,
  x: 10,
  y: 20,
  originY: 0,
  lastY: -256,
};

function pair(x: number, y: number): string {
  return `${String(x)},${String(y)}`;
}

function segmentText(segment: {
  readonly x0: number;
  readonly y0: number;
  readonly x1: number;
  readonly y1: number;
}): string {
  return `${pair(segment.x0, segment.y0)} to ${pair(segment.x1, segment.y1)}`;
}

/** `on` only when the browser flag is true and the section is the active element. */
export function fullscreenWord(
  enabled: boolean,
  active: boolean,
): "unavailable" | "on" | "off" {
  if (!enabled) {
    return "unavailable";
  }
  return active ? "on" : "off";
}

/** Grid, stack drag, reference lines and reference cursor. No second formula. */
export function layoutReport(input: LayoutInput): LayoutReport {
  const grid = overlayGrid(input.columns, input.rows, input.width, input.height);
  const lines = referenceLines(input.x, input.y, input.width, input.height);
  const cursor = referenceCursor(input.x, input.y, input.width, input.height);
  const step = stackDrag(input.originY, input.lastY);
  let horizontal: string | null = null;
  let vertical: string | null = null;
  if (lines !== null) {
    const across = lines[0];
    const down = lines[1];
    if (across !== undefined) {
      horizontal = segmentText(across);
    }
    if (down !== undefined) {
      vertical = segmentText(down);
    }
  }
  return {
    gridLines: grid === null ? null : grid.length,
    dragStep: step,
    horizontal,
    vertical,
    cursor: cursor === null ? null : pair(cursor.x, cursor.y),
  };
}
