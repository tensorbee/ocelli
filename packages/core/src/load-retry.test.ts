import { describe, expect, it } from "vitest";

import { coreAvailable, loadCore } from "./index.js";

/**
 * A synchronous throw must not latch the flag. The next load is a new attempt.
 */
describe("loadCore retry", () => {
  it("tries again after an initialiser throws before it returns a promise", async () => {
    expect(coreAvailable()).toBe(false);

    let calls = 0;
    const refused = await loadCore(() => {
      calls += 1;
      throw new Error("sync");
    });
    expect(refused).toBe(false);
    expect(coreAvailable()).toBe(false);
    expect(calls).toBe(1);

    const recovered = await loadCore(() => {
      calls += 1;
      return Promise.resolve({ started: true });
    });
    expect(recovered).toBe(true);
    expect(coreAvailable()).toBe(true);
    expect(calls).toBe(2);
  });
});
