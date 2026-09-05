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

## Workflow source authority corrected after S01, recorded 2026-09-04

The operator confirmed that the HLD and backlog source material was sanitized
and fully converted into the tracked Markdown and JSON files during bootstrap.
The external DOCX, XLSX and private redaction bundle are no longer project
inputs. The `docs` gate and the spreadsheet step in `/sync-status` were removed.
Repository-native backlog, sprint-plan, deviation, provenance, prose and content
guards continue to validate the tracked sources.

## Repository-local corpus tooling correction after S01, recorded 2026-09-04

The operator required both untracked runtime assets to live inside the
checkout. Corpus bytes now have one fixed location under ignored
`corpus/data`, and Python tooling now uses a locked uv project with an ignored
`.venv`. Sibling `ocelli-corpus` and `ocelli-tools` fallbacks were removed.

`scripts/populate_corpus.py` can acquire the four public TCIA series, rebuild
the deterministic synthetic layer, and run the corpus gate. It can also seed
from an existing directory in offline mode, but copies only manifest-matching
bytes. CI uses `uv sync --locked`, and the corpus pin test reads the uv project
metadata instead of duplicated install commands.

The former implicit source-document sibling fallback was also removed. The
tracked Markdown and JSON remain authoritative, while the dormant bootstrap
converters require an explicit source path if someone deliberately runs them.
This entry records the corresponding `.claude/WORKFLOW.md` wording change.

## F-002, wasm-pack build pipeline with a hard size budget gate, completed 2026-09-04

**What was built.** The wasm build pipeline, end to end, and the first size
measurement this project has ever had. `ocelli-wasm` declares `wasm-bindgen`
under its existing wasm32 target gate and exports one function,
`ocelli_version()`. `wasm-pack build --target web` produces
`crates/ocelli-wasm/pkg` under HLD section 15.2's release profile, and
`scripts/pin_and_size_check.py --with-size` measures it. The `wasm` gate no
longer skips, so `gate --floor` now reports 17 gates green with no skips.

**The story that lands wasm-bindgen changed, and the tree said otherwise.**
Both `bin/ocelli.sh` and `crates/ocelli-wasm/Cargo.toml` named F-096 as the
story that would declare the dependency. That is not workable: `wasm-pack`
refuses to build a crate that does not depend on `wasm-bindgen`, so the
pipeline and the budget cannot exist before the dependency does.
`CURRENT_SPRINT.md` assigns the removal of the skip to F-002 and is the later
authority. Both comments are corrected rather than left to mislead. F-096
still builds the boundary.

**HLD sections implemented.** `docs/hld/12-workspace-and-build.md` sections
15.2 and 15.3. `docs/hld/A-spike-gates.md` gate A4, first measurement only, and
the gate stays open.
**Deviations.** None. D-10 is cited by the design plan's `Cargo.toml` comment
correction for wgpu's activating story and is F-008's deviation, not this
story's.
**Crates / packages modified.** `crates/ocelli-wasm/`, the root `Cargo.toml`
for the `wasm-bindgen` exact pin, `ci/check-bindgen-isolation.sh`,
`scripts/pin_and_size_check.py`, `bin/ocelli.sh`, `.github/workflows/ci.yml`.
**Tests added.** One, `exported_version_is_the_workspace_version`, plus four
mutation proofs that are evidence rather than tests. `cargo test -p
ocelli-wasm` reports 2.
**Fixture provenance.** No pixel arithmetic and no geometry. This story
computes no value that a DICOM section governs, so HLD 27.2 R3 does not apply
and the design plan's test table says so explicitly rather than omitting the
row.
**Verification.** `/verify --profile feature` at tree `4e67a07cb781`, 18 gates
green and none skipped.
**Corpus.** pass. 91 rows verified, 0 missing, 0 mismatched, and the metadata
audit agrees.
**Tier coverage.** A (WebGPU) n/a, B (WebGL2) n/a, C (CPU) n/a. A build
pipeline resolves no tier. The rows are named rather than omitted because an
omission and a deliberate "no tier here" read identically later.
**LLD updated.** `docs/lld/build-targets.md`, created. `docs/lld/README.md`
gained a row.
**Deviations from the design plan.** None on decisions. Two facts the plan did
not know, both discovered by building rather than by reading:

