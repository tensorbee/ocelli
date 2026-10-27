import { coreAvailable, VERSION, viewportId } from "@ocelli/core";
import { OcelliViewport } from "@ocelli/react";
import type { OcelliSession } from "@ocelli/react";

/**
 * The example viewer.
 *
 * It starts as the smallest thing that proves the boundary and grows one
 * panel per landed viewport or tool story. It is also the manual smoke test
 * `/verify` points a human at, so it must run from a clean clone.
 *
 * A clean clone has no built wasm module, so the honest state is "core not
 * built" rather than a crash. F-002 (`bin/ocelli.sh wasm`) produces the
 * module. This story does not load that module. The factory returns null,
 * and the viewport stays absent until a caller supplies a session.
 */
function sessionFromModule(): OcelliSession | null {
  return null;
}

export function App() {
  const ready = coreAvailable();
  const session = sessionFromModule();

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
            The viewport below mounts its host element and reports the absent
            state. It starts a frame pump only when a session is supplied.
          </p>
        </section>
      )}

      <section>
        <h2>Stack viewport</h2>
        <OcelliViewport
          id="stack-1"
          className="viewport"
          viewport={viewportId(1)}
          session={session}
        />
      </section>
    </main>
  );
}
