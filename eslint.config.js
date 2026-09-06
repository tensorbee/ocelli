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
// The view rule is type-aware. It asks TypeScript for the receiver of
// `.buffer`, so a parameter, assignment, call result, getter, renamed field,
// for-of binding or computed access cannot hide WebAssembly.Memory behind a
// different spelling. This is deliberately narrower than banning every view
// over every ArrayBuffer. `decodeRecord` builds a safe DataView over its
// caller-owned Uint8Array and remains legal.
const TYPED_ARRAY_CONSTRUCTORS = new Set([
  "BigInt64Array",
  "BigUint64Array",
  "DataView",
  "Float32Array",
  "Float64Array",
  "Int8Array",
  "Int16Array",
  "Int32Array",
  "Uint8Array",
  "Uint8ClampedArray",
  "Uint16Array",
  "Uint32Array",
]);

const WASM_VIEW_RULE = "ocelli/no-wasm-memory-view";
const WASM_VIEW_MESSAGE =
  "Do not build a view over WebAssembly.Memory here. Any wasm memory growth " +
  "detaches it and the next write fails far from the cause. Build the view " +
  "only in packages/core/src/bulk.ts or packages/core/src/panic.ts, use it " +
  "immediately, and let it go. See HLD section 17.2.";

function memberName(node) {
  if (!node.computed && node.property.type === "Identifier") {
    return node.property.name;
  }
  if (node.computed && node.property.type === "Literal") {
    return node.property.value;
  }
  return undefined;
}

function hasGlobalSymbol(checker, type, names) {
  const members = type.isUnionOrIntersection() ? type.types : [type];
  return members.some((member) => {
    const resolved = checker.getBaseConstraintOfType(member) ?? member;
    const symbol = resolved.aliasSymbol ?? resolved.getSymbol();
    return symbol !== undefined && names.has(checker.getFullyQualifiedName(symbol));
  });
}

function isTypedArrayConstruction(checker, nodeMap, callee) {
  const type = checker.getTypeAtLocation(nodeMap.get(callee));
  return type.getConstructSignatures().some((signature) =>
    hasGlobalSymbol(
      checker,
      checker.getReturnTypeOfSignature(signature),
      TYPED_ARRAY_CONSTRUCTORS,
    ),
  );
}

const noWasmMemoryViewRule = {
  meta: {
    type: "problem",
    docs: { description: "ban ephemeral views over wasm linear memory" },
    schema: [],
    messages: { detachedView: WASM_VIEW_MESSAGE },
  },
  create(context) {
    const services = context.sourceCode.parserServices;
    const checker = services.program?.getTypeChecker();
    const nodeMap = services.esTreeNodeToTSNodeMap;
    if (checker === undefined || nodeMap === undefined) {
      throw new Error(
        `${WASM_VIEW_RULE} requires TypeScript project-service information`,
      );
    }
    return {
      NewExpression(node) {
        if (!isTypedArrayConstruction(checker, nodeMap, node.callee)) {
          return;
        }
        const argument = node.arguments[0];
        if (
          argument?.type !== "MemberExpression" ||
          memberName(argument) !== "buffer"
        ) {
          return;
        }
        const receiver = nodeMap.get(argument.object);
        if (
          hasGlobalSymbol(
            checker,
            checker.getTypeAtLocation(receiver),
            new Set(["WebAssembly.Memory"]),
          )
        ) {
          context.report({ node, messageId: "detachedView" });
        }
      },
    };
  },
};

const ocelliPlugin = {
  rules: { "no-wasm-memory-view": noWasmMemoryViewRule },
};

// Keep the syntax refusal for aliases of the buffer itself. Once `.buffer`
// has been stored in a variable its type is only ArrayBuffer and its wasm
// provenance is intentionally gone from the TypeScript type system.
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

const RESTRICTED = "no-restricted-syntax";
const SYNTAX_BAN = [NO_CACHED_WASM_MEMORY_ALIAS];

// The file lists permitted to switch the ban off, and nothing else may. Each
// appears exactly once. The reasoning for each is at the block that uses it.
const ALLOWED_TO_DISABLE = [
  ["packages/core/src/bulk.ts", "packages/core/src/panic.ts"],
  ["packages/core/src/*.test.ts"],
];

/**
 * Refuse a config that weakens either half of the wasm-memory ban anywhere.
 */
function assertTheBanIsIntact(config) {
  const disabledSyntax = [];
  const disabledTyped = [];
  let enforcedSyntax = 0;
  let enforcedTyped = 0;
  for (const entry of config) {
    const setting = entry?.rules?.[RESTRICTED];
    const where = JSON.stringify(entry.files ?? null);
    if (setting !== undefined) {
      const level = Array.isArray(setting) ? setting[0] : setting;
      if (level === "off" || level === 0) {
        disabledSyntax.push(where);
      } else {
        const selectors = Array.isArray(setting) ? setting.slice(1) : [];
        if (
          level !== "error" ||
          selectors.length !== SYNTAX_BAN.length ||
          !SYNTAX_BAN.every((rule, index) => selectors[index] === rule)
        ) {
          throw new Error(
            `eslint.config.js: the block for ${where} weakens ${RESTRICTED}`,
          );
        }
        enforcedSyntax += 1;
      }
    }
    const typed = entry?.rules?.[WASM_VIEW_RULE];
    if (typed === "off" || typed === 0) {
      disabledTyped.push(where);
    } else if (typed !== undefined) {
      if (typed !== "error") {
        throw new Error(
          `eslint.config.js: the block for ${where} weakens ${WASM_VIEW_RULE}`,
        );
      }
      enforcedTyped += 1;
    }
  }
  if (enforcedSyntax !== 1 || enforcedTyped !== 1) {
    throw new Error(
      `eslint.config.js: expected one enforcement block for ${RESTRICTED} ` +
        `and ${WASM_VIEW_RULE}, got ${enforcedSyntax} and ${enforcedTyped}`,
    );
  }
  const allowed = ALLOWED_TO_DISABLE.map((files) => JSON.stringify(files));
  const exact = (disabled) =>
    disabled.length === allowed.length &&
    disabled.every((where) => allowed.includes(where));
  if (!exact(disabledSyntax) || !exact(disabledTyped)) {
    throw new Error(
      `eslint.config.js: wasm-memory allowances must be exactly ${allowed}`,
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
    languageOptions: {
      parserOptions: {
        projectService: {
          allowDefaultProject: [
            "vitest.config.ts",
            "examples/viewer-react/vite.config.ts",
          ],
        },
        tsconfigRootDir: import.meta.dirname,
      },
    },
    plugins: { ocelli: ocelliPlugin },
    rules: {
      "no-restricted-syntax": ["error", ...SYNTAX_BAN],
      [WASM_VIEW_RULE]: "error",
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
      [WASM_VIEW_RULE]: "off",
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
      [WASM_VIEW_RULE]: "off",
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
