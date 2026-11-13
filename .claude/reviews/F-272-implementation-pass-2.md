# F-272 implementation microscope, pass 2

Reviewer: Codex, independent of GLM semantic authorship and GLM own filesystem and lifecycle microscopes. Reviewed Rust SHA256 `908fc03604fee95b673bdbbdce0c8d0f5fea4a587301c7baa5e240574b287267`, plan `dd7b178c7fa5f8af2b2277499fc6bc9d631031cd21ef41f0e2802248893a229e`, and LLD `dbdff6e1cbe7dbecfe749810332aafa94a3644ab4a64cc4ad7892032d2d8fba7` against worker base `1d627bd93f0c0e3b44231be9a0f3e76607a8eeb9`.

Result: 1 defect, 0 smells, 0 nitpicks. Runtime repairs from pass 1 are closed. Completion remains blocked on the narrow documentation correction and final feature verification.

## Defect

D1. `docs/lld/oracle.md:1310` claims producer tests use unique, per-invocation, guard-owned temporary directories cleaned on finish and unwind. Unchanged consumer `tools/oracle/src/produce_current.rs:2801` instead returns a label-and-PID `PathBuf`, removes a pre-existing path and creates it again. Its producer tests are outside F-272's selected `produce::tests::` namespace. The broad claim therefore exceeds both the implementation and guard coverage. Narrow the sentence to producer-writing tests in `produce.rs`. Do not expand this story into changing the other producer. Source inspection of the named helper is the discriminating check, no new runtime test is needed for this wording repair.

## Verified repairs and observations

Pass 1 D1 is repaired by the declared immediate real dependency-cache directory exception. Nested names, locks, references and outputs remain protected. A file or symlink occupying the cache entry is not silently excluded. The final actual guard passes with an ordinary real cache directory and relative `.bin` symlink. This is representative synthetic installed topology, not a new npm installation observation.

D2 is repaired by independent workers returning owners and actual disposition bytes, with both joins collected before outcome inspection. Parent-held owners overlap through byte validation and cleanup is checked after drop. The real caught one-sided failure does not leave another worker waiting on a barrier. This proves overlapping ownership, not simultaneous producer instruction execution.

D3 now derives exclusions from actual full and selected inventories of the same binary. The final guard records 26 selected and 104 excluded. The actual child passes 26 tests with 104 filtered out. S1 is repaired by returning the owner directly from successful exclusive creation. N1's false libtest attribution and redundant branch are removed.

The author ran six separately bound disposable mutants. All compiled and exited 101 at intended runtime assertions: overbroad cache exclusion, disabled failure injection, false saved bytes, zero excluded count, disabled cleanup and restored historical checkout writer. In the historical writer case, the child passed all 26 selected tests before the parent digest assertion failed. Distinct sources, targets and binary hashes are retained under `/private/tmp/ocelli-s46-codex/f272-resume-20261110`. These are executed controls, not hypothetical mutations.

Root independently replayed the exact final compiled guard with a pre-existing synthetic `compare-out/dispositions.json`, a protected `out` sentinel and the standard cache topology. Direct exit 0 in 0.185 seconds, child 26 passed, parent guard 1 passed, no ignored tests. The dispositions SHA256 stayed `1b6a52267a5485ff5fab931c7ce7d1f667786e61a55c31251788dc0abd6ba585`. Binary SHA256 `3519210ab92f86d30946ec0570175b09042409e3b4f6c77370c0071923f137fc` and exact Rust source were checked before execution. Evidence is `root-existing-dispositions-guard.json` and its log in that directory. No canonical output or study data was touched.

The final actual applicable-success producer observation exited 0 on seven synthetic rows and six existing rendered views. Its dispositions digest matches the preserved baseline, `610db5f52ae64b46042bbcb53b7a9edcbf03bbfec31e5a174f4114f29cf63914`. Root inspected the disposable observer diff and direct command receipt. Between that source and current Rust, only the observer removal and a duplicate article correction differ. No new render, whole-corpus comparison or clinical parity is claimed. Exact final Rust clippy exited 0. None of these focused checks substitutes for the final staged feature profile.

## Scope and handoff

Root read all changed test plumbing and the unchanged producer consumers, including `produce_current.rs`, which exposed D1. Production code before the `produce.rs` test module is unchanged. README, Use and More introduce no new completion claim for this maintenance story. GLM own reports are preserved with factual qualifications, including incorrect regular-file wording and pending checks that subsequently completed. They are not independent approval.

The final-state digest does not detect a transient write followed by restoration. No new gate, profile, tolerance, production API or pixel arithmetic is introduced. GLM owns the one-sentence repair. Root will review that repair before authorizing staged final feature verification and a local prepared commit. No S46 integration, publication or sprint closure.
