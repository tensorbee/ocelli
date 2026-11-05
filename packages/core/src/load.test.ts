import { describe, expect, it } from "vitest";

import { coreAvailable, loadCore } from "./index.js";
import { OcelliError } from "./ocelli-error.js";

/**
 * The flag is false until `loadCore` resolves true. A thrown initialiser
 * rejects and must not flip the flag. A second success must not call the
 * initialiser again.
 *
 * The shared-failure case is first. A later success latches the flag for
 * this realm, and the module state is not reset between tests.
 */
describe("loadCore", () => {
  it("concurrent callers share one failure", async () => {
    expect(coreAvailable()).toBe(false);
    let calls = 0;
    const cause = new Error("once");
    let release: () => void = () => {};
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    const first = loadCore(() => {
      calls += 1;
      return gate.then(() => Promise.reject(cause));
    });
    const second = loadCore(() => {
      calls += 1;
      return gate.then(() => Promise.reject(cause));
    });
    release();
    const results = await Promise.allSettled([first, second]);
    expect(calls).toBe(1);
    expect(results[0]?.status).toBe("rejected");
    expect(results[1]?.status).toBe("rejected");
    if (results[0]?.status !== "rejected" || results[1]?.status !== "rejected") {
      throw new Error("expected both callers to reject");
    }
    expect(results[0].reason).toBe(results[1].reason);
    expect(results[0].reason).toBeInstanceOf(OcelliError);
    expect(results[0].reason.cause).toBe(cause);
    expect(results[0].reason.code).toBe("CoreLoadFailed");
    expect(coreAvailable()).toBe(false);
  });

  it("rejects with the initialiser cause and a later success runs once", async () => {
    expect(coreAvailable()).toBe(false);
    let calls = 0;
    const cause = new Error("package absent");
    let caught: unknown;
    try {
      await loadCore(() => {
        calls += 1;
        return Promise.reject(cause);
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
