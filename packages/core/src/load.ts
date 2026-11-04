/**
 * Whether a built wasm core has finished loading in this realm.
 *
 * The flag starts false. `loadCore` sets it only after the initialiser
 * returns. A thrown initialiser leaves it false. A second success does not
 * call the initialiser again, so one realm keeps one wasm instance.
 */

let ready = false;
let pending: Promise<boolean> | null = null;

export function coreAvailable(): boolean {
  return ready;
}

export function loadCore(initialise: () => Promise<unknown>): Promise<boolean> {
  if (ready) {
    return Promise.resolve(true);
  }
  if (pending !== null) {
    return pending;
  }
  pending = run(initialise);
  return pending;
}

async function run(initialise: () => Promise<unknown>): Promise<boolean> {
  try {
    await initialise();
    ready = true;
    return true;
  } catch {
    return false;
  } finally {
    pending = null;
  }
}
