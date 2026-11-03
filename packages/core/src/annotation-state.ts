/**
 * Annotation records indexed by frame of reference, with one undo step.
 *
 * Decision D13 says each record holds the F-094 content tree. The HLD does
 * not define an undo stack. The local rules are in
 * `.claude/plans/F-095-design.md`. This module does not measure and it does
 * not edit the drawing layer.
 */

import { contentTree, type ContentItem } from "./sr.js";

export type AnnotationRecord = {
  id: string;
  frameOfReference: string;
  tree: ContentItem;
};

export type AnnotationState = {
  readonly records: readonly AnnotationRecord[];
  readonly undo: readonly (readonly AnnotationRecord[])[];
  readonly redo: readonly (readonly AnnotationRecord[])[];
};

function copyRecord(record: AnnotationRecord): AnnotationRecord | null {
  const tree = contentTree(record.tree);
  if (tree === null) {
    return null;
  }
  return {
    id: record.id,
    frameOfReference: record.frameOfReference,
    tree,
  };
}

function copyList(records: readonly AnnotationRecord[]): readonly AnnotationRecord[] | null {
  const copied: AnnotationRecord[] = [];
  for (const record of records) {
    const next = copyRecord(record);
    if (next === null) {
      return null;
    }
    copied.push(next);
  }
  return copied;
}

/** An empty record list and two empty stacks. */
export function createAnnotationState(): AnnotationState {
  return { records: [], undo: [], redo: [] };
}

/**
 * Append one record, or replace the same id.
 * A blank id, a blank UID, or a tree `contentTree` refuses returns nothing
 * and does not write `state`.
 */
export function putAnnotation(
  state: AnnotationState,
  id: string,
  frameOfReference: string,
  tree: ContentItem,
): AnnotationState | null {
  if (id.length === 0 || frameOfReference.length === 0) {
    return null;
  }
  const copiedTree = contentTree(tree);
  const records = copyList(state.records);
  if (copiedTree === null || records === null) {
    return null;
  }
  const next: AnnotationRecord = { id, frameOfReference, tree: copiedTree };
  const index = records.findIndex((record) => record.id === id);
  const stored = records.slice();
  if (index >= 0) {
    stored[index] = next;
  } else {
    stored.push(next);
  }
  const prior = copyList(state.records);
  const history = [];
  for (const step of state.undo) {
    const copied = copyList(step);
    if (copied === null) {
      return null;
    }
    history.push(copied);
  }
  if (prior === null) {
    return null;
  }
  history.push(prior);
  return { records: stored, undo: history, redo: [] };
}

/** Records whose UID is exactly `frameOfReference`, in insertion order. */
export function annotationsForFrame(
  state: AnnotationState,
  frameOfReference: string,
): readonly AnnotationRecord[] | null {
  const found: AnnotationRecord[] = [];
  for (const record of state.records) {
    if (record.frameOfReference !== frameOfReference) {
      continue;
    }
    const copied = copyRecord(record);
    if (copied === null) {
      return null;
    }
    found.push(copied);
  }
  return found;
}

/** The record list from before the latest successful put. */
export function undoAnnotation(state: AnnotationState): AnnotationState | null {
  const prior = state.undo[state.undo.length - 1];
  if (prior === undefined) {
    return null;
  }
  const records = copyList(prior);
  const current = copyList(state.records);
  if (records === null || current === null) {
    return null;
  }
  const undo = [];
  for (const step of state.undo.slice(0, -1)) {
    const copied = copyList(step);
    if (copied === null) {
      return null;
    }
    undo.push(copied);
  }
  const redo = [];
  for (const step of state.redo) {
    const copied = copyList(step);
    if (copied === null) {
      return null;
    }
    redo.push(copied);
  }
  redo.push(current);
  return { records, undo, redo };
}

/** The record list that the latest undo left. */
export function redoAnnotation(state: AnnotationState): AnnotationState | null {
  const next = state.redo[state.redo.length - 1];
  if (next === undefined) {
    return null;
  }
  const records = copyList(next);
  const current = copyList(state.records);
  if (records === null || current === null) {
    return null;
  }
  const undo = [];
  for (const step of state.undo) {
    const copied = copyList(step);
    if (copied === null) {
      return null;
    }
    undo.push(copied);
  }
  undo.push(current);
  const redo = [];
  for (const step of state.redo.slice(0, -1)) {
    const copied = copyList(step);
    if (copied === null) {
      return null;
    }
    redo.push(copied);
  }
  return { records, undo, redo };
}
