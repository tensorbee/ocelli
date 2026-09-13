# S08 sprint review, pass 1

**Reviewed**: full sprint history from
`7025d6fe66342bb3933ab3e17038ff93bfd7389b` through
`9e7e22d85498ec4c9539d495859e05f1556ff80d`, plus exact pre-report staged
integration-repair tree `0cffd6e73fafe8e22da71128c5a6c6f666848831`
**Result**: 5 defects, 0 smells, 0 nitpicks

## Defects

### D1, F-019 never projects its manifest-backed multiframe conformance row

**Where**: `.claude/plans/F-019-design.md:114`,
`crates/ocelli-dicom/tests/corpus.rs:22`, and
`crates/ocelli-dicom/tests/multiframe.rs:77`

**What**: The approved F-019 test table says the manifest-backed multiframe
row is projected through the ignored corpus test. The corpus test parses every
file and checks its Transfer Syntax UID, but never constructs
`MetadataSet::from_object` or `MultiframeMetadata`. The only six tests that use
`MultiframeMetadata` construct local synthetic `MetadataSet` values.

**Why it is wrong**: The row specifically binds the new projection contract to
the tracked manifest and ignored corpus bytes. A malformed production
projection of the real nested sequence representation can therefore leave all
F-019 tests and `gate corpus` green. This is distinct from the synthetic
functional-group fixtures, which prove the resolver after metadata has already
been projected into its expected shape.

**Evidence**: The test-list command for the multiframe target listed six
synthetic tests. The same command for the corpus target listed only generic
manifest parsing and the F-025 native, RLE, and Deflate equivalence test. A
repository-wide search found no use of
`MultiframeMetadata` outside `tests/multiframe.rs`. The corpus gate still
exited 0 over all 92 rows, confirming that its current success does not prove
the approved F-019 conformance claim.

Add one ignored corpus assertion for the exact manifest-backed multiframe row.
Project the parsed object through `MetadataSet`, construct
`MultiframeMetadata`, and assert the declared count plus distinct shared and
per-frame sources without logging attribute values. Keep the existing
synthetic tests because they own the detailed refusal cases.

### D2, F-019's named cross-target commands do not compile F-019 for wasm

**Where**: `.claude/plans/F-019-design.md:116`, `bin/ocelli.sh`, and
`crates/ocelli-wasm/Cargo.toml`

**What**: The approved cross-target row cites `bin/ocelli.sh check
ocelli-dicom` and `bin/ocelli.sh wasm` as proof that the new types compile for
native and wasm. The first command is a native all-target check. The second
builds `ocelli-wasm`, whose dependency tree contains only `ocelli-core` and
does not reach `ocelli-dicom`.

**Why it is wrong**: Both named commands can pass while wasm compilation of
the F-019 source is broken. The current `native` gate independently compiles
shared crates for `wasm32-unknown-unknown`, so this is an evidence and plan
defect rather than a confirmed portability defect in the implementation.

**Evidence**: `bin/ocelli.sh cargo tree -p ocelli-wasm --depth 1` printed only
`ocelli-core` beneath `ocelli-wasm`. The fresh `gate native` run did compile
`ocelli-dicom` for wasm and exited 0, which proves the implementation today but
does not make the plan's two named commands truthful.

Correct the test row to name `bin/ocelli.sh gate native` and a direct
`wasm32-unknown-unknown` check of `ocelli-dicom`, following the already
reviewed F-018 evidence pattern.

### D3, three batch-integrated stories have no completion records

**Where**: `docs/sprints/BACKLOG.md:137`,
`docs/sprints/BACKLOG.md:138`, `docs/sprints/BACKLOG.md:143`,
`docs/sprints/AS_BUILT.md`, `docs/sprints/SPRINT_TRACKER.md`, `CHANGELOG.md`,
and `.claude/scratch/S08-run.json`

**What**: F-018, F-019, and F-024 have reviewed and verified integration
commits, but the backlog still marks all three pending. They have no AS_BUILT
sections, no tracker rows, and no user-visible changelog entries. The run state
leaves them `integrated` instead of `completed`.

**Why it is wrong**: `/complete-feature` defines those records and the
completed state as part of closing a story. `/close-sprint` requires every
story to be done or explicitly carried. The sprint cannot be handed to the
operator as finished while three delivered stories remain incomplete in every
delivery authority. F-020 also still reads as depending on a pending F-019.

**Evidence**: Exact searches found only F-025 and F-026 S08 completion
sections and tracker rows. `python3 scripts/sprint_workflow.py close-preflight
S08` exited 1 and named `F-018, F-019, F-024` as neither completed nor carried.
The additional missing-review and dirty-tree messages are expected before this
report is recorded and committed. They do not explain the three feature-state
failures.

Add append-only AS_BUILT sections and tracker rows for all three stories, mark
their backlog rows and run-state entries completed, and cover their
user-visible image-plane, multiframe, and JPEG additions in `CHANGELOG.md`.
Do not rewrite the existing F-025 or F-026 completion entries.

### D4, the sprint design commit has no provenance trailers

**Where**: commit `586e503496a1762f49651bc1411e18350ceda4a8`

**What**: The commit that approves all five S08 design plans and the initial
source-policy and deviation decisions carries neither `Ocelli-Verify` nor
`Ocelli-Generated-By`.

**Why it is wrong**: HLD 27.2 R6 and the repository workflow require every
commit to bind its exact tree to verification and name the generator. This is
a substantive sprint commit containing 770 added lines across the approved
plans and policy records.

