# F-272, oracle producer tests write nothing into the checkout

**Status**: approved
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

## Resolved design questions

All three review questions are settled once, here and in D1 to D3 of the Lead design decisions.

Q1, temp lifetime: every producer-writing invocation owns a uniquely named `TempDir`-style directory (PID plus process-local atomic counter, so concurrent and repeated calls in one PID never collide), removed on normal finish and on unwind. 2286 is converted to this helper like every other producer-writing site and its old PID-only path is retired. AC1, AC8.

Q2, guard scope: the permanent Rust guard hashes the actual `tools/oracle` tree before and after the child run and requires content equality plus final absence of any new entry. No watcher, writer hook, or polling, and no transient-write claim. No exclusions are declared unless a named dependency cache in the source justifies one. AC3, AC4.

Q3, selection: the guard locates the actual suite via `std::env::current_exe`, spawns that exact binary with the producer test selection followed by `--skip produce::tests::guard_no_checkout_write`, and checks child exit. No parent-global env mutation, no new gate. AC2.

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

## Scope

In scope: all producer-writing tests in `tools/oracle/src/produce.rs`, including 2286 and the 2329 success path, switch their reference subset, candidate run, and dispositions file to unique-per-invocation fixture directories owned by the helper, with cleanup on finish and unwind. `tools/oracle/Cargo.lock` is not treated as an exclusion, consistent with D2. A test fails if a run changes `tools/oracle/compare-out`. Real reference and corpus reads are preserved. The guard is permanent Rust-only plumbing in the actual-producer test binary, discovered via existing cargo tests, selecting the producer suite through `std::env::current_exe` with the producer test selection and `--skip` of its own guard name.

Non-goals: real producer output paths, dispositions format, corpus, oracle harness, any new gate, profile, or dependency, and any transient-write detection guarantee. The 3134/3211 success paths stay read-only. AC1, AC2.

## API and compatibility

The helper is a private Rust API inside `tools/oracle/src/produce.rs`. It returns a guard object owning the fixture path, removes the tree on finish and on unwind, and never reuses the retired 2286 PID-only path or any caller-supplied path. Existing public signatures, dispositions format, and real `ProducePaths` behaviour are unchanged. The guard test is additive, self-selecting through the exact producer test binary and its own `--skip`, so no new gate, profile, dependency, or policy is introduced and external callers of the oracle see no change. AC1, AC2, AC8.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Producer-writing test sites (2286, 2310/2329) | Unique-per-invocation fixture `TempDir` owned by the helper | Distinct name per invocation, 2286 converted, its PID-only path retired, real reference and corpus reads preserved | Same summaries, no checkout writes |
| Guard test `produce::tests::guard_no_checkout_write` | `std::env::current_exe` child spawn with producer test selection and `--skip` of itself, before/after tree hash | Runs in existing cargo suite, no parent-global env mutation, no exclusions declared (D2) | Child exit status plus content-equality verdict |
| 3134 and 3211 success paths | Direct render and reference read, no `ProducePaths` | Unchanged, read-only | Unchanged artefacts |
| Real oracle runs | Unchanged `ProducePaths` | Unchanged | Unchanged artefacts |

## State transitions and invariants

