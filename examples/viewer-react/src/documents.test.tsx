// @vitest-environment jsdom
/**
 * The example page shows the TID 2010 title and the four secondary-capture
 * bytes. The package tests lock the same fixture.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { App } from "./App.js";

afterEach(() => {
  cleanup();
});

describe("key object and secondary capture", () => {
  it("shows the title and pixels 0, 1, 2, 3", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Synthetic documents" }));
    expect(screen.getByText("Title Key image")).toBeTruthy();
    expect(screen.getByText("Referenced class 1.2.840.10008.5.1.4.1.1.2")).toBeTruthy();
    expect(screen.getByText("Referenced instance 2.25.1964")).toBeTruthy();
    expect(screen.getByText("Pixels 0, 1, 2, 3")).toBeTruthy();
  });
});
