# `@ocelli/react`

React bindings for [`@ocelli/core`](https://www.npmjs.com/package/@ocelli/core).

Ocelli is a Rust and WebAssembly medical imaging core for the browser. This
package is the React framework binding, which HLD section 10 keeps in
TypeScript deliberately rather than as a compromise.

F-102 binds one viewport to a session prop. A frame with queued commands
submits one buffer, and every frame drains the ring once. The component
renders a host `div` and does not render a canvas.

## Licence

MIT or Apache-2.0, at your option.