Not applicable to runtime, since test plumbing changes no runtime state. Invariants: first, a test run never replaces a real comparison artefact (evidence boundary, AC1). Second, the guard proves only final absence and content equality of the protected tree. A final digest is valid evidence of final content equality, and it does not establish absence of transient writes within the child run, so no transient-write claim is made (proof boundary, AC3, AC4). Third, the old checkout-path semantic RED runs only in a disposable isolated source checkout with a synthetic sentinel and is then restored to GREEN, never against operator outputs (AC5). Fourth, each private fixture with a valid reference, absent corpus, and refused output reaches its own reason and count without sharing paths (AC6). Independent design approval is recorded in `.claude/reviews/F-272-design-pass-4.md`.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Stage / owner |
|----|-------------------------------|---------------------|-------------------------------|---------------|
| AC1 | All producer-writing tests in `produce.rs` (2286, 2310/2329, and any dispositions writer) own a unique-per-invocation fixture directory, cleaned on finish and on unwind, repeated and concurrent calls in one PID never collide. 2286's PID-only path is retired. | Diff review plus the `concurrent_fixture_invocations_use_distinct_paths` test showing distinct paths per invocation | Missing a site, reusing the PID-only name, sharing a path across invocations | implementation |
| AC2 | On a clean tree, the permanent Rust guard test `produce::tests::guard_no_checkout_write` locates the suite binary via `std::env::current_exe`, first validates the selection by running `current_exe --list` filtered with the exact literal `produce::tests::` prefix plus `--skip produce::tests::guard_no_checkout_write` and fails closed unless that list confirms both `a_missing_reference_run_is_unrun` and `applicable_monochrome_rows_reach_the_comparator` are selected, with the selected and filtered counts, the filtered list and the list exit recorded before any child run, then spawns that exact binary as a child with the exact literal `produce::tests::` selection followed by `--skip produce::tests::guard_no_checkout_write`, in that order, checks the child's own exit status, and hashes the actual `tools/oracle` tree before and after, where the verdict is byte-equality plus final absence of any new entry in that tree. Real reference and corpus reads are preserved. | Guard observation at the story tree: list-run counts and filtered list with fail-closed zero-selected control, then child exit, before/after hash of the actual tree, no parent-global env mutation, a zero-selected or filter-stale list run is a permanent semantic RED control and never a pass | A guard that skips itself incorrectly, recurses, mutates global env, trusts mtime or git status, treats a final digest as proof about writes during the run, or evaluates only compare-out, plus a Python or new-gate substitute | implementation, reviewer |
| AC3 | Final content equality: the actual `tools/oracle/compare-out/dispositions.json`, when present, is byte-identical after the run. The guarantee is final absence and content equality only, and no transient-write claim is made. | Guard before/after digest of the actual tree, no raw-content copy | A guard claiming to observe writes during the child run, or a fix that rewrites real dispositions | implementation, reviewer |
| AC4 | Same final content-equality guarantee for the corpus-absent branch. | Same before/after guard digest | Fixing only one branch | implementation, reviewer |
| AC5 | In a disposable isolated source checkout with a synthetic sentinel, a child run including a planted writer touching `tools/oracle` makes the guard fail. a no-writer child run passes and the sentinel is restored GREEN. Neither control touches operator outputs. | Recorded planted-failure run and a passing no-writer run | A guard that cannot fail or that touches operator outputs | implementation, reviewer |
| AC6 | The real run path is unchanged: 2310 on its valid-success branch (2329) keeps real reference and corpus reads and yields the same dispositions before and after the change. | Actual-producer before-and-after dispositions digest pair from the real oracle producer via unchanged `ProducePaths`, recorded in review, not comparator-only proof | A path change breaking the real harness, or comparator-only digest substitute | implementation, reviewer |
| AC7 | Each of the three old absent callers forces a missing reference and still asserts its unchanged reasons and counts. independent private fixtures with a valid reference, absent corpus, and refused output reach their own reasons and counts. | Assertions at 2310, 3134, and 3211 | Weakened assertions, dropped reasons or counts | implementation |
| AC8 | Two independent concurrent fixture invocations in one process use distinct paths. each preserves its own dispositions byte-for-byte. Proven by `concurrent_fixture_invocations_use_distinct_paths`. | That test, with planted-fail and no-writer controls confined to disposable fixtures, never operator outputs | Process-only shared path, collision, or control outside disposable fixtures | implementation, reviewer |

Unrun ownership. No test ran for this filing. AC2 to AC6 and AC8 run at the story tree.

## Approach

1. Convert every producer-writing site, including 2286 and 2329, to a helper returning a guard object owning a unique `TempDir`-style fixture (PID plus process-local atomic counter), cleaned on finish and unwind, and retire the 2286 PID-only path.
2. Add the permanent guard `produce::tests::guard_no_checkout_write`. Step order is strict: list validation first, then capture the BEFORE tree snapshot, then the child run, then capture the AFTER snapshot and compare. List validation: resolve the actual test binary via `std::env::current_exe`, run it with `--list --format terse` filtered by the literal `produce::tests::` prefix plus `--skip produce::tests::guard_no_checkout_write`, fail closed unless the filtered list includes `a_missing_reference_run_is_unrun` and `applicable_monochrome_rows_reach_the_comparator`, and record the selected and filtered counts, the filtered list and the list exit status. A stale or nonmatching filter with zero selected tests is a permanent semantic RED control and never passes. Next hash the actual `tools/oracle` tree as the BEFORE snapshot. Child run: spawn that exact binary with `produce::tests:: --skip produce::tests::guard_no_checkout_write` in that order, then check the child exit status. Finally hash the actual `tools/oracle` tree again as the AFTER snapshot and compare before and after. No writer hook, no polling, no transient-write claim. Discovery via existing cargo tests, no new gate.
3. Keep 3134/3211 success paths read-only and preserve real reference and corpus reads. Prove the real producer unchanged by actual-producer before/after dispositions digests.
4. Run the old checkout-path semantic RED only in a disposable isolated source checkout with a synthetic sentinel, then restored GREEN, never on operator outputs.

