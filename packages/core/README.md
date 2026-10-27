# `@ocelli/core`

The TypeScript shell of [Ocelli](https://github.com/tensorbee/ocelli), a Rust
and WebAssembly medical imaging core for the browser.

The shell owns everything the DOM touches. The core owns everything a pixel
touches. Three channels cross the boundary between them and none of them is an
image.

**The package publishes the shell.** F-100 designed the viewport surface,
F-101 built the boundary, F-104 generates its constants, F-050 serialises
the camera and the presentation, and F-103 names the events.
`coreAvailable()` returns `false` until a built module is loaded, which is
the honest answer rather than a failure to start.

## Licence

MIT or Apache-2.0, at your option.
