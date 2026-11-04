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

export default defineConfig({
  plugins: [react()],
  define: {
    __OCELLI_WASM_URL__: JSON.stringify(`/@fs${wasmPackage.pathname}`),
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
});
