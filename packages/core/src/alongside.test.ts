import { describe, expect, it } from "vitest";

import { attachSideBySide } from "./alongside.js";
import type { SeamEvent } from "./seam.js";

interface FakeElement {
  readonly calls: Event[];
  dispatchEvent: (event: Event) => boolean;
}

function fake(): FakeElement {
  const calls: Event[] = [];
  return {
    calls,
    dispatchEvent(event) {
      calls.push(event);
      return true;
    },
  };
}

function target(element: FakeElement): EventTarget {
  return element as unknown as EventTarget;
}

function presented(): SeamEvent {
  return {
    name: "frame.presented",
    viewport: 1,
    seq: 1n,
    payload: { generation: 4, drawnWidth: 2, drawnHeight: 3, flags: 0 },
  };
}

describe("F-106 side by side", () => {
  it("enables only the Ocelli element", () => {
    const ocelli = fake();
    const neighbour = fake();
    const pair = attachSideBySide(target(ocelli), target(neighbour));
    expect(pair).not.toBeNull();
    expect(ocelli.calls.map((event) => event.type)).toEqual(["viewport.mounted"]);
    expect(neighbour.calls).toEqual([]);
    expect(pair?.ocelli.dispatch(presented())).toBe(true);
    expect(neighbour.calls).toEqual([]);
    neighbour.dispatchEvent(new CustomEvent("other.library"));
    expect(neighbour.calls.map((event) => event.type)).toEqual(["other.library"]);
    expect(pair?.neighbour).toBe(target(neighbour));
  });

  it("refuses one element used as both", () => {
    const element = fake();
    expect(attachSideBySide(target(element), target(element))).toBeNull();
    expect(element.calls).toEqual([]);
  });

  it("releases only the Ocelli element", () => {
    const ocelli = fake();
    const neighbour = fake();
    const pair = attachSideBySide(target(ocelli), target(neighbour));
    expect(pair).not.toBeNull();
    expect(pair?.release()).toBe(true);
    neighbour.dispatchEvent(new CustomEvent("other.library"));
    expect(neighbour.calls.map((event) => event.type)).toEqual(["other.library"]);
    expect(pair?.ocelli.dispatch(presented())).toBe(false);
    expect(pair?.release()).toBe(false);
    expect(ocelli.calls.map((event) => event.type)).toEqual([
      "viewport.mounted",
      "viewport.unmounted",
    ]);
  });

  it("leaves the neighbour alone when attach fails", () => {
    const ocelli = fake();
    const neighbour = fake();
    expect(attachSideBySide(target(ocelli), 1 as unknown as EventTarget)).toBeNull();
    expect(ocelli.calls).toEqual([]);
    expect(neighbour.calls).toEqual([]);

    ocelli.dispatchEvent = () => {
      throw new Error("mounted");
    };
    expect(attachSideBySide(target(ocelli), target(neighbour))).toBeNull();
    expect(neighbour.calls).toEqual([]);

    ocelli.dispatchEvent = (event) => {
      ocelli.calls.push(event);
      return true;
    };
    expect(attachSideBySide(target(ocelli), target(neighbour))).not.toBeNull();
  });

  it("keeps the first neighbour when attach is repeated", () => {
    const ocelli = fake();
    const neighbour = fake();
    const other = fake();
    const pair = attachSideBySide(target(ocelli), target(neighbour));
    expect(pair).not.toBeNull();
    expect(attachSideBySide(target(ocelli), target(other))).toBeNull();
    expect(pair?.ocelli.dispatch(presented())).toBe(true);
    expect(pair?.neighbour).toBe(target(neighbour));
    expect(other.calls).toEqual([]);
    expect(ocelli.calls.map((event) => event.type)).toEqual([
      "viewport.mounted",
      "frame.presented",
    ]);
  });
});
