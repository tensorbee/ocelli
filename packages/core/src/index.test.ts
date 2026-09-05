import { describe, expect, it } from "vitest";

import { coreAvailable, VERSION } from "./index.js";

describe("@ocelli/core", () => {
  /**
   * The package version and the Rust workspace version are one number.
   * `docs/RELEASE.md` says the crates and the packages version together and
   * that a skew between them is not a supported configuration.
   *
   * This asserts the literal rather than reading `package.json`, for the same
   * reason `ocelli_version()`'s test in `crates/ocelli-wasm` asserts a
   * literal: comparing the constant against the file it was copied from
   * restates it, and passes whatever either says.
   *
   * `scripts/package_check.py` is what compares the two files. This is what
   * catches the constant drifting away from its own manifest.
   */
  it("declares the workspace version", () => {
    expect(VERSION).toBe("0.1.0");
  });

  /**
   * A clean clone has no built wasm core, so this is `false` and that is the
   * honest answer rather than a failure to start. The example viewer uses it
   * to render a "core not built" state.
   *
   * **When F-096 makes this detect a real core, this test has to change**, and
   * that is the point of asserting it now. The change becomes visible in a
   * diff instead of happening quietly.
   */
  it("reports no core until one is built", () => {
    expect(coreAvailable()).toBe(false);
  });
});