- **`wasm-opt` fails out of the box.** rustc for wasm32-unknown-unknown enables
  six WebAssembly proposals by default and the `wasm-opt` wasm-pack downloads
  validates without them, so a stock build dies on
  `Bulk memory operations require bulk memory`. Fixed with an explicit flag
  list in `[package.metadata.wasm-pack.profile.release]`, read off
  `rustc --print cfg` rather than from memory. **`wasm-opt = false` is the
  other documented fix and it is the wrong one**, because it produces a green
  build and a larger artefact, so the recorded number would stop describing
  what ships.
- **The isolation check the HLD gives is target-blind.** `cargo tree` filters
  to the host platform by default, so section 15.3's loop cannot see a
  `wasm-bindgen` declared under `[target.'cfg(target_arch = "wasm32")'.dependencies]`,
  which is the form `ocelli-wasm` itself uses. A second pass over wasm32 was
  added beneath the transcribed one.

**Notes for future sessions.**
- **The 14,104 byte baseline is not an answer to gate A4** and must not be
  cited as one. A4 estimates 3 to 8 MB with Naga dominating. This module has
  one function, no wgpu and no Naga.
- **Re-baselining is the expected path for the whole build-out phase.** A 5%
  tolerance on a 14 KB module is blown by the first story that adds anything
  real. During buildup the gate means "the module changed size and nobody said
  so", not "you exceeded a budget". `--accept-size` is the declaration and the
  design plan that used it says why.
- **Reverting a mutation with `mv file.bak file` restores an older mtime** and
  cargo then reuses the build from the mutated source. It surfaced here as a
  false red on reverted code, which is the harmless direction. The same
  mechanism can produce a false green. `touch` after any revert.
- **A missing `wasm-pack` now fails `gate --floor`** rather than skipping it.
  Deliberate, and it matches how CI already treats an absent documented
  prerequisite for the corpus tooling.

## F-007, Cross-target build proof, native desktop and server binary, completed 2026-09-05

**What was built.** `ocelli-native` gained two binary entry points,
`ocelli-desktop` and `ocelli-server`, both stubs that print the four extension
points of HLD section 13 they will implement. `bin/ocelli.sh native` became a
four-step proof and a gate in the floor, replacing a single host build of one
crate. `scripts/target_feature_check.py` compares resolved features across the
host and wasm32 against a declared baseline.

**The plan was wrong about two facts and the code says what is true instead.**

- The plan asserted `cargo tree -p ocelli-native --target
  wasm32-unknown-unknown` must not resolve. It resolves, and
  `cargo check` for that target **succeeded**, because nothing declared the
  crate native-only. Rather than assert something untrue, `lib.rs` now carries
  a `#[cfg(target_arch = "wasm32")] compile_error!` naming section 4 as its
  source, so the table cell is true instead of claimed.
- The plan assumed the two targets' package sets are comparable. The wasm32
  tree legitimately carries eleven packages the host does not, the
  `wasm-bindgen` chain and its proc-macro plumbing. A naive set comparison
  would have reported eleven differences on day one and been re-baselined
  immediately.

**HLD sections implemented.** `docs/hld/03-architecture-and-crates.md` section
4's crate table, both `no` cells. `docs/hld/10-extension-points.md` section 13,
named by the entry points rather than implemented. `docs/hld/12-workspace-and-build.md`
section 15.1's `ocelli-native` entry.
**Deviations.** None.
**Crates / packages modified.** `crates/ocelli-native/`, `bin/ocelli.sh`,
`scripts/target_feature_check.py`, `ci/target-feature-baseline.json`,
`.github/workflows/ci.yml`.
**Tests added.** Two, `banner_names_the_binary_and_all_four_extension_points`
and `the_two_entry_points_are_distinguishable`. `cargo test -p ocelli-native`
reports 3.
**Fixture provenance.** No pixel arithmetic and no geometry. HLD 27.2 R3 does
not apply, and the design plan's test table names the row rather than omitting
it.
**Verification.** `/verify --profile feature`, 19 gates green and none skipped.
**Corpus.** pass. 91 rows verified, 0 missing, 0 mismatched.
**Tier coverage.** A (WebGPU) n/a, B (WebGL2) n/a, C (CPU) n/a. A build proof
resolves no tier. Worth naming here because it is easy to confuse a build
TARGET with a rendering TIER: `ocelli-native` is a target, and tier C is what a
browser session resolves to when it has no GPU.
**LLD updated.** `docs/lld/build-targets.md`, extended with the native half.
`docs/lld/README.md` row updated.
**Deviations from the design plan.** The two facts above. No decision changed.
**Notes for future sessions.**
- **Step 2 omits `--all-targets` and step 3 keeps it, deliberately.** For
  wasm32 the flag pulls in dev-dependencies and `proptest` reaches
  `wait-timeout`, which does not compile for wasm32 and is not meant to.
  Observed, not anticipated. Running the suite under wasm32 needs
  `wasm-bindgen-test` and a browser runner.
