# F-268 policy microscope, pass 1

Reviewer: Codex, independent of GLM implementation and own microscope.

Result: 0 defects, 0 smells, 1 nitpick. Policy slice approved for its separate local checkpoint. This is not full feature verification, canonical integration or push readiness.

## Reviewed identity and scope

Reviewed staged tree `bfac9e776a25e8c6e8e1bf5c758cd43efffcb091` on doc-comment checkpoint `2c4306da43d3a182675aa50505bef0541ccc75ec`. Exact staged patch SHA-256 is `c1c8c8e48f45faeb2ef41db5aa77754a5be5fd5a296c36da38b6dd812599ff6f`. Eleven policy paths include the canonical workflow/verify command, generated adapters, native CI step, gate, catalogue, budget, generated runbook, unit tests and build-target LLD. The existing unstaged backlog status is outside this slice.

The docs gate is additive to the floor and explicitly invoked in the native CI job. It unsets encoded rustdoc flags only for the Cargo child and supplies both denials with --workspace, --no-deps, --locked and --keep-going. It preserves the caller environment and target directory. It introduces no GPU, corpus, browser, all-feature or all-cfg requirement. Existing floor exclusions remain unchanged.

The two-member synthetic fixture puts the subject outside default-members. Each invocation owns a fresh temporary target, checks actual generated docs on success, and cleans through the context manager. Private and broken cases name distinct actual rustdoc targets. Native Cargo probes, rather than a fake Cargo transport, exercise the gate. The mock no-op unit test tests only the harness witness refusal.

## Independent verification and evidence limits

Root directly replayed the two permanent probes on the reviewed worker: command exit 0, both expected refusals, one real healthy control, approximately 4 seconds. Root inspected Sandbox.run's environment merge and the full gate/catalogue/test diff. Root independently normalized generated runbook ordinals: no old numbered row was removed or altered, and exactly the two named docs rows were added. Budget changes are catalogue sites 90 to 92 and declared gates 36 to 37, with other fields preserved.

The author evidence reports a fresh full-workspace docs gate exit 0 in 24.964565917 seconds, 67 catalogue unit tests, CI reachability, adapter sync and census checks. These are attributed author runs, not additional root reruns. Root inspected the actual three-child receipt: healthy 0, private 1, broken 1, distinct fresh targets removed, caller environment unchanged. Root inspected all three disposable mutant receipts: retaining encoded allows, dropping workspace scope, and a no-op gate each made the harness exit 1. Source hashes were restored and unchanged. Their overlapping launch scheduling is recorded and supports no timing claim.

GLM's own 0D/2S/1N report is retained. Its first smell describes a hypothetical future movement of assertions, while current TemporaryDirectory cleanup already holds on exceptions. The second requests sharing currently matching allow literals. Neither demonstrates a current contract failure or reduces current cases enough to require abstraction. Its claim that the two new unit tests cover nonzero Cargo returns is incorrect and explicitly corrected in the controller qualification. Actual nonzero Cargo routes have separate permanent probes.

## Nitpick and next step

The long explanatory sentences in verify and catalogue could be wrapped for readability. No semantic change is required. Keep F-266's overlapping catalogue, budget and runbook changes for a later reviewed three-way integration. A separate policy checkpoint may now be made under the authorized local workflow after staging this review and running affected checks. Complete-feature metadata, final union review and the full feature verification profile remain owed before prepared status. No full receipt is implied by this checkpoint.
