// @vitest-environment jsdom
/**
 * The example page shows the worker's tier and whether the core loaded.
 * The fetch numbers are not a patient study. A panel that parsed a tag
 * would not be able to forward these four bytes unchanged.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { App } from "./App.js";

const posted = vi.hoisted(() => ({
  bytes: null as Uint8Array | null,
  boundary: "",
  accept: null as number | null,
}));
const status = vi.hoisted(() => ({
  available: false,
  tier: "",
  published: false,
}));

vi.mock("./core-worker-client.js", () => ({
  openCoreWorker: () => ({
    subscribe(listener: (value: { available: boolean; tier: string; published: boolean }) => void) {
      listener({
        available: status.available,
        tier: status.tier,
        published: status.published,
      });
      return () => {};
    },
    postInstance(bytes: Uint8Array, boundary: string) {
      posted.bytes = bytes;
      posted.boundary = boundary;
      return posted.accept ?? bytes.byteLength;
    },
    close() {},
  }),
}));

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  status.available = false;
  status.tier = "";
  status.published = false;
  posted.bytes = null;
  posted.boundary = "";
  posted.accept = null;
});

function instanceEnvelope(): Uint8Array {
  const head = new TextEncoder().encode("--ocelli\r\nContent-Type: application/dicom\r\n\r\n");
  const part = new Uint8Array([0x10, 0x20, 0x30, 0x40]);
  const tail = new TextEncoder().encode("\r\n--ocelli--\r\n");
  const out = new Uint8Array(head.byteLength + part.byteLength + tail.byteLength);
  out.set(head, 0);
  out.set(part, head.byteLength);
  out.set(tail, head.byteLength + part.byteLength);
  return out;
}

describe("wasm core panel", () => {
  it("stays on core not built and does not fetch when the URL is empty", async () => {
    const fetcher = vi.fn(async () => new Response(null, { status: 500 }));
    vi.stubGlobal("fetch", fetcher);
    render(<App />);
    expect(screen.getByText("core not built")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Fetch instance" }));
    expect(await screen.findByText("This server has no synthetic Part 10 route.")).toBeTruthy();
    expect(fetcher).not.toHaveBeenCalled();
    expect(screen.getByRole("heading", { name: "Stack viewport" })).toBeTruthy();
  });

  it("shows the tier the worker reports and that the canvas published", async () => {
    status.available = true;
    status.tier = "B";
    status.published = true;
    render(<App />);
    expect(await screen.findByText("core ready")).toBeTruthy();
    expect(screen.getByText("Tier B")).toBeTruthy();
    expect(screen.getByText("Canvas published")).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "No core in this clone" })).toBeNull();
  });

  it("posts one WADO-RS body to the worker", async () => {
    status.available = true;
    status.tier = "C";
    status.published = true;
    const body = instanceEnvelope();
    const fetcher = vi.fn<typeof fetch>(async () => {
      const copy = new ArrayBuffer(body.byteLength);
      new Uint8Array(copy).set(body);
      return new Response(copy, {
        status: 200,
        headers: {
          "Content-Type": 'multipart/related; type="application/dicom"; boundary=ocelli',
        },
      });
    });
    vi.stubGlobal("fetch", fetcher);
    render(<App />);
    fireEvent.change(screen.getByLabelText("DICOMweb base URL"), {
      target: { value: "https://example.test/dicomweb/" },
    });
    fireEvent.change(screen.getByLabelText("Study UID"), {
      target: { value: "2.25.11" },
    });
    fireEvent.change(screen.getByLabelText("Series UID"), {
      target: { value: "2.25.12" },
    });
    fireEvent.change(screen.getByLabelText("Instance UID"), {
      target: { value: "2.25.13" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Fetch instance" }));
    expect(await screen.findByText("The instance bytes were handed to the worker.")).toBeTruthy();
    expect(fetcher).toHaveBeenCalledTimes(1);
    const input = fetcher.mock.calls[0]?.[0];
    expect(String(input)).toBe(
      "https://example.test/dicomweb/studies/2.25.11/series/2.25.12/instances/2.25.13",
    );
    expect(Array.from(posted.bytes ?? [])).toEqual(Array.from(instanceEnvelope()));
    expect(posted.boundary).toBe("ocelli");
  });

  it("refuses the note when the worker keeps nothing", async () => {
    status.available = true;
    status.tier = "C";
    status.published = true;
    posted.accept = 0;
    const body = instanceEnvelope();
    vi.stubGlobal(
      "fetch",
      vi.fn<typeof fetch>(async () => {
        const copy = new ArrayBuffer(body.byteLength);
        new Uint8Array(copy).set(body);
        return new Response(copy, {
          status: 200,
          headers: {
            "Content-Type": 'multipart/related; type="application/dicom"; boundary=ocelli',
          },
        });
      }),
    );
    render(<App />);
    fireEvent.change(screen.getByLabelText("DICOMweb base URL"), {
      target: { value: "https://example.test/dicomweb/" },
    });
    fireEvent.change(screen.getByLabelText("Study UID"), {
      target: { value: "2.25.11" },
    });
    fireEvent.change(screen.getByLabelText("Series UID"), {
      target: { value: "2.25.12" },
    });
    fireEvent.change(screen.getByLabelText("Instance UID"), {
      target: { value: "2.25.13" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Fetch instance" }));
    expect(await screen.findByText("The instance request was refused.")).toBeTruthy();
  });
});
