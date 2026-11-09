# F-271, the ICC PCS to sRGB inverse measured both ways, adopted only if closer on every row

**Status**: draft
**Epic ref**: E38.63
**Sprint**: S48
**Estimate**: 2w

Filed at the S46 close from the recovered close-backlog draft, original item
7, from F-240's lead rulings after pass 3. The estimate is inherited and is
provisional, lead-suggested and not capacity-approved. This draft claims no
implementation, no measurement, no review cleanliness and no completion, and
the experiment is not performed by this filing.

## Normative source, transcribed

`.claude/plans/F-240-design.md`, "Lead rulings after pass 3", quoted: "The
matrix choice stays for this story. The plan rules the note's printed matrix,
and pass 3 measured Ocelli further from the colorimetric answer than Little
CMS on this row, by about 1.0e-4 linear. A backlog story is filed at sprint
close to measure an inverse derived from the sRGB primaries with Bradford
adaptation against Little CMS across every AC12 row, and to adopt it only if
it is closer on every row."

`.claude/plans/F-240-design.md`, AC12, quoted in part: "Little CMS 2's
`transicc`, a host-side tool installed on the developer host at 2.18 by
Homebrew, MIT licensed, not shipped and not a dependency, which
`docs/SOURCE-POLICY.md`'s 'What a dependency is' places outside the policy,
converts the AC1 to AC5 and AC8 sample pixels with each synthetic profile as
input, its built-in sRGB as output and media-relative colorimetric intent."
And: "No Little CMS source is read or copied."

`docs/hld/22-testing-and-tolerance.md` 25.1: "Write it down once and hold
it." No threshold is adjusted by this story. The adoption rule is the
ruling's and is all or nothing.

`docs/hld/11-decision-log.md` D14 for the divergence record's shape.

## What the specification does not cover

Which inverse is better is a measurement, not a specification question.
Neither ICC.1 nor IEC 61966-2-1 ranks the two candidate matrices against each
other, and the HLD names no PCS-to-device matrix. The printed-note inverse's
provenance is the ICC's sRGB characterisation note, recorded in F-240's plan
as the inverse of the note's printed D50 colourant matrix.

## Problem and evidence

`PCS_TO_SRGB` at `crates/ocelli-pixel/src/icc.rs:397` at 31323a79 is the
printed-note inverse F-240 ships. F-240's pass 3 measured, on the Adobe red
row, Ocelli's green at +1.073e-4 linear, 0.3535 of 255 encoded, where Little
CMS's is -2.3e-6 linear. That is below AC12's one-code threshold at the
identity window, and the ruling keeps the printed matrix for F-240 and files
this measurement story at sprint close.

"Every AC12 row" is the set AC12 compares, the AC1 to AC5 and AC8 sample
pixels through each synthetic profile, with the `transicc` command line
F-240's LLD records. The row set is confirmed against F-240's completed AC12
record at design time, not asserted here.

The alternative is derived, not printed: the D50 to linear sRGB matrix
computed from the IEC 61966-2-1 primaries with Bradford adaptation to D50.
The derivation is the story's, shown with its arithmetic in the plan's
approving change.

This filing ran no measurement and read no Little CMS source.

## Scope, API and compatibility

In scope: the derivation, the two-way measurement against `transicc` on every
AC12 row, the all-rows adoption rule, and the record either way. If adopted,
expected fixtures are first derived independently from the approved published
primaries and Bradford arithmetic in AC1, then the constant and its comment
change, and the AC12 comparison is re-run in full. Expected values never come
from reading back the edited product constant or its output. If kept, the measurement is recorded and no code
changes.

Non-goals: any other profile or transform, tone curves, lookup-table profiles
beyond their existing refusal, Little CMS as a dependency or shipped
artefact, and any threshold change.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `IccTransform` stage 4 | `PCS_TO_SRGB` and the colourant product | Once per held frame | The converted colour |
| The single viewer's colour contract | The same stage 4 through the shared provider | Per draw | The pixels the front door shows |
| The AC12 record | `docs/lld/pixel-pipeline.md` | Tool version and command beside every figure | The two-way comparison |

