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
const NO_CACHED_WASM_VIEW = {
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

export default tseslint.config(
  {
    ignores: [
      "**/dist/**",
      "**/pkg/**",
      "**/node_modules/**",
      "target/**",
      "corpus/**",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["**/*.{ts,tsx}"],
    rules: {
      "no-restricted-syntax": ["error", NO_CACHED_WASM_VIEW],
    },
  },
  {
    // The one file permitted to build a view over linear memory.
    files: ["packages/core/src/bulk.ts"],
    rules: {
      "no-restricted-syntax": "off",
    },
  },
  {
    files: ["packages/react/**/*.{ts,tsx}", "examples/**/*.{ts,tsx}"],
    plugins: { "react-hooks": reactHooks },
    rules: reactHooks.configs.recommended.rules,
  },
);
