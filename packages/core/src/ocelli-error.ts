/**
 * Shell errors.
 *
 * A caller matches `code`. The message is for a person. These string codes
 * are a different namespace from the numeric ring codes in `errors.ts`.
 */

export type DicomwebErrorCode =
  | "InvalidRequest"
  | "Network"
  | "Aborted"
  | "HttpStatus"
  | "ContentType";

export type OcelliErrorCode =
  | DicomwebErrorCode
  | "UnknownOption"
  | "InvalidOptions"
  | "AllocationRefused"
  | "CommitRefused"
  | "LengthMismatch"
  | "InvalidValue"
  | "ExportUnavailable"
  | "CoreLoadFailed";

/** One coded failure raised by the shell. */
export class OcelliError extends Error {
  readonly code: OcelliErrorCode;
  /** Set only for `UnknownOption`. The value is the refused key. */
  readonly option?: string;

  constructor(
    code: OcelliErrorCode,
    message: string,
    details?: { readonly option?: string; readonly cause?: unknown },
  ) {
    super(
      message,
      details !== undefined && "cause" in details ? { cause: details.cause } : undefined,
    );
    this.name = "OcelliError";
    this.code = code;
    if (details?.option !== undefined) {
      this.option = details.option;
    }
  }
}
