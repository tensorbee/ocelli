# As Built, completion log

Append-only record of every completed F-ID. New entries land at the **bottom**.
**Never edit a prior entry.** A correction goes in a follow-up entry that
references the original, because the value of this file is that it records what
was believed at the time.

Written by `/complete-feature` step 2.

## Entry format

```markdown
## F-XXX, {short title}, completed {YYYY-MM-DD}

**What was built.** {1-3 sentences describing the deliverable}

**HLD sections implemented.** {docs/hld/<file>.md sections, with section numbers}
**Deviations.** {D-NN rows in docs/hld/DEVIATIONS.md, or "none"}
**Crates / packages modified.** {paths}
**Tests added.** {paths, count by category from the taxonomy in WORKFLOW.md}
**Fixture provenance.** {for pixel arithmetic: the DICOM section each hand-computed
                         fixture cites. HLD 27.2 R3. Or "no pixel arithmetic".}
**Verification.** {gate set, date, and the Ocelli-Verify trailer's tree}
**Corpus.** {pass with N cases | absent, with the reason | failed-and-justified}
**Tier coverage.** {per tier: A (WebGPU), B (WebGL2 downlevel), C (CPU).
                    full, degraded (how), unavailable, or n/a. All three.}
**LLD updated.** {docs/lld/*.md files updated}
**Deviations from the design plan.** {list with reasons, or "none"}
**Notes for future sessions.** {non-obvious details, limitations, follow-up F-IDs}
```

## Why `Fixture provenance` and `Tier coverage` are their own fields

**Fixture provenance.** HLD section 27.2 R3: every function doing pixel
arithmetic needs a fixture test with hand-computed values, citing the DICOM
section. R2 says why a generic "tests added" line is not enough: an agent asked
to test a function will assert what it does, not what it should do. Naming the
specification section is the difference.

**Tier coverage.** HLD section 7 has two capability tiers and deviation D-07
adds a third, C, for CPU. A feature that works on tier A and silently does
something different on another tier is the failure mode section 31 calls out:
a kernel with no fallback marks its feature unavailable, it never silently
produces a different answer. A story that touched rendering and does not say
which tiers it was exercised on has not answered the question, and "both" is
now an ambiguous answer because there are three.

## Entries

## F-001, Cargo workspace, crate skeleton, lint/CI baseline, completed 2026-09-04

**What was built.** The `ocelli-core` coordinate and value spaces, entries 1 and
2 of the first-ten-files list. `space.rs` carries the three uninhabited marker
spaces, `Pt<S>` and `Transform<A, B>` with `apply`, `inverse` and `then`.
`value.rs` carries `Stored`, `Modality` and `Display` and deliberately no
arithmetic. The workspace, the thirteen crates, the lint baseline, the gate
runner and the bindgen isolation check were already present from the bootstrap,
so this story finished the crate rather than creating it.

