import { describe, expect, it } from "vitest";

import {
  holdSessionBytes,
  postWadoInstance,
  sessionBulkSink,
  writeFrame,
  type SessionBulkExports,
} from "./index.js";

const STUDY = "2.25.11";
const SERIES = "2.25.12";
const INSTANCE = "2.25.13";
const BOUNDARY = "ocelli";
const PART = new Uint8Array([0x10, 0x20, 0x30, 0x40]);

function envelope(): Uint8Array {
  const head = new TextEncoder().encode(
    `--${BOUNDARY}\r\nContent-Type: application/dicom\r\n\r\n`,
  );
  const tail = new TextEncoder().encode(`\r\n--${BOUNDARY}--\r\n`);
  const out = new Uint8Array(head.byteLength + PART.byteLength + tail.byteLength);
  out.set(head, 0);
  out.set(PART, head.byteLength);
  out.set(tail, head.byteLength + PART.byteLength);
  return out;
}

function instanceResponse(): Response {
  const bytes = envelope();
  const copy = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(copy).set(bytes);
  return new Response(copy, {
    status: 200,
    headers: {
      "Content-Type": `multipart/related; type="application/dicom"; boundary=${BOUNDARY}`,
    },
  });
}

describe("one WADO-RS instance", () => {
  it("posts the response body and does not read a DICOM tag", async () => {
    const urls: string[] = [];
    const posted: Uint8Array[] = [];
    const boundaries: string[] = [];
    const fetcher: typeof fetch = async (input) => {
      urls.push(String(input));
      return instanceResponse();
    };

    await postWadoInstance({
      studiesServiceUrl: "https://example.test/dicomweb/",
      wadoUriUrl: "https://example.test/wado",
      fetch: fetcher,
      studyUid: STUDY,
      seriesUid: SERIES,
      instanceUid: INSTANCE,
      post(bytes, boundary) {
        posted.push(bytes);
        boundaries.push(boundary);
      },
    });

    expect(urls).toEqual([
      `https://example.test/dicomweb/studies/${STUDY}/series/${SERIES}/instances/${INSTANCE}`,
    ]);
    expect(posted).toHaveLength(1);
    const body = posted[0];
    expect(body).toBeInstanceOf(Uint8Array);
    expect(Array.from(body ?? [])).toEqual(Array.from(envelope()));
    expect(boundaries).toEqual([BOUNDARY]);
  });

  it("sends the requested cache mode and defaults to no-cache", async () => {
    const inits: Array<RequestInit | undefined> = [];
    const fetcher: typeof fetch = async (input, init) => {
      inits.push(init);
      return instanceResponse();
    };
    const request = {
      studiesServiceUrl: "https://example.test/dicomweb/",
      wadoUriUrl: "https://example.test/wado",
      fetch: fetcher,
      studyUid: STUDY,
      seriesUid: SERIES,
      instanceUid: INSTANCE,
      post: () => {},
    };

    await postWadoInstance({ ...request, cache: "no-store" });
    expect(inits[0]?.cache).toBe("no-store");

    // Without the field the client's own default applies, which is the
    // revalidating "no-cache" and not the Fetch "default".
    await postWadoInstance(request);
    expect(inits[1]?.cache).toBe("no-cache");
  });
});

describe("session bulk slot", () => {
  it("copies the bytes into the slot and does not commit them as a frame", () => {
    const memory = new WebAssembly.Memory({ initial: 1, maximum: 4 });
    let commits = 0;
    let released = 0;
    const wasm: SessionBulkExports = {
      memory,
      alloc(len: number): number {
        const ptr = 32;
        if (len !== PART.byteLength) {
          return 0;
        }
        return ptr;
      },
      bulk_ticket(): number {
        return 9;
      },
      release_bulk(ticket: number): number {
        released = ticket;
        return 1;
      },
      commit_frame(): number {
        commits += 1;
        return 1;
      },
    };

    const ticket = holdSessionBytes(wasm, PART, 4);
    expect(ticket).toBe(9);
    expect(released).toBe(4);
    expect(commits).toBe(0);
    expect(Array.from(new Uint8Array(memory.buffer, 32, PART.byteLength))).toEqual([
      0x10, 0x20, 0x30, 0x40,
    ]);
  });

  it("refuses a later frame while the instance ticket is outstanding", () => {
    const memory = new WebAssembly.Memory({ initial: 1, maximum: 4 });
    let held = 0;
    const wasm: SessionBulkExports = {
      memory,
      alloc(len: number): number {
        if (held !== 0 || len !== PART.byteLength) {
          return 0;
        }
        held = 9;
        return 32;
      },
      bulk_ticket(): number {
        return held;
      },
      release_bulk(ticket: number): number {
        if (ticket !== held) {
          return 0;
        }
        held = 0;
        return 1;
      },
      commit_frame(): number {
        return 1;
      },
    };

    const ticket = holdSessionBytes(wasm, PART, 0);
    expect(ticket).toBe(9);
    expect(() =>
      writeFrame({ memory }, sessionBulkSink(wasm), PART, {
        viewport: 1,
        columns: 1,
        rows: 1,
        generation: 1,
      }),
    ).toThrow("allocation refused");
  });
});
