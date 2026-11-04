// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

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

  it("shows the TID number read back, not the stored document", async () => {
    const core = await import("@ocelli/core");
    const real = core.readTid1500;
    const spy = vi.spyOn(core, "readTid1500").mockImplementation((bytes) => {
      const items = real(bytes);
      if (items === null) {
        return null;
      }
      return items.map((item) => {
        if (item.valueType !== "NUM") {
          return item;
        }
        return { ...item, number: 3 };
      });
    });
    try {
      render(<App />);
      fireEvent.click(screen.getByRole("button", { name: "Show measurement" }));
      expect(spy).toHaveBeenCalled();
      expect(screen.getByText("Measurement 12.5")).toBeTruthy();
      expect(screen.getByText("TID 1500 3")).toBeTruthy();
    } finally {
      spy.mockRestore();
    }
  });
});
