import { defineConfig } from "vitest/config";

/**
 * The wasm boundary file is excluded from the default `npm run test` config
 * because that command does not build the module. This config runs only that
 * file, after the dev wasm-pack build named in the test header.
 */
export default defineConfig({
  test: {
    include: ["packages/core/src/boundary.wasm.test.ts"],
    passWithNoTests: false,
  },
});
