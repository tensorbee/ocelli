/**
 * The integration seam. F-105, HLD section 12.
 *
 * One plain element receives the named shell events. The ring is still
 * drained once per frame by the pump. This module does not listen.
 */

import { OCELLI_EVENTS } from "./events.js";
import type { OcelliEventName, OcelliNamedEvent } from "./events.js";

export type SeamEvent =
  | OcelliNamedEvent
  | { readonly name: OcelliEventName };

export interface ElementSeam {
  readonly dispatch: (event: SeamEvent) => boolean;
  readonly disable: () => boolean;
}

const NAMES: ReadonlySet<string> = new Set(OCELLI_EVENTS);
const seams = new WeakMap<object, ElementSeam>();

function isTarget(value: EventTarget): value is EventTarget {
  return typeof value === "object"
    && value !== null
    && typeof value.dispatchEvent === "function";
}

function deliver(element: EventTarget, event: SeamEvent): boolean {
  if (!NAMES.has(event.name)) {
    return false;
  }
  try {
    element.dispatchEvent(new CustomEvent<SeamEvent>(event.name, {
      bubbles: false,
      cancelable: false,
      detail: event,
    }));
  } catch {
    return false;
  }
  return true;
}

export function enableElement(element: EventTarget): ElementSeam | null {
  if (!isTarget(element) || seams.has(element)) {
    return null;
  }
  const seam: ElementSeam = {
    dispatch(event) {
      if (seams.get(element) !== seam) {
        return false;
      }
      return deliver(element, event);
    },
    disable() {
      if (seams.get(element) !== seam) {
        return false;
      }
      seams.delete(element);
      try {
        element.dispatchEvent(new CustomEvent<SeamEvent>("viewport.unmounted", {
          bubbles: false,
          cancelable: false,
          detail: { name: "viewport.unmounted" },
        }));
      } catch {
        return true;
      }
      return true;
    },
  };
  seams.set(element, seam);
  if (!seam.dispatch({ name: "viewport.mounted" })) {
    seams.delete(element);
    return null;
  }
  return seam;
}

export function disableElement(element: EventTarget): boolean {
  if (typeof element !== "object" || element === null) {
    return false;
  }
  const seam = seams.get(element);
  if (seam === undefined) {
    return false;
  }
  return seam.disable();
}

export function dispatchOnElement(element: EventTarget, event: SeamEvent): boolean {
  if (typeof element !== "object" || element === null) {
    return false;
  }
  const seam = seams.get(element);
  if (seam === undefined) {
    return false;
  }
  return seam.dispatch(event);
}
