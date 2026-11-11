# F-239 AC17, primary-source arithmetic review

Reviewer: Codex root, 2026-11-08. Reviewed source tree
`e4a3b620ea853f22a1bf1824008f134c7cdf9ae3`.
This is a source and arithmetic review, not an executed corpus comparison.

## Editions actually read

The official current HTML reports DICOM PS3.3 and PS3.5 2026d. Read PS3.3
[C.7.6.3](https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.7.6.3.html),
including attribute descriptions .1.2, .1.3, .1.5 and .1.6, and
[C.7.9](https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.7.9.html).
For the retired YBR_PARTIAL_422 definition, read the explicitly linked
[PS3.3 2017b C.7.6.3](https://dicom.nema.org/medical/dicom/2017b/output/chtml/part03/sect_C.7.6.3.html).
Also read current PS3.5
[8.2.4](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_8.2.4.html)
and [A.4.4](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.4.html).

The latest T.800 full text was not accessible. The official freely available
[ITU-T T.800 (08/2002)](https://www.itu.int/rec/T-REC-T.800-200208-S/en),
identical to ISO/IEC 15444-1:2002(E), was read at Annex G.1, G.2.2 and G.3.2.
PDF pages 137 and 138 were rendered and inspected. Download SHA-256:
`f6623d00cc7451c339db45dfcce3dd5d2524490e45e01f42e1eb5221cb4abf28`.
No claim is made that the 2024 full text was read, or that its equation numbers
match those in the 2002 edition.

## Rules checked against source

The published full-range and retired partial-range forward coefficients
match the plan. Native 4:2:2 packs two luminances before their shared chroma.
The planar cases and decoder-owned transform exception are consistent with
the source rules. The codec determines encapsulated sample layout.
Palette count zero means 65536. First-mapped clamping and declared entry
range are checked. Normative descriptor depths are 8 and 16. The project's
12-bit admission and R1 fallback are explicit compatibility decisions, not
normative DICOM requirements. Segmented palette refusal remains explicit.

PS3.5 makes the codestream authoritative for JPEG 2000 decoding. The
reversible and irreversible component transforms and unsigned level shift
must be applied in the specified order. The plan's inverse RCT uses floor
before restoring the unsigned offset. Its ICT inverse coefficients agree
with T.800 G.3.2. No exact finite-precision ICT result is required by that
text. DICOM's current forward ICT equations were checked separately.

A.4.4 permits a JPEG 2000 frame to span fragments while each fragment belongs
to one frame. HTJ2K requires one fragment per frame. F-244's already approved
lenient HTJ2K split remains a project compatibility policy.

## Independent arithmetic and outstanding finding

Rational arithmetic recomputed the approved RCT example: RGB [200,100,50]
becomes level-shifted [72,-28,-78], transformed [-16,-50,100], and reconstructs
the original samples. The approved ICT input [-28,22,-38] reconstructs signed
[-81.276,-8.43354,10.984]. Adding 128 and rounding gives [47,120,139].
Inputs were read from the approved plan before calculation. Expected values
were not obtained by executing or reading back product results. This was
not a blind-source experiment.

The additional exact rational inversion found three small coefficient
literals that are not nearest f32 to the exact inverse despite the source's
claim. Two are in FULL_RANGE and one in PARTIAL_RANGE. The exact fractions,
actual and nearest float bits are retained in
`f239-ac17-exact-ybr-inverses.json`. Root corrected those three literals
through the recorded F-239 implementation amendment. Independent repair
review at `06f98d497fcaee7f3755c4cfe1f5c8b9713ebb0f`, recorded in
`F-239-ybr-nearest-working-pass-1.md`, verifies all eighteen final
coefficients. Public basis and narrow-window controls fail on the old
literals and pass after correction. The source reviewer authored that
repair, so its independent arithmetic approval comes from the separate
named reviewer, not this note. Existing tolerances were unchanged.

This review supplies the previously missing at-source edition record.
Its timing is a takeover correction, not evidence that the check happened
before the original implementation.