- **The feature check starts vacuous**, zero differences over sixteen shared
  packages, and that is expected while the crates are scaffolds. Its value
  arrives with the first dependency somebody adds without thinking about the
  other target. It was proved red by construction rather than left unproved.
- **`ocelli-wasm`'s `native: no` is deliberately NOT enforced**, unlike
  `ocelli-native`'s `wasm: no`. The crate compiles natively so its logic can be
  unit-tested without a browser, and the table cell means "not shipped
  natively". The asymmetry is in the LLD.
- **A mutation that does not mutate proves nothing.** One attempt here, adding
  `rand` with default features off, was meant to create a per-target feature
  difference and did not, so the gate correctly stayed green. It was replaced
  with a `glam` `scalar-math` target gate, which does. Check that a mutation
  actually changed the thing under test before reading the result.

## F-008, ocelli-compute crate skeleton and GPU device-sharing contract, completed 2026-09-05

**What was built.** HLD section 38's Phase 1 hook, as types and compile errors
rather than as prose. `ocelli-render` gained `Tier`, `Caps` and `GpuContext`,
and is the only crate permitted to create a device. `ocelli-compute` gained
`ComputeCtx<'a>`, `ComputeError` and section 31's `Kernel` trait, and depends
on `ocelli-render` because section 31 fixes that direction. Three mechanisms
enforce the contract, and none needs a GPU: the types, two trybuild
compile-fail cases, and `ci/check-device-ownership.sh` behind a new `device`
gate.

**wgpu is activated two sprints early. That is deviation D-10**, approved in
the sprint design round, on section 38's own argument that this hook costs a
few weeks now and a device-sharing retrofit later. `ocelli-render` and
`ocelli-compute` drop `no_std` because wgpu needs `std`. The pin is untouched
and `ocelli-wasm` does not reach wgpu, so the wasm size budget is unchanged.

**Activating wgpu exposed a contradiction inside the HLD, which is deviation
D-12.** Section 15.2 specifies wgpu, section 4 says `ocelli-render` builds for
wasm, and section 15.3 forbids any crate but `ocelli-wasm` from reaching
wasm-bindgen. On wasm32 all three cannot hold, because wgpu talks to the
browser's WebGPU through js-sys and web-sys. The route was traced, not assumed:
`wgpu -> js-sys/web-sys -> wasm-bindgen`, and on the host no such route exists.
Section 15.3's loop is unchanged for the host. For wasm32 the rule became
direct declaration in a crate's own manifest, because D2 is a rule about this
repository's source and wgpu abstracts the target for us.

