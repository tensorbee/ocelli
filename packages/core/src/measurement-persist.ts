/**
 * JSON persistence for the SR content tree.
 *
 * The HLD does not give the document layout. The local rules are in
 * `.claude/plans/F-096-design.md`. A count of 4000 is the bulk tier and
 * returns nothing. This module does not define that binary layout.
 */

import { contentTree, type ContentItem } from "./sr.js";

const FORMAT = "ocelli-measurement";
const VERSION = 1;

/** Local bulk threshold. The package index does not re-export it. */
export const MEASUREMENT_ITEM_LIMIT = 4000;

function countNodes(item: ContentItem): number {
  let count = 1;
  for (const child of item.children) {
    count += countNodes(child);
  }
  return count;
}

function copyRoot(value: unknown): ContentItem | null {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }
  try {
    return contentTree(value as ContentItem);
  } catch {
    return null;
  }
}

function copyAll(items: readonly unknown[]): { items: ContentItem[]; count: number } | null {
  const copied: ContentItem[] = [];
  let count = 0;
  for (const item of items) {
    const next = copyRoot(item);
    if (next === null) {
      return null;
    }
    count += countNodes(next);
    if (count >= MEASUREMENT_ITEM_LIMIT) {
      return null;
    }
    copied.push(next);
  }
  return { items: copied, count };
}

/** Copy every root into one JSON document. One bad node returns nothing. */
export function persistMeasurements(items: readonly ContentItem[]): string | null {
  const copied = copyAll(items);
  if (copied === null) {
    return null;
  }
  return JSON.stringify({
    format: FORMAT,
    version: VERSION,
    items: copied.items,
  });
}

/** Read a document back through the content tree. A bad document returns nothing. */
export function loadMeasurements(document: string): readonly ContentItem[] | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(document);
  } catch {
    return null;
  }
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    return null;
  }
  if (!("format" in parsed) || !("version" in parsed) || !("items" in parsed)) {
    return null;
  }
  if (parsed.format !== FORMAT || parsed.version !== VERSION || !Array.isArray(parsed.items)) {
    return null;
  }
  const copied = copyAll(parsed.items);
  if (copied === null) {
    return null;
  }
  return copied.items;
}
