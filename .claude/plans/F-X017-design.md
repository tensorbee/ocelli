# F-X017, Close the measured escapes from the wasm linear memory view ban, which needs type-aware linting

**Status**: approved
**Epic ref**: Y1.12
**Sprint**: S04
**Estimate**: 2w

## Normative source, transcribed

From `docs/hld/14-the-boundary-in-code.md`, section 17.2:

```typescript
#[wasm_bindgen]
impl Session {
    /// Reserve `len` bytes and return a pointer into linear memory.
    pub fn alloc(&mut self, len: usize) -> *mut u8 { /* ... */ }
    /// Hand ownership back; `ptr` must be the value returned by `alloc`.
    pub fn commit_frame(&mut self, ptr: *mut u8, len: usize, meta: &FrameMeta) { /* ... */ }
}
// packages/core/src/bulk.ts -- the ONLY correct order
const ptr = session.alloc(bytes.byteLength);
// Build the view AFTER the allocation. Use it immediately. Let it go.
new Uint8Array(wasm.memory.buffer, ptr, bytes.byteLength).set(bytes);
session.commit_frame(ptr, bytes.byteLength, meta);
```

The section's trap, transcribed with source punctuation normalised:

> **NEVER CACHE THE VIEW** A module-level const HEAP = new
> Uint8Array(wasm.memory.buffer) is the classic failure. Any wasm memory growth
> relocates the ArrayBuffer and detaches every outstanding view. The next write
> silently targets a detached buffer or throws far from the cause. Add an
> ESLint rule banning new Uint8Array(wasm.memory.buffer) outside the two
> functions that are allowed to do it.

From `docs/hld/24-agent-code-standards.md`, section 27.2:

| **#** | **Rule** | **Why** |
|----|----|----|
| R2 | Tests derive from the spec or the oracle, never from reading the implementation | An agent asked to test a function will assert what it does, not what it should do |

## What the specification does not cover

The HLD does not define an ESLint selector, an alias analysis, or the exact two
allowed functions. The repository currently allows production files
`packages/core/src/bulk.ts` and `packages/core/src/panic.ts`, plus test files
under a distinct allowance. F-005 records why those sites are safe.

Type-aware linting is limited to TypeScript files included by the workspace
projects. The local rule identifies a `NewExpression` whose first argument is
the `buffer` property of an expression whose TypeScript type resolves to
`WebAssembly.Memory`. It therefore follows parameters, assignments, call
results, getters, class fields and `for-of` bindings without inventing a
general data-flow engine. Existing syntax checks remain for aliases of the
buffer itself, whose static type has already lost its wasm-memory provenance.

## Approach

1. Enable typescript-eslint project-service parsing for `**/*.{ts,tsx}` using
   the existing project references. Keep JavaScript harness files untyped.
2. Define a local ESLint plugin rule in `eslint.config.js`. On every typed-array
   or `DataView` construction, resolve computed and noncomputed `buffer`
   property access through parser services and the TypeScript checker. Report
   when the object type is `WebAssembly.Memory`.
3. Replace the three syntax-only view selectors with the typed rule while
   retaining the buffer-alias syntax refusal. Keep the two production-file and
   test-file allowances exact. Update the self-check so a missing rule,
   weakened severity, surplus allowance, or second enforcement block refuses.
4. Add guard probes for every measured escape, with the computed
   `wasm.memory["buffer"]` route as the acceptance-defining case. Add accept
   controls for `decodeRecord`'s caller-owned `Uint8Array.buffer`, ordinary
   `ArrayBuffer`, and both permitted production functions.
5. Observe the computed route green under the old rule and red under the new
   rule. Run lint and typecheck together so type-information setup cannot pass
   by silently dropping files.

Anticipated implementation write set, exactly:

- `eslint.config.js`
- `scripts/guards/catalogue.py`
- `ci/guard-probe-budget.json`
- `docs/lld/errors.md`
- `docs/lld/typescript-packaging.md`

No package or lockfile change is needed. The installed `typescript-eslint` and
`typescript` packages provide the parser service and checker.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | Parameters, computed properties, assignments, call results, getters, renamed fields and `for-of` bindings cannot hide a `WebAssembly.Memory.buffer` view | `scripts/guards/catalogue.py` lint probes |
| unit | Caller-owned `Uint8Array.buffer` and ordinary buffers remain accepted | guard accept controls and `packages/core/src/errors.ts` under `npm run lint` |
| conformance | Every TypeScript workspace file receives type information and the existing type build stays green | `npm run lint` and `npm run typecheck` |
| mutation | The computed-property escape passes before the rule and fails at the new rule afterwards | guard probe observed on both revisions |

No pixel or geometry arithmetic is added, so no DICOM fixture applies.

## Parity surface covered

None. Appendix B has no row whose `Covered by` value is Y1.12.

## Deviations

None. The plan strengthens the exact section 17.2 rule.

## LLD impact

- `docs/lld/errors.md`
- `docs/lld/typescript-packaging.md`

## Open questions

None.
