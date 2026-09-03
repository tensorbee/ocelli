import { coreAvailable, VERSION } from "@ocelli/core";
import { OcelliViewport } from "@ocelli/react";

/**
 * The example viewer.
 *
 * It starts as the smallest thing that proves the boundary and grows one
 * panel per landed viewport or tool story. It is also the manual smoke test
 * `/verify` points a human at, so it must run from a clean clone.
 *
 * A clean clone has no built wasm module, so the honest state is "core not
 * built" rather than a crash. F-002 (`bin/ocelli.sh wasm`) produces the
 * module, F-096 makes this panel show a frame.
 */
export function App() {
  const ready = coreAvailable();

  return (
    <main>
      <header>
        <h1>Ocelli example viewer</h1>
        <p>
          <code>@ocelli/core</code> {VERSION} &middot;{" "}
          {ready ? "core ready" : "core not built"}
        </p>
      </header>

      {!ready && (
        <section>
          <h2>No core in this clone</h2>
          <p>
            The wasm module is built, not committed. Run{" "}
            <code>bin/ocelli.sh wasm</code> to produce{" "}
            <code>crates/ocelli-wasm/pkg</code>, then reload.
          </p>
          <p>
            Until F-096 lands there is no boundary to attach to, so the
            viewport below mounts its host element and reports its state and
            does nothing else. That is the whole of the current milestone.
          </p>
        </section>
      )}

      <section>
        <h2>Stack viewport</h2>
        <OcelliViewport id="stack-1" className="viewport" />
      </section>
    </main>
  );
}
