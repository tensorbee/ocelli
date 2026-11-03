import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { keyAction } from "./keys.js";

const SOURCE = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), "keys.ts"),
  "utf8",
);

describe("keyAction", () => {
  it("pans on the arrow keys", () => {
    expect(keyAction("ArrowLeft", false)).toBe("pan-left");
    expect(keyAction("ArrowRight", false)).toBe("pan-right");
    expect(keyAction("ArrowUp", false)).toBe("pan-up");
    expect(keyAction("ArrowDown", false)).toBe("pan-down");
  });

  it("scrolls on a shifted vertical arrow", () => {
    expect(keyAction("ArrowUp", true)).toBe("scroll-previous");
    expect(keyAction("ArrowDown", true)).toBe("scroll-next");
  });

  it("keeps a shifted horizontal arrow as a pan", () => {
    expect(keyAction("ArrowLeft", true)).toBe("pan-left");
    expect(keyAction("ArrowRight", true)).toBe("pan-right");
  });

  it("zooms on plus, equals and minus", () => {
    expect(keyAction("+", false)).toBe("zoom-in");
    expect(keyAction("=", false)).toBe("zoom-in");
    expect(keyAction("+", true)).toBe("zoom-in");
    expect(keyAction("-", false)).toBe("zoom-out");
    expect(keyAction("-", true)).toBe("zoom-out");
  });

  it("returns null for an unknown or lowercase name", () => {
    expect(keyAction("arrowleft", false)).toBeNull();
    expect(keyAction("Enter", false)).toBeNull();
    expect(keyAction("", false)).toBeNull();
  });

  it("does not register a listener", () => {
    expect(/addEventListener\s*\(/.test(SOURCE)).toBe(false);
  });
});
