/**
 * Two elements, one library each. F-106, HLD section 12.
 *
 * Ocelli enables only its own element. The neighbour stays with the
 * application. This module does not import another viewer.
 */

import { enableElement } from "./seam.js";
import type { ElementSeam } from "./seam.js";

export interface SideBySide {
  readonly ocelli: ElementSeam;
  readonly neighbour: EventTarget;
  release(): boolean;
}

const pairs = new WeakMap<object, SideBySide>();

function isObject(value: EventTarget): value is EventTarget {
  return typeof value === "object" && value !== null;
}

export function attachSideBySide(
  ocelliElement: EventTarget,
  neighbour: EventTarget,
): SideBySide | null {
  if (!isObject(ocelliElement) || !isObject(neighbour) || Object.is(ocelliElement, neighbour)) {
    return null;
  }
  if (pairs.has(ocelliElement)) {
    return null;
  }
  const seam = enableElement(ocelliElement);
  if (seam === null) {
    return null;
  }
  const pair: SideBySide = {
    ocelli: seam,
    neighbour,
    release() {
      if (pairs.get(ocelliElement) !== pair) {
        return false;
      }
      pairs.delete(ocelliElement);
      return seam.disable();
    },
  };
  pairs.set(ocelliElement, pair);
  return pair;
}
