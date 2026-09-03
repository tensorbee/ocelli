# Ocelli high-level design

The authored document is `Ocelli-HLD.docx`, held **outside this
repository**. These files are cut from it by `scripts/split_hld.py`,
in document order, with nothing reordered or reworded.

**Appendix C is deliberately absent.** It is competitor pricing and
go-to-market analysis, and it is not published with the library. Its
one normative part, the source-provenance table, lives in
`docs/SOURCE-POLICY.md` and the build still enforces it.

To regenerate or verify, point at the source and re-run:

```bash
export OCELLI_SOURCE_DIR=~/Desktop/ocelli/source-documents
python3 scripts/split_hld.py
```

Without it the `docs` gate SKIPS with a stated reason. A check that
cannot run is not a check that passed.

`python3 scripts/split_hld.py --check` asserts every line of the
converted document lands in exactly one file here, so a mapping edit
cannot silently drop a section.

## Document header

**OCELLI**
**Rust / WebAssembly Imaging Core**
High-Level Design and Implementation Guidance
Version 0.1 · Draft for review
Tensorbee · September 2026
*Phase 1 replaces cornerstone3D v5.8.9 in the browser. Part III adds the eight capabilities the field does not currently have. Phases 2 and 3 — server-side DICOM services and workstation features — remain roadmap.*
**Contents**
*If this page appears empty, select all and press F9 to build the contents.*

## Files

| File | Covers |
|------|--------|
| [`01-purpose-and-scope.md`](01-purpose-and-scope.md) | Purpose and scope, Part I, section 1 |
| [`02-standing-decisions.md`](02-standing-decisions.md) | Standing decisions, section 2 |
| [`03-architecture-and-crates.md`](03-architecture-and-crates.md) | System architecture and crate layout, sections 3 to 4 |
| [`04-boundary-and-data-path.md`](04-boundary-and-data-path.md) | The boundary contract and the data path, sections 5 to 6 |
| [`05-rendering.md`](05-rendering.md) | Rendering, section 7 |
| [`06-memory-and-cache.md`](06-memory-and-cache.md) | Memory and cache, section 8 |
| [`07-concurrency-and-typescript.md`](07-concurrency-and-typescript.md) | Concurrency, and what stays TypeScript, sections 9 to 10 |
| [`08-validation-architecture.md`](08-validation-architecture.md) | Validation architecture, section 11 |
| [`09-migration.md`](09-migration.md) | Migration, section 12 |
| [`10-extension-points.md`](10-extension-points.md) | Designed-in extension points, section 13 |
| [`11-decision-log.md`](11-decision-log.md) | Decision log, section 14 |
| [`12-workspace-and-build.md`](12-workspace-and-build.md) | Workspace and build, Part II, section 15 |
| [`13-core-types.md`](13-core-types.md) | Core types, coordinate spaces and value spaces, section 16 |
| [`14-the-boundary-in-code.md`](14-the-boundary-in-code.md) | The boundary, in code, section 17 |
| [`15-lut-chain.md`](15-lut-chain.md) | The LUT chain, section 18 |
| [`16-volume-representation.md`](16-volume-representation.md) | Volume representation, section 19 |
| [`17-cache-and-allocation.md`](17-cache-and-allocation.md) | Cache and allocation discipline, section 20 |
| [`18-codec-registry.md`](18-codec-registry.md) | Codec registry, section 21 |
| [`19-render-graph.md`](19-render-graph.md) | Render graph, section 22 |
| [`20-errors-and-panics.md`](20-errors-and-panics.md) | Error and panic policy, section 23 |
| [`21-worker-protocol.md`](21-worker-protocol.md) | Worker protocol, section 24 |
| [`22-testing-and-tolerance.md`](22-testing-and-tolerance.md) | Testing and the tolerance policy, section 25 |
| [`23-performance-rules.md`](23-performance-rules.md) | Performance rules, section 26 |
| [`24-agent-code-standards.md`](24-agent-code-standards.md) | Standards for agent-generated code, section 27 |
| [`25-first-ten-files.md`](25-first-ten-files.md) | The first ten files, section 28 |
| [`A-spike-gates.md`](A-spike-gates.md) | Appendix A, open questions and spike gates, Appendix A |
| [`B-parity-surface.md`](B-parity-surface.md) | Appendix B, parity surface, Appendix B |
| [`26-differentiating-capabilities.md`](26-differentiating-capabilities.md) | Differentiating capabilities (Part III), Part III, sections 30 to 37 |
| [`27-phase1-hooks.md`](27-phase1-hooks.md) | The hooks Phase 1 must include, section 38 |
| [`C-competitive-position.md`](C-competitive-position.md) | Appendix C, competitive position, Appendix C |

## Deviations from this document

Recorded rather than silently applied. See `docs/hld/DEVIATIONS.md`.

