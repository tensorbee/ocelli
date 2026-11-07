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
const wasmUrl = `/@fs${wasmPackage.pathname}`;

// The build output depends on whether the pkg exists when `vite build` runs.
// The define below folds the worker's dynamic import into a constant string,
// and Vite 7's Rollup worker sub-build resolves a constant specifier and loads
// the file. With the pkg present the wasm glue and the `.wasm` are emitted as
// their own chunks. With it absent, as in a clean clone or the CI frontend
// job, the URL is marked external so the import stays a runtime import and
// the worker's probe reports the core as not built. The check runs once when
// the config loads, so the pkg must be built before `vite build` starts for
// the bundled form.
// `scripts/viewer_build_check.py` builds both states.
const wasmBuilt = existsSync(fileURLToPath(wasmPackage));

export default defineConfig({
  plugins: [react()],
  define: {
    __OCELLI_WASM_URL__: JSON.stringify(wasmUrl),
  },
  server: {
    port: 5173,
    fs: {
      allow: [repoRoot.pathname],
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
    rollupOptions: wasmBuilt ? undefined : { external: [wasmUrl] },
  },
});
