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
// A SECOND selector was added because the first did not
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
// A THIRD selector was added by the S03 review's second pass, because the
// first two are keyed on the identifier NAME `memory`. Four shapes were
// measured escaping them:
//
//   const { buffer } = wasm.memory;  new DataView(buffer);
//   const mem = wasm.memory;         new DataView(mem.buffer);
//   const { memory: m } = wasm;      new DataView(m.buffer);
//   const buf = wasm.memory.buffer;  new DataView(buf);
//
// A view cannot be matched once its argument is a bare identifier the rule
// has never seen, so the third selector matches the ALIAS instead, at the
// declaration that takes linear memory or its buffer out of the module
// object. That is one step earlier than the view and it is the step every
// escaping shape above has in common.
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

// The alias shape. `wasm.memory` or `wasm.memory.buffer` bound to a name, in
// either declaration form, which is what every shape the two view selectors
// miss does first.
const NO_CACHED_WASM_MEMORY_ALIAS = {
  selector:
    'VariableDeclarator[init.property.name="memory"],' +
    'VariableDeclarator[init.property.name="buffer"]' +
    '[init.object.property.name="memory"],' +
    'VariableDeclarator > ObjectPattern > Property[key.name="memory"]',
  message:
    "Do not take wasm memory or its buffer out into a binding here. A view " +
    "built over the alias is the same hazard as a view built over " +
    "`wasm.memory.buffer`, and renaming what it is reached through is what " +
    "makes it invisible to a lint. Build the view inside " +
    "packages/core/src/bulk.ts, immediately after the alloc that returns " +
    "the pointer, use it, and let it go. See HLD section 17.2.",
};

// **What still escapes, MEASURED rather than reasoned about.** The S03
// review's fourth pass wrote a probe file with seven routes to a view over
// linear memory and ran `npx eslint` over it. Five escape and two are caught.
// All three selectors are anchored on a variable declaration or on the literal
// member chain, so a view over memory reached any other way is not matched.
// **The seven below are a sample and not the set.** The S03 review's fifth pass
// measured three further routes past the same three selectors,
// `for (const m of [wasm.memory]) new DataView(m.buffer)`,
// `new DataView(wasm.memory["buffer"])` and a getter returning `wasm.memory`,
// and the general sentence above already covers them. A new route is expected
// rather than surprising, and adding one to this list changes nothing about the
// rule.
//
//   caught    new DataView(wasm.memory.buffer)
//   caught    const { memory } = wasm;      new DataView(memory.buffer)
//   ESCAPES   function heap(mem) { return new Uint8Array(mem.buffer); }
//   ESCAPES   new DataView(wasm["memory"].buffer)
//   ESCAPES   let m; m = wasm.memory;       new DataView(m.buffer)
//   ESCAPES   new DataView(fetchMemory(wasm).buffer)
//   ESCAPES   class C { mem = wasm.memory;  view() { return new DataView(this.mem.buffer); } }
//
// **The function-parameter route is the one that matters**, because it is not
// exotic. It is how anyone would write a drain helper, and
// `let HEAP = null; export function heap(mem) { HEAP ??= new Uint8Array(mem.buffer); return HEAP; }`
// is HLD 17.2's named failure with one indirection in front of it and a green
// lint behind it.
//
// **The old wording of this note said "a class field read through `this`"
// escapes, and that is both too wide and too narrow.** `this.memory.buffer` is
// CAUGHT, because argument 0's object property is still named `memory` and the
// first selector matches it. It escapes only when the field is RENAMED, which
// is the same shape as every other escape here: the alias is what the rule
// cannot see, not the `this`.
//
// **A fourth selector was written and measured and is deliberately NOT here.**
//
//   NewExpression[callee.name=/(Array|DataView)$/][arguments.0.property.name="buffer"]
//
// unconditioned on the object. Measured: it catches all five escapes above and
// therefore all seven routes, **and it does not catch every route there is**.
// `new DataView(wasm.memory["buffer"])` escapes it too, because it keys on
// `arguments.0.property.name="buffer"` and a computed member's property is a
// `Literal` with no `name`. So F-X017 cannot be closed by landing this
// selector, which is why the story names type-aware linting and makes the
// computed route its acceptance test. Its cost across `packages/` and `examples/` is
// exactly ONE site, `decodeRecord` at packages/core/src/errors.ts, which takes
// `new DataView(payload.buffer, payload.byteOffset, RECORD_BYTES)` over a
// caller's `Uint8Array`. That is a true instance of the syntactic pattern and
// a safe instance of the hazard, built inside the function, used immediately
// and neither stored nor returned, which is exactly what `panic.ts` is allowed
// for.
//
// **One site against five closed routes is a good trade, and landing it is
// still a design decision rather than a remediation**, because this repository
// has no exception narrower than a file-wide allowance. Accommodating that one
// site means adding `errors.ts` to ALLOWED_TO_DISABLE, which switches off all
// the selectors there and would make `new DataView(wasm.memory.buffer)` legal
// in that file, and `docs/lld/errors.md` says granting a third file is a
// design-plan decision. The alternatives measured are worse: an inline
// `eslint-disable` would be the first in this tree and `assertTheBanIsIntact`
// cannot see one, and a per-file narrowing block would require weakening that
// self-check. Restricting the selector to `[arguments.length=1]` would spare
// `errors.ts` and reopen the hole one comma away, which is the shape of every
// escape this rule has already been through.
//
// HLD 17.2's rule is about intent and no AST selector expresses intent.
//
// **What watches these three.** All three selectors and the allowance list are
// declared constants in `scripts/guards/catalogue.py`'s ratchet, so weakening
// one of them lands in front of a reviewer as a changed digest.
// `assertTheBanIsIntact` below is what watches the OTHER direction, a further
// config block switching the rule off tree-wide, which the ratchet cannot
// see because it reads two named strings rather than the whole file. **No
// probe drives this rule red**, so nothing here observes the selectors
// failing to fire. That is the standing gap, and it is a gap in `gate lint`
// rather than in the ratchet.
const RESTRICTED = "no-restricted-syntax";
const BAN = [
  NO_CACHED_WASM_VIEW_MEMBER,
  NO_CACHED_WASM_VIEW_DESTRUCTURED,
  NO_CACHED_WASM_MEMORY_ALIAS,
];

