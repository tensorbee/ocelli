import { describe, expect, it } from "vitest";

import { coreAvailable, loadCore } from "./index.js";
import { OcelliError } from "./ocelli-error.js";

/**
 * A synchronous throw must not latch the flag. The next load is a new attempt.
 */
describe("loadCore retry", () => {
  it("tries again after an initialiser throws before it returns a promise", async () => {
    expect(coreAvailable()).toBe(false);

    let calls = 0;
    const cause = new Error("sync");
    let caught: unknown;
    try {
      await loadCore(() => {
        calls += 1;
        throw cause;
      });
    } catch (error) {
      caught = error;
    }
    expect(caught).toBeInstanceOf(OcelliError);
    if (!(caught instanceof OcelliError)) {
      throw new Error("expected OcelliError");
    }
    expect(caught.code).toBe("CoreLoadFailed");
    expect(caught.message).toBe("The imaging core did not load.");
    expect(caught.cause).toBe(cause);
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
