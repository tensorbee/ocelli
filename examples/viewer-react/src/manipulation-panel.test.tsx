// @vitest-environment jsdom
import { ERROR_CODE, describeError } from "@ocelli/core";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { App } from "./App.js";
import { PAGE_MANIPULATION, manipulationReport } from "./manipulation-report.js";

afterEach(() => {
  cleanup();
});

describe("manipulation panel", () => {
  it("shows the manipulation report and does not relabel the crop", () => {
    render(<App />);
    expect(screen.getByText("One to one unavailable")).toBeTruthy();
    expect(screen.getByText("Histogram unavailable")).toBeTruthy();
    expect(screen.getByText("Four corners unavailable")).toBeTruthy();
    expect(screen.getByText("Overlay planes and display shutters stay on the overlay panel.")).toBeTruthy();
    fireEvent.change(screen.getByLabelText("Preset"), { target: { value: "bone" } });
    fireEvent.change(screen.getByLabelText("Colour map"), { target: { value: "grey" } });
    fireEvent.click(screen.getByRole("button", { name: "Show manipulation" }));
    const report = manipulationReport({
      ...PAGE_MANIPULATION,
      preset: "bone",
      colormap: "grey",
    });
    expect(screen.getByText(`Preset ${report.preset}`)).toBeTruthy();
    expect(screen.getByText(`Preset bytes ${String(report.presetBytes)}`)).toBeTruthy();
    expect(screen.getByText("Window centre 41")).toBeTruthy();
    expect(screen.getByText("Window width 401")).toBeTruthy();
    expect(screen.getByText("Region centre 5")).toBeTruthy();
    expect(screen.getByText(`Region width ${String(report.regionWidth)}`)).toBeTruthy();
    expect(screen.getByText("Rotate radians 2")).toBeTruthy();
    expect(screen.getByText("Quadrant 1")).toBeTruthy();
    expect(screen.getByText(`Flip bytes ${String(report.flipBytes)}`)).toBeTruthy();
    expect(screen.getByText("Marker A L P R")).toBeTruthy();
    expect(screen.getByText("Magnify scale 1")).toBeTruthy();
    expect(screen.getByText(`Colour map ${report.colormap}`)).toBeTruthy();
    expect(screen.getByText(`Colour map bytes ${String(report.colormapBytes)}`)).toBeTruthy();
    expect(screen.getByText("Scale pixels 20")).toBeTruthy();
    expect(screen.getByText("Crop planes 1")).toBeTruthy();
    expect(screen.getByText(`Warning ${describeError(ERROR_CODE.Unavailable)}`)).toBeTruthy();
  });
});