**HLD sections implemented.** `docs/hld/13-core-types.md` sections 16 and 16.1.
`docs/hld/22-testing-and-tolerance.md` section 25's round-trip property and
section 25.1's geometry tolerances. `docs/hld/25-first-ten-files.md` entries 1
and 2.
**Deviations.** D-08 and D-09, both raised by this story's design plan and
recorded in `docs/hld/DEVIATIONS.md` in the design commit. D-01 relied on.
**Crates / packages modified.** `crates/ocelli-core/`, and the root `Cargo.toml`
for the D-09 glam feature change and the `proptest` and `trybuild` entries.
**Tests added.** `cargo test -p ocelli-core` reports 24, in four categories.
`unit`, 14 under `#[cfg(test)]` in `src/lib.rs`, `src/space.rs` and
`src/value.rs`. `fixture`, 6 in `tests/geometry_ps3_3_c7_6_2.rs`. `property`, 3
in `tests/roundtrip.rs`, being two proptest cases and one fixed projective
case. `compile-fail`, one harness in `tests/compile_fail.rs` driving 4 cases
under `tests/ui/`, so the 24 undercounts what actually runs.
**Fixture provenance.** DICOM PS3.3 C.7.6.2.1.1, the voxel to patient
transform. Four hand-computed positions from `IPP = (-45.2, 118.7, -32.5)`, an
oblique orthonormal `IOP = (0.6, -0.64, 0.48, 0.8, 0.48, -0.36)` and a
non-square `PixelSpacing = (0.5, 0.25)`. The integrator and two review passes
each recomputed all four independently with exact rational arithmetic, from the
standard rather than from the Rust, and confirmed the frame is exactly
orthonormal with `FRAME_Z` exactly `FRAME_X` cross `FRAME_Y`. A transposed
`PixelSpacing` index moves the far corner by 63.72 mm, which is 6.4e7 times the
1e-6 mm tolerance.
**Verification.** `/verify --profile feature` at tree `0259e9b61107`, 12 gates
green and 4 skipped, and a skipped gate is not a pass. The `Ocelli-Verify` trailer on commit `8dfb558` records
`gates=backlog,bindgen,clippy,content,deviations,fmt,pins,prose,provenance,skills,test,unsafe`.
**Corpus.** absent. This branch carries the empty base manifest, so
`gate corpus` passed vacuously and was recorded as `absent` rather than `pass`.
F-009 fills the corpus.
**Tier coverage.** A (WebGPU) n/a, B (WebGL2) n/a, C (CPU) n/a. CPU-side type
and arithmetic code with no rendering or compute path. Every tier consumes
these types and none needs a variant of them.
**LLD updated.** `docs/lld/core-types.md`, created. `docs/lld/README.md`
gained its first row.
**Deviations from the design plan.** One, and it corrected the plan rather than
the code. The plan's third mutation check, replacing `project_point3` with
`transform_point3` and expecting the round-trip property to go red, cannot go
red against the test the plan specifies: HLD section 25's listing fixes `z = 0`
on an affine transform, and the two glam calls are then bit identical, measured
at 10201 sample points with a worst difference of exactly zero. Section 25's
test was kept and a projective round trip, a fixed projective case and a
hand-worked `apply_divides_by_the_resulting_w` unit test were added. Those do
go red under the mutation.
**Notes for future sessions.**
- **`Transform::inverse` on a singular transform returns non-finite values
  rather than failing.** Measured: `DVec3(NaN, NaN, NaN)`. F-023 owns the
  camera constructor and must not assume this layer checked. A test asserts the
  behaviour so the claim is executable rather than folklore.
- **The review loop took four passes**, 3 defects and 2 smells, then 2 and 1,
  then 0 and 1, then clean. Three of the findings across those passes were the
  same shape: a test that could not fail. A vacuous `Copy` test, then a D-08
  that nothing exercised, then a D-08 guard that reached two of its six
  derives. The guard now reaches all six and the full and partial reverts both
  fail to compile.
- **D-09 is guarded**, but only from this sprint's later commit. Reverting the
  workspace glam entry leaves every gate green including
  `cargo check --target wasm32-unknown-unknown`, because that target ships a
  `std` implementation and a `no_std` crate may depend on a `std` crate.
  `scripts/no_std_check.py` reads the dependency graph instead and is what
  actually holds the deviation.
- `value.rs` has no `From` between the three newtypes on purpose. That
  conversion is the modality and VOI LUT arithmetic, and HLD section 18
  requires it to exist exactly once, in `ocelli-pixel`.
- `docs/hld/B-parity-surface.md` has no `Covered by` column, so this story's
  `## Parity surface covered` says "none" for want of anywhere to look it up
  rather than because nothing is covered. `.claude/commands/design.md` step 6,
  `.claude/commands/parity.md` step 1 and `docs/sprints/BACKLOG.md`'s header all
  reference that column. Regenerating Appendix B from the authored document
  reproduces the tracked file byte for byte, so it is not a lost section. This
  needs an operator decision and no story owns it.
- **`.claude/WORKFLOW.md` changed during this sprint**, which its own rule says
  to record here. Two triggers: it stated that `CHANGELOG.md` released sections
  are exempt from the voice rules, which reads as though unreleased sections are
  checked, and `scripts/prose_check.py` does not cover that file at all.

## F-009, Golden corpus ingest and de-identified fixture store, completed 2026-09-04

**What was built.** A 91-case golden corpus behind a committed manifest, with
47 byte-deterministic synthetic cases and 44 real cases from four TCIA series.
The manifest covers all 16 transfer syntaxes in the declared codec registry,
both tolerance classes, case digests and actionable licence records while the
DICOM bytes remain outside git.

