import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import {
  DEFAULT_DECODE_WORKERS,
  MAX_DECODE_WORKERS,
  openDecodePool,
  threadingEscalation,
} from "./pool.js";

interface Handle {
  terminated: number;
  terminate: () => void;
}

function starter(log: number[], failAt?: number): (index: number) => Handle {
  return (index) => {
    log.push(index);
    if (failAt === index) {
      throw new Error("start failed");
    }
    const handle: Handle = {
      terminated: 0,
      terminate() {
        handle.terminated += 1;
      },
    };
    return handle;
  };
}

describe("decode pool", () => {
  it("refuses a count outside 1 to 8 and does not start", () => {
    const owner = {};
    const log: number[] = [];
    for (const count of [0, 1.5, 9, -1]) {
      expect(openDecodePool(owner, count, starter(log))).toBeNull();
    }
    expect(log).toEqual([]);
    expect(openDecodePool(null as unknown as object, 1, starter(log))).toBeNull();
    expect(log).toEqual([]);
  });

  it("starts one worker at index 0", () => {
    const log: number[] = [];
    const pool = openDecodePool({}, 1, starter(log));
    expect(pool).not.toBeNull();
    expect(log).toEqual([0]);
    expect(pool?.count).toBe(1);
    expect(pool?.sharedMemory).toBe(false);
    expect(pool?.threading).toBe("stable");
  });

  it("starts two workers at 0 and 1", () => {
    const log: number[] = [];
    const pool = openDecodePool({}, 2, starter(log));
    expect(pool).not.toBeNull();
    expect(log).toEqual([0, 1]);
  });

  it("starts eight workers at indexes 0 through 7", () => {
    const log: number[] = [];
    const pool = openDecodePool({}, 8, starter(log));
    expect(pool).not.toBeNull();
    expect(pool?.count).toBe(8);
    expect(log).toEqual([0, 1, 2, 3, 4, 5, 6, 7]);
    expect(pool?.close()).toBe(true);
  });

  it("returns null for a second open of the same owner", () => {
    const owner = {};
    const first: number[] = [];
    const second: number[] = [];
    expect(openDecodePool(owner, 1, starter(first))).not.toBeNull();
    expect(openDecodePool(owner, 2, starter(second))).toBeNull();
    expect(second).toEqual([]);
    expect(first).toEqual([0]);
  });

  it("lets a different owner open its own pool", () => {
    const log: number[] = [];
    expect(openDecodePool({}, 1, starter(log))).not.toBeNull();
    expect(openDecodePool({}, 1, starter(log))).not.toBeNull();
    expect(log).toEqual([0, 0]);
  });

  it("terminates each worker once and a second close returns false", () => {
    const handles: Handle[] = [];
    const pool = openDecodePool({}, 2, (index) => {
      const handle: Handle = {
        terminated: 0,
        terminate() {
          handle.terminated += 1;
          if (index === 0 && handle.terminated === 1) {
            throw new Error("terminate failed");
          }
        },
      };
      handles.push(handle);
      return handle;
    });
    expect(pool?.close()).toBe(true);
    expect(handles.map((handle) => handle.terminated)).toEqual([1, 1]);
    expect(pool?.close()).toBe(false);
    expect(handles.map((handle) => handle.terminated)).toEqual([1, 1]);
  });

  it("drops the slot when start throws and terminates what already started", () => {
    const owner = {};
    const started: Handle[] = [];
    expect(() =>
      openDecodePool(owner, 2, (index) => {
        const handle: Handle = {
          terminated: 0,
          terminate() {
            handle.terminated += 1;
          },
        };
        if (index === 1) {
          throw new Error("start failed");
        }
        started.push(handle);
        return handle;
      }),
    ).toThrow("start failed");
    expect(started.map((handle) => handle.terminated)).toEqual([1]);
    const log: number[] = [];
    expect(openDecodePool(owner, 1, starter(log))).not.toBeNull();
    expect(log).toEqual([0]);
  });

  it("stays on stable Rust and the default count is 1", () => {
    expect(DEFAULT_DECODE_WORKERS).toBe(1);
    expect(MAX_DECODE_WORKERS).toBe(8);
    expect(threadingEscalation()).toBe("stable");
    const source = readFileSync(
      join(dirname(fileURLToPath(import.meta.url)), "pool.ts"),
      "utf8",
    );
    expect(source.includes("SharedArrayBuffer")).toBe(false);
  });
});
