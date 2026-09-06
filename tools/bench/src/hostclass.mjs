// The host class, and why a duration needs one.
//
// A byte count is deterministic. `ci/wasm-size-budget.json` can hold one number
// and any machine can check it. A duration is not: it belongs to a CPU, an
// operating system build and a browser build, and comparing one machine's
// milliseconds against another's is the numeric equivalent of the
// bit-exactness claim decision D14 already refuses for pixels.
//
// So every recorded figure carries a host class, and a comparison against a
// baseline recorded under a different one is REFUSED rather than made. That
// refusal is the point. A harness that quietly compared across machines would
// report a regression on a slower laptop and an improvement on a faster one,
// and both would be noise wearing a verdict's clothes.
//
// The oracle's `run.json` already carries `platform`, `release`, `arch`, `node`
// and a `page.rendering` block, which is enough to identify a reference
// environment for PIXELS. It carries no CPU model, no core count and no memory,
// which is not enough to normalise a DURATION. Those three are what this adds,
// and they are the one place the harness extends rather than reuses what F-010
// built. F-011 may read this fingerprint. It shares nothing else with F-006.

/**
 * The fields that decide comparability. Everything else about a run is
 * recorded and none of it is compared.
 */
export const HOST_CLASS_FIELDS = [
  "platform",
  "release",
  "arch",
  "cpu_model",
  "cpu_count",
  "memory_bytes",
];

/**
 * The host class of a machine.
 *
 * @param {{platform: () => string, release: () => string, arch: () => string,
 *          cpus: () => {model: string}[], totalmem: () => number}} os the
 *   `node:os` module, or a stand-in. Injected so the tests can drive every
 *   field without depending on the machine they run on.
 */
export function hostClass(os) {
  const cpus = os.cpus();
  return {
    platform: os.platform(),
    release: os.release(),
    arch: os.arch(),
    cpu_model: cpus.length > 0 ? cpus[0].model : "unknown",
    cpu_count: cpus.length,
    memory_bytes: os.totalmem(),
  };
}

/**
 * The key `ci/bench-baseline.json` is a map on.
 *
 * A map from the start, and not a scalar, because migrating a scalar to a map
 * later rewrites every recorded entry, and a recorded measurement that gets
 * rewritten stops being a measurement.
 */
export function hostClassKey(hc) {
  return HOST_CLASS_FIELDS
    .map((field) => String(hc[field]).replace(/[^A-Za-z0-9._-]+/g, "_"))
    .join("|");
}

/**
 * Whether two host classes are the same, and which fields differ if not.
 *
 * Exact equality on every field. There is no "close enough" here: a tolerance
 * on the host class would let two machines share a baseline because nobody
 * chose a boundary, which is the shape of decision the tolerance mechanism is
 * supposed to make visible.
 */
export function sameHostClass(a, b) {
  const differing = HOST_CLASS_FIELDS.filter((field) => a[field] !== b[field]);
  return { same: differing.length === 0, differing };
}

/**
 * The machine's state at the moment of the run, recorded and NOT compared.
 *
 * Load average is here rather than in the host class on purpose. It changes
 * between two runs on one machine, so putting it in the class would make every
 * run incomparable with every other, which is a comparison mechanism that never
 * compares. It is recorded because a figure taken on a loaded machine is worth
 * less than the same figure taken on an idle one, and the reader can only know
 * that if the number is there.
 */
export function conditions(os) {
  return {
    load_average: os.loadavg(),
    free_memory_bytes: os.freemem(),
    uptime_seconds: Math.round(os.uptime()),
  };
}

/**
 * Whether two instrument blocks match.
 *
 * The host class says which machine. This says which tools, and it is a
 * separate question with a separate answer: the same machine running a
 * different Chromium produces a different cold start for a reason that has
 * nothing to do with the artefact. A mismatch is `incomparable`, exactly as a
 * host-class mismatch is.
 */
export function sameInstrument(a, b) {
  const fields = new Set([...Object.keys(a), ...Object.keys(b)]);
  const differing = [...fields].filter((field) => a[field] !== b[field]).sort();
  return { same: differing.length === 0, differing };
}