**HLD sections implemented.** `docs/hld/26-differentiating-capabilities.md`
section 31, the trait and the device-sharing rule.
`docs/hld/19-render-graph.md` section 22's `Caps`.
`docs/hld/27-phase1-hooks.md` section 38's E1.8 row.
**Deviations.** D-10 and D-12, both added by this story. D-07 relied on for the
third tier.
**Crates / packages modified.** `crates/ocelli-render/`,
`crates/ocelli-compute/`, `ci/check-device-ownership.sh`,
`ci/check-bindgen-isolation.sh`, `scripts/target_feature_check.py`,
`scripts/no_std_check.py`, `bin/ocelli.sh`, `.github/workflows/ci.yml`.
**Tests added.** Six. `cargo test -p ocelli-render` reports 4 and
`-p ocelli-compute` reports 2 plus the trybuild harness driving 2 UI cases.
**Fixture provenance.** No pixel arithmetic and no geometry. HLD 27.2 R3 does
not apply, and the design plan's test table names the row rather than omitting
it.
**Verification.** `/verify --profile feature`, 20 gates green and none skipped.
**Corpus.** pass. 91 rows verified, 0 missing, 0 mismatched.
**Tier coverage.** A (WebGPU) full, compute kernels are tier A by definition
and `Caps.compute` says so. B (WebGL2) the contract holds and no kernel runs,
because tier B has no compute shaders, and a kernel with no fallback marks its
feature unavailable. C (CPU) the contract is not constructible, because a tier
C session has no device and therefore no `GpuContext`. `ComputeError::Unavailable`
names both the required and the resolved tier, because "unavailable" without
them is a message nobody can act on.
**LLD updated.** `docs/lld/gpu-ownership.md`, created.
`docs/lld/build-targets.md`, corrected. `docs/lld/README.md` gained a row.
**Deviations from the design plan.** One correction to landed work.
**F-007's feature guard was machine-specific and only wgpu could show it.**
Step 4 of `gate native` reported 42 findings, 32 of them packages on one target
only, and every one legitimate. Most were host-specific: `objc2-metal` and
`raw-window-metal` on macOS, where a Linux runner reports `ash` and
`gpu-alloc`. A baseline listing them would have been correct on one laptop and
red in CI, and the fix for a red CI would have been to re-declare it, which is
tolerance-tuning wearing a different hat. The check now makes one claim, that
every dependency `[workspace.dependencies]` names directly resolves the same
features on both targets. A `cargo tree` parsing bug went with it: the ` (*)`
dedup marker lands inside the feature field and produced four phantom
differences between a package and itself.
**Notes for future sessions.**
- **`wgpu::Device` is `Clone`**, measured, and it is a refcounted handle, so a
  clone is the SAME device. Section 31's "two devices cannot share textures" is
  about a second `request_device`, which the guard script refuses. Do not read
  the absence of `Clone` on `GpuContext` as the defence against a second
  device. It is the one-owner rule for the triple, and it is a separate
  smaller claim.
- **Three tests that could not fail appeared in this one story**, and two of
  them appeared while fixing something else, when attention was on the fix
  rather than on whether the new assertion discriminates. Mutate every test,
  including one written during a remediation.
- **`Kernel` has no implementers and `AGENTS.md` forbids that shape.** The
  collision is real and was decided in the sprint design round rather than
  resolved silently. Section 31 prescribes the signature and HLD Part II says a
  given signature is the intended implementation. F-125 (E31.1) supplies the
  kernels.
- **`scripts/no_std_check.py` carries no exemption list**, by design. It reads
  the attribute from each crate's source, so the two crates that dropped
  `no_std` here left the check by construction.

## F-003, TS package scaffold, bundling, npm publish pipeline, completed 2026-09-05

**What was built.** `scripts/package_check.py` and a `packages` gate that
proves what a consumer receives rather than what compiles. It builds the real
tarballs, asserts their contents against what the manifests advertise,
installs them into a temporary project **outside the npm workspace**, imports
them under `node` and type-checks them under both `bundler` and `node16`
resolution, then runs `npm publish --dry-run`. Both packages gained a
`README.md` and both licence files. `vitest.config.ts` and the first two
TypeScript tests landed with it.

**No bundler, and that was the sprint design round's decision.** `@ocelli/core`
has no runtime dependency and already emits ESM with declarations. What the
story needs from the word "bundling" is that a consumer's resolver handles the
published tarball, which is a property of the tarball rather than of a build
step here, so the pipeline proves it directly. The revisit condition is named:
`wasm-pack --target web` emits a `.wasm` asset that bundlers treat specially,
so F-096 decides again with a real reason.

**The packages shipped no licence text and nobody had looked.** `npm pack
--dry-run` before this story listed `dist/` and `package.json` and nothing
else, while both manifests declare `MIT OR Apache-2.0`. A tarball carrying
neither licence makes that a claim rather than a grant.

