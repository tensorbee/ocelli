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

This package depends on dcmjs 0.52.2, which declares adm-zip 0.5.17 as a
runtime dependency. Nothing in this package or in dcmjs's published files
loads adm-zip. The repository's `gate packages` checks that on every run.
It records ESM and CommonJS resolutions on the main thread while importing
this package and requiring `dcmjs`. It refuses a trace that misses
`dcmjs/build/dcmjs.es.js` or `dcmjs/build/dcmjs.js`, or that resolves
adm-zip, and it scans every other installed `.js`, `.mjs` or `.cjs` file
for an adm-zip module specifier in any quote, with or without a subpath,
including a relative `../` or `../../` path. A load whose specifier is
built at run time in code the smoke run does not execute on its main
thread (for example a Worker or a child process, or a function the import
never calls) is outside the check. A consumer's own npm audit also lists
`@ocelli/core` and `@ocelli/react` as high, because they depend on dcmjs.
Add `"overrides": { "adm-zip": "0.6.1" }` to the project's own root
`package.json` when a clean audit is required. That override is safe
because nothing calls adm-zip.

## Licence

MIT or Apache-2.0, at your option.
