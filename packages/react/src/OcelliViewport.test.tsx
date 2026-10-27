// @vitest-environment jsdom
/**
 * F-102 component lifecycle. The host element is the seam. A missing session
 * is the absent state and does not throw.
 */
import { useEffect } from "react";
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TIER_C_DRAWING, viewportId } from "@ocelli/core";

import { OcelliViewport } from "./OcelliViewport.js";
import { useOcelliViewport } from "./useOcelliViewport.js";
import type { OcelliSession } from "./pump.js";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

function installFrames(): { requested: number; cancelled: number } {
  const seen = { requested: 0, cancelled: 0 };
  vi.stubGlobal("requestAnimationFrame", () => {
    seen.requested += 1;
    return seen.requested;
  });
  vi.stubGlobal("cancelAnimationFrame", () => {
    seen.cancelled += 1;
  });
  return seen;
}

function session(): OcelliSession {
  const memory = new WebAssembly.Memory({ initial: 1 });
  return {
    memory,
    alloc: () => 64,
    bulk_ticket: () => 1,
    release_bulk: () => 0,
    apply_commands: () => 0,
    drain_events: () => new Uint8Array(16),
    copy_state: () => new Uint8Array(96),
    snapshot: () => new Uint8Array(),
    snapshot_error: () => 1,
    restore_snapshot: () => 1,
  };
}

function DrawingProbe({
  live,
  session,
}: {
  readonly live: boolean;
  readonly session: OcelliSession;
}) {
  const handle = useOcelliViewport({
    viewport: viewportId(1),
    session: live ? session : null,
  });
  return (
    <div
      data-testid="drawing"
      data-drawing={handle.drawing}
      data-pump={handle.pump === null ? "no" : "yes"}
    />
  );
}

describe("F-102 OcelliViewport", () => {
  it("renders_the_absent_state_without_a_session", () => {
    const frames = installFrames();
    const view = render(
      <OcelliViewport id="stack-1" viewport={viewportId(1)} />,
    );
    const host = view.getByTestId("viewport-stack-1");
    expect(host.getAttribute("data-ocelli-viewport")).toBe("stack-1");
    expect(host.getAttribute("data-ocelli-core")).toBe("absent");
    expect(host.tagName).toBe("DIV");
    expect(frames.requested).toBe(0);
  });

  it("starts_with_a_session_and_stops_on_unmount", () => {
    const frames = installFrames();
    const view = render(
      <OcelliViewport id="stack-1" viewport={viewportId(1)} session={session()} />,
    );
    expect(view.getByTestId("viewport-stack-1").getAttribute("data-ocelli-core")).toBe("ready");
    expect(frames.requested).toBe(1);
    view.unmount();
    expect(frames.cancelled).toBe(1);
  });

  it("keeps_one_loop_when_the_callback_changes", () => {
    const frames = installFrames();
    const liveSession = session();
    const seen: number[] = [];
    const sink: { flush: (() => void) | null } = { flush: null };
    function Probe({ marker }: { readonly marker: number }) {
      const handle = useOcelliViewport({
        viewport: viewportId(1),
        session: liveSession,
        onEvents() {
          seen.push(marker);
        },
      });
      useEffect(() => {
        sink.flush = () => {
          handle.pump?.flushNow();
        };
      });
      return null;
    }
    const view = render(<Probe marker={1} />);
    expect(frames.requested).toBe(1);
    view.rerender(<Probe marker={2} />);
    expect(frames.requested).toBe(1);
    expect(frames.cancelled).toBe(0);
    sink.flush?.();
    expect(seen).toEqual([2]);
  });

  it("reports_tier_c_drawing", () => {
    installFrames();
    const liveSession = session();
    const absent = render(<DrawingProbe live={false} session={liveSession} />);
    expect(TIER_C_DRAWING).toBe("unavailable");
    expect(absent.getByTestId("drawing").getAttribute("data-drawing")).toBe(TIER_C_DRAWING);
    expect(absent.getByTestId("drawing").getAttribute("data-pump")).toBe("no");
    absent.unmount();
    const live = render(<DrawingProbe live session={liveSession} />);
    expect(live.getByTestId("drawing").getAttribute("data-drawing")).toBe(TIER_C_DRAWING);
    expect(live.getByTestId("drawing").getAttribute("data-pump")).toBe("yes");
  });
});
