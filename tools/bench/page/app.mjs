// The browser half of `wasm.cold_start`.
//
// TIMING IS TAKEN FROM OUTSIDE THE CORE, NEVER BY INSTRUMENTING IT. The obvious
// implementation is a `#[wasm_bindgen] pub fn bench_mark()` in `ocelli-wasm`
// and an `Instant::now()` in each crate, and both are refused:
//
//   - a timing export in `ocelli-wasm` adds a boundary function that exists for
//     the harness and ships to every user, and F-101 (E16.2) owns what the
//     boundary exports
//   - `std::time::Instant` does not work on wasm32-unknown-unknown, so a Rust
//     timing primitive would need a browser branch, which is browser-specific
//     code in a core crate and is what decision D2 exists to prevent
//
// So this file times with `performance.now()` from the page, outside the
// module, and `crates/` gains not one line.
//
// The five phases are separated because the total on its own would hide which
// half moved. A module that grew would move `compile`, a module that gained an
// import would move `instantiate`, and a network change would move `fetch`.

/** The phases, in the order they happen. Recorded separately and summed. */
export const PHASES = [
  "glue_module_script",
  "fetch",
  "compile",
  "instantiate",
  "first_call",
];

/**
 * One cold start.
 *
 * Called exactly once per page. A second call in the same page would be nearly
 * free, because the dynamic import is module-cached and the compiled module is
 * held by the realm, so the runner uses a fresh browser per iteration rather
 * than calling this twice.
 *
 * @returns {Promise<object>} the phase durations in milliseconds, the total,
 *   the version string the module returned, and the artefact's byte length
 */
export async function measure() {
  const marks = [];
  marks.push(performance.now());

  const glue = await import("./ocelli_wasm.js");
  marks.push(performance.now());

  // `cache: "no-store"` on the request as well as `cache-control: no-store` on
  // the response. The server sets the header and this sets the request, and the
  // two are not the same instruction: a fetch that came out of the HTTP cache
  // would report a duration for a copy the browser already had, and the number
  // would then drift with how many times the page had been opened rather than
  // with anything about the artefact.
  const response = await fetch("./ocelli_wasm_bg.wasm", { cache: "no-store" });
  if (!response.ok) {
    throw new Error(
      `the wasm artefact fetched as HTTP ${response.status}. The page is ` +
        `served from a copy of crates/ocelli-wasm/pkg and that copy is ` +
        `incomplete.`,
    );
  }
  const bytes = await response.arrayBuffer();
  marks.push(performance.now());

  const module = await WebAssembly.compile(bytes);
  marks.push(performance.now());

  // `initSync` and not the default async init, deliberately. The default export
  // fetches AND instantiates, which would fold two phases into one and re-fetch
  // the bytes that were just fetched above.
  glue.initSync({ module });
  marks.push(performance.now());

  const version = glue.ocelli_version();
  marks.push(performance.now());

  const phases = {};
  for (const [index, name] of PHASES.entries()) {
    phases[name] = marks[index + 1] - marks[index];
  }
  return {
    phases,
    total: marks[marks.length - 1] - marks[0],
    version,
    artefact_bytes: bytes.byteLength,
  };
}

// The runner drives the page through this handle. A page that loaded and did
// nothing would otherwise be indistinguishable from a page that measured
// nothing, which is the oracle's stated defect one level up.
globalThis.__bench = { measure, PHASES };
document.getElementById("status").textContent = "ready";
