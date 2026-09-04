<!-- Originally converted by scripts/split_hld.py from Ocelli-HLD.docx.
     This tracked Markdown file is authoritative.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the tracked Markdown wins. -->

# Appendix B, parity surface

**Source**: bootstrap import from `Ocelli-HLD.docx`, Appendix B. This tracked Markdown is authoritative.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

## Appendix B, Parity surface

Measured from cornerstone3D v5.8.9 source. The accompanying backlog maps each of these to the story that covers it.

| **Surface** | **Count** | **Notes** |
|----|----|----|
| Viewport types | 12 | STACK, ORTHOGRAPHIC, PERSPECTIVE, VOLUME_3D, PLANAR, VIDEO, WHOLE_SLIDE, ECG and \_NEXT variants |
| Tool classes | ~63 | 26 annotation, 12 segmentation, 25 manipulation and utility |
| Blend modes | 5 | Composite, MIP, MinIP, average, labelmap edge projection |
| VOI LUT functions | 3 | LINEAR, LINEAR_EXACT, SAMPLED_SIGMOID |
| Transfer syntaxes | ~13 | Two of them, JPEG-LS and HTJ2K, are the open gates in Appendix A |
| Segmentation representations | 3 | Labelmap, contour, surface |
| Core events | 50 | Plus 53 tool events; re-shaped rather than copied |
| Adapters | 4 | SR TID 1500, SEG, RTSTRUCT, parametric map |

Source line counts, excluding tests: tools 124,939; core 81,889 of which RenderingEngine is 52,057; adapters 12,592; dicomImageLoader 10,539; metadata 6,395. Approximately 243,000 lines in total, plus the vtk.js rendering layer which the repository does not contain.
