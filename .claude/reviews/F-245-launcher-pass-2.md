# F-245 launcher review, pass 2

**Author / reviewer**: GLM launcher and tests plus completed own microscope, independent Codex root review.
**Identity**: launcher-root-review-manifest.json binds product SHA256 7dc1a22530da8a8d8d7867ed366d67a307e988b50eb97bc3ea3df403a15aa4e0 and test SHA256 65825cff608966aacb0de1984ed009239de53827f7ebf757bebc1607f4b1823e.
**Result**: 1 defect, 0 smells, 1 nonblocking naming nit. This is a partial launcher review, not full feature approval.

## D1, Cargo pipe inheritance prevents normal cleanup from starting

startOwnedPhase waits for stopCalled or closedPromise. The latter resolves only on ChildProcess close. Cargo has piped stdout, unlike the inherited stdio used for npm and the backend. If Cargo exits normally while a descendant retains that pipe, Node emits exit but delays close until the descendant releases stdout. Normal group cleanup is started only after close, so it never starts for that still-live group.

Root reproduced the actual consumer combination with an isolated fake Cargo process. It starts a descendant inheriting stdout and then exits successfully after emitting its compiler artifact. The descendant ignores INT and TERM and retains the pipe. The control first confirms the Cargo parent has actually exited, waits beyond the four-second cleanup bound, then fails because the descendant is still alive. This is one actual selected test and an intended assertion failure, not compilation, absent implementation, an external watchdog or a zero-test run. The backend never advances. The actual reviewed product hash is unchanged.

Exact source, direct exit1, complete output and source identity are /private/tmp/ocelli-s46-codex/f245-root-launcher-review/cargo-pipe.test.mjs, cargo-pipe-before.json and cargo-pipe-before.log. The test kills only its own recorded fixture launcher, Cargo group and descendants during teardown. No user viewer process or port is involved.

Have GLM add the permanent Cargo stdout-inheritance control, then trigger normal group cleanup from direct-process exit rather than requiring stdio close first. Preserve a bounded close and pipe-drain confirmation after group shutdown, the failed-spawn path, first-signal exit130/143, non-extending deadlines and no next phase while stopping. Keep group signaling restricted to successfully spawned owned detached positive PIDs. The repair must not turn every running phase into a timed process or treat an error as process close. Run own review and return exact source for root replay.

## Own review disposition

The two raw own defects assume a JS signal callback can interrupt synchronous code between phase creation and current assignment or between handler registration and main. No await or event-loop yield exists at either interval. Those are not established counterexamples in the actual Node execution model. Duplication of caller handling, timer names and polling commentary are preferences, not new smells. The owned-group helper remains subject to D1 through its actual Cargo consumer.

Four author-run synthetic controls and the direct-child and skip-normal-cleanup semantic mutants remain useful at their actual bound identities. They do not cover this pipe-inheritance combination. This review neither relabels them as root runs nor discards them.

## N1 and remaining acceptance

The normal-exit test title says TERM reaches an orphaned descendant, but its body first checks that descendant is gone and later signals the viewer. Adopt the GLM-authored factual title correction after the source repair. This naming nit does not affect the reproduced defect.

Actual npm/Cargo/browser execution, Cargo interruption, early failures, missing/malformed/duplicate artifact refusals, complete cleanup inventory, unsupported-host behavior and permanent test registration remain separate required acceptance work. Passing the four current fixtures is not full launcher or feature completion.
