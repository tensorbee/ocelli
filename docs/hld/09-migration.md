<!-- Originally converted by scripts/split_hld.py from Ocelli-HLD.docx.
     This tracked Markdown file is authoritative.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the tracked Markdown wins. -->

# Migration

**Source**: bootstrap import from `Ocelli-HLD.docx`, section 12. This tracked Markdown is authoritative.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

## 12. Migration

A new API plus an existing product means the migration is engineering work, not documentation. The mitigation is to keep the **integration seam** identical even though the API is not: the new library enables on a plain DOM element and dispatches events on it, exactly as cornerstone does. That single constraint is what makes incremental replacement possible.

- **Both libraries coexist** in the application, with a feature flag per viewport.

- **Strangler by viewport type** — stack viewports first, being the highest-volume and lowest-risk surface, then MPR, then 3D.

- **Shadow mode before cutover** on each viewport type: render both, diff, alert, and only then flip the flag.

- **Codemods** for the mechanical call-site changes, with genuinely different concepts documented rather than auto-migrated.
