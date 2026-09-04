<!-- Originally converted by scripts/split_hld.py from Ocelli-HLD.docx.
     This tracked Markdown file is authoritative.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the tracked Markdown wins. -->

# Memory and cache

**Source**: bootstrap import from `Ocelli-HLD.docx`, section 8. This tracked Markdown is authoritative.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

## 8. Memory and cache

One budget, three tiers, explicit eviction. Encoded bytes are transient and dropped as soon as a frame decodes. Decoded frames sit in an LRU sized by the caller. GPU textures are their own tier with their own pressure signal, because evicting a texture and evicting a frame have very different costs.

Volume assembly is progressive by default: the viewport renders a partial volume and refines as slices land, which matters more for perceived speed than any decode optimisation. The absence of a garbage collector is the real memory story — a 300 MB volume load has no pause behaviour to tune around, only a budget to respect.
