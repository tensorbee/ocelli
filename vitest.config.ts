import { defineConfig } from "vitest/config";

/**
 * `npm run test` is `vitest run`. Without this file vitest looks under the
 * repository root and also walks `crates/` and `corpus/`, which is slow and
 * which would let a stray `.test.ts` anywhere be picked up silently.
 */
export default defineConfig({
  test: {
    include: ["packages/*/src/**/*.test.ts", "examples/*/src/**/*.test.ts"],
    // The wasm boundary file needs a built module. `npm run test` does not
    // build one. Run it with `npx vitest run --config vitest.boundary.config.ts`
    // after `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm`.
    // The default config excludes the file, so naming it without `--config`
    // finds no test files and exits 1.
    exclude: ["packages/core/src/boundary.wasm.test.ts"],
    // A run that finds no test files is a failure here. This project's rule is
    // that a check which could not run must never read as one that ran and was
    // happy, and vitest's default is to pass an empty run.
    passWithNoTests: false,
  },
});
