/**
 * Refuse an own key an options object does not declare.
 *
 * Inherited keys are not configuration. A declared key whose value is
 * undefined is still that key. The lists below are complete for the type
 * they name, so a new field fails `npm run typecheck` until the list names it.
 */

import { OcelliError } from "./ocelli-error.js";
import type { FrameCommitMeta } from "./bulk.js";
import type { CursorControllerOptions } from "./cursor.js";
import type { DicomwebClientOptions, QidoSearchOptions } from "./dicomweb.js";
import type { WadoInstanceRequest } from "./instance-bytes.js";
import type { PointerDispatchOptions } from "./pointer.js";
import type { SecondaryCaptureRequest } from "./secondary-capture.js";
import type { KeyObjectRequest } from "./tid2010.js";

const OPTION_TEXT_LIMIT = 64;

export const DICOMWEB_CLIENT_OPTION_KEYS = [
  "studiesServiceUrl",
  "wadoUriUrl",
  "fetch",
  "wasm",
  "sink",
  "cache",
] as const satisfies readonly (keyof DicomwebClientOptions)[];

type MissingDicomwebClient = Exclude<
  keyof DicomwebClientOptions,
  (typeof DICOMWEB_CLIENT_OPTION_KEYS)[number]
>;
const dicomwebClientComplete: [MissingDicomwebClient] extends [never] ? true : never = true;

export const QIDO_SEARCH_OPTION_KEYS = [
  "filters",
  "includeFields",
  "limit",
  "offset",
  "fuzzyMatching",
  "signal",
] as const satisfies readonly (keyof QidoSearchOptions)[];

type MissingQidoSearch = Exclude<
  keyof QidoSearchOptions,
  (typeof QIDO_SEARCH_OPTION_KEYS)[number]
>;
const qidoSearchComplete: [MissingQidoSearch] extends [never] ? true : never = true;

export const POINTER_DISPATCH_OPTION_KEYS = [
  "registry",
  "groupId",
  "viewport",
  "target",
  "basis",
  "now",
] as const satisfies readonly (keyof PointerDispatchOptions)[];

type MissingPointerDispatch = Exclude<
  keyof PointerDispatchOptions,
  (typeof POINTER_DISPATCH_OPTION_KEYS)[number]
>;
const pointerDispatchComplete: [MissingPointerDispatch] extends [never] ? true : never = true;

export const CURSOR_CONTROLLER_OPTION_KEYS = [
  "registry",
  "groupId",
  "style",
  "cursors",
] as const satisfies readonly (keyof CursorControllerOptions)[];

type MissingCursorController = Exclude<
  keyof CursorControllerOptions,
  (typeof CURSOR_CONTROLLER_OPTION_KEYS)[number]
>;
const cursorControllerComplete: [MissingCursorController] extends [never] ? true : never = true;

export const WADO_INSTANCE_REQUEST_KEYS = [
  "studiesServiceUrl",
  "wadoUriUrl",
  "fetch",
  "studyUid",
  "seriesUid",
  "instanceUid",
  "post",
  "cache",
] as const satisfies readonly (keyof WadoInstanceRequest)[];

type MissingWadoInstance = Exclude<
  keyof WadoInstanceRequest,
  (typeof WADO_INSTANCE_REQUEST_KEYS)[number]
>;
const wadoInstanceComplete: [MissingWadoInstance] extends [never] ? true : never = true;

export const KEY_OBJECT_REQUEST_KEYS = [
  "title",
  "study",
  "series",
  "instance",
  "referencedClass",
  "referencedInstance",
] as const satisfies readonly (keyof KeyObjectRequest)[];

type MissingKeyObject = Exclude<
  keyof KeyObjectRequest,
  (typeof KEY_OBJECT_REQUEST_KEYS)[number]
>;
const keyObjectComplete: [MissingKeyObject] extends [never] ? true : never = true;

export const SECONDARY_CAPTURE_REQUEST_KEYS = [
  "study",
  "series",
  "instance",
  "columns",
  "rows",
  "pixels",
] as const satisfies readonly (keyof SecondaryCaptureRequest)[];

type MissingSecondaryCapture = Exclude<
  keyof SecondaryCaptureRequest,
  (typeof SECONDARY_CAPTURE_REQUEST_KEYS)[number]
>;
const secondaryCaptureComplete: [MissingSecondaryCapture] extends [never] ? true : never = true;

export const FRAME_COMMIT_META_KEYS = [
  "viewport",
  "columns",
  "rows",
  "generation",
  "stackIndex",
  "stackEpoch",
] as const satisfies readonly (keyof FrameCommitMeta)[];

type MissingFrameCommit = Exclude<
  keyof FrameCommitMeta,
  (typeof FRAME_COMMIT_META_KEYS)[number]
>;
const frameCommitComplete: [MissingFrameCommit] extends [never] ? true : never = true;

/** True when every key list names every field of its type. */
export const OPTION_LISTS_COMPLETE =
  dicomwebClientComplete &&
  qidoSearchComplete &&
  pointerDispatchComplete &&
  cursorControllerComplete &&
  wadoInstanceComplete &&
  keyObjectComplete &&
  secondaryCaptureComplete &&
  frameCommitComplete;

/** Type name to the keys `refuseUnknownOptions` allows for that type. */
export const OPTION_KEY_LISTS: Readonly<Record<string, readonly string[]>> = {
  DicomwebClientOptions: DICOMWEB_CLIENT_OPTION_KEYS,
  QidoSearchOptions: QIDO_SEARCH_OPTION_KEYS,
  PointerDispatchOptions: POINTER_DISPATCH_OPTION_KEYS,
  CursorControllerOptions: CURSOR_CONTROLLER_OPTION_KEYS,
  WadoInstanceRequest: WADO_INSTANCE_REQUEST_KEYS,
  KeyObjectRequest: KEY_OBJECT_REQUEST_KEYS,
  SecondaryCaptureRequest: SECONDARY_CAPTURE_REQUEST_KEYS,
  FrameCommitMeta: FRAME_COMMIT_META_KEYS,
};

function shownKey(key: string | symbol): string {
  const text = typeof key === "symbol" ? String(key) : key;
  return text.length > OPTION_TEXT_LIMIT ? text.slice(0, OPTION_TEXT_LIMIT) : text;
}

/**
 * Throw when `value` is not an options object or carries an unknown own key.
 *
 * `keys` is the declared field list. A symbol is never a declared field.
 */
export function refuseUnknownOptions(
  owner: string,
  value: unknown,
  keys: readonly string[],
): void {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new OcelliError("InvalidOptions", `${owner} is not an options object`);
  }
  for (const key of Reflect.ownKeys(value)) {
    if (typeof key === "string" && keys.includes(key)) {
      continue;
    }
    const option = shownKey(key);
    throw new OcelliError("UnknownOption", `${owner} has no option ${JSON.stringify(option)}`, {
      option,
    });
  }
}

