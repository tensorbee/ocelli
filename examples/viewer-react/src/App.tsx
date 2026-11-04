import { coreAvailable, VERSION, viewportId } from "@ocelli/core";
import { OcelliViewport } from "@ocelli/react";
import type { OcelliSession } from "@ocelli/react";
import { useState } from "react";

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

interface FrameOpenView {
  display: number | null;
  paddingCount: number;
  colour: string;
  lossy: string;
  burnedIn: string;
  windowLabel: string;
  creators: string;
}

function readView(value: unknown): FrameOpenView | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const record = value as Record<string, unknown>;
  if (typeof record.padding_count !== "number") {
    return null;
  }
  if (typeof record.colour !== "string") {
    return null;
  }
  if (typeof record.lossy !== "string") {
    return null;
  }
  if (typeof record.burned_in !== "string") {
    return null;
  }
  if (typeof record.window !== "string") {
    return null;
  }
  const display = record.display;
  if (display !== undefined && typeof display !== "number") {
    return null;
  }
  return {
    display: typeof display === "number" ? display : null,
    paddingCount: record.padding_count,
    colour: record.colour,
    lossy: record.lossy,
    burnedIn: record.burned_in,
    windowLabel: record.window,
    creators: creatorLine(record.creators),
  };
}

function creatorLine(value: unknown): string {
  if (!Array.isArray(value)) {
    return "";
  }
  const lines: string[] = [];
  for (const item of value) {
    if (typeof item !== "object" || item === null) {
      continue;
    }
    const record = item as Record<string, unknown>;
    if (
      typeof record.group !== "number" ||
      typeof record.slot !== "number" ||
      typeof record.creator !== "string"
    ) {
      continue;
    }
    lines.push(
      `${record.group.toString(16)}:${record.slot.toString(16)} ${record.creator}`,
    );
  }
  return lines.join(", ");
}

function problemSentence(text: string): string {
  try {
    const parsed: unknown = JSON.parse(text);
    if (typeof parsed === "object" && parsed !== null && "error" in parsed) {
      const code = (parsed as { error?: unknown }).error;
      if (typeof code === "string") {
        return `The frame could not be opened (${code}).`;
      }
    }
  } catch {
    return "The frame could not be opened.";
  }
  return "The frame could not be opened.";
}

function FrameOpenPanel() {
  const [view, setView] = useState<FrameOpenView | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  async function run(body: BodyInit): Promise<void> {
    setPending(true);
    setProblem(null);
    try {
      const response = await fetch("/frame-open", { method: "POST", body });
      const text = await response.text();
      if (!response.ok) {
        setProblem(problemSentence(text));
        return;
      }
      const parsed: unknown = JSON.parse(text);
      const next = readView(parsed);
      if (next === null) {
        setProblem("The frame-open response could not be read.");
        return;
      }
      setView(next);
    } catch {
      setProblem("The frame-open helper is not running.");
    } finally {
      setPending(false);
    }
  }

  return (
    <section>
      <h2>Frame open</h2>
      <p>
        The synthetic button opens a built-in monochrome sample. Stored 50,
        rescale slope 1, intercept 0, LINEAR_EXACT centre 100 and width 200.
        A file you choose is posted to the helper on this machine and is not
        saved.
      </p>
      <p>
        <button
          type="button"
          disabled={pending}
          onClick={() => {
            void run("synthetic");
          }}
        >
          Synthetic frame
        </button>{" "}
        <input
          type="file"
          disabled={pending}
          aria-label="Choose a Part 10 file"
          onChange={(event) => {
            const file = event.currentTarget.files?.item(0);
            if (file !== undefined && file !== null) {
              void run(file);
            }
          }}
        />
      </p>
      {problem !== null && <p>{problem}</p>}
      {view !== null && (
        <div>
          {view.display !== null && <p>Display {view.display}</p>}
          {view.display !== null ? (
            <p>Padding count {view.paddingCount}</p>
          ) : (
            <p>Declared padding ranges {view.paddingCount}</p>
          )}
          <p>Colour {view.colour}</p>
          <p>Window {view.windowLabel}</p>
          <p>Lossy {view.lossy}</p>
          <p>Burned in {view.burnedIn}</p>
          {view.creators !== "" && <p>Creators {view.creators}</p>}
        </div>
      )}
    </section>
  );
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

      <FrameOpenPanel />

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
