/**
 * Hand one WADO-RS instance body to the caller.
 *
 * `DicomwebClient` still owns the fetch, the content type and the UID check.
 * This file does not read a DICOM tag. The bytes are the response body.
 */

import {
  exportDecodedFrame,
  type DicomwebResponseKind,
  type WasmMemory,
} from "./bulk.js";
import { DicomwebClient } from "./dicomweb.js";

export interface WadoInstanceRequest {
  readonly studiesServiceUrl: string;
  readonly wadoUriUrl: string;
  readonly fetch: typeof fetch;
  readonly studyUid: string;
  readonly seriesUid: string;
  readonly instanceUid: string;
  readonly post: (bytes: Uint8Array, boundary: string) => void;
}

const PAGE = 65536;

export async function postWadoInstance(request: WadoInstanceRequest): Promise<void> {
  const memory = new WebAssembly.Memory({ initial: 1, maximum: 1024 });
  const wasm: WasmMemory = { memory };
  let held: Uint8Array | null = null;
  let boundary = "";
  const client = new DicomwebClient({
    studiesServiceUrl: request.studiesServiceUrl,
    wadoUriUrl: request.wadoUriUrl,
    fetch: request.fetch,
    wasm,
    sink: {
      alloc(len: number): number {
        if (!Number.isSafeInteger(len) || len < 0) {
          return 0;
        }
        const ptr = memory.buffer.byteLength;
        const need = ptr + len;
        if (need > memory.buffer.byteLength) {
          const pages = Math.ceil((need - memory.buffer.byteLength) / PAGE);
          memory.grow(pages);
        }
        return ptr;
      },
      commit_dicomweb_response(
        ptr: number,
        len: number,
        kind: DicomwebResponseKind,
      ): void {
        if (kind.type !== "wado-rs-instances") {
          return;
        }
        held = new Uint8Array(exportDecodedFrame(wasm, ptr, len));
        boundary = kind.boundary;
      },
    },
  });
  await client.retrieveInstance(
    request.studyUid,
    request.seriesUid,
    request.instanceUid,
  );
  if (held === null) {
    throw new Error("the instance body was not committed");
  }
  request.post(held, boundary);
}
