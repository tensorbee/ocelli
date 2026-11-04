import { describe, expect, it, vi } from "vitest";

import {
  PAGE_MEASUREMENT,
  measurementReport,
  pointerStartName,
  touchStartName,
} from "./measurement-report.js";

describe("measurement report", () => {
  it("persists 12.5 mm and reads the same number from TID 1500", () => {
    const report = measurementReport(PAGE_MEASUREMENT);
    expect(report.stored).toBe(12.5);
    expect(report.tid).toBe(12.5);
  });

  it("refuses TID 1500 when the source instance is blank and still loads the document", () => {
    const report = measurementReport({ ...PAGE_MEASUREMENT, sopInstance: "" });
    expect(report.stored).toBe(12.5);
    expect(report.tid).toBeNull();
  });

  it("reads the TID number back when it differs from the stored document", () => {
    const report = measurementReport({ ...PAGE_MEASUREMENT, tidMillimetres: 7 });
    expect(report.stored).toBe(12.5);
    expect(report.tid).toBe(7);
  });

  it("shows the number readTid1500 returns rather than the number written", async () => {
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
      const report = measurementReport({ ...PAGE_MEASUREMENT, tidMillimetres: 7 });
      expect(spy).toHaveBeenCalled();
      expect(report.stored).toBe(12.5);
      expect(report.tid).toBe(3);
    } finally {
      spy.mockRestore();
    }
  });

  it("refuses a non-finite number", () => {
    const report = measurementReport({
      ...PAGE_MEASUREMENT,
      millimetres: Number.NaN,
      tidMillimetres: Number.NaN,
    });
    expect(report.stored).toBeNull();
    expect(report.annotations).toBeNull();
    expect(report.tid).toBeNull();
  });

  it("lists one annotation and undoes it", () => {
    const report = measurementReport(PAGE_MEASUREMENT);
    expect(report.annotations).toBe(1);
    expect(report.undo).toBe(0);
  });

  it("names the key image, the shifted arrow, and a touch start", () => {
    const report = measurementReport(PAGE_MEASUREMENT);
    expect(report.keyImage).toBe("KEY");
    expect(report.shortcut).toBe("scroll-previous");
    expect(measurementReport({ ...PAGE_MEASUREMENT, shift: false }).shortcut).toBe("pan-up");
    expect(report.touch).toBe("touch.start");
    expect(touchStartName()).toBe("touch.start");
    expect(pointerStartName("mouse")).toBe("pointer.down");
    expect(pointerStartName("mouse")).not.toBe("touch.start");
  });
});