**HLD sections implemented.** `docs/hld/07-concurrency-and-typescript.md`
section 10, what stays TypeScript. `docs/hld/12-workspace-and-build.md` section
15.1's `packages/` entries.
**Deviations.** None.
**Crates / packages modified.** `packages/core/`, `packages/react/`,
`scripts/package_check.py`, `vitest.config.ts`, `bin/ocelli.sh`,
`.github/workflows/ci.yml`.
**Tests added.** Two vitest cases in `packages/core/src/index.test.ts`, plus
six assertions in the packaging check.
**Fixture provenance.** No pixel arithmetic and no geometry. HLD 27.2 R3 does
not apply, and the design plan's test table names the row rather than omitting
it.
**Verification.** `/verify --profile feature`, 21 gates green and none skipped.
**Corpus.** pass. 91 rows verified, 0 missing, 0 mismatched.
**Tier coverage.** A (WebGPU) n/a, B (WebGL2) n/a, C (CPU) n/a. A packaging
pipeline resolves no tier.
**LLD updated.** `docs/lld/typescript-packaging.md`, created.
`docs/lld/README.md` gained a row.
**Deviations from the design plan.** None on decisions. One guard was designed,
built and then removed before it landed, and the reasoning is kept because the
next person will have the same idea. The check originally refused to run when
`NPM_TOKEN` or an authenticated `.npmrc` existed. `npm publish --dry-run`
cannot publish, so that defends against a hypothetical future edit while
failing for every developer logged into npm for an unrelated project, with no
action available except deleting credentials or disabling the gate. Building a
gate that invites being disabled is the wrong trade.
**Notes for future sessions.**
- **The consumer install is outside the npm workspace and that is the whole
  point.** Inside it, `@ocelli/core` resolves through the workspace link to
  `src/` and the tarball is never consulted, so the check would pass while the
  defect it exists for was present.
- **Both resolution modes are checked because an `exports` map can satisfy one
  and not the other.** `bundler` is what a Vite or webpack consumer uses,
  `node16` is what a plain `tsc` consumer uses and is the stricter.
- **`@ocelli/react` resolves `@ocelli/core` from the sibling tarball**, not
  from the registry, where `0.1.0` does not exist. If that npm behaviour ever
  changes, the failure will look like a network problem rather than a
  resolution one.
- **`vitest.config.ts` sets `passWithNoTests: false`.** Vitest passes an empty
  run by default, which would have made the whole TypeScript suite vacuous the
  first time a config change stopped matching the test files.
- **`VERSION` is asserted against a literal**, for the same reason
  `ocelli_version()` is in `crates/ocelli-wasm`. Comparing a constant to the
  file it was copied from restates it. Comparing the two files is
  `package_check.py`'s job.

## F-010, Headless cornerstone3D reference renderer, completed 2026-09-05

**What was built.** The reference half of the differential harness, and the
instrument every later story is validated against. `tools/oracle/run.mjs`
drives Playwright-controlled headless Chromium on SwiftShader, renders every
applicable corpus row through cornerstone3D 5.8.2, and writes one raw frame,
one PNG and one metadata sidecar each, or a precise failure. It compares
nothing. Comparison is F-011.

**Measured, and reproduced independently.** The oracle was run in the canonical
worktree, a different checkout from the one that built it, and reproduced the
worker's numbers exactly: 91 rows applicable, 91 reached, 90 decoded, 90
presented, 89 read back, 2 accounted for by `unsupported.json`, determinism
identical across two passes on one browser build, 89 sidecars agreeing with an
independent pydicom read, and 12 fault injections red at their named boundary.

**The two rows cornerstone3D 5.8.2 cannot render**, both diagnosed against the
standard rather than reported as failures:

- `synthetic/us_ybr_full_422.dcm` fails at **read back and not at decode**.
  PS3.3 C.7.6.3.1.2: 4:2:2 stores Y1 Y2 Cb Cr per pixel pair, so the frame is
  480 bytes where `Rows*Columns*SamplesPerPixel` would be 720. cornerstone
  sizes the texture the naive way, the browser refuses the short upload, and
  the frame reads back uniform black while the load RESOLVES. Without the
  read-back guard this row would have been given a stable digest for a blank
  frame and counted as covered.
- `syntax/deflated_explicit_vr_le.dcm`, PS3.5 A.5. The default loader path does
  not inflate before parsing.

All three HTJ2K rows and both JPEG-LS rows render.

