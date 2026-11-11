import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

interface ExampleProcess {
  env: Record<string, string | undefined>;
}

const helper = (globalThis as { process?: ExampleProcess }).process?.env
  .VITE_EXAMPLE_SERVER;

const wasmPackage = new URL(
  "../../crates/ocelli-wasm/pkg/ocelli_wasm.js",
  import.meta.url,
);
const repoRoot = new URL("../..", import.meta.url);

// The one source value. `URL.pathname` encodes the path exactly once, so a
// real space is `%20`, a literal percent is `%25` and a unicode name is its
// UTF-8 percent form. From it, two specifier forms:
//
// - `browserWasmUrl`, the encoded form, for the browser. Dev serves it and a
//   build with the pkg absent marks exactly it external. The browser never
//   re-encodes it, so the percent encoding must already be right, which is
//   why a decoded value failed the pc%t and lit%20sp dev cases in F-217 pass
//   2 and why the encoded form is not blind decode-reencode.
// - `rollupWasmUrl`, the decoded form, for a build with the pkg present
//   only. Rollup resolves a `/@fs` specifier as a literal filesystem path,
//   so `%20` there is ENOENT for a checkout with a space, and the decoded
//   path is what it needs. The emitted worker references the rewritten
//   hashed glue chunk, not this specifier, so no build-machine path lands
//   in the dist.
const browserWasmUrl = `/@fs${wasmPackage.pathname}`;
const rollupWasmUrl = `/@fs${decodeURIComponent(wasmPackage.pathname)}`;

// The build output depends on whether the pkg exists when `vite build` runs.
// Vite 7's Rollup worker sub-build resolves a constant specifier and loads
// the file. With the pkg present the wasm glue and the `.wasm` are emitted as
// their own chunks, and the worker's import is rewritten to the emitted
// chunk. With it absent, as in a clean clone or the CI frontend job, the
// encoded browser URL is marked external so the import stays a runtime
// import, and the core reads as not built because that import fails. The
// check needs the glue and the `.wasm` both, and runs once when the config
// loads, so the pkg must be built before `vite build` starts for the bundled
// form. `scripts/viewer_build_check.py` builds both states.
const wasmBuilt =
  existsSync(fileURLToPath(wasmPackage)) &&
  existsSync(fileURLToPath(new URL("ocelli_wasm_bg.wasm", wasmPackage)));

export default defineConfig(({ command }) => {
  // Mode AND presence: the decoded specifier is proven for Rollup alone, so
  // it is used only when `command == 'build'` AND the pkg is there. Every
  // other case, dev present, dev absent and build absent, serves the encoded
  // browser URL.
  const buildPresent = command === "build" && wasmBuilt;
  const wasmUrl = buildPresent ? rollupWasmUrl : browserWasmUrl;
  return {
    plugins: [react()],
    define: {
      __OCELLI_WASM_URL__: JSON.stringify(wasmUrl),
    },
    server: {
      port: 5173,
      fs: {
        // The minimal dev filesystem allowance: the checkout root decoded
        // to a path. Vite 7 resolves each allow entry with path.resolve and
        // compares the URL-decoded request path against it with plain
        // prefix checks, realpathing neither side, so the entry must spell
        // the checkout exactly as the `/@fs` request does. Both spellings
        // come from this config's `import.meta.url`, which the decoded form
        // preserves and a realpathed entry could contradict. This is a
        // dev-server allowance only, not a build resolver and not a
        // production HTTP policy. The old value was the percent-encoded
        // `repoRoot.pathname`, which answered 403 for the pkg URL under any
        // checkout path that `URL.pathname` encodes.
        allow: [fileURLToPath(repoRoot)],
      },
      proxy:
        helper === undefined
          ? undefined
          : {
              "/frame-open": { target: helper },
              "/series-open": { target: helper },
              "/overlay": { target: helper },
              "/interpolate": { target: helper },
              "/calibration": { target: helper },
              "/tilt": { target: helper },
            },
    },
    // The wasm module lands in crates/ocelli-wasm/pkg via `bin/ocelli.sh wasm`.
    // It is not committed, so a clean clone runs this app in its "core not
    // built" state rather than failing to start. That is deliberate.
    optimizeDeps: { exclude: ["@ocelli/core"] },
    // The page already constructs this worker as a module. Vite 7's default
    // iife worker bundle cannot emit the wasm import, which is its own chunk.
    worker: {
      format: "es",
      rollupOptions: buildPresent ? undefined : { external: [browserWasmUrl] },
    },
  };
});
