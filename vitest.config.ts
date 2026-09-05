import { defineConfig } from "vitest/config";

/**
 * `npm run test` is `vitest run`. Without this file vitest looks under the
 * repository root and also walks `crates/` and `corpus/`, which is slow and
 * which would let a stray `.test.ts` anywhere be picked up silently.
 */
export default defineConfig({
  test: {
    include: ["packages/*/src/**/*.test.ts", "examples/*/src/**/*.test.ts"],
    // A run that finds no test files is a failure here. This project's rule is
    // that a check which could not run must never read as one that ran and was
    // happy, and vitest's default is to pass an empty run.
    passWithNoTests: false,
  },
});
