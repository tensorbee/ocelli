import { afterEach, describe, expect, it } from "vitest";

import { enableFlagged, kindEnabled, setKindEnabled } from "./flag.js";
import type { ViewportKind } from "./flag.js";
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

function restore(): void {
  setKindEnabled("volume", false);
  setKindEnabled("mpr", false);
  setKindEnabled("stack", false);
}

afterEach(restore);

describe("F-107 kind flags", () => {
  it("AC1 refuses MPR while stack is off", () => {
    expect(setKindEnabled("mpr", true)).toBe(false);
    expect(kindEnabled("mpr")).toBe(false);
    expect(kindEnabled("stack")).toBe(false);
    expect(kindEnabled("volume")).toBe(false);
  });

  it("AC2 keeps stack, then MPR, then volume", () => {
    const stack = fake();
    const mpr = fake();
    expect(setKindEnabled("stack", true)).toBe(true);
    expect(setKindEnabled("volume", true)).toBe(false);
    expect(kindEnabled("volume")).toBe(false);
    expect(setKindEnabled("mpr", true)).toBe(true);
    expect(setKindEnabled("volume", true)).toBe(true);
    const stackSeam = enableFlagged(target(stack), "stack");
    const mprSeam = enableFlagged(target(mpr), "mpr");
    expect(stackSeam).not.toBeNull();
    expect(mprSeam).not.toBeNull();
    expect(setKindEnabled("stack", false)).toBe(false);
    expect(kindEnabled("stack")).toBe(true);
    expect(stackSeam?.dispatch(presented())).toBe(true);
    expect(mprSeam?.dispatch(presented())).toBe(true);
    expect(setKindEnabled("volume", false)).toBe(true);
    expect(kindEnabled("volume")).toBe(false);
    expect(kindEnabled("mpr")).toBe(true);
    expect(stackSeam?.dispatch(presented())).toBe(true);
  });

  it("AC3 enables one element once", () => {
    const element = fake();
    expect(setKindEnabled("stack", true)).toBe(true);
    const first = enableFlagged(target(element), "stack");
    const second = enableFlagged(target(element), "stack");
    expect(first).not.toBeNull();
    expect(second).toBeNull();
    expect(element.calls.map((event) => event.type)).toEqual(["viewport.mounted"]);
    expect(first?.dispatch(presented())).toBe(true);
  });

  it("AC4 does not enable while the flag is off, and disables on the way off", () => {
    const element = fake();
    expect(enableFlagged(target(element), "stack")).toBeNull();
    expect(element.calls).toEqual([]);
    expect(setKindEnabled("stack", true)).toBe(true);
    const seam = enableFlagged(target(element), "stack");
    expect(seam).not.toBeNull();
    expect(setKindEnabled("stack", false)).toBe(true);
    expect(kindEnabled("stack")).toBe(false);
    expect(element.calls.map((event) => event.type)).toEqual([
      "viewport.mounted",
      "viewport.unmounted",
    ]);
    expect(seam?.dispatch(presented())).toBe(false);
  });

  it("AC4 leaves the element disabled when unmounted throws", () => {
    const element = fake();
    expect(setKindEnabled("stack", true)).toBe(true);
    const seam = enableFlagged(target(element), "stack");
    expect(seam).not.toBeNull();
    element.dispatchEvent = (event) => {
      element.calls.push(event);
      if (event.type === "viewport.unmounted") {
        throw new Error("unmounted");
      }
      return true;
    };
    expect(setKindEnabled("stack", false)).toBe(true);
    expect(kindEnabled("stack")).toBe(false);
    expect(seam?.dispatch(presented())).toBe(false);
    expect(setKindEnabled("stack", true)).toBe(true);
    const again = enableFlagged(target(element), "stack");
    expect(again).not.toBeNull();
    expect(again).not.toBe(seam);
  });

  it("AC4 refuses to turn the flag back on from unmounted", () => {
    const element = fake();
    expect(setKindEnabled("stack", true)).toBe(true);
    const seam = enableFlagged(target(element), "stack");
    expect(seam).not.toBeNull();
    let nested: boolean | null = null;
    element.dispatchEvent = (event) => {
      element.calls.push(event);
      if (event.type === "viewport.unmounted") {
        nested = setKindEnabled("stack", true);
      }
      return true;
    };
    expect(setKindEnabled("stack", false)).toBe(true);
    expect(nested).toBe(false);
    expect(kindEnabled("stack")).toBe(false);
    expect(seam?.dispatch(presented())).toBe(false);
  });

  it("AC4 does not keep an element enabled when mounted turns the kind off", () => {
    const element = fake();
    expect(setKindEnabled("stack", true)).toBe(true);
    let once = true;
    element.dispatchEvent = (event) => {
      element.calls.push(event);
      if (event.type === "viewport.mounted" && once) {
        once = false;
        expect(setKindEnabled("stack", false)).toBe(true);
      }
      return true;
    };
    expect(enableFlagged(target(element), "stack")).toBeNull();
    expect(kindEnabled("stack")).toBe(false);
    expect(setKindEnabled("stack", false)).toBe(true);
    expect(element.calls.map((event) => event.type)).toEqual([
      "viewport.mounted",
      "viewport.unmounted",
    ]);
  });

  it("AC4 disables both elements when the second mount turns the kind off", () => {
    const first = fake();
    const second = fake();
    expect(setKindEnabled("stack", true)).toBe(true);
    const firstSeam = enableFlagged(target(first), "stack");
    expect(firstSeam).not.toBeNull();
    second.dispatchEvent = (event) => {
      second.calls.push(event);
      if (event.type === "viewport.mounted") {
        expect(setKindEnabled("stack", false)).toBe(true);
      }
      return true;
    };
    const secondSeam = enableFlagged(target(second), "stack");
    expect(kindEnabled("stack")).toBe(false);
    expect(firstSeam?.dispatch(presented())).toBe(false);
    expect(secondSeam).toBeNull();
    expect(second.calls.map((event) => event.type)).toEqual([
      "viewport.mounted",
      "viewport.unmounted",
    ]);
    expect(setKindEnabled("stack", false)).toBe(true);
    expect(firstSeam?.dispatch(presented())).toBe(false);
  });

  it("AC3 does not store an element whose mounted handler throws", () => {
    const element = fake();
    expect(setKindEnabled("stack", true)).toBe(true);
    element.dispatchEvent = () => {
      throw new Error("mounted");
    };
    expect(enableFlagged(target(element), "stack")).toBeNull();
    expect(kindEnabled("stack")).toBe(true);
    element.dispatchEvent = (event) => {
      element.calls.push(event);
      return true;
    };
    const seam = enableFlagged(target(element), "stack");
    expect(seam).not.toBeNull();
    expect(seam?.dispatch(presented())).toBe(true);
  });

  it("AC5 repeats a true set and refuses an unknown kind", () => {
    const element = fake();
    expect(setKindEnabled("stack", true)).toBe(true);
    const seam = enableFlagged(target(element), "stack");
    expect(setKindEnabled("stack", true)).toBe(true);
    expect(element.calls.map((event) => event.type)).toEqual(["viewport.mounted"]);
    expect(seam?.dispatch(presented())).toBe(true);
    expect(setKindEnabled("oblique" as ViewportKind, true)).toBe(false);
    expect(kindEnabled("stack")).toBe(true);
    expect(kindEnabled("mpr")).toBe(false);
    expect(enableFlagged(target(element), "oblique" as ViewportKind)).toBeNull();
    expect(element.calls.map((event) => event.type)).toEqual([
      "viewport.mounted",
      "frame.presented",
    ]);
  });
});
