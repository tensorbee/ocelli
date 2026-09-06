# F-X012, SIGMOID divergence review, pass 2

**Reviewer**: independent root agent, did not write the implementation
**Diff reviewed**: the complete remediated staged diff in
`/private/tmp/ocelli-f-x012`, base
`f3c7176f801dd691402248d0e7ed357bf0f9be2b`
**Result**: 2 defects, 0 smells, 0 nitpicks

## Defects

### D1, the register rung can conceal an unrelated geometry defect

**Where**: `tools/oracle/src/attribution.rs:808-868`

**What**: the remediated register entry now proves the declared pointwise
monochrome inversion, which closes pass 1's arbitrary-pixel hole. The
attribution ladder nevertheless evaluates that effect before
`!geometry.is_empty()`. A candidate carrying the exact inversion and an
unrelated camera or rectangle defect is therefore attributed to the reference
at rung `register`. The candidate geometry defect is not allowed to reach rung
`geometry`.

**Why it is wrong**: the register declares one pixel effect caused by the
reference helper. It does not explain geometry. This file already documents
the same causal constraint as load-bearing for the volume-divergence rung.
The register rung needs the equivalent geometry-agreement condition, with an
adversarial test that combines the exact inversion with a candidate geometry
mutation.

**Evidence**: `effect_entry` is selected solely from `gate_failed` and
`Effect::holds`. The ladder takes `else if let Some(entry) = effect_entry` at
line 831, while the first unconditional geometry failure is not evaluated
until line 866. The existing inversion fixture gives both runs the same
sidecar and camera, so it cannot detect this ordering error.

### D2, the local F-ID validator rejects a valid repository F-ID

**Where**: `tools/oracle/src/attribution.rs:255-260`

**What**: `valid_f_id` accepts only exactly three digits after `F-` or `F-X`.
The canonical sprint workflow accepts an optional lowercase suffix, for
example `F-001a`, through `^F-X?\\d{3}[a-z]?$` in
`scripts/sprint_workflow.py:45` and advertises that spelling in its refusal at
line 425.

**Why it is wrong**: `raisedBy` and `resolvedBy` are repository feature IDs.
Two parsers with different grammars make valid provenance fail depending on
which repository surface reads it. Use the canonical grammar here, or a shared
reader if one is available, and cover suffixed IDs in both ordinary and `X`
forms.

## Smells

None.

## Nitpicks

None.

## Pass 1 remediation verified

- A register entry now has an explicit, validated effect rather than firing
  on any pixel mismatch.
- Parameter divergence remains ahead of the register rung and retains its own
  side attribution.
- Empty and malformed unsuffixed provenance values are refused.
- The centre-code prose now describes the observed quantisation without
  claiming nearest-integer rounding.
- The unrelated pixel and rescale fixtures exercise the two original causal
  holes.

## Independent checks

- The staged diff passes `git diff --cached --check`.
- The focused Rust attribution implementation and fixtures were inspected
  together with the complete staged write set.
- `scripts/sprint_workflow.py` is the measured canonical F-ID grammar.
- No HLD or tolerance source is changed.
