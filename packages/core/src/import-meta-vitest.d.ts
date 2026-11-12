// Types for vitest's in-source testing marker. The dicomweb-viewer module
// carries its own release tests behind an import.meta.vitest guard, which
// vitest injects only when it runs that file as the test entry. The shape
// matches vitest's own importMeta.d.ts. This file is an input declaration
// only: tsc never emits it, and the published package ships only dist.
interface ImportMeta {
  readonly vitest?: typeof import("vitest");
}
