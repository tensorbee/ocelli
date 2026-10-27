import type { OcelliNamedEvent, PackedCommand, ViewportId } from "@ocelli/core";

import type { OcelliSession } from "./pump.js";
import { useOcelliViewport } from "./useOcelliViewport.js";

export interface OcelliViewportProps {
  /** Viewport id, unique within a session. */
  readonly id: string;
  /** The viewport the pump addresses. A slice index is a different number. */
  readonly viewport: ViewportId;
  /** Applied to the host element. The element is the integration seam. */
  readonly className?: string;
  /** The caller's session. Null is the absent state. There is no singleton. */
  readonly session?: OcelliSession | null;
  readonly onEvents?: (events: readonly OcelliNamedEvent[], dropped: number) => void;
  readonly onRefused?: (commands: readonly PackedCommand[]) => void;
}

/**
 * A viewport hosted on a plain DOM element.
 *
 * The element is the seam. The pump submits one command buffer per frame
 * and drains the ring once. This story does not render a canvas.
 */
export function OcelliViewport({
  id,
  viewport,
  className,
  session = null,
  onEvents,
  onRefused,
}: OcelliViewportProps) {
  useOcelliViewport({ viewport, session, onEvents, onRefused });
  const core = session === null ? "absent" : "ready";

  return (
    <div
      className={className}
      data-ocelli-viewport={id}
      data-ocelli-core={core}
      data-testid={`viewport-${id}`}
    />
  );
}