## State transitions and invariants

Not applicable. The constant is fixed at build time and changes only in an
approved change that recomputes its fixtures.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Stage / owner |
|----|-------------------------------|---------------------|-------------------------------|---------------|
| AC1 | The derived matrix is computed from the published primaries with Bradford adaptation, and the reviewer checks the derivation against the published IEC 61966-2-1 and the Bradford annex, naming the editions | Review note | A matrix copied from an unquoted source | implementation, reviewer |
| AC2 | Both matrices convert every AC12 row's sample pixels, measured against `transicc` with its version and exact command line recorded | The recorded table | One row only, or a missing version stamp | implementation |
| AC3 | Adoption if and only if the derived matrix is closer to Little CMS on every row. Otherwise the printed matrix stays and the measurement is recorded | The recorded decision | Adopting on a majority of rows | implementation, reviewer |
| AC4 | If adopted, expected fixtures are derived independently from AC1's approved published primaries and Bradford hand arithmetic before the product constant changes, and the full Little CMS AC12 comparison is re-run | The independent calculation, suites and comparison record | Expected values copied from the edited constant or product readback | implementation |
| AC5 | If kept, no code changes and the record says so with the numbers | The record | A silent keep | implementation |
| AC6 | Either way, existing untagged and non-ICC colour fixtures stay byte-identical | The suites | A matrix experiment that perturbs the path without an ICC transform | implementation |
| AC7 | Decision controls use hand-written per-row distances: printed `[1, 1]`, derived `[0.5, 1.5]` keeps printed, derived `[0.5, 1]` also keeps printed, and derived `[0.5, 0.5]` adopts derived. Replacing the all-rows strict comparison with any-row comparison makes the mixed case red | Recorded in the review | Adopting on one improved row, accepting a tie, or assuming in advance that the actual experiment must have a worse row | reviewer |

Unrun ownership. The `transicc` comparison runs on the operator's host, where
F-240's AC12 ran. If that host or tool is unavailable the rows stay open with
that owner named and are never passed, per F-240's evidence-feasibility rule.

## Approach

1. Derive the matrix from the published primaries with Bradford adaptation,
   with the arithmetic shown in the approving change.
2. Run the AC12 command set for both matrices over every row.
3. Apply the all-rows rule and record the outcome and the numbers.

Rejected: averaging the two matrices, switching per row, and adopting on the
identity window alone.

## Dependencies, write ownership and resources

Depends on F-240, in-progress in S46, as the recovered draft had. S48 holds
no ordering constraint, and this story integrates after F-240 completes
because it measures F-240's shipped constant. Writes
`crates/ocelli-pixel/src/icc.rs` and its tests only if adopted, plus
`docs/lld/pixel-pipeline.md`. `transicc` is a host tool, never shipped and
never a dependency.

## Verification and evidence reuse

The AC2 table is the evidence, with the tool's version and command line.
The feature profile covers `ocelli-pixel` either way. Evidence invalidates on
any later stage 4 change, which re-runs the comparison.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. The matrix product is per held frame
- unsafe: none
- Tier A (WebGPU): the converted frame draws as any colour frame, unchanged
- Tier B (WebGL2): the same
- Tier C (CPU): the conversion is CPU arithmetic on every tier, unchanged

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | The derived matrix's hand-computed row, IEC 61966-2-1 cited, if adopted | `crates/ocelli-pixel/tests/icc.rs` |
| conformance | The two-way `transicc` comparison | AC2, recorded |

## Parity surface covered

None.

## Deviations

None new.

## LLD impact

`docs/lld/pixel-pipeline.md`, the AC12 record's extension.

## User-facing documentation impact

No change unless adopted, in which case the ICC sentence in
`examples/folder-viewer/src/more.html` is checked for any number it quotes.

## Open questions

1. Whether the record names one number or a per-row table for the bound.
   Affects the record's shape. Owner: the design round.
2. The exact row set, confirmed against F-240's completed AC12 record at
   design time. Affects AC2. Owner: the design round.
