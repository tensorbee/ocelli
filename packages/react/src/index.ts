/**
 * `@ocelli/react`, React bindings for `@ocelli/core`.
 *
 * The binding's one hard constraint comes from HLD section 12: the library
 * enables on a plain DOM element and dispatches events on it, exactly as
 * cornerstone does. Matching that seam is what lets an application adopt
 * Ocelli one viewport at a time rather than all at once, so the React
 * component owns a `<div>` and nothing about the seam depends on React.
 *
 * The second constraint comes from HLD section 23: the shell must always hold
 * enough state to rebuild a viewport from nothing, because a Rust panic
 * poisons the wasm instance and the recovery path is to tear the worker down
 * and reconstruct from the shell's own copy. That is a design constraint on
 * this package, not an afterthought.
 *
 * Scaffold. F-097 (E16.3) implements it.
 */

export { OcelliViewport } from "./OcelliViewport.js";
export type { OcelliViewportProps } from "./OcelliViewport.js";