// The file lists permitted to switch the ban off, and nothing else may. Each
// appears exactly once. The reasoning for each is at the block that uses it.
const ALLOWED_TO_DISABLE = [
  ["packages/core/src/bulk.ts", "packages/core/src/panic.ts"],
  ["packages/core/src/*.test.ts"],
];

/**
 * Refuse a config that weakens the cached-wasm-view ban anywhere.
 *
 * The S03 review's second pass measured this: a FOURTH block naming any
 * wider glob and setting `"no-restricted-syntax": "off"` turns the ban off
 * across the tree, eslint reports nothing because that is exactly what a flat
 * config is for, and the ratchet over the two selector strings sees no change
 * because neither string moved. So the config checks itself as it loads:
 * exactly one block may set the rule, it must set all three selectors at
 * error, and every other mention of the rule must be one of the declared
 * allowances. A throw here fails `npm run lint` and every
 * gate that runs it, which is the loudest failure this file can produce.
 */
function assertTheBanIsIntact(config) {
  const disabled = [];
  let enforced = 0;
  for (const entry of config) {
    const setting = entry?.rules?.[RESTRICTED];
    if (setting === undefined) continue;
    const level = Array.isArray(setting) ? setting[0] : setting;
    const where = JSON.stringify(entry.files ?? null);
    if (level === "off" || level === 0) {
      disabled.push(where);
      continue;
    }
    const selectors = Array.isArray(setting) ? setting.slice(1) : [];
    const intact =
      level === "error" &&
      selectors.length === BAN.length &&
      BAN.every((rule, index) => selectors[index] === rule);
    if (!intact) {
      throw new Error(
        `eslint.config.js: the block for ${where} sets ${RESTRICTED} to ` +
          "something other than the three cached-wasm-view selectors at " +
          "error. HLD 17.2's ban is not a preference. See the comment above " +
          "NO_CACHED_WASM_VIEW_MEMBER.",
      );
    }
    enforced += 1;
  }
  if (enforced !== 1) {
    throw new Error(
      `eslint.config.js: ${enforced} block(s) enforce ${RESTRICTED} and ` +
        "exactly one must. HLD 17.2's ban is stated once, over " +
        "**/*.{ts,tsx}, so that a reader can find it.",
    );
  }
  const allowed = ALLOWED_TO_DISABLE.map((files) => JSON.stringify(files));
  const surplus = disabled.filter((where) => !allowed.includes(where));
  if (surplus.length > 0 || disabled.length !== allowed.length) {
    throw new Error(
      `eslint.config.js: ${RESTRICTED} is switched off for ${disabled} and ` +
        `the only lists permitted to switch it off are ${allowed}. ` +
        "Widening the allowance is a design-plan decision, and adding a " +
        "further block is how the ban gets turned off tree-wide without " +
        "anyone editing the rule. See HLD section 17.2.",
    );
  }
}

const config = tseslint.config(
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
      "no-restricted-syntax": ["error", ...BAN],
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
    // that is deliberate rather than an oversight. The cached-wasm-view ban
    // above is scoped to `**/*.{ts,tsx}` and does not reach a `.mjs` file,
    // and the
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

assertTheBanIsIntact(config);

export default config;
