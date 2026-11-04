// @vitest-environment jsdom
/**
 * The example page renders the tilt reply. These numbers are not the
 * fixture, so a panel that ignored the reply and printed 3 would fail.
 * The route test locks spacing 3 mm and lateral length 3 mm.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { App } from "./App.js";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("tilted volume", () => {
  it("shows the orthogonal refusal and the reply spacing", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        return new Response(
          "{\"orthogonal\":\"sheared\",\"spacingMm\":4,\"lateralMm\":5}",
          { status: 200 },
        );
      }),
    );
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Synthetic tilt" }));
    expect(await screen.findByText("Orthogonal sheared")).toBeTruthy();
    expect(screen.getByText("Spacing 4 mm")).toBeTruthy();
    expect(screen.getByText("Lateral 5 mm")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Stack viewport" })).toBeTruthy();
  });
});
