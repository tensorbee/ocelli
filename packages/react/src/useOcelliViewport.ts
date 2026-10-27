/**
 * React binding for one viewport pump. F-102.
 *
 * The pump is created during render and started after commit. The recommended
 * hooks rules refuse a ref that render reads and refuse setState from the
 * effect that would otherwise own the pump. Latest callbacks are bound after
 * commit, so a new function does not rebuild the pump.
 */

import { useEffect, useLayoutEffect, useMemo, useState } from "react";
import { TIER_C_DRAWING } from "@ocelli/core";
import type {
  OcelliNamedEvent,
  PackedCommand,
  TierAvailability,
  ViewportId,
  ViewportReadback,
} from "@ocelli/core";

import { bindViewportPump, createViewportPump } from "./pump.js";
import type { OcelliSession, ViewportPump } from "./pump.js";

export interface UseOcelliViewportOptions {
  readonly viewport: ViewportId;
  readonly session: OcelliSession | null;
  readonly onEvents?:
    | ((events: readonly OcelliNamedEvent[], dropped: number) => void)
    | undefined;
  readonly onRefused?:
    | ((commands: readonly PackedCommand[]) => void)
    | undefined;
}

export interface OcelliViewportHandle {
  readonly pump: ViewportPump | null;
  readonly state: ViewportReadback | null;
  readonly drawing: TierAvailability;
}

export function useOcelliViewport(
  options: UseOcelliViewportOptions,
): OcelliViewportHandle {
  const [held, setHeld] = useState<{
    readonly epoch: number;
    readonly state: ViewportReadback;
  } | null>(null);
  const [generation, setGeneration] = useState(() => ({
    session: options.session,
    viewport: options.viewport,
    epoch: 0,
  }));

  if (
    generation.session !== options.session
    || generation.viewport !== options.viewport
  ) {
    setGeneration({
      session: options.session,
      viewport: options.viewport,
      epoch: generation.epoch + 1,
    });
  }

  const epoch = generation.session === options.session
    && generation.viewport === options.viewport
    ? generation.epoch
    : generation.epoch + 1;

  const pump = useMemo(() => {
    const session = options.session;
    if (session === null) {
      return null;
    }
    return createViewportPump(session, {
      viewport: options.viewport,
    });
  }, [options.session, options.viewport]);

  useLayoutEffect(() => {
    if (pump === null) {
      return;
    }
    bindViewportPump(pump, {
      onEvents: options.onEvents,
      onRefused: options.onRefused,
      onState(next) {
        setHeld({ epoch, state: next });
      },
    });
  }, [epoch, options.onEvents, options.onRefused, pump]);

  useEffect(() => {
    if (pump === null) {
      return;
    }
    pump.start();
    return () => {
      pump.stop();
    };
  }, [pump]);

  const state = held !== null && held.epoch === epoch ? held.state : null;
  return { pump, state, drawing: TIER_C_DRAWING };
}
