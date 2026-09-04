<!-- Originally converted by scripts/split_hld.py from Ocelli-HLD.docx.
     This tracked Markdown file is authoritative.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the tracked Markdown wins. -->

# Performance rules

**Source**: bootstrap import from `Ocelli-HLD.docx`, section 26. This tracked Markdown is authoritative.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

## 26. Performance rules

- No allocation in the render loop.

- One queue.submit() per frame.

- Prefer a uniform update to a texture update. Window/level is thirty-two bytes, not a re-upload.

- Batch pointer events into one command buffer per animation frame. Never cross the boundary per event.

- Measure with the benchmark harness before optimising anything. The intuitions that work in JavaScript do not transfer.
