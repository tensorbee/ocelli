import init, * as api from "/wasm/ocelli_wasm.js";
import { writeFrame, writeCommands, writeStack, sessionBulkSink } from "/core/bulk.js";
import { sessionCommandSink, packCommands } from "/core/commands.js";

const exports = await init();
const memory = exports.memory;
const bulk = sessionBulkSink({ ...api, memory });
const commands = sessionCommandSink(api);
let canvas;
let batches = 0;
let cineKey = 0;
let stackEpoch = 0;
const cineFrames = [new Uint8Array(new Float32Array(128 * 128).fill(0).buffer), new Uint8Array(new Float32Array(128 * 128).fill(100).buffer)];

function cineFrame(key) {
  writeFrame({ memory }, bulk, cineFrames[key], { viewport: 1, columns: 128, rows: 128, generation: 0,
    ...(stackEpoch ? { stackIndex: key, stackEpoch } : {}),
  });
  api.publishCpuCanvas();
  cineKey = key;
}

function events() {
  return api.drain_events();
}

globalThis.self.onmessage = ({ data }) => {
  try {
    if (data.op === "bind") {
      canvas = data.canvas;
      api.bindCpuCanvas(canvas, 128, 128, 1);
      const samples = Float32Array.from({ length: 128 * 128 }, (_, i) => 40 + (i % 128));
      writeFrame({ memory }, bulk, new Uint8Array(samples.buffer), {
        viewport: 1, columns: 128, rows: 128, generation: 1,
      });
      globalThis.self.postMessage({ id: data.id, ready: true });
      return;
    }
    if (data.op === "batch") {
      const bytes = data.bytes;
      const applied = writeCommands({ memory }, commands, bytes);
      if (!applied) throw new Error("typed command batch was refused");
      batches += 1;
      if (data.scroll) {
        const state = api.copy_state();
        const index = new DataView(state.buffer, state.byteOffset, state.byteLength).getUint32(72, true);
        if (index !== 1 - cineKey) throw new Error("scroll did not select the other resident frame");
        cineFrame(index);
        globalThis.self.postMessage({ id: data.id, ring: events(), applied, batches, key: index });
        return;
      }
      if (data.interacting) api.publishCpuInteraction();
      else api.publishCpuCanvas();
      globalThis.self.postMessage({ id: data.id, ring: events(), applied, batches });
      return;
    }
    if (data.op === "settle") {
      api.publishCpuCanvas();
      globalThis.self.postMessage({ id: data.id, ring: events(), batches });
      return;
    }
    if (data.op === "first") {
      const bytes = packCommands(1, [{ kind: "Fit", width: 128, height: 128, columns: 128, rows: 128 }]);
      const applied = writeCommands({ memory }, commands, bytes);
      if (!applied) throw new Error("first-frame Fit was refused");
      api.publishCpuCanvas();
      globalThis.self.postMessage({ id: data.id, ring: events(), applied, batches: ++batches });
      return;
    }
    if (data.op === "verifyCine") {
      // Outside the measurement window. A constant candidate must fail this
      // positive control. Pixels remain in the owning worker.
      cineFrame(0);
      const black = canvas.getContext("2d").getImageData(64, 64, 1, 1).data[0];
      cineFrame(1);
      const white = canvas.getContext("2d").getImageData(64, 64, 1, 1).data[0];
      events();
      if (black !== 0 || white !== 255) throw new Error("cine frames did not change the actual canvas");
      globalThis.self.postMessage({ id: data.id, verified: true });
      return;
    }
    if (data.op === "stack") {
      stackEpoch = writeStack({ ...api, memory }, [
        { series: 1n, instance: 1, frame: 0 }, { series: 1n, instance: 1, frame: 1 },
      ]);
      cineFrame(0);
      globalThis.self.postMessage({ id: data.id, ring: events(), key: 0 });
      return;
    }
    if (data.op === "cine") {
      cineFrame(1 - cineKey);
      globalThis.self.postMessage({ id: data.id, ring: events(), key: cineKey });
      return;
    }
    throw new Error("unknown session instrument operation");
  } catch (error) {
    globalThis.self.postMessage({ id: data.id, error: String(error) });
  }
};
globalThis.self.postMessage({ loaded: true });
