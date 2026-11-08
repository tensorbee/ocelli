# @ocelli/wasm

The WebAssembly module of the Ocelli imaging core, built by `wasm-pack` from
`crates/ocelli-wasm` and published beside `@ocelli/core`, which declares it as
a peer dependency at the same version.

## Install and load

Install both packages at one version and load the module in a worker:

```js
import init, * as wasm from "@ocelli/wasm";
import { loadCore } from "@ocelli/core";

await loadCore(async () => {
  await init();     // in a browser, fetches the .wasm beside the glue
  wasm.install_panic_hook();
});
```

The module's version is `ocelli_version()` from the same import. It matches the
`VERSION` string `@ocelli/core` exports.

## The decode module

`@ocelli/wasm/decode` is the second module, built with no renderer and no wgpu.
It carries the panic surface exports and `ocelli_version`, and none of the
render surface: `apply_commands` and the rest of the session exports are
compiled only with the render-worker feature.

## What the tarball carries

Both modules and their glue, `THIRD_PARTY_LICENSES` and `THIRD_PARTY_NOTICES`,
and `dist/build-id.json`, which records the package version, whether the tree
was clean when the modules were built, the git SHA of that HEAD, and the
SHA-256 digest of each module. `gate packages` refuses a tarball whose build
id is absent, stale against the current HEAD, recorded from a dirty tree, or
disagrees with its own module bytes.
