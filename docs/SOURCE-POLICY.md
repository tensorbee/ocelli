# Source policy

**This file is the source-provenance policy and it is normative.**
`scripts/source_provenance_check.py` enforces it over every tracked text file
and every manifest, with no allowlist.

HLD Appendix A gate A6 requires this policy to be agreed in writing **before
any agent touches any repository**. It originated as HLD Appendix C.2.1 and was
moved here when Appendix C was taken out of the repository, because the
appendix around it is commercial analysis and this is a legal and engineering
constraint. Nothing normative was lost in the move, and the table below is
C.2.1's, unchanged.

Why the bar is higher here than in an ordinary project:

> Translating source into Rust is a translation, which is an exclusive right of
> the copyright holder, so a copyleft licence blocks **reading**, not merely
> depending. Agent-assisted development sharpens this: exposure cannot be shown
> to be absent after the fact, which weakens any clean-room position.

## What we may read, and what we may depend on

| Project | Licence | Read? | Depend? |
|---------|---------|-------|---------|
| cornerstone3D | MIT | yes | yes |
| dicom-rs, wgpu | MIT / Apache-2.0 | yes | yes |
| VTK | BSD-3 | yes | yes |
| ITK, elastix | Apache-2.0 | yes | yes |
| DCMTK | BSD-style (OFFIS) | yes | yes |
| OpenJPEG, CharLS, OpenJPH | BSD-2 / BSD-3 | yes | yes |
| BlueLight, dicom-microscopy-viewer | MIT | yes | yes |
| NiiVue | BSD-2 | yes | yes |
| Neuroglancer | Apache-2.0 | yes | yes |
| **dwv** | GPL-3.0 | **NO** | no |
| **Horos** | LGPL-3 with a linked AGPL-3 component (Grok) | **NO** | no |
| **Grok JPEG 2000** | AGPL-3 | **NO** | no |

**Two entries are read-blocked: dwv and Horos.** Both are architecturally
interesting and neither may be opened by a person or an agent on this project.
Where their ideas are worth having, and dwv's annotations-as-DICOM-SR certainly
is, take them from the standard, which is where those projects took them from.
Grok is listed because Horos links it, and an AGPL component in a
browser-delivered product would trigger network-use disclosure, which is close
to a worst case.

## Third-party obligations

Track them from day one. Bridging to `@cornerstonejs/codec-charls` is fine and
so are OpenJPEG, CharLS and OpenJPH, all permissive. Grok is not. Attribution
files are cheap to maintain incrementally and expensive to reconstruct.

## Extensions to the table

Decisions made after the policy was written, in the same form.

## The rule for anything not yet listed

**No licence is not the same as permissive.** A repository with no LICENSE
file, and no `license` field in its metadata, is **all rights reserved** by
default under the Berne Convention. It may be read as a person reads any
published page. It may not be copied into this repository, in whole or in
part, and its text may not be used as the base for a file here.

Before taking anything from a source not in either table, check three things
and record the answer:

1. Is there a LICENSE, LICENCE, COPYING or NOTICE file at the repository root?
2. Does the hosting platform's metadata report a licence?
3. Does the licence permit the specific use, which for source is usually
   **derivative works**, not merely use?

If 1 and 2 are both absent, the answer is no.

## Extensions to the C.2.1 table

| Source | Licence | Read? | Copy or derive? | Decided |
|--------|---------|-------|-----------------|---------|
| `aurabx/skills` (`skills/dicom-processing`) | **None declared.** No LICENSE, LICENCE, COPYING or NOTICE at the root, and the GitHub API reports `license: null` | yes, as a published page | **NO** | Bootstrap |

### Why `aurabx/skills` is listed

It was proposed as a source for a DICOM skill. It is a useful pydicom and
DCMTK reference and it is **not** copyleft, so it is not a clean-room hazard in
the way dwv and Horos are. It is simply unlicensed, which means copying it here
would be a plain infringement rather than a subtle one.

`.claude/skills/dicom-tooling/SKILL.md` covers the same ground for this
project's actual need, which is corpus and fixture work, and it was written
from the DICOM standard, the pydicom documentation and the DCMTK manual pages.
It is not derived from that repository.

### One technical note worth keeping

That skill's window and level example computes

```python
img_min = window_center - window_width // 2
img_max = window_center + window_width // 2
```

**This is neither `LINEAR` nor `LINEAR_EXACT`.** It is the naive formula, and
it is the single most common way a DICOM viewer gets windowing wrong. Against
PS3.3 C.11.2.1.2 it omits the `c - 0.5` and `w - 1` adjustment, and against
C.11.2.1.3.2 it omits the `+ 0.5` centring term, so it disagrees with both.

It is recorded here not as criticism of that repository, whose purpose is to
show a reader how to get a picture on the screen, but because it is a live
example of the exact defect class `docs/hld/15-lut-chain.md` section 18.3
exists to catch: code that produces an entirely plausible image with values
that are wrong by a fraction of a level, invisible to a screenshot review and
immediate to a pixel diff.

**Any window and level implementation that reaches this repository must match
one of the three formulas in section 18.2 exactly, and must be proved against
the four-row fixture in section 18.3 before the shader is written.**
