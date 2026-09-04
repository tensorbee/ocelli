<!-- Originally converted by scripts/split_hld.py from Ocelli-HLD.docx.
     This tracked Markdown file is authoritative.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the tracked Markdown wins. -->

# Appendix A, open questions and spike gates

**Source**: bootstrap import from `Ocelli-HLD.docx`, Appendix A. This tracked Markdown is authoritative.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

## Appendix A, Open questions and spike gates

Each of these can end or reshape the programme, and each is cheap to answer. They belong in the first six weeks, with the authority to stop.

| **Gate** | **Question** | **Consequence if it fails** |
|----|----|----|
| A1 | Does HTJ2K decode correctly in openjp2 under wasm32, bit-exact against OpenJPH? | You maintain C codec builds regardless, and one of the four arguments for the rewrite weakens |
| A2 | What is the JPEG-LS answer — CharLS bridge, self-compiled CharLS, or a young pure-Rust crate? | Changes the architecture, not just a dependency line. Decide before anything else |
| A3 | Does a wgpu volume ray-cast run on WebGPU and degrade acceptably on WebGL2 at your real volume sizes? | Sizes the least-compressible phase in the plan |
| A4 | Do binary size and cold start land within budget? | Estimated 3–8 MB uncompressed before tuning, Naga dominating. Unmeasured today |
| A5 | Are WSI and ECG viewports in scope? | Marked DEFER in the parity checklist. A product decision that moves the Phase 1 total |
| A6 | Is the source-provenance policy agreed in writing? | Must precede any agent touching any repository |
