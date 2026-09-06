import assert from "node:assert/strict";
import { unlink, writeFile } from "node:fs/promises";
import { dirname, join, relative, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { ESLint } from "eslint";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const PROBE_DIRECTORY = join(ROOT, "packages/core/src");
const RULE = "ocelli/no-wasm-memory-view";

async function lintProbe(name, source) {
  const path = join(PROBE_DIRECTORY, `__eslint_wasm_probe_${name}.ts`);
  await writeFile(path, source, "utf8");
  try {
    const eslint = new ESLint({ cwd: ROOT });
    const [result] = await eslint.lintFiles([relative(ROOT, path)]);
    assert.equal(result.fatalErrorCount, 0, JSON.stringify(result.messages));
    return result.messages.filter((message) => message.ruleId === RULE);
  } finally {
    await unlink(path);
  }
}

test("every standard view constructor is refused over wasm memory", async () => {
  const constructors = [
    "BigInt64Array",
    "BigUint64Array",
    "DataView",
    "Float32Array",
    "Float64Array",
    "Int8Array",
    "Int16Array",
    "Int32Array",
    "Uint8Array",
    "Uint8ClampedArray",
    "Uint16Array",
    "Uint32Array",
  ];
  const constructions = constructors
    .map((constructor) => `new ${constructor}(memory.buffer);`)
    .join("\n");
  const messages = await lintProbe(
    "constructors",
    `function views(memory: WebAssembly.Memory) {\n${constructions}\n}\n`,
  );
  assert.equal(messages.length, constructors.length);
});

test("type information closes every measured alias route", async () => {
  const messages = await lintProbe(
    "aliases",
    `
declare const wasm: { memory: WebAssembly.Memory };
function fromParameter(memory: WebAssembly.Memory) {
  return new Uint8Array(memory.buffer);
}
let assigned;
assigned = wasm.memory;
new DataView(assigned.buffer);
function fetchMemory(): WebAssembly.Memory { return wasm.memory; }
new DataView(fetchMemory().buffer);
class Holder {
  renamed = wasm.memory;
  get current(): WebAssembly.Memory { return wasm.memory; }
  views() {
    new DataView(this.renamed.buffer);
    new DataView(this.current.buffer);
  }
}
for (const item of [wasm.memory]) new DataView(item.buffer);
new DataView(wasm.memory["buffer"]);
const RenamedView = Uint8Array;
new RenamedView(wasm.memory.buffer);
`,
  );
  assert.equal(messages.length, 8);
});

test("ordinary buffers remain accepted", async () => {
  const messages = await lintProbe(
    "ordinary",
    `
declare const payload: Uint8Array;
new DataView(payload.buffer, payload.byteOffset, payload.byteLength);
const buffer = new ArrayBuffer(16);
new Uint8Array(buffer);
`,
  );
  assert.deepEqual(messages, []);
});

test("the two production allowance files remain exact and green", async () => {
  const eslint = new ESLint({ cwd: ROOT });
  const results = await eslint.lintFiles([
    "packages/core/src/bulk.ts",
    "packages/core/src/panic.ts",
  ]);
  const messages = results.flatMap((result) =>
    result.messages.filter((message) => message.ruleId === RULE),
  );
  assert.deepEqual(messages, []);
});
