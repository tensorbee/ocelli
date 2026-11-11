# F-272, oracle producer tests write nothing into the checkout

**Status**: draft
**Epic ref**: E2.8
**Sprint**: S47
**Estimate**: 1w

Filed at the S46 close from the recovered close-backlog draft, original item
8. The estimate is inherited and is provisional, lead-suggested and not
capacity-approved. This draft claims no implementation, no test run and no
completion, and it touched no output directory.

## Normative source, transcribed

No HLD section names the producer's output paths. A search of
`docs/hld/08-validation-architecture.md` finds no test output directory. The
evidence is the review and the source, quoted.

`.claude/reviews/S46-oracle-test-fix-pass-1.md`, N1: "in a clean tree,
`cargo test` creates `tools/oracle/compare-out/dispositions.json`, and it was
observed. The path is gitignored (`.gitignore:62`), so git is unaffected. But
a tree with a real `compare-out/dispositions.json` and no reference has it
overwritten with a `reference-run-absent` record." And the fix it names:
"point `reference_subset`, `candidate_run` and `dispositions` at a
per-process `temp_dir()` path, as `a_missing_reference_run_is_unrun` does."

The pattern, `tools/oracle/src/produce.rs:2286` at 31323a79:

```rust
let dir =
    std::env::temp_dir().join(format!("ocelli-produce-missing-{}", std::process::id()));
```

## What the specification does not cover

Whether the per-process directory is removed after the run, and whether the
guard watches only `tools/oracle/compare-out` or the whole `tools/oracle`
tree. Both are design decisions.

## Problem and evidence

At 31323a79 three places pass checkout paths to the producer on the absent
branch, read and verified:

- `applicable_monochrome_rows_reach_the_comparator`, produce.rs:2310, absent
  branch at 2316 to 2324
- `uniform_slice_edge_matches_the_reference_within_one_lsb`, produce.rs:3134,
  absent branch at 3138 to 3148
- `rendered_pair`, produce.rs:3211, absent branch at 3217 to 3221, shared by
  the tests that call it

Each one runs `produce_applicable` with `reference_run` at
`tools/oracle/missing-run`, so `produce_with` takes its early return at 309
to 312 and calls `write_dispositions`, produce.rs:2145, on
`tools/oracle/compare-out/dispositions.json`. The reference-run-absent return
is not the only writer: the corpus-absent branch at 344 to 348 and the
output-directory-refused branch at 322 to 328 write dispositions too, so a
tree with a real reference and no corpus has its dispositions overwritten as
well. `.gitignore:62` ignores the directory, which is why nothing red ever
showed. The review observed the overwrite in the `ocelli-wt-f-223` worktree,
and the canonical worktree's dispositions file carried an mtime beside an
older `compare.json`, which fits a test run rewriting it. That fit was not
confirmed as the cause and is not claimed here.

## Scope, API and compatibility

In scope: every produce.rs test that runs the producer when the reference run
or the corpus is absent points its reference subset, candidate run and
dispositions file at a per-process temporary root with a unique directory for
each test invocation, and a test fails if a test run changes
`tools/oracle/compare-out`. A PID alone cannot distinguish parallel tests or
repeated `rendered_pair` invocations in the same process.

Non-goals: the producer's real output paths, the dispositions format, the
corpus, the oracle harness itself, and any gate.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| The three absent-branch test sites | `produce_applicable` with temp paths | Unique per-test/per-invocation directory under a per-process root | The same summaries, no checkout writes |
| The guard test | A before-and-after digest of `tools/oracle/compare-out` | Around the suite | Red on any change |
| Real oracle runs | Unchanged `ProducePaths` | Unchanged | Unchanged artefacts |

## State transitions and invariants

