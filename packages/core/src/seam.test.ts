import { describe, expect, it } from "vitest";

import { disableElement, dispatchOnElement, enableElement } from "./seam.js";
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

function presented(): SeamEvent {
  return {
    name: "frame.presented",
    viewport: 1,
    seq: 1n,
    payload: { generation: 4, drawnWidth: 2, drawnHeight: 3, flags: 0 },
  };
}

describe("F-105 element seam", () => {
  it("delivers a presented frame on the enabled element", () => {
    const element = fake();
    const seam = enableElement(element as unknown as EventTarget);
    expect(seam).not.toBeNull();
    expect(dispatchOnElement(element as unknown as EventTarget, presented())).toBe(true);
    expect(element.calls.map((event) => event.type)).toEqual([
      "viewport.mounted",
      "frame.presented",
    ]);
    const delivered = element.calls[1];
    if (!(delivered instanceof CustomEvent)) {
      throw new Error("missing presented event");
    }
    const detail: SeamEvent = delivered.detail as SeamEvent;
    expect(delivered.bubbles).toBe(false);
    expect(delivered.cancelable).toBe(false);
    expect(detail).toMatchObject({
      name: "frame.presented",
      payload: { generation: 4 },
    });
  });

  it("refuses a second enable of the same element", () => {
    const element = fake();
    const target = element as unknown as EventTarget;
    expect(enableElement(target)).not.toBeNull();
    expect(enableElement(target)).toBeNull();
    expect(element.calls.map((event) => event.type)).toEqual(["viewport.mounted"]);
  });

  it("stops delivery after disable", () => {
    const element = fake();
    const target = element as unknown as EventTarget;
    expect(enableElement(target)).not.toBeNull();
    expect(disableElement(target)).toBe(true);
    expect(dispatchOnElement(target, presented())).toBe(false);
    expect(disableElement(target)).toBe(false);
    expect(element.calls.map((event) => event.type)).toEqual([
      "viewport.mounted",
      "viewport.unmounted",
    ]);
  });

  it("refuses a name outside the catalogue", () => {
    const element = fake();
    const target = element as unknown as EventTarget;
    expect(enableElement(target)).not.toBeNull();
    const before = element.calls.length;
    const refused = { name: "not.an.event" } as unknown as SeamEvent;
    expect(dispatchOnElement(target, refused)).toBe(false);
    expect(element.calls).toHaveLength(before);
  });

  it("drops the registration when mounted delivery throws", () => {
    const element = fake();
    element.dispatchEvent = () => {
      throw new Error("closed");
    };
    const target = element as unknown as EventTarget;
    expect(enableElement(target)).toBeNull();
    element.dispatchEvent = (event) => {
      element.calls.push(event);
      return true;
    };
    expect(enableElement(target)).not.toBeNull();
    expect(element.calls.map((event) => event.type)).toEqual(["viewport.mounted"]);
  });

  it("keeps the element disabled when unmounted delivery throws", () => {
    const element = fake();
    const target = element as unknown as EventTarget;
    const seam = enableElement(target);
    expect(seam).not.toBeNull();
    element.dispatchEvent = () => {
      throw new Error("closed");
    };
    expect(seam?.disable()).toBe(true);
    element.dispatchEvent = (event) => {
      element.calls.push(event);
      return true;
    };
    expect(enableElement(target)).not.toBeNull();
  });
});