Rejected: writer hooks, polling, PID-only naming, Python harness, new gate or profile, and any claim that a final digest alone proves absence of transient writes. A final digest is valid evidence of final content equality, which is the guarantee this guard makes, and it is not evidence about writes during the child run.

## Dependencies, write ownership and resources

Depends on F-X021 (S20). Writes `tools/oracle/src/produce.rs` tests only and integrates with F-244 and F-239 in sequence. No new dependency, gate, profile, or patient/corpus access change. The helper owns unique fixture directories, cleans on finish and unwind, and never reuses or deletes other paths. The guard controls only disposable fixture roots, never launches itself, never mutates parent-global env, and observes digests without copying raw contents. No exclusions are declared unless a named dependency cache in the source justifies one. The Rust file is shared with F-244's and F-239's oracle evidence work in sequence, so coordination ensures a test run never replaces a real comparison artefact. No exclusive resource.

## Verification and evidence reuse

`cargo test -p ocelli-oracle` at the story tree, with an actual-producer before-and-after dispositions digest comparison recorded beside it. The digest evidence invalidates if any new producer test writes a checkout path, which is exactly what the guard exists to catch. Evidence must come from the real producer run, because a comparator-only digest substitute is not accepted (AC6). The guard proves only final absence and content equality of the protected actual tree via before/after hashes plus child exit status. A final-state snapshot is valid evidence of that final state, and the guard makes no claim about transient writes during the child run, so no writer hook or polling exists. A planted-failure run and a passing no-writer run at the story tree are recorded in review, confined to disposable fixture roots (AC5). The old checkout-path semantic RED runs only in a disposable isolated source checkout with a synthetic sentinel, then restored GREEN, never against operator outputs. No test results are claimed here, and all runs are pending this repair.

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
| unit | All producer-writing sites, including 2286 and 2329, write only to their unique helper-owned temp paths. 2286's PID-only path is retired. real reference and corpus reads preserved | `produce.rs` tests, AC1 |
| unit | The guard selects the actual suite via `std::env::current_exe`, spawns the exact child with the literal `produce::tests::` selection then `--skip produce::tests::guard_no_checkout_write`, checks child exit, no recursion or global env mutation | `produce::tests::guard_no_checkout_write`, AC2 |
| unit | Guard before/after hash of the actual `tools/oracle` tree yields content equality and final absence of new entries. no transient-write claim | Same guard, AC3, AC4 |
| unit | Planted writer in a disposable isolated source checkout fails the guard. no-writer run passes. controls never touch operator outputs | AC5 |
| integration | `concurrent_fixture_invocations_use_distinct_paths`: two independent concurrent fixture invocations in one process use distinct paths and each keeps its dispositions byte-identical | AC1, AC8 |
| integration | Three old absent callers force a missing reference and keep reasons and counts. private fixtures with valid reference, absent corpus, refused output reach their own reasons and counts | AC7 |

All test names are proposed, none claimed to exist or to have passed.

## Parity surface covered

None.

## Deviations

None.

## LLD impact

`docs/lld/oracle.md`, one sentence on where test output goes.

## User-facing documentation impact

No change.

## Open questions

None remain that block implementation. The three review questions are settled in D1 to D3: D1 owns temp naming, uniqueness, and cleanup lifetime, D2 owns guard scope and detection as final absence plus content equality of the actual tree, and D3 owns selection via `std::env::current_exe` with the producer filter and the guard's own `--skip`. The 2286 contradiction is settled: it is converted to the helper and its PID-only path is retired. No exclusions are declared until an actual named dependency cache in the source justifies one. If implementation discovers an infeasible branch, the exact design blocker is reported then, per D2 and D3. Independent design approval is recorded in `.claude/reviews/F-272-design-pass-4.md`.

## Lead design decisions

**D1. Temporary directory ownership (settles Q1).** Every producer-writing test invocation owns a uniquely named `TempDir`-style directory, removed on finish and on unwind. Naming combines PID with a process-local atomic counter, and the counter only proposes a candidate name and never proves uniqueness. Creation uses `create_dir` in exclusive mode so the path is owned only if the create succeeded, and collisions retry with a fresh candidate without deleting or touching any existing other-owned path. `a_missing_reference_run_is_unrun` (tools/oracle/src/produce.rs:2286) is converted to this helper like every other producer-writing site, and its old PID-only path is retired, not preserved. The helper never reuses or deletes any other path. Normal finish and unwind cleanup removes only this owned path, no other path is ever removed, and no watcher, transient guarantee, prose semicolon, or em dash is introduced here. AC8.