Not applicable. Test plumbing changes no runtime state. The one invariant is
the evidence boundary: a test run never replaces a real comparison artefact.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Stage / owner |
|----|-------------------------------|---------------------|-------------------------------|---------------|
| AC1 | The three sites use temporary roots and unique per-test/per-invocation directories, including concurrent and repeated rendered_pair calls within one PID | Diff review | One site missed, or PID-only names shared by parallel tests | implementation |
| AC2 | On a clean tree, `cargo test -p ocelli-oracle` creates nothing under `tools/oracle/compare-out` | A before-and-after listing at the story tip | A fix that only renames the checkout path | implementation |
| AC3 | A tree with a real `compare-out/dispositions.json` and no reference keeps that file byte-identical through the run | The digest before and after | The overwrite N1 observed | implementation |
| AC4 | The corpus-absent branch is covered too. A tree with a reference and no corpus keeps its dispositions | The same digest method | Fixing only the reference-absent path | implementation |
| AC5 | The guard test fails when a planted writer touches `compare-out` during a test run | The planted-failure run recorded | A guard that cannot fail | implementation, reviewer |
| AC6 | The producer's real run path is unchanged. A real reference run and corpus produce the same dispositions as before | The comparator's own tests | A path change that breaks the real harness | implementation |
| AC7 | The summaries are unchanged. Each site still asserts `reference-run-absent` with its counts | The tests' own assertions | A fix that weakens an assertion | implementation |
| AC8 | Two independently constructed concurrent fixture invocations within one process use distinct paths and preserve their own dispositions byte-for-byte, including repeated rendered_pair calls | Isolated concurrent fixture control | A process-only path shared by all tests | implementation, reviewer |

Unrun ownership. No test ran for this filing. AC2 to AC6 run at the story
tree, and the planted-failure run of AC5 is recorded in the review.

## Approach

1. Start from `a_missing_reference_run_is_unrun`, but give every test and
   helper invocation its own temporary subdirectory, not just a PID.
2. Add the guard around the suite, digesting `tools/oracle/compare-out` before
   and after through read-only observations. Absence and planted-writer
   controls run only in disposable fixture-owned source/output roots. They
   never remove or overwrite the operator's real ignored compare-out or corpus.
3. Challenge simultaneous and repeated invocations independently.
4. Leave the real `ProducePaths` and every producer function unchanged.

Rejected: gitignoring harder, deleting the directory in a build step, and
asserting on mtimes, which the digest makes unnecessary.

## Dependencies, write ownership and resources

Depends on F-X021, done in S20, as the recovered draft had. Writes
`tools/oracle/src/produce.rs` tests. If the suite-level digest guard requires a
separate process, the proposed additional test-only write is
`scripts/tests/test_oracle_output_isolation.py`, subject to design confirmation
and coordination before implementation. It does not authorize a new gate or
profile edit. The Rust file is shared with F-244's and
F-239's oracle evidence work in sequence, so the story integrates through the
usual order and coordinates so a test run never replaces a real comparison
artefact. No exclusive resource.

## Verification and evidence reuse

`cargo test -p ocelli-oracle` at the tree, the digest evidence recorded
beside it. Evidence invalidates on any new producer test that writes a
checkout path, which is what the guard exists to catch.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: n/a. Test plumbing only
- unsafe: none
- Tier A (WebGPU): n/a. The absent-branch summaries are tier C records and
  the story changes none of them
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

All three tier rows are not applicable. The story touches no rendering or
compute.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | The absent branches run against temp paths and keep their summaries | `tools/oracle/src/produce.rs` tests, AC1, AC7 |
| unit | The guard fails on a planted checkout write | AC5 |

## Parity surface covered

None.

## Deviations

None.

## LLD impact

`docs/lld/oracle.md`, one sentence on where test output goes.

## User-facing documentation impact

No change.

## Open questions

1. Whether the temp directories are removed after the run. Affects AC2's
   listing. Owner: the design round.
2. Whether the guard watches only `tools/oracle/compare-out` or the whole
   `tools/oracle` tree. Affects AC5. Owner: the design round.
3. Whether the suite-level guard needs the proposed standalone harness rather
   than Rust-only test plumbing. Affects the coordinated write set. Owner:
   the design round, before implementation. All destructive controls use
   disposable fixture roots either way.
