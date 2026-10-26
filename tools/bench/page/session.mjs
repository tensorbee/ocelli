import { packCommands } from "/core/commands.js";
import { drainRing } from "/core/ring.js";

const canvas = document.getElementById("viewport");
const worker = new globalThis.Worker("./session-worker.mjs", { type: "module" });
const pending = new Map();
let nextId = 1;
let queued = [];
let scheduled = false;
let activeDrag;
let activeScroll;
let selectedKey = 0;
let changedFrames = 0;
let lastGeneration = 0;

function request(message, transfer = []) {
  const id = nextId++;
  return new Promise((resolve, reject) => {
    const timer = globalThis.setTimeout(() => { pending.delete(id); reject(new Error("session instrument timed out")); }, 10000);
    pending.set(id, { resolve, reject, timer });
    worker.postMessage({ id, ...message }, transfer);
  });
}

worker.onmessage = ({ data }) => {
  if (data.loaded) {
    const offscreen = canvas.transferControlToOffscreen();
    request({ op: "bind", canvas: offscreen }, [offscreen]).then(() => { globalThis.__sessionReady = true; }).catch(error => { globalThis.__sessionError = String(error); });
    return;
  }
  const waiter = pending.get(data.id);
  if (!waiter) return;
  pending.delete(data.id);
  globalThis.clearTimeout(waiter.timer);
  if (data.error) waiter.reject(new Error(data.error));
  else waiter.resolve(data);
};
worker.onerror = event => { globalThis.__sessionError = event.message; };

canvas.addEventListener("wheel", event => {
  if (!activeScroll) return;
  event.preventDefault();
  const start = event.timeStamp;
  const previous = selectedKey;
  globalThis.requestAnimationFrame(async () => {
    try {
      const bytes = packCommands(1, [{ kind: "Scroll", step: event.deltaY > 0 ? 1 : -1 }]);
      const result = await drainLater(request({ op: "batch", bytes, scroll: true }));
      if (result.key === previous) throw new Error("scroll frame key did not change");
      selectedKey = result.key;
      activeScroll?.({ value: result.end - start, previous, key: selectedKey, generation: result.event.generation });
    } catch (error) { activeScroll?.({ error: String(error) }); }
    activeScroll = undefined;
  });
}, { passive: false });

function presented(result) {
  const raw = drainRing(result.ring).events.find(event => event.kind === 6);
  if (!raw) throw new Error("no presented-frame kind 6 event");
  const data = new DataView(raw.payload.buffer, raw.payload.byteOffset, raw.payload.byteLength);
  const event = { kind: raw.kind, generation: data.getUint32(0, true), flags: data.getUint32(12, true) };
  if (event.generation !== lastGeneration) changedFrames += 1;
  lastGeneration = event.generation;
  return event;
}

// Drain the worker's copied ring in a later animation frame. No pixel readback
// and no per-pointer command crossing.
async function drainLater(reply) {
  const result = await reply;
  await new Promise(resolve => globalThis.requestAnimationFrame(resolve));
  const event = presented(result);
  return { ...result, event, end: performance.now() };
}

canvas.addEventListener("pointermove", event => {
  if (!activeDrag) return;
  const samples = event.getCoalescedEvents?.() ?? [];
  const received = samples.length ? samples : [event];
  for (const sample of received) {
    queued.push({ kind: "SetWindow", centre: 40 + sample.clientX, width: 128, function: "linear" });
  }
  if (scheduled) return;
  scheduled = true;
  const start = event.timeStamp;
  globalThis.requestAnimationFrame(async () => {
    scheduled = false;
    const batch = queued;
    queued = [];
    try {
      const result = await drainLater(request({ op: "batch", bytes: packCommands(1, batch), interacting: true }));
      const resolve = activeDrag;
      activeDrag = undefined;
      resolve?.({ value: result.end - start, start, end: result.end, commands: result.applied, batches: result.batches, generation: result.event.generation });
    } catch (error) {
      globalThis.__sessionError = String(error);
      activeDrag?.({ error: String(error) });
      activeDrag = undefined;
    }
  });
});

globalThis.__sessionBench = {
  async first() {
    const start = performance.now();
    const result = await drainLater(request({ op: "first" }));
    return { value: result.end - start, kind: 6, generation: result.event.generation };
  },
  armDrag() {
    globalThis.__dragResult = new Promise(resolve => { activeDrag = resolve; });
  },
  async stack() {
    const result = await drainLater(request({ op: "stack" }));
    selectedKey = result.key;
  },
  armScroll() { globalThis.__scrollResult = new Promise(resolve => { activeScroll = resolve; }); },
  async settle() { return drainLater(request({ op: "settle" })); },
  async interactive(durationMs) {
    const start = performance.now();
    let iterations = 0;
    while (performance.now() - start < durationMs) {
      this.armDrag();
      canvas.dispatchEvent(new globalThis.PointerEvent("pointermove", { clientX: iterations % 64, bubbles: true }));
      const result = await globalThis.__dragResult;
      if (result.error) throw new Error(result.error);
      iterations += 1;
    }
    await this.settle();
    return { iterations, changedFrames };
  },
  async idle(durationMs) {
    const before = changedFrames;
    await new Promise(resolve => globalThis.setTimeout(resolve, durationMs));
    return { changedFrames: changedFrames - before, scheduled: scheduled || queued.length > 0 };
  },
  async verifyCine() { return request({ op: "verifyCine" }); },
  async cine(durationMs) {
    const start = performance.now();
    let previousKey = 1;
    let changes = 0;
    while (performance.now() - start < durationMs) {
      const result = await drainLater(request({ op: "cine" }));
      if (result.key === previousKey) throw new Error("cine did not select a changed resident frame");
      previousKey = result.key;
      changes += 1;
    }
    return { changedFrames: changes };
  },
};
