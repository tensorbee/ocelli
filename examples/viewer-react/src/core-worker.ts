/**
 * One wasm instance for the example page.
 *
 * The shell posts a WADO-RS body here. This worker copies it into the
 * session bulk slot and does not post pixels back. The canvas commit is
 * `publishCpuCanvas` of one stored monochrome sample.
 */

import {
  OcelliError,
  coreAvailable,
  holdSessionBytes,
  loadCore,
  sessionBulkSink,
  writeFrame,
  type SessionBulkExports,
} from "@ocelli/core";

declare const __OCELLI_WASM_URL__: string;

interface WasmGlue extends SessionBulkExports {
  bindCpuCanvas(
    canvas: OffscreenCanvas,
    cssWidth: number,
    cssHeight: number,
    dpr: number,
  ): void;
  publishCpuCanvas(): void;
  resolvedTier(): Promise<string>;
  install_panic_hook(): void;
}

interface StartMessage {
  readonly type: "start";
  readonly canvas: OffscreenCanvas;
}

interface InstanceMessage {
  readonly type: "instance";
  readonly bytes: ArrayBuffer;
  readonly boundary: string;
}

type WorkerIn = StartMessage | InstanceMessage;

let glue: WasmGlue | null = null;
let ticket = 0;
let instanceBoundary = "";
let published = false;

function postStatus(tier: string): void {
  self.postMessage({
    type: "status",
    available: coreAvailable(),
    tier,
    published,
  });
}

self.onmessage = (event: MessageEvent<WorkerIn>) => {
  const data = event.data;
  if (data.type === "start") {
    void start(data.canvas);
    return;
  }
  void accept(data.bytes, data.boundary);
};

async function start(canvas: OffscreenCanvas): Promise<void> {
  try {
    await loadCore(async () => {
      glue = await openPackage();
    });
  } catch (error) {
    console.error(error instanceof OcelliError ? error.code : error);
    published = false;
    postStatus("");
    return;
  }
  if (glue === null) {
    published = false;
    postStatus("");
    return;
  }
  const api = glue;
  try {
    api.bindCpuCanvas(canvas, 1, 1, 1);
    const sample = new Uint8Array(4);
    new DataView(sample.buffer).setFloat32(0, 40, true);
    writeFrame({ memory: api.memory }, sessionBulkSink(api), sample, {
      viewport: 1,
      columns: 1,
      rows: 1,
      generation: 1,
    });
    api.publishCpuCanvas();
    published = true;
  } catch {
    // A failed publish leaves the last successful canvas. The tier is still
    // the one `resolve` kept.
  }
  let tier = "";
  try {
    tier = await api.resolvedTier();
  } catch {
    tier = "";
  }
  postStatus(tier);
}

async function accept(buffer: ArrayBuffer, boundary: string): Promise<void> {
  if (glue === null) {
    self.postMessage({ type: "accepted", bytes: 0, boundary: "" });
    return;
  }
  const bytes = new Uint8Array(buffer);
  ticket = holdSessionBytes(glue, bytes, ticket);
  instanceBoundary = ticket === 0 ? "" : boundary;
  self.postMessage({
    type: "accepted",
    bytes: ticket === 0 ? 0 : bytes.byteLength,
    boundary: instanceBoundary,
  });
}

async function openPackage(): Promise<WasmGlue> {
  if (__OCELLI_WASM_URL__ === "") {
    throw new Error("wasm package is absent");
  }
  const probed = await fetch(__OCELLI_WASM_URL__);
  if (!probed.ok) {
    throw new Error("wasm package is absent");
  }
  const imported = (await import(
    /* @vite-ignore */
    __OCELLI_WASM_URL__
  )) as {
    default: (input?: unknown) => Promise<{ memory: WebAssembly.Memory }>;
    bindCpuCanvas: WasmGlue["bindCpuCanvas"];
    publishCpuCanvas: WasmGlue["publishCpuCanvas"];
    resolvedTier: WasmGlue["resolvedTier"];
    install_panic_hook: WasmGlue["install_panic_hook"];
    alloc: WasmGlue["alloc"];
    bulk_ticket: WasmGlue["bulk_ticket"];
    release_bulk: WasmGlue["release_bulk"];
    commit_frame: WasmGlue["commit_frame"];
  };
  const started = await imported.default();
  imported.install_panic_hook();
  return {
    memory: started.memory,
    alloc: imported.alloc,
    bulk_ticket: imported.bulk_ticket,
    release_bulk: imported.release_bulk,
    commit_frame: imported.commit_frame,
    bindCpuCanvas: imported.bindCpuCanvas,
    publishCpuCanvas: imported.publishCpuCanvas,
    resolvedTier: imported.resolvedTier,
    install_panic_hook: imported.install_panic_hook,
  };
}
