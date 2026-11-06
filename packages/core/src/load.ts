/**
 * Whether a built wasm core has finished loading in this realm.
 *
 * The flag starts false. `loadCore` sets it only after the initialiser
 * returns. A thrown initialiser rejects and leaves the flag false. A second
 * success does not call the initialiser again, so one realm keeps one wasm
 * instance.
 */

import { OcelliError } from "./ocelli-error.js";

let ready = false;
let pending: Promise<true> | null = null;

export function coreAvailable(): boolean {
  return ready;
}

export function loadCore(initialise: () => Promise<unknown>): Promise<true> {
  if (ready) {
    return Promise.resolve<true>(true);
  }
  if (pending !== null) {
    return pending;
  }
  // `Promise.resolve().then` keeps a synchronous throw off the caller.
  // An async function would run that throw, clear `pending` in `finally`,
  // and only then return, so the assignment would store the failed promise
  // with nothing left to clear it.
  const running = Promise.resolve()
    .then(() => initialise())
    .then((): true => {
      ready = true;
      return true;
    })
    .catch((cause: unknown) => {
      throw new OcelliError("CoreLoadFailed", "The imaging core did not load.", { cause });
    })
    .finally(() => {
      if (pending === running) {
        pending = null;
      }
    });
  pending = running;
  return running;
}