**HLD sections implemented.** None. Built against
`docs/hld/08-validation-architecture.md` section 11 and
`docs/hld/22-testing-and-tolerance.md` section 25.1.
**Deviations.** None new. D-05 is implemented and D-04 is relied on.
**Crates / packages modified.** `corpus/`, `scripts/corpus_check.py`,
`scripts/corpus_synth.py`, `scripts/corpus_tests.py`, their Python tests,
`.github/workflows/ci.yml`, `bin/ocelli.sh`, `docs/SOURCE-POLICY.md` and the
verification workflow.
**Tests added.** 56 Python tests behind `gate corpus-tests`: 17 coverage and
registry unit tests, and 39 generator tests covering deterministic output,
stored-value fixtures, synthetic traps, encoder provenance and codestream
conformance. The corpus gate separately verifies coverage and all 91 digests.
**Fixture provenance.** DICOM PS3.3 C.7.6.3.1.4 and PS3.5 section 8.1.1. The
stored-value tests unpack eight hand-chosen 12-bit signed words in both right
and left alignment, with the expected values computed from the standard.
**Verification.** `/verify --profile feature` at tree `026fed3ec8e5`, with 14
gates green. The `Ocelli-Verify` trailer on commit `f36b3db` records
`gates=backlog,bindgen,clippy,content,corpus,corpus-tests,deviations,fmt,pins,prose,provenance,skills,test,unsafe`.
**Corpus.** pass with 91 cases, 16 of 16 declared transfer syntaxes, 85
monochrome 16-bit rows and 6 colour or ultrasound rows.
**Tier coverage.** A (WebGPU) n/a, B (WebGL2 downlevel) n/a, C (CPU) n/a. The
corpus and its verification tools are tier-independent inputs to later parity
work and contain no rendering or compute path.
**LLD updated.** `docs/lld/corpus.md`, created, and `docs/lld/README.md`, which
indexes it.
**Deviations from the design plan.** The real class-two case is 8-bit
monochrome ultrasound rather than colour. It meets the approved colour or
ultrasound condition, but does not exercise real-world chroma. The coverage
tool reports that limitation and the synthetic layer supplies the YBR cases.
**Notes for future sessions.**
- The real layer has no chroma, so chroma subsampling and YBR conversion are
  covered only by deterministic synthetic cases.
- The JPEG 2000 and JPEG-LS generator cases are encoded and decoded by the
  same libraries. Independent decoder conformance remains work for the codec
  stories.
- Encapsulation edge cases are absent. Multi-fragment frames, encapsulated
  multiframe instances and empty Basic Offset Tables belong to E2.6.
- The review loop took seven passes: 4 defects and 4 smells, 3 and 2, 4 and 2,
  1 and 1, 0 and 2, 1 and 1, then clean. Pass 7 also recorded two non-blocking
  nitpicks.

## Correction to F-009, S01 verification profile, recorded 2026-09-04

The F-009 entry was already committed when consolidated sprint verification
found a bootstrap contradiction. `/run-sprint` required the strict oracle gate
in S01, but F-010 in S02 is the story that builds the oracle. The operator chose
a narrow workflow exception rather than moving F-010 into S01.

`gate --sprint` now records the absent oracle as a named skip only while the
active sprint is S01 and F-010 remains pending in S02. S01 contains no port
code and builds the corpus the oracle will consume. `gate --all` and release
remain strict, and the exception stops applying when F-010 moves from pending.
This is the workflow-change trigger required by `.claude/WORKFLOW.md`.

## Corrections from S01 sprint review, recorded 2026-09-04

The F-001 entry says `scripts/no_std_check.py` holds D-09. The script existed
but no gate, hook or CI job invoked it, so the claim was false at completion.
Sprint review pass 1 wired it into the floor as the `nostd` gate and added the
same check to CI. The guard now runs in feature, sprint and release profiles.

The F-009 entry records 56 corpus tooling tests. Sprint review added six
metadata-audit tests and one fail-closed dispatch test, making the current total
63. The corpus gate now compares each manifest row's coverage-driving labels
with non-patient DICOM metadata in the corresponding file. A digest-valid row
can no longer claim the wrong modality, transfer syntax or tolerance class
silently.
