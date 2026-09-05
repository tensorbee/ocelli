import js from "@eslint/js";
import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";

// HLD section 17.2, NEVER CACHE THE VIEW.
//
//   "A module-level `const HEAP = new Uint8Array(wasm.memory.buffer)` is the
//    classic failure. Any wasm memory growth relocates the ArrayBuffer and
//    detaches every outstanding view; the next write silently targets a
//    detached buffer or throws far from the cause. Add an ESLint rule banning
//    `new Uint8Array(wasm.memory.buffer)` outside the two functions that are
//    allowed to do it."
//
// This is the sharpest edge in the whole design, and it fails silently, which
// is why it is a lint and not a convention. The selector matches ANY typed
// array or DataView constructed over something ending `.memory.buffer`, so
// swapping Uint8Array for Uint16Array does not evade it.
//
// The HLD says "outside the two FUNCTIONS". ESLint scopes overrides by file,
// so the allowance is file-scoped to `packages/core/src/bulk.ts` instead, and
// that file is expected to stay small enough that the difference does not
// matter. Widening the allowance to a second file is a design-plan decision.
// TWO selectors, not one, and the second exists because the first did not
// catch what this repository already writes. The original matched only
// `new DataView(wasm.memory.buffer)`, where argument 0's object is itself a
// member expression ending in `memory`. `packages/core/src/panic.ts`
// destructures first, `const { memory } = wasm`, which makes argument 0's
// object a bare identifier and the selector misses it. Measured in the S03
// sprint review: appending both shapes to a file with no allowance produced
// exactly ONE eslint error, on the literal form, and the destructured form
// passed silently. So the ban this gate advertises was not banning the shape
// the codebase uses.
//
// **The residual limit, stated rather than left to be discovered.** A caller
// that destructures twice, `const { buffer } = wasm.memory`, reaches
// `new DataView(buffer)` with a bare identifier as argument 0, and neither
// selector matches that without banning the identifier `buffer` everywhere,
// which would fire on ordinary code. HLD 17.2's rule is about intent and no
// AST selector expresses intent. What these two cover is every shape present
// in this repository today, and F-X009 owns the standing probe that keeps
// them honest.
const NO_CACHED_WASM_VIEW_MEMBER = {
  selector:
    'NewExpression[callee.name=/(Array|DataView)$/]' +
    '[arguments.0.property.name="buffer"]' +
    '[arguments.0.object.property.name="memory"]',
  message:
    "Do not build a view over wasm memory here. Any wasm memory growth " +
    "detaches it and the next write fails far from the cause. Build the view " +
    "inside packages/core/src/bulk.ts, immediately after the alloc that " +
    "returns the pointer, use it, and let it go. See HLD section 17.2.",
};

// The destructured shape, `const { memory } = wasm; new DataView(memory.buffer)`.
const NO_CACHED_WASM_VIEW_DESTRUCTURED = {
  selector:
    'NewExpression[callee.name=/(Array|DataView)$/]' +
    '[arguments.0.property.name="buffer"]' +
    '[arguments.0.object.name="memory"]',
  message:
    "Do not build a view over wasm memory here, and destructuring the " +
    "memory out first does not change that. Any wasm memory growth detaches " +
    "the view and the next write fails far from the cause. Build the view " +
    "inside packages/core/src/bulk.ts, immediately after the alloc that " +
    "returns the pointer, use it, and let it go. See HLD section 17.2.",
};