**Evidence**: `git show -s --format=fuller 586e503` displayed only the one-line
subject. `python3 scripts/verify_ledger.py check-commit --require-corpus
586e503` exited 1 with `carries no Ocelli-Verify trailer`. The other six
commits in the reviewed sprint range each passed the same exact check with a
matching tree and corpus evidence.

The branch is local, so the normal remediation is a deliberate history rewrite
that recreates this commit through the verification ledger and hook, then
regenerates every descendant hash and review-tree reference. Do not type the
trailers by hand. If history may not be rewritten, obtain an explicit HLD
deviation that records the permanent provenance gap before close.

### D5, the sprint contract still promises the rejected codec architecture

**Where**: `docs/sprints/CURRENT_SPRINT.md:89`,
`docs/sprints/CURRENT_SPRINT.md:92`,
`docs/sprints/CURRENT_SPRINT.md:111`, and
`docs/sprints/BACKLOG.md:145`

**What**: The sprint-level completion contract says F-025 registers Deflate as
a frame path and says F-026 uses and proves the selected `openjp2` path. The
approved F-025 plan deliberately leaves Deflate at whole-data-set ingest and
`KnownUnavailable` to frame dispatch. The approved F-026 plan, D-20, current
source, LLD, tracker, and AS_BUILT record use the reviewed `ritk-codecs` 0.6.0
replacement because `openjp2` failed native and wasm evidence. The backlog and
current-sprint story title still name `openjp2` too.

**Why it is wrong**: These are incompatible product and dependency claims in
the same sprint. A later reader using the sprint contract can conclude that a
Deflate frame decoder exists or that the rejected C-derived dependency must be
restored. Both would undo deliberate architecture and safety boundaries.

**Evidence**: `.claude/plans/F-025-design.md:58-67` assigns Deflate only to
F-016 ingest. `.claude/plans/F-026-design.md:55-85` records the failed
`openjp2` evidence and approved pure-Rust replacement. D-20 binds the exact
vendor inventory and no-Rayon graphs. Fresh `gate native`, `gate pins`, the
14 pin tests, and the codec tests all passed the replacement implementation.

Reconcile the sprint and backlog prose with the approved plans. Say that
F-025 registers native and RLE frame decoders while verifying the existing
Deflate ingest path, and name `ritk-codecs` as F-026's approved replacement.

## Smells

None.

## Nitpicks

None.

## Verified clean outside the findings

- The exact pre-report staged tree was
  `0cffd6e73fafe8e22da71128c5a6c6f666848831`, matching the requested review
  identity. `python3 scripts/verify_ledger.py assert` exited 0 and confirmed a
  sprint-profile record with corpus pass and all 30 declared gates, including
  oracle, for that exact tree.
- The full staged integration repair changes only
  `scripts/guards/catalogue.py` and `scripts/tests/test_guard_catalogue.py`.
  It preserves the existing vendor exclusion while adding the probe member,
  emits one TOML `exclude` key, and keeps the probe aimed at the lint-policy
  refusal. Its focused unit test, runbook census, exact deep probe, and complete
  guards gate all exited 0. The full gate drove 341 refusal probes red across
  32 guards, with 41 accept probes and 50 controls green.
- A fresh arithmetic mutation changed `NativeFrameIndex` frame start from
  `frame_bits * frame` to `frame_bits + frame`. The exact mid-byte one-bit
  multiframe test exited 101 with `[22]` instead of `[13]`. The mutation was
  reversed exactly, the same test exited 0, and no unstaged tracked diff
  remained.
- `bin/ocelli.sh test ocelli-pixel`, `bin/ocelli.sh test ocelli-dicom`, and
  `bin/ocelli.sh test ocelli-codec` exited 0. The S08-focused suites covered
  image-plane geometry, stored extraction, Modality and VOI LUT arithmetic,
  multiframe provenance, Basic and Extended frame indexing, JPEG, JPEG 2000,
  native Values, RLE, registry atomicity, and caller-buffer atomicity.
- `gate native` exited 0 through all eight steps. It compiled the shared crates
  for native and wasm, found 18 direct dependencies with equal target features,
  executed JPEG 2000 natively, and executed the same production decoder as
  plain and SIMD wasm under Node.
- `gate corpus` exited 0 over 92 verified manifest rows and both ignored corpus
  tests. D1 states the exact F-019 claim that those two tests do not cover.
- `gate bench` exited 0 with 12 subjects, three baselines, 27 Python tests, and
  97 Node tests where 96 passed and the browser-only test was the declared
  skip. A fresh production comparison measured JPEG 2000 at 0.7214 ms within
  its band. The first sandboxed comparison failed only because `uv_uptime` was
  denied, then the same command exited 0 outside the sandbox.
- The vendor tree remains 78 files and 2,044,376 bytes. Its published-package
  inventory has 74 rows. The pins gate, its 14 focused tests, and the size and
  licence check exited 0. No `rayon` entry appears in the lockfile or either
  patched manifest.
- `gate packages` passed 79 workspace tests and verified two package tarballs
  and an outside-workspace consumer after using a private npm cache. The first
  run failed only because the user's default npm cache contained root-owned
  files.
- The focused `fmt`, `unsafe`, `provenance`, `prose`, `content`, `deviations`,
  `bindgen`, and `errors` gates all exited 0. Unsafe inspected 159 Rust files
  with only the two existing permitted files. Provenance inspected 714 files,
  prose inspected 271 files, and no patient data or build artefact was staged.
- The reviewed implementation keeps F-020 geometry interpretation, F-027
  HTJ2K, F-028 JPEG-LS, presentation inversion, palette execution, and render
  work outside S08. No new trait, unapproved generic, render-loop allocation,
  GPU submission, out-of-crate `wasm-bindgen`, or undeclared tolerance change
  was found.
