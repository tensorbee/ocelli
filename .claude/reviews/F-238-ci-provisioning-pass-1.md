# F238 locked cross-target provisioning review

**Scope:** the six-line Rust-job provisioning addition in .github/workflows/ci.yml and six-line F238 plan append, against exact c27c2d310dffabf37ba47be7677ac7d06a5d4481. Author is root Codex. Reviewer is the separate Codex participant f225_review, who authored neither changed file. This is private and read-only, with no Git/index, source, runtime or hardware change.

**Result:** 0 defects, 0 smells, 0 nitpicks.

The new `cargo fetch --locked` runs in the existing Rust job at the repository root immediately before Cross-target proof. It precedes the actual consumer, `scripts/shipped_native_check.py`, whose command remains `cargo tree --locked --offline -p ocelli-native` with each explicit Apple target. Host test compilation did not provide the complete cold Apple dependency cache in the observed run. Fetching the locked workspace dependencies supplies those archives before the offline resolve rather than removing its offline requirement.

The [official Cargo fetch documentation](https://doc.rust-lang.org/cargo/commands/cargo-fetch.html) confirms that omitting a target filter fetches dependencies for all architectures. It also confirms that --locked fails if the lockfile is missing or resolution would change. This invocation has no --target filter or working-directory override. The repository has no .cargo directory, and the workflow declares no CARGO_BUILD_TARGET or build.target override. The existing rust-toolchain target input installs wasm support, not a filter on this fetch command. No hidden repository/job target selector was found. No user credential/config store was inspected.

The step has no conditional skip, continue-on-error, swallowed pipeline status or fallback. An unavailable download still fails CI. Vendored decoder tests remain their earlier locked/offline step, and native proof remains the same subsequent command. `bin/ocelli.sh`, `scripts/shipped_native_check.py` and `Cargo.lock` have no diff. No graph assertion, executable selection, tier report check, notice rule, tolerance, backend feature or gate was weakened.

The plan append accurately records run42607216734/job129126141139: workspace tests passed, then native Step5 failed because both Apple graph resolves could not download locked block2 v0.6.2. Native Step6 did not execute. That is a provisioning failure, not an observed Linux runtime pass or proof that objc2 appeared in a successfully resolved shipped graph. Earlier failed runs remain distinct.

Root reports local fetch, graph/refusal tests and CI-floor parser checks green. This review does not reclassify a warm local fetch as a cold Linux observation. The concrete new tree still requires normal final-state verification and a newly authorized exact-source CI qualification. Its actual Linux Step6 must be observed before completion. The successful command and its cold-cache behavior remain execution work, not proved by this source review alone.