**HLD sections implemented.** `docs/hld/08-validation-architecture.md` section
11. `docs/hld/25-first-ten-files.md` entry 4.
`docs/hld/22-testing-and-tolerance.md` section 25.1, transcribed as what F-011
will apply, not applied here.
**Deviations.** D-11, cornerstone3D pinned at 5.8.2 because Appendix B's
v5.8.9 does not exist. Raised by the design round, not by this story.
**Crates / packages modified.** `tools/oracle/` in full, plus `bin/ocelli.sh`,
`eslint.config.js`, `scripts/staged_content_check.py`, `CLAUDE.md`,
`docs/lld/`, `docs/runbooks/guard-verification.md`, `.claude/WORKFLOW.md`,
`.claude/commands/verify.md`, `.claude/commands/close-sprint.md` and the two
generated adapters.
**Tests added.** 137 in seven `node:test` suites, plus 12 fault injectors run
by the gate on every pass, plus `check_sidecars.py` re-reading every row with
pydicom.
**Fixture provenance.** Nine hand-written fixture rows agree with PS3.3, and
89 sidecars agree with an independent pydicom read of the same files. The
expected values come from the standard through pydicom, per HLD 27.2 R2, never
from what the harness printed.
**Verification.** `bin/ocelli.sh gate --sprint` on the merged tree, **ALL GREEN
over 23 gates with zero skips**. The first run in this project in which every
gate, `wasm` and `oracle` included, passed together on one tree.
**Corpus.** pass. 91 rows verified, 0 missing, 0 mismatched.
**Tier coverage.** A (WebGPU) n/a, B (WebGL2) n/a, C (CPU) n/a. This story runs
somebody else's renderer. Ocelli's tiers are resolved by F-004 and exercised
against this output by F-011. The rows are named because it would be easy to
read a story that renders as declaring a tier, and it does not.
**LLD updated.** `docs/lld/oracle.md`, created. `docs/lld/README.md` gained a
row. `docs/lld/corpus.md` gained a pointer.
**Deviations from the design plan.** Four, all reported rather than worked
around, and two of them corrections to the plan itself:

- The plan says `CLAUDE.md` and `README.md` both state the parity target as
  v5.8.9. `README.md` states no version at all.
- The plan describes four boundary faults. Twelve injectors are implemented,
  because a boundary can fail in more than one way worth separating.
- The plan mentions no page server or bundler. Both are needed, because a
  module worker cannot start from `file://` and cornerstone3D's ESM tree does
  not load without one. The plan's actual constraint holds: corpus bytes reach
  the page through `page.evaluate`, so no server in this harness reads
  `corpus/data`.
- The plan says `vitest` and the suites are `node:test`.

**Notes for future sessions.**
- **A defect was found in the REFERENCE, not in this project.** cornerstone3D
  5.8.2's `toLowHighRange` applies LINEAR's `(w - 1) / 2` to SIGMOID, where
  PS3.3 C.11.2.1.3.1 gives SIGMOID its own constraint. Unreachable today
  because all 85 windowed rows resolve LINEAR. It matters because D14 commits
  to publishing a measured divergence, and here the oracle would measure our
  correct arithmetic against the reference's incorrect arithmetic and report it
  as ours. **F-X012.**
- **`PS3.3 C.11.2.1.2.1` was withdrawn**, not defended. Nobody could quote the
  sentence, so the rule is stated without a clause number and grounded in
  cornerstone3D 5.8.2's own default parameter instead. A rule grounded in the
  artefact the harness must match beats one grounded in an unquotable clause.
- **The review loop ran thirteen passes and never came back clean.** It
  oscillated at 20, 9, 5, 7, 4, 3, 4, 2, 5, 5, 5, 3, 2 blocking items while the
  diff grew from +5762 to +10321, because each remediation added surface for
  the next pass to find defects in. It was closed by changing strategy, not by
  more passes, and the final review was taken by the integrator, who is
  independent of the author. See `.claude/reviews/S02-sprint-pass-2.md`.
- **The guard sweep found the mutation harness itself was broken.**
  `node --test tests/` treats the directory as a test and fails before running
  anything, so earlier rounds' "all refusals red" results had a red baseline
  and proved nothing. Re-run against a 137-pass baseline, six of 26 refusals
  were watched by nothing. **F-X009** generalises this to every guard in the
  repository.
- **Nothing under `tools/oracle/out/` is committed**, 269 files produced and
  zero tracked. A reference frame of a real corpus row is a rendered picture of
  patient data and every real row is `burned-in-unchecked`.
