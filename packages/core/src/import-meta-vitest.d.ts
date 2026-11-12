// Types for vitest's in-source testing marker. The dicomweb-viewer module
// carries its own release tests behind an import.meta.vitest guard, which
// vitest injects only when it runs that file as the test entry. The guard
// must be spelled as the literal that vitest's collector and loader match,
// so the marker keeps a declared type here rather than being read through
// a cast. The shape below is the minimal surface the guarded blocks use,
// written structurally so that no build of this package loads vitest's own
// types. This file is an input declaration only: tsc never emits it, and
// the published package ships only dist.
interface ImportMeta {
  readonly vitest?: {
    it(name: string, fn: () => void | Promise<void>): void;
    afterEach(fn: () => void | Promise<void>): void;
    expect(actual: unknown): { toBe(expected: unknown): void; toBeNull(): void };
    vi: {
      stubGlobal(name: string, value: unknown): void;
      unstubAllGlobals(): void;
    };
  };
}
