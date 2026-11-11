# S46 launcher fixture review, pass 1

**Reviewed:** private two-path patch `FINAL-CODEX-RECOVERY.patch`, SHA256 `1d52369ce80c8531e411e17b6229fc949222b45d58e22828242e6cbaed1dd8ce`, based on canonical `9a4456b6a027d2fb1755a2f8bcccb5d64c8853f2`. The validation workspace is `/private/tmp/ocelli-s46-example-dev-background-validation`. Final catalogue blob is `c52c26bd69a74322090645eb0a6f4aac8d5ee051`, final catalogue-test blob is `a17d5516f8a3930b668c4c4eb8c36feb7f905345`. Both match the patch's postimages.

**Author / reviewer:** the author is the separate Codex recovery participant. The GLM attempt is preserved separately and did not author this patch. Reviewer is Codex participant `f225_review`, who diagnosed the CI failure but authored neither changed file. This review approves only this fixture repair, not the reviewer's own earlier CI audit.

**Result:** 0 defects, 0 smells, 0 nitpicks.

## Consumer and semantic checks

- The production `bin/example-dev.sh` is byte-exact, SHA256 `ab2eef2dacaea3cf182708dc1e55c748070dd0d9f99f371d8fb403b22f644aef`. Only the disposable positive-control substitution gains the trailing ampersand that the real helper launch already has. The original `pid=$!`, cleanup trap, URL wait and Vite invocation are preserved.
- The target mutant remains `_example_dev_silent_helper`, with its original refusal text, shortened test wait and background silent helper. Its expected refusal is unchanged. No guard, gate, timeout, tolerance or production launch condition is weakened.
- New `control_expect` requires the existing production diagnostic `example helper http://127.0.0.1:9`. It cannot be satisfied merely by the new unit's separate completion marker. It strengthens the unmutated positive control against the observed local zero-exit masking of an earlier shell failure.
- The unit runs the actual control builder over the real launcher bytes in a disposable sandbox. It appends PID/nonempty and exact exported-URL assertions after the builder's normal substitutions, then requires a distinct marker after both assertions. Thus an early shell failure cannot masquerade as assertion coverage even if an EXIT cleanup masks its exit status.
- The mocked `box.run` calls the saved original bound method, so there is no recursive mock call. The final assertion also proves the original launcher bytes remain unchanged. No network endpoint is contacted by the URL witness.

## Existing primary execution evidence read

All receipts below are under `/private/tmp/ocelli-s46-codex/`. These commands were executed by the recovery author, not rerun by this reviewer. Their direct JSON exits and actual logs were read.

| Receipt | Direct result and semantic observation |
|---|---|
| `example-dev-recovery-probe-green` | exit 0, original silent-helper mutant red for the declared reason and positive control green |
| `example-dev-recovery-old-helper-unit-red` | exit 1, old synchronous substitution fails the required product URL observation, with the nounset diagnostic retained |
| `example-dev-recovery-old-helper-probe-red` | exit 1 despite local shell return 0, because the required product URL is absent |
| `example-dev-recovery-empty-pid-unit-red` | exit 1, actual PID assertion returns 71 and the completion witness cannot establish success |
| `example-dev-recovery-wrong-url-unit-red` | exit 1, actual exported-URL assertion returns 72 |
| `example-dev-recovery-catalogue-suite-restored` | exit 0, 59 tests pass after restoration |
| `example-dev-recovery-census` | exit 0 |

The historical receipt named `example-dev-background-old-target-red` actually exited 0 locally. Its misleading filename is not a RED claim. The separate observed dash exit 2 and the original Ubuntu failure remain distinct. The new explicit positive witness closes the locally masked result rather than discarding it.

## Scope and handoff

This is an independent source and retained-receipt review. No tests, native build, browser or hardware job ran for this review. Canonical staging and fresh affected checks remain root's work. It does not make the failed CI run green, prove the separate Rust or magnifier repair, complete F238 Linux acceptance or close S46.
