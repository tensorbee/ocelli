#!/usr/bin/env node
// Execute the production codec proof under Node. Successful instantiation is
// insufficient because it does not exercise a decoder instruction.

import { readFile } from "node:fs/promises";

const modules = process.argv.slice(2);
if (modules.length !== 2) {
  throw new Error("usage: run_codec_wasm.mjs <plain.wasm> <simd.wasm>");
}
for (const path of modules) {
  const bytes = await readFile(path);
  const { instance } = await WebAssembly.instantiate(bytes, {});
  if (typeof instance.exports.main !== "function") {
    throw new Error(`${path} does not export main`);
  }
  instance.exports.main();
  console.log(`  executed ${path}`);
}
