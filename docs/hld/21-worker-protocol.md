<!-- Originally converted by scripts/split_hld.py from Ocelli-HLD.docx.
     This tracked Markdown file is authoritative.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the tracked Markdown wins. -->

# Worker protocol

**Source**: bootstrap import from `Ocelli-HLD.docx`, section 24. This tracked Markdown is authoritative.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

## 24. Worker protocol

```text
main -> decode : { kind:'decode', seriesId, frameIndex, buffer } [transfer]
decode -> render : { kind:'frame', frameId, buffer, meta } [transfer]
main -> render : { kind:'commands', buffer } [transfer]
render -> main : drained event ring [copy]
// Always transfer, never copy:
// worker.postMessage(msg, [msg.buffer]);
```

Three roles: the main thread, N decode workers each with its own WebAssembly instance, and one render worker owning the GPUDevice and every OffscreenCanvas. Decode workers never touch the GPU; the render worker never decodes.
