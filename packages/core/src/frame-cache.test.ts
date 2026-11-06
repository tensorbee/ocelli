/**
 * The frame-cache calls, against a stub of the exports they drive.
 *
 * The stub records every call, which is what makes the refusals of AC15
 * checkable: a bad input must throw before any export call, so the record
 * has to still be empty. The bigint passthrough of AC14 and AC15 is the
 * other half: a u64 that reached the export truncated would read back
 * differently than it went in.
 */
import { describe, expect, it } from "vitest";

import {
  frameResident,
  releaseSeries,
  releaseStudy,
  requestFramePressure,
  setFrameBudget,
  showResident,
  type FrameCacheExports,
  type ResidentFrameRequest,
} from "./frame-cache.js";
import { OcelliError } from "./ocelli-error.js";

type Recorded = FrameCacheExports & { readonly calls: string[] };

function stub(overrides: Partial<FrameCacheExports> = {}): Recorded {
  const calls: string[] = [];
  const base: FrameCacheExports = {
    set_frame_budget(bytes: number): number {
      calls.push(`set_frame_budget ${bytes}`);
      return 1;
    },
    frame_resident(
      index: number,
      epoch: number,
      content: bigint,
      columns: number,
      rows: number,
    ): number {
      calls.push(`frame_resident ${index} ${epoch} ${content} ${columns} ${rows}`);
      return 1;
    },
    show_resident(
      index: number,
      epoch: number,
      content: bigint,
      columns: number,
      rows: number,
    ): number {
      calls.push(`show_resident ${index} ${epoch} ${content} ${columns} ${rows}`);
      return 1;
    },
    release_series(series: bigint): number {
      calls.push(`release_series ${series}`);
      return series === 5n ? 3 : 1;
    },
    request_frame_pressure(): number {
      calls.push("request_frame_pressure");
      return 1;
    },
  };
  return { ...base, ...overrides, calls };
}

/** The refusal code of one drive, or a marker when nothing was refused. */
function codeOf(drive: () => unknown): string {
  try {
    drive();
  } catch (error) {
    if (error instanceof OcelliError) {
      return error.code;
    }
    throw error;
  }
  return "no refusal";
}

const REQUEST: ResidentFrameRequest = {
  index: 1,
  epoch: 4,
  content: 0x1_0000_0001n,
  columns: 512,
  rows: 512,
};

describe("frame cache calls", () => {
  it("passes the budget through and nothing else", () => {
    const wasm = stub();
    setFrameBudget(wasm, 3_145_728);
    expect(wasm.calls).toEqual(["set_frame_budget 3145728"]);
  });

  it("passes a u64 content id and a full request through untouched", () => {
    const wasm = stub();
    expect(frameResident(wasm, REQUEST)).toBe(true);
    expect(wasm.calls).toEqual(["frame_resident 1 4 4294967297 512 512"]);
    wasm.calls.length = 0;
    expect(showResident(wasm, REQUEST)).toBe(true);
    expect(wasm.calls).toEqual(["show_resident 1 4 4294967297 512 512"]);
  });

  it("maps the export's zero to false", () => {
    const wasm = stub({ frame_resident: () => 0, show_resident: () => 0 });
    expect(frameResident(wasm, REQUEST)).toBe(false);
    expect(showResident(wasm, REQUEST)).toBe(false);
  });

  it("turns an unselected-index refusal into the coded CommitRefused", () => {
    // The lead ruling after review pass 2. The bind answers 2 for the
    // Selection refusal and 0 for a miss, so a caller can tell the refusal
    // from a miss. Before the ruling the 2 answered false.
    expect(codeOf(() => showResident(stub({ show_resident: () => 2 }), REQUEST))).toBe(
      "CommitRefused",
    );
    expect(showResident(stub({ show_resident: () => 0 }), REQUEST)).toBe(false);
  });

  it("releases one series and returns the frame count", () => {
    const wasm = stub();
    expect(releaseSeries(wasm, 0x1_0000_0005n)).toBe(1);
    expect(wasm.calls).toEqual(["release_series 4294967301"]);
  });

  it("releases a study once per series and sums the counts", () => {
    // F-231 AC14. Duplicates are skipped, so 5n is called once and the
    // counts of 5n and 6n are summed: 3 + 1.
    const wasm = stub();
    expect(releaseStudy(wasm, [5n, 6n, 5n])).toBe(4);
    expect(wasm.calls).toEqual(["release_series 5", "release_series 6"]);
  });

  it("pushes one pressure reading and returns nothing", () => {
    const wasm = stub();
    expect(requestFramePressure(wasm)).toBeUndefined();
    expect(wasm.calls).toEqual(["request_frame_pressure"]);
  });

  it("refuses a bad input before any export call", () => {
    const cases: readonly ((wasm: Recorded) => unknown)[] = [
      (wasm) => frameResident(wasm, { ...REQUEST, index: -1 }),
      (wasm) => frameResident(wasm, { ...REQUEST, index: 2 ** 32 }),
      (wasm) => frameResident(wasm, { ...REQUEST, epoch: 1.5 }),
      (wasm) => frameResident(wasm, { ...REQUEST, columns: -1 }),
      (wasm) => showResident(wasm, { ...REQUEST, rows: 2 ** 32 }),
      (wasm) => setFrameBudget(wasm, -1),
      (wasm) => setFrameBudget(wasm, 1.5),
      (wasm) => releaseSeries(wasm, 2n ** 64n),
      (wasm) => releaseSeries(wasm, -1n),
      (wasm) => releaseStudy(wasm, [1n, 2n ** 64n]),
    ];
    for (const drive of cases) {
      const wasm = stub();
      expect(codeOf(() => drive(wasm))).toBe("InvalidValue");
      expect(wasm.calls).toEqual([]);
    }
  });

  it("refuses an unknown request key before any export call", () => {
    const wasm = stub();
    const refused = { ...REQUEST, series: 1n } as unknown as ResidentFrameRequest;
    expect(codeOf(() => frameResident(wasm, refused))).toBe("UnknownOption");
    expect(wasm.calls).toEqual([]);
  });

  it("refuses a request that is not an object", () => {
    const wasm = stub();
    expect(codeOf(() => showResident(wasm, 7 as unknown as ResidentFrameRequest))).toBe(
      "InvalidOptions",
    );
    expect(wasm.calls).toEqual([]);
  });
});
