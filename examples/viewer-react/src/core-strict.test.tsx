// @vitest-environment jsdom
/**
 * React StrictMode runs the worker effect twice in development. The second
 * setup must transfer a fresh canvas. Transferring the same element twice
 * throws, and the header would stay on "core not built".
 */
import { StrictMode } from "react";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { App } from "./App.js";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  Reflect.deleteProperty(HTMLCanvasElement.prototype, "transferControlToOffscreen");
});

describe("strict mode worker", () => {
  it("reports the second worker after the effect runs twice", async () => {
    const transferred = new WeakSet<object>();
    let transfers = 0;
    HTMLCanvasElement.prototype.transferControlToOffscreen = function transfer() {
      transfers += 1;
      if (transferred.has(this)) {
        throw new DOMException("canvas already transferred", "InvalidStateError");
      }
      transferred.add(this);
      return {} as OffscreenCanvas;
    };
    vi.stubGlobal(
      "Worker",
      class {
        onmessage: ((event: MessageEvent<unknown>) => void) | null = null;

        postMessage(data: { type?: string }): void {
          if (data.type !== "start") {
            return;
          }
          const status = {
            type: "status",
            available: true,
            tier: "B",
            published: true,
          };
          queueMicrotask(() => {
            this.onmessage?.({ data: status } as MessageEvent<unknown>);
          });
        }

        terminate(): void {}

        addEventListener(): void {}

        removeEventListener(): void {}
      },
    );

    render(
      <StrictMode>
        <App />
      </StrictMode>,
    );
    expect(await screen.findByText("core ready")).toBeTruthy();
    expect(screen.getByText("Tier B")).toBeTruthy();
    expect(transfers).toBe(2);
  });
});
