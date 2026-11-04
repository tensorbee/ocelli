import { postWadoInstance } from "@ocelli/core";
import { useEffect, useRef, useState } from "react";

import {
  openCoreWorker,
  type CoreStatus,
  type CoreWorkerHandle,
} from "./core-worker-client.js";

export function CorePanel(props: {
  onAvailable: (ready: boolean) => void;
}) {
  const hostRef = useRef<HTMLDivElement>(null);
  const handleRef = useRef<CoreWorkerHandle | null>(null);
  const [status, setStatus] = useState<CoreStatus | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const [baseUrl, setBaseUrl] = useState("");
  const [studyUid, setStudyUid] = useState("");
  const [seriesUid, setSeriesUid] = useState("");
  const [instanceUid, setInstanceUid] = useState("");
  const [pending, setPending] = useState(false);
  const onAvailable = props.onAvailable;

  useEffect(() => {
    const host = hostRef.current;
    if (host === null) {
      return undefined;
    }
    const canvas = document.createElement("canvas");
    canvas.width = 1;
    canvas.height = 1;
    canvas.setAttribute("aria-label", "Worker canvas");
    host.replaceChildren(canvas);
    const handle = openCoreWorker(canvas);
    handleRef.current = handle;
    const stop = handle?.subscribe((next) => {
      setStatus(next);
      onAvailable(next.available);
    });
    return () => {
      stop?.();
      handle?.close();
      handleRef.current = null;
      host.replaceChildren();
      setStatus(null);
      onAvailable(false);
    };
  }, [onAvailable]);

  async function fetchInstance(): Promise<void> {
    if (baseUrl.trim() === "") {
      setNote("This server has no synthetic Part 10 route.");
      return;
    }
    const handle = handleRef.current;
    if (handle === null) {
      setNote("The core is not loaded.");
      return;
    }
    setPending(true);
    setNote(null);
    try {
      let accepted: Promise<number> | null = null;
      await postWadoInstance({
        studiesServiceUrl: baseUrl.trim(),
        wadoUriUrl: baseUrl.trim(),
        fetch: globalThis.fetch.bind(globalThis),
        studyUid: studyUid.trim(),
        seriesUid: seriesUid.trim(),
        instanceUid: instanceUid.trim(),
        post(bytes, boundary) {
          accepted = handle.postInstance(bytes, boundary);
        },
      });
      const count = accepted === null ? 0 : await accepted;
      setNote(
        count > 0
          ? "The instance bytes were handed to the worker."
          : "The instance request was refused.",
      );
    } catch {
      setNote("The instance request was refused.");
    } finally {
      setPending(false);
    }
  }

  return (
    <section>
      <h2>Wasm core</h2>
      <p>
        One worker loads the built module, binds this canvas and publishes
        one monochrome frame. The form fetches one WADO-RS instance and posts
        the body to that worker. An empty address does not fetch a study.
      </p>
      <div ref={hostRef} />
      {status !== null && (
        <div>
          <p>Tier {status.tier}</p>
          <p>{status.published ? "Canvas published" : "Canvas not published"}</p>
        </div>
      )}
      <p>
        <label>
          DICOMweb base URL{" "}
          <input
            value={baseUrl}
            onChange={(event) => {
              setBaseUrl(event.target.value);
            }}
          />
        </label>
      </p>
      <p>
        <label>
          Study UID{" "}
          <input
            value={studyUid}
            onChange={(event) => {
              setStudyUid(event.target.value);
            }}
          />
        </label>
      </p>
      <p>
        <label>
          Series UID{" "}
          <input
            value={seriesUid}
            onChange={(event) => {
              setSeriesUid(event.target.value);
            }}
          />
        </label>
      </p>
      <p>
        <label>
          Instance UID{" "}
          <input
            value={instanceUid}
            onChange={(event) => {
              setInstanceUid(event.target.value);
            }}
          />
        </label>
      </p>
      <p>
        <button
          type="button"
          disabled={pending}
          onClick={() => {
            void fetchInstance();
          }}
        >
          Fetch instance
        </button>
      </p>
      {note !== null && <p>{note}</p>}
    </section>
  );
}