export default tseslint.config(
  {
    ignores: [
      "**/dist/**",
      "**/pkg/**",
      "**/node_modules/**",
      // `**/` rather than a bare prefix: wasm-pack writes the panic-probe
      // module into `crates/ocelli-wasm/target/panic-probe`, which
      // `target/**` does not match, and eslint would then lint generated
      // wasm-bindgen glue.
      "**/target/**",
      // The benchmark harness's run output (F-006). Gitignored, and it holds a
      // COPY of the wasm-pack glue plus the page that loads it, so without this
      // eslint lints generated wasm-bindgen output and a second copy of a page
      // it already lints in place. `**/dist/**` covers the oracle's equivalent
      // and does not reach here, because this directory is named for a run
      // record rather than for a bundle.
      "tools/bench/out/**",
      "corpus/**",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["**/*.{ts,tsx}"],
    rules: {
      "no-restricted-syntax": ["error", NO_CACHED_WASM_VIEW_MEMBER, NO_CACHED_WASM_VIEW_DESTRUCTURED],
    },
  },
  {
    // The TWO files permitted to build a view over linear memory, which is
    // what HLD section 17.2 says: "outside the two functions that are allowed
    // to do it". The specification expected two and this repository had one
    // only because nothing else needed linear memory yet.
    //
    // `bulk.ts` writes bytes down, between an `alloc` and the `commit` that
    // takes ownership back. `panic.ts` reads the panic record up, AFTER the
    // instance has trapped, which is the safest possible instance of the
    // hazard the rule guards: no wasm code can run, so memory cannot grow
    // between the view's construction and its last use.
    //
    // Widening this list is a design-plan decision and
    // `.claude/plans/F-005-design.md` item F is the one that added the second
    // entry. A THIRD is not granted: `packages/core/src/ring.ts` will need one
    // when F-101 gives it a real ring to drain, and that is F-101's plan to
    // argue.
    files: ["packages/core/src/bulk.ts", "packages/core/src/panic.ts"],
    rules: {
      "no-restricted-syntax": "off",
    },
  },
  {
    // A SEPARATE block, deliberately not folded into the production list
    // above, because it is a different category and merging the two would
    // make the production allowance read as three files when it is two.
    //
    // `panic.test.ts` constructs its own `new WebAssembly.Memory(...)` and
    // builds a view over it to write the fixture record. There is no module,
    // no `alloc` and nothing that can grow that memory, so HLD 17.2's hazard,
    // a cached view detaching when linear memory grows, cannot arise. The rule
    // is syntactic and cannot tell a standalone memory from the core's.
    //
    // This surfaced in the S03 sprint review and is worth recording. The rule
    // matched only `new DataView(wasm.memory.buffer)` and missed
    // `const { memory } = wasm; new DataView(memory.buffer)`, which is what
    // this repository actually writes. Tightening it to catch the destructured
    // shape is what made this file visible. It was never exempt, it was never
    // matched.
    files: ["packages/core/src/*.test.ts"],
    rules: {
      "no-restricted-syntax": "off",
    },
  },
  {
    files: ["packages/react/**/*.{ts,tsx}", "examples/**/*.{ts,tsx}"],
    plugins: { "react-hooks": reactHooks },
    rules: reactHooks.configs.recommended.rules,
  },
  {
    // The repository's own node scripts. Plain ESM JavaScript, run by
    // `bin/ocelli.sh`, never bundled and never shipped. The globals are listed
    // rather than pulled from a `globals` package, for the reason the oracle
    // block below gives, and `no-undef` still catches a typo in any of them.
    files: ["scripts/**/*.mjs"],
    languageOptions: {
      globals: {
        console: "readonly",
        process: "readonly",
        WebAssembly: "readonly",
        TextDecoder: "readonly",
        TextEncoder: "readonly",
        URL: "readonly",
      },
    },
  },
  {
    // The oracle harness (F-010). Plain ESM JavaScript rather than TypeScript,
    // and the only part of the repository whose files run in BOTH node and a
    // browser: `tools/oracle/src/voi.mjs` is bundled into the render page and
    // is also imported under node by the unit tests, so the window a frame is
    // rendered with and the window recorded in its sidecar come from one
    // tested function.
    //
    // The globals are listed rather than pulled from a `globals` package,
    // because adding a dependency to name a dozen identifiers is worse than
    // naming them. `no-undef` still catches a typo in any of them.
    files: ["tools/oracle/**/*.mjs"],
    ignores: ["tools/oracle/page/**"],
    languageOptions: {
      globals: {
        Buffer: "readonly",
        process: "readonly",
        console: "readonly",
        globalThis: "readonly",
        URL: "readonly",
        structuredClone: "readonly",
        setTimeout: "readonly",
        clearTimeout: "readonly",
      },
    },
  },
  {
    // The render page, and only it. The split runs both ways: the node block
    // above excludes this directory, and this block grants no node globals, so
    // `no-undef` catches a driver file reaching for `document` and a page file
    // reaching for `process`.
    files: ["tools/oracle/page/**/*.mjs"],
    languageOptions: {
      globals: {
        window: "readonly",
        document: "readonly",
        navigator: "readonly",
        crypto: "readonly",
        atob: "readonly",
        btoa: "readonly",
        File: "readonly",
        CustomEvent: "readonly",
      },
    },
  },
  {
    // The benchmark harness (F-006). Plain ESM JavaScript, node only, and the
    // split against the page block below runs both ways for the reason the
    // oracle's pair gives: `no-undef` then catches a driver file reaching for
    // `document` and a page file reaching for `process`.
    //
    // The globals are listed rather than pulled from a `globals` package, the
    // same choice the two blocks above make.
    files: ["tools/bench/**/*.mjs"],
    ignores: ["tools/bench/page/**"],
    languageOptions: {
      globals: {
        process: "readonly",
        console: "readonly",
        URL: "readonly",
        setTimeout: "readonly",
        clearTimeout: "readonly",
      },
    },
  },
  {
    // The cold-start page, and only it. It is granted no node global, so a
    // page file reaching for `process` is caught, which matters here more than
    // usual: the whole point of the design is that timing is taken from the
    // page with `performance.now()` and never by instrumenting the module, so
    // the page and the driver must not be able to blur into each other.
    files: ["tools/bench/page/**/*.mjs"],
    languageOptions: {
      globals: {
        window: "readonly",
        document: "readonly",
        performance: "readonly",
        fetch: "readonly",
        WebAssembly: "readonly",
        globalThis: "readonly",
      },
    },
  },
  {
    // The Appendix A spike harnesses (F-X006, gates A1 and A2). Node-only
    // throwaway ESM, deleted when both gates are closed, and listed here for
    // the same reason the oracle block exists: `no-undef` should still catch a
    // typo, so the globals are named rather than switched off.
    //
    // `WebAssembly` is granted because these drivers instantiate a module
    // directly. **They do build a view over that module's linear memory**, and
    // that is deliberate rather than an oversight. NO_CACHED_WASM_VIEW above is
    // scoped to `**/*.{ts,tsx}` and does not reach a `.mjs` file, and the
    // allowance is NOT widened here. The harnesses obey section 17.2's actual
    // requirement anyway: each view is built immediately after the exported
    // `out_ptr()` that returns the offset, copied out with `.slice()`, and let
    // go. Nothing is cached across a call that could grow the memory.
    files: ["tools/spikes/**/*.mjs"],
    languageOptions: {
      globals: {
        Buffer: "readonly",
        console: "readonly",
        process: "readonly",
        WebAssembly: "readonly",
      },
    },
  },
);
