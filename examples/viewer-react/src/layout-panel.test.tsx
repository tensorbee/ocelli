// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { App } from "./App.js";

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(document, "fullscreenEnabled");
  Reflect.deleteProperty(document, "fullscreenElement");
  Reflect.deleteProperty(HTMLElement.prototype, "requestFullscreen");
});

describe("layout panel", () => {
  it("shows the layout report and the unavailable lines", () => {
    render(<App />);
    expect(screen.getByText("Prior studies unavailable")).toBeTruthy();
    expect(screen.getByText("Linked scroll unavailable")).toBeTruthy();
    expect(screen.getByText("Synced window unavailable")).toBeTruthy();
    expect(screen.getByText("MPR unavailable")).toBeTruthy();
    expect(screen.getByText("Slab unavailable")).toBeTruthy();
    expect(screen.getByText("Volume rendering unavailable")).toBeTruthy();
    expect(screen.getByText("Fusion unavailable")).toBeTruthy();
    expect(screen.getByText("Full screen unavailable")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Show layout" }));
    expect(screen.getByText("Grid lines 2")).toBeTruthy();
    expect(screen.getByText("Stack drag 1")).toBeTruthy();
    expect(screen.getByText("Horizontal reference 0,20 to 100,20")).toBeTruthy();
    expect(screen.getByText("Vertical reference 10,0 to 10,80")).toBeTruthy();
    expect(screen.getByText("Reference cursor 10,20")).toBeTruthy();
  });

  it("turns full screen on when the section becomes the active element", () => {
    Object.defineProperty(document, "fullscreenEnabled", {
      configurable: true,
      get: () => true,
    });
    Object.defineProperty(document, "fullscreenElement", {
      configurable: true,
      get: () => null,
    });
    HTMLElement.prototype.requestFullscreen = function requestFullscreen() {
      Object.defineProperty(document, "fullscreenElement", {
        configurable: true,
        get: () => this,
      });
      document.dispatchEvent(new Event("fullscreenchange"));
      return Promise.resolve();
    };
    render(<App />);
    expect(screen.getByText("Full screen off")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Full screen" }));
    expect(screen.getByText("Full screen on")).toBeTruthy();
  });
});
