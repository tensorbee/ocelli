/**
 * The wasm session's own decoded frame cache, F-231.
 *
 * The session keeps committed frames in a cache whose budget the caller
 * sets, and the default is zero, which is off: a caller that never calls
 * {@link setFrameBudget} sees today's behaviour exactly. A commit that
 * carries a content id through `writeFrame` fills the cache, and
 * {@link frameResident} and {@link showResident} read it back without bytes
 * crossing the boundary again. {@link releaseSeries} and
 * {@link releaseStudy} empty it, one event per series through the ring.
 *
 * Every function here refuses before any export call. A bad number is an
 * `OcelliError` `InvalidValue` and an unknown option key is `UnknownOption`,
 * both F-230's codes.
 */

import { OcelliError } from "./ocelli-error.js";
import { RESIDENT_FRAME_REQUEST_KEYS, refuseUnknownOptions } from "./options.js";

/** The session exports this file drives. All of them are new in F-231. */
export interface FrameCacheExports {
  set_frame_budget(bytes: number): number;
  frame_resident(
    index: number,
    epoch: number,
    content: bigint,
    columns: number,
    rows: number,
  ): number;
  show_resident(
    index: number,
    epoch: number,
    content: bigint,
    columns: number,
    rows: number,
  ): number;
  release_series(series: bigint): number;
  request_frame_pressure(): number;
}

/** One frame slot the shell asks about: where, which bytes, which extent. */
export interface ResidentFrameRequest {
  readonly index: number;
  readonly epoch: number;
  readonly content: bigint;
  readonly columns: number;
  readonly rows: number;
}

function u32(value: unknown, name: string): number {
  if (
    typeof value !== "number" ||
    !Number.isInteger(value) ||
    value < 0 ||
    value > 0xffff_ffff
  ) {
    throw new OcelliError("InvalidValue", `${name} is not a u32`);
  }
  return value;
}

function u64(value: unknown, name: string): bigint {
  if (
    typeof value !== "bigint" ||
    value < 0n ||
    value > 0xffff_ffff_ffff_ffffn
  ) {
    throw new OcelliError("InvalidValue", `${name} is not a u64`);
  }
  return value;
}

function residentRequest(request: ResidentFrameRequest): {
  index: number;
  epoch: number;
  content: bigint;
  columns: number;
  rows: number;
} {
  refuseUnknownOptions("ResidentFrameRequest", request, RESIDENT_FRAME_REQUEST_KEYS);
  return {
    index: u32(request.index, "index"),
    epoch: u32(request.epoch, "epoch"),
    content: u64(request.content, "content"),
    columns: u32(request.columns, "columns"),
    rows: u32(request.rows, "rows"),
  };
}

/**
 * Move the session frame cache's budget in bytes. Zero is off. Evictions
 * and the budget change leave through the ring, evictions first.
 */
export function setFrameBudget(wasm: FrameCacheExports, bytes: number): void {
  wasm.set_frame_budget(u32(bytes, "budget"));
}

/**
 * Whether the frame at `request.index` is cached from `request.content` at
 * `request.columns` by `request.rows`. A query is not a use: it moves no
 * recency and removes no mismatch.
 */
export function frameResident(
  wasm: FrameCacheExports,
  request: ResidentFrameRequest,
): boolean {
  const fields = residentRequest(request);
  return (
    wasm.frame_resident(fields.index, fields.epoch, fields.content, fields.columns, fields.rows) ===
    1
  );
}

/**
 * Show the cached frame at `request.index` when it answers for
 * `request.content` at `request.columns` by `request.rows`. A mismatch
 * removes the entry and leaves the displayed frame unchanged.
 */
export function showResident(
  wasm: FrameCacheExports,
  request: ResidentFrameRequest,
): boolean {
  const fields = residentRequest(request);
  return (
    wasm.show_resident(fields.index, fields.epoch, fields.content, fields.columns, fields.rows) ===
    1
  );
}

/** Release every cached frame of `series` and return the frame count. */
export function releaseSeries(wasm: FrameCacheExports, series: bigint): number {
  return wasm.release_series(u64(series, "series"));
}

/**
 * Release a study the caller holds as a list of series, one export call
 * per series, duplicates skipped, counts summed. The cache holds no study
 * identity, so the list is the caller's. Every entry is validated before
 * the first call, so a bad list makes none.
 */
export function releaseStudy(
  wasm: FrameCacheExports,
  series: readonly bigint[],
): number {
  const valid = series.map((id) => u64(id, "series"));
  const seen = new Set<bigint>();
  let frames = 0;
  for (const id of valid) {
    if (seen.has(id)) {
      continue;
    }
    seen.add(id);
    frames += wasm.release_series(id);
  }
  return frames;
}

/** Push one pressure reading into the ring on demand. */
export function requestFramePressure(wasm: FrameCacheExports): void {
  wasm.request_frame_pressure();
}
