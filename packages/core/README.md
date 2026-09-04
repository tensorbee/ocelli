# `@ocelli/core`

The TypeScript shell of [Ocelli](https://github.com/tensorbee/ocelli), a Rust
and WebAssembly medical imaging core for the browser.

The shell owns everything the DOM touches. The core owns everything a pixel
touches. Three channels cross the boundary between them and none of them is an
image.

**This package is a scaffold.** Its public API is designed in F-095 and the
boundary it wraps is built in F-096. `coreAvailable()` returns `false` until a
core is built, which is the honest answer rather than a failure to start.

## Licence

MIT or Apache-2.0, at your option.
