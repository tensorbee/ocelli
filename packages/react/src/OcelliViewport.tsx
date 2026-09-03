import { useEffect, useRef } from "react";
import { coreAvailable } from "@ocelli/core";

export interface OcelliViewportProps {
  /** Viewport id, unique within a session. */
  readonly id: string;
  /** Applied to the host element. The element is the integration seam. */
  readonly className?: string;
}

/**
 * A viewport hosted on a plain DOM element.
 *
 * The element is the seam. Everything Ocelli does to it, another framework
 * binding could do identically, and cornerstone can do to the same element
 * during a strangler migration (HLD section 12).
 *
 * Scaffold. It mounts the element and reports whether a core is present.
 * F-097 attaches a real session.
 */
export function OcelliViewport({ id, className }: OcelliViewportProps) {
  const host = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const element = host.current;
    if (element === null) {
      return;
    }
    element.dataset["ocelliViewport"] = id;
    element.dataset["ocelliCore"] = coreAvailable() ? "ready" : "absent";
    return () => {
      delete element.dataset["ocelliViewport"];
      delete element.dataset["ocelliCore"];
    };
  }, [id]);

  return <div ref={host} className={className} data-testid={`viewport-${id}`} />;
}
