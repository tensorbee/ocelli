/**
 * Per-viewport kind flags. F-107, HLD section 12.
 *
 * Stack turns on before MPR, and MPR turns on before volume.
 * An element is enabled only while its kind is on.
 * This module does not import another viewer.
 */

import { enableElement } from "./seam.js";
import type { ElementSeam } from "./seam.js";

export type ViewportKind = "stack" | "mpr" | "volume";

const ORDER: readonly ViewportKind[] = ["stack", "mpr", "volume"];

const flags = new Map<ViewportKind, boolean>();
const stackElements = new Set<EventTarget>();
const mprElements = new Set<EventTarget>();
const volumeElements = new Set<EventTarget>();

for (const kind of ORDER) {
  flags.set(kind, false);
}

interface Stored {
  readonly kind: ViewportKind;
  readonly seam: ElementSeam;
}

const stored = new WeakMap<object, Stored>();
const reserved = new WeakMap<object, ViewportKind>();

let changing = false;

function indexOfKind(kind: string): number {
  let index = 0;
  for (const item of ORDER) {
    if (item === kind) {
      return index;
    }
    index += 1;
  }
  return -1;
}

function heldFor(kind: ViewportKind): Set<EventTarget> {
  switch (kind) {
    case "stack":
      return stackElements;
    case "mpr":
      return mprElements;
    case "volume":
      return volumeElements;
  }
}

export function kindEnabled(kind: ViewportKind): boolean {
  return flags.get(kind) === true;
}

export function setKindEnabled(kind: ViewportKind, enabled: boolean): boolean {
  const index = indexOfKind(kind);
  if (index < 0) {
    return false;
  }
  if (flags.get(kind) === enabled) {
    return true;
  }
  if (changing) {
    return false;
  }
  if (enabled) {
    if (index > 0) {
      const previous = ORDER[index - 1];
      if (previous === undefined || flags.get(previous) !== true) {
        return false;
      }
    }
    flags.set(kind, true);
    return true;
  }
  const next = ORDER[index + 1];
  if (next !== undefined && flags.get(next) === true) {
    return false;
  }
  const held = heldFor(kind);
  const seams: ElementSeam[] = [];
  // A listener on viewport.unmounted must not turn this flag back on.
  changing = true;
  try {
    for (const element of held) {
      const row = stored.get(element);
      stored.delete(element);
      reserved.delete(element);
      if (row !== undefined) {
        seams.push(row.seam);
      }
    }
    held.clear();
    flags.set(kind, false);
    for (const seam of seams) {
      seam.disable();
    }
  } finally {
    changing = false;
  }
  return true;
}

export function enableFlagged(
  element: EventTarget,
  kind: ViewportKind,
): ElementSeam | null {
  if (indexOfKind(kind) < 0 || flags.get(kind) !== true) {
    return null;
  }
  if (
    typeof element !== "object"
    || element === null
    || stored.has(element)
    || reserved.has(element)
  ) {
    return null;
  }
  // Reserve before mounted runs, so a listener that turns this kind off
  // drops the element instead of leaving it mounted with the flag off.
  reserved.set(element, kind);
  heldFor(kind).add(element);
  const seam = enableElement(element);
  const kept = reserved.get(element) === kind && flags.get(kind) === true;
  if (!kept) {
    reserved.delete(element);
    heldFor(kind).delete(element);
    stored.delete(element);
    if (seam !== null) {
      changing = true;
      try {
        seam.disable();
      } finally {
        changing = false;
      }
    }
    return null;
  }
  reserved.delete(element);
  if (seam === null) {
    heldFor(kind).delete(element);
    return null;
  }
  stored.set(element, { kind, seam });
  return seam;
}
