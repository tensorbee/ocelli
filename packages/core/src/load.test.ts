import { describe, expect, it } from "vitest";

import { coreAvailable, loadCore } from "./index.js";

/**
 * The flag is false until `loadCore` resolves true. A thrown initialiser must
 * not flip it. A second success must not call the initialiser again.
 */
describe("loadCore", () => {
  it("stays false until one load succeeds and does not start a second instance", async () => {
    expect(coreAvailable()).toBe(false);

    let calls = 0;
    const refused = await loadCore(() => {
      calls += 1;
      return Promise.reject(new Error("package absent"));
    });
    expect(refused).toBe(false);
    expect(coreAvailable()).toBe(false);
    expect(calls).toBe(1);

    const first = loadCore(() => {
      calls += 1;
      return Promise.resolve({ started: true });
    });
    const second = loadCore(() => {
      calls += 1;
      return Promise.resolve({ started: true });
    });
    const [left, right] = await Promise.all([first, second]);
    expect(left).toBe(true);
    expect(right).toBe(true);
    expect(coreAvailable()).toBe(true);
    expect(calls).toBe(2);

    const again = await loadCore(() => {
      calls += 1;
      return Promise.resolve({ started: true });
    });
    expect(again).toBe(true);
    expect(calls).toBe(2);
  });
});
