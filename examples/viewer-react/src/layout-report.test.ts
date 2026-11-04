import { describe, expect, it } from "vitest";

import { PAGE_LAYOUT, fullscreenWord, layoutReport } from "./layout-report.js";

describe("layout report", () => {
  it("draws two interior lines for a 2 by 2 grid and none for one cell", () => {
    expect(layoutReport(PAGE_LAYOUT).gridLines).toBe(2);
    expect(layoutReport({ ...PAGE_LAYOUT, columns: 1, rows: 1 }).gridLines).toBe(0);
    expect(layoutReport({ ...PAGE_LAYOUT, columns: 1.5 }).gridLines).toBeNull();
  });

  it("reports the stack step and refuses a zero step", () => {
    expect(layoutReport(PAGE_LAYOUT).dragStep).toBe(1);
    expect(layoutReport({ ...PAGE_LAYOUT, lastY: 0 }).dragStep).toBeNull();
  });

  it("uses the caller point for the lines and the cursor", () => {
    const report = layoutReport(PAGE_LAYOUT);
    expect(report.horizontal).toBe("0,20 to 100,20");
    expect(report.vertical).toBe("10,0 to 10,80");
    expect(report.cursor).toBe("10,20");
    const outside = layoutReport({ ...PAGE_LAYOUT, x: -1 });
    expect(outside.cursor).toBeNull();
    expect(outside.horizontal).toBeNull();
    expect(outside.vertical).toBeNull();
  });

  it("names full screen unavailable when the browser flag is off", () => {
    expect(fullscreenWord(false, true)).toBe("unavailable");
    expect(fullscreenWord(true, false)).toBe("off");
    expect(fullscreenWord(true, true)).toBe("on");
  });
});