**D2. Guard scope and detection method (settles Q2).** The guard hashes the protected actual `tools/oracle` tree before and after the child run and requires content equality plus final absence of any new entry. No exclusions are declared. If an actual named dependency cache in the source tree is later justified from the source itself, it will be named explicitly then. `mtime` and `git status` are not trusted. The guarantee is final absence and content equality only. A final-state snapshot is valid evidence of that final state, and transient write-and-restore within the run is not claimed as detectable, per AC3 and AC4.

**D3. Guard form and selection mechanism (settles Q3).** No watcher, writer hook, polling thread, or transient-write guarantee. A permanent Rust guard test inside the actual producer test binary locates the real suite via `std::env::current_exe`, first validates the selection with the binary's `--list` mode under the same literal `produce::tests::` filter and `--skip produce::tests::guard_no_checkout_write`, fails closed unless the filtered list selects `a_missing_reference_run_is_unrun` and `applicable_monochrome_rows_reach_the_comparator` with zero selected treated as a permanent semantic RED control, records the selected and filtered counts and list exit, and only then spawns that exact binary as a child with the producer test selection followed by `--skip produce::tests::guard_no_checkout_write` and checks the child's own exit status. Discovery is through existing cargo test enumeration, no new gate, profile, or Python harness. The guard mutates no parent-global environment. AC2, AC7.

**D4. Caller-by-caller expectations.** `applicable_monochrome_rows_reach_the_comparator` (2310) and its 2329 success branch, plus `a_missing_reference_run_is_unrun` (2286), redirect all producer output to the helper-owned unique fixture directory. `uniform_slice_edge_matches_the_reference_within_one_lsb` (3134) and `rendered_pair` (3211) success paths render or read directly and remain read-only. Three old absent callers force a missing reference. Independent private fixtures with a valid reference, absent corpus, and refused output each reach their own reasons and counts. Actual producer before/after dispositions are the required proof, not comparator-only output. AC1, AC3, AC5, AC6.

### Implementation clarification (2026-11-09, lead-approved)

This clarifies the dependency-cache exception already permitted in the plan. Only the immediate `tools/oracle/node_modules` directory generated by pinned `npm ci` is excluded from the guard snapshot. A directory merely named `node_modules` deeper in the tree remains protected. Files or symlinks at the excluded entry are not silently treated as a cache directory. `Cargo.lock`, `package-lock.json`, reference files, `out` and `compare-out` remain protected. This is an implementation clarification, not a general gitignore exclusion and not a new workflow exception. It applies consistently to the protected-tree scope in AC2 to AC5.

Lead-approved controller design refinement. Each successful test worker returns its owner and its actual disposition-file snapshot bytes. The full-inventory and filtered-inventory commands run as separate subprocesses, and their exits are checked and recorded. The parent retains every returned owner through validation of the actual disposition-file bytes, including both owners on the healthy path, then drops retained owners and checks cleanup. There are no worker waits, channels or timeout policy. This proves overlapping owner lifetimes, not simultaneous execution of producer instructions. Count accounting uses the dynamic full and filtered inventories from the same test binary. No measured capacity, full story pass, new oracle render or clinical parity is claimed.

## Lead amendment after independent review passes 2 and 3, 2026-11-11

No in-process snapshot can precede every in-process test, so the guard's child run uses a fresh temporary mirror root, selected through one test helper that reads an environment override. Any write by a selected test then appears as a new entry under the mirror's `tools/oracle`, whatever the parent ran first and however deterministic the bytes are.

This supersedes AC3's "no raw-content copy" and the "observes digests without copying raw contents" sentence for `tools/oracle/out` only. The mirror copies `tools/oracle/out`, because the mirror digest must see a write inside it. `corpus/data` is never copied. The mirror holds a directory symlink to the real `corpus/data`, which `open_corpus` canonicalises to the same root, so the tests read the same bytes and no copy of corpus data is ever made. The child runs with its working directory inside the mirror, so a relative path also lands in the mirror. A source scan in the test module asserts that `CARGO_MANIFEST_DIR` is read in exactly one place, the real-root helper, so a producer test cannot bypass the override. There is still no new dependency, gate or profile, and corpus access is unchanged.
