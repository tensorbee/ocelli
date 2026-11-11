import { describe, expect, it } from "vitest";

import { loadCore } from "./index.js";

type LoadResult = Awaited<ReturnType<typeof loadCore>>;
type ExactlyTrue<T> = [T] extends [true] ? ([true] extends [T] ? true : never) : never;

/** `Promise<true>` assigns to boolean. `Promise<void>` does not. */
export async function loadResultIsBoolean(): Promise<boolean> {
  const loaded: boolean = await loadCore(() => Promise.resolve(true));
  return loaded;
}

/**
 * `false` is not a success value. `Promise<true>` and `Promise<boolean>`
 * both fail this line. The exact check below is what rejects `boolean`.
 */
export async function loadResultIsNotFalse(): Promise<false> {
  // @ts-expect-error true is not assignable to false
  const loaded: false = await loadCore(() => Promise.resolve(true));
  return loaded;
}

const loadResultIsTrue: ExactlyTrue<LoadResult> = true;

describe("loadCore return type", () => {
  it("is Promise<true> in the type fixture", () => {
    expect(loadResultIsTrue).toBe(true);
    expect(loadResultIsBoolean).toEqual(expect.any(Function));
    expect(loadResultIsNotFalse).toEqual(expect.any(Function));
  });
});
