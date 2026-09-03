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

## Which product names appear here, and why

Three categories, and only one of them is redacted.

**Commercial competitor names are removed.** They are competitive intelligence
rather than engineering, and `scripts/split_hld.py` strips them from the
generated specification on the way into this repository. Each replacement keeps
the point the sentence was making, because a vendor's published memory constant
is evidence for a design decision and the evidence survives without the name.
The unredacted text stays in the authored document, outside this repository.

**Read-blocked projects are named, deliberately.** A policy whose function is to
say what must not be opened cannot do that without naming them. HLD gate A6
requires this agreed in writing, and an unnamed prohibition is not one.

**Dependencies are named, necessarily.** `Cargo.toml` declares several by name,
the codec registry is specified in terms of its decoders, and two of the six
spike gates are questions about specific libraries. Removing them would stop
the documents describing the software.

## The rule for anything not yet listed

**No licence is not the same as permissive.** A repository with no LICENSE
file, and no `license` field in its metadata, is **all rights reserved** by
default under the Berne Convention. It may be read as a person reads any
published page. It may not be copied into this repository, in whole or in
part, and its text may not be used as the base for a file here.

Before taking anything from a source not in the table above, check three
things and record the answer:

1. Is there a LICENSE, LICENCE, COPYING or NOTICE file at the repository root?
2. Does the hosting platform's metadata report a licence?
3. Does the licence permit the specific use, which for source is usually
   **derivative works**, not merely use?

If 1 and 2 are both absent, the answer is no.

## Extensions to the table

None yet. A source assessed after this policy was written gets a row here, in
the same form as the table above, with the date it was decided.

## One defect worth naming, because it is everywhere

Reference material for DICOM windowing very commonly computes:

```python
# WRONG. Neither LINEAR nor LINEAR_EXACT. Do not copy this shape.
img_min = window_center - window_width // 2
img_max = window_center + window_width // 2
```

**This is neither `LINEAR` nor `LINEAR_EXACT`.** Against PS3.3 C.11.2.1.2 it
omits the `c - 0.5` and `w - 1` adjustment, and against C.11.2.1.3.2 it omits
the `+ 0.5` centring term, so it disagrees with both.

It is extremely common in tutorials, blog posts and example code, and it
survives because it produces a picture that looks entirely correct. It is a
live instance of the exact defect class `docs/hld/15-lut-chain.md` section 18.3
exists to catch: values wrong by a fraction of a level, invisible to a
screenshot review and immediate to a pixel diff.

**Any window and level implementation reaching this repository must match one
of the three formulas in section 18.2 exactly, and must be proved against the
four-row fixture in section 18.3 before the shader is written.** If a reference
you are consulting contains the shape above, it is not authoritative on the
LUT chain, whatever else it gets right.
