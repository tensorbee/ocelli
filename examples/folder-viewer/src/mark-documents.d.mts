// Types for `mark-documents.mjs`, which the page loads as plain JavaScript.

import type { ContentItem, DocumentReference, KeyObjects, TidGroups } from "@ocelli/core";

export type MarkPoint = { x: number; y: number };

/** A page mark. `result` is its `/measure` reply, when it has one. */
export type Mark = {
  type: string;
  points: readonly MarkPoint[];
  text?: string;
  tracking: string;
  result?: unknown;
};

/** The page image a mark is on, with the `/inspect` fields a document copies. */
export type MarkImage = {
  key: string;
  sopClass: string;
  sopInstance: string;
  series: string;
  study: string;
  /** The page frame, counted from 0. */
  frame: number;
  /** Number of Frames of the object. */
  frames: number;
  columns: number;
  rows: number;
  patientName: string;
  patientId: string;
  studyDate: string;
  studyTime: string;
  accession: string;
};

export type PlacedImage = Pick<MarkImage, "key" | "sopInstance" | "frame" | "frames" | "columns" | "rows">;

export type BuiltDocument = {
  kind: "sr" | "ko";
  /** An SR of no groups. */
  empty?: boolean;
  study: string;
  bytes: Uint8Array;
  name: string;
  reference: DocumentReference;
};

export type Built = {
  documents: BuiltDocument[];
  saved: { marks: number; keyImages: number };
  unsaved: Record<string, number>;
  noUid: number;
  withoutNumber: number;
  /** Studies whose every mark was deleted, written as a report of no groups. */
  cleared: number;
  /** Studies with no change since the folder was opened or the study last saved, for which nothing is written. */
  unchanged: number;
  /** Studies whose last key image was unflagged, for which nothing is written. */
  unflagged: number;
  /** Reports of a study in this folder read but never placed whole, which this save does not replace. */
  kept: number;
  failed: "" | "sr" | "ko";
};

export type LoadedDocument =
  | ({ name: string; kind: "sr" } & TidGroups)
  | ({ name: string; kind: "ko" } & KeyObjects);

export type Placed = {
  marks: { key: string; mark: Mark; report: string; group: number }[];
  keys: string[];
  keyObjects: string[];
  replaced: string[];
  unplaced: Record<string, number>;
  repeated: number;
  /** The reports placed whole, and those skipped as replaced, which a later save names as predecessors, by study. */
  predecessors: { study: string; reference: DocumentReference; empty: boolean }[];
  /** The reports not placed whole, by study. */
  partial: { study: string; sopInstance: string; reference: DocumentReference }[];
  /** Every SOP Instance UID a report of this read names in Predecessor Documents Sequence. */
  named: string[];
  /** Studies whose report was hidden by an earlier read into this folder. */
  hidden: string[];
};

export const TRACKING_PREFIX: string;
export const SAVED_KINDS: readonly string[];
export const DOCUMENT_CLASSES: readonly string[];

export function froundPoints<T extends { points: readonly MarkPoint[] }>(mark: T): T;
export function markToGroup(mark: Mark, image: MarkImage): ContentItem | null;
export function groupToMark(
  group: ContentItem | null,
): { mark: Mark; reference: { sopInstance: string; frame?: number } } | { reason: string };
export function buildDocuments(state: {
  entries: readonly { image: MarkImage; marks: readonly Mark[]; key: boolean }[];
  predecessors: Readonly<Record<string, readonly (DocumentReference & { empty?: boolean })[]>>;
  /** Studies with a mark added, moved, deleted or edited, or a key image changed, since the folder was opened or the study last saved. */
  changed?: readonly string[];
  /** The SOP Instance UIDs of reports read but not placed whole, by study. */
  partialReports?: Readonly<Record<string, readonly string[]>>;
  /** One image of each study in the folder, for a study with no entry. */
  studies?: Readonly<Record<string, MarkImage>>;
  /** Studies with a Key Object Selection saved or read since the folder was opened. */
  keyStudies?: readonly string[];
  contentDate: string;
  contentTime: string;
}): Built;
export function stableTracking(
  stable: Map<string, string>,
  placed: Placed,
  fresh: () => string,
): void;
export function readDocument(name: string, bytes: Uint8Array): LoadedDocument | null;
export function rememberReport(
  record: Map<string, (DocumentReference & { empty?: boolean })[]>,
  study: string,
  entry: DocumentReference & { empty?: boolean },
): Map<string, (DocumentReference & { empty?: boolean })[]>;
export function notePartialReports(
  partialReports: Map<string, Map<string, DocumentReference>>,
  placed: Pick<Placed, "partial" | "named">,
  replaced?: Set<string>,
): { study: string; reference: DocumentReference }[];
export function placeDocuments(
  documents: readonly (LoadedDocument | null)[],
  images: readonly PlacedImage[],
  shown: ReadonlySet<string>,
  replacedBefore?: ReadonlySet<string>,
): Placed;
