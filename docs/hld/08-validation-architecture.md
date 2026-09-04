<!-- Originally converted by scripts/split_hld.py from Ocelli-HLD.docx.
     This tracked Markdown file is authoritative.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the tracked Markdown wins. -->

# Validation architecture

**Source**: bootstrap import from `Ocelli-HLD.docx`, section 11. This tracked Markdown is authoritative.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

## 11. Validation architecture

Cornerstone3D is a correct reference implementation that can render any series you own. The harness pushes the same study through both stacks and compares frames within a written per-modality tolerance, with metadata diffed alongside pixels because a wrong rescale slope can still produce a plausible image.

Every pull request renders the corpus in CI. Every field bug becomes a permanent fixture. In production, shadow mode renders both libraries and alerts on divergence — the oracle running against real clinical traffic, and the same corpus a regulatory submission would want to see.
