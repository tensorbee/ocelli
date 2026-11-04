// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { App } from "./App.js";

afterEach(() => {
  cleanup();
});

describe("measurement panel", () => {
  it("shows the measurement report without a second export control", () => {
    render(<App />);
    expect(screen.getByText("Metadata browser unavailable")).toBeTruthy();
    expect(screen.getByText("Preferences unavailable")).toBeTruthy();
    expect(screen.getByText("Secondary capture export stays on the document panel.")).toBeTruthy();
    expect(screen.getAllByRole("button", { name: "Synthetic documents" })).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: "Show measurement" }));
    expect(screen.getByText("Measurement 12.5")).toBeTruthy();
    expect(screen.getByText("Annotations 1")).toBeTruthy();
    expect(screen.getByText("Undo 0")).toBeTruthy();
    expect(screen.getByText("TID 1500 12.5")).toBeTruthy();
    expect(screen.getByText("Key image KEY")).toBeTruthy();
    expect(screen.getByText("Shortcut scroll-previous")).toBeTruthy();
    expect(screen.getByText("Touch touch.start")).toBeTruthy();
    fireEvent.change(screen.getByLabelText("Shortcut key"), { target: { value: "ArrowDown" } });
    fireEvent.click(screen.getByRole("button", { name: "Show measurement" }));
    expect(screen.getByText("Shortcut scroll-next")).toBeTruthy();
    expect(screen.getByText("Annotations 1")).toBeTruthy();
    fireEvent.click(screen.getByLabelText("Shortcut shift"));
    fireEvent.click(screen.getByRole("button", { name: "Show measurement" }));
    expect(screen.getByText("Shortcut pan-down")).toBeTruthy();
  });
});
