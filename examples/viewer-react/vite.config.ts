import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: { port: 5173 },
  // The wasm module lands in crates/ocelli-wasm/pkg via `bin/ocelli.sh wasm`.
  // It is not committed, so a clean clone runs this app in its "core not
  // built" state rather than failing to start. That is deliberate.
  optimizeDeps: { exclude: ["@ocelli/core"] },
});
