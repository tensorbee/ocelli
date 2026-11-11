# F-237 review, pass 2

**Reviewed**: `39d7b9ac..c32231f1` on `work/f-237-xai`, five commits (af9f69eb, 0ce0bcb7, 3e816f82, efe8128b, c32231f1). The worktree was clean apart from untracked `.venv` and `tools/oracle/out`, which are not part of the review. Only this review file was written in the worktree.
**Author / reviewer**: remediation and self-review (`.claude/scratch/F-237-self-review-2.md`, `-3.md`) by GLM in opencode. Reviewer is Claude, independent of GLM. The reviewer did not author any F-237 material.
**Result**: 3 defects, 1 smell, 4 nitpicks

F-237 pass 2: 3 defects, 1 smell, 4 nitpicks.

## Acceptance challenged

- **Pass 1 D1, CI provisions the toolchain.** The `frontend` job now installs `dtolnay/rust-toolchain@1.97.1` with `wasm32-unknown-unknown`, `Swatinem/rust-cache@v2` and `jetli/wasm-pack-action@v0.4.0`, the same three steps as the `wasm` job. `ci_floor_check.py` exits 0. Reproducing the job's steps in fresh clones (`npm ci`, `gate lint`, `npm run typecheck`, `gate packages`) with wasm-pack hidden and then with it on PATH: both runs fail at `npm run build`, before any pack, because the example viewer cannot load `crates/ocelli-wasm/pkg/ocelli_wasm.js`. That failure reproduces on `sprint/s46` itself (aba1cb09) and not on `main` (7099f0c0), so it is the sprint regression the lead already ruled "Not F-237's". With `bin/ocelli.sh wasm` run first and wasm-pack on PATH, the same fresh clone passes `gate packages` with `OK: 3 package(s) at 0.1.0`. So the provisioning is what the gate needs, and the job is still not shown green (see D3 and the handoff).
- **Pass 1 D2, `/release` publishes `@ocelli/wasm` first.** Closed. Preflight 8 and publish step 5 name the package, the script and the order, `@ocelli/wasm` then `@ocelli/core` then `@ocelli/react`. `sync_agent_skills.py --check` exits 0, and both adapters carry the SHA-256 of the canonical file, `c839a130...`, which `shasum` reproduces. The old-rule grep (`embeds the wasm`, `present in @ocelli/core`) is empty outside plans, reviews and the HLD. When the comparison runs is a new problem (D2).
- **Pass 1 D3, cold build compared with what is published.** The script itself is correct. End to end on HEAD it built cold in `/var/folders/.../T`, packed, compared and copied a `.tgz` whose module digests are `9d2c7927...` and `07cf6db9...`, the same pair pass 1 measured. `npm publish --dry-run <that file>` run from inside `packages/wasm` left `dist/build-id.json`'s mtime unchanged (no prepack), reported `public access`, and its shasum `4a8afef9...` equals the file's. Refusals: HEAD~1 (wrong SHA) exit 1 with only the SHA refusal, `v9.9.9` exit 1, a committed stage that writes `"a".repeat(64)` for the render digest exit 1 on the digest, a committed render/decode swap exit 1 with four refusals. No refused run left a file in its `--out`.
- **Pass 1 S1, dirty tree refused.** Closed as ruled. An unstaged README edit makes the release script refuse ("dirty is True"), and a staged one makes `gate packages` refuse with the same reason. That second observation is D1.
- **Pass 1 S2.** Partly closed. The deletion the probe performs goes red, and three other ways to unwire the call do not (S1 below).
- **Pass 1 S3.** Closed. A committed swap of the two copies in `stage.mjs` turned the real `gate packages` red with `Error: render module: apply_commands` from the smoke and TS2339 plus TS2578 from `check.ts` under both resolutions. Both catalogue probes are red under their mutation and green on their control.
- **Pass 1 D4, D5, N1, N2.** Closed. The decode module's `.d.ts` exports `install_panic_hook`, `ocelli_version`, `panic_record_len`, `panic_record_ptr` and the glue's `initSync` and default, and no `apply_commands`. The root `.d.ts` has 26 exported functions including `apply_commands`. `session` is `#[cfg(feature = "render-worker")]` in `lib.rs:45`, as the README and the `RENDER_ONLY_EXPORT` comment say. The prepack sentences now name the ignored-scripts case, the ordering reason no longer claims the module is built from the published crates, and the LLD says the smoke reads decode's bytes the same way and initialises it directly.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `python3 scripts/ci_floor_check.py` | worktree at c32231f1 | now | 0, all 28 floor gates invoked |
| same, frontend `wasm-pack-action` step deleted | scratch clone `m2` | now | 0. Nothing asserts the provisioning (N3) |
| CI emulation, wasm-pack hidden, fresh clone | `ci-nowp` at c32231f1, own target dir | now | npm ci 0, lint 0, typecheck 0, `gate packages` 1 at `FAIL: npm run build` |
| CI emulation, action's wasm-pack dir on PATH, fresh clone | `ci-wp` | now | same failure at `npm run build` |
| `bin/ocelli.sh wasm`, then `gate packages`, same PATH | `ci-wp` | now | 0 and 0, `OK: 3 package(s) at 0.1.0` |
| `npm run build` in a fresh clone | `main` 7099f0c0 and `sprint/s46` aba1cb09 | now | main 0, sprint/s46 1 with the same vite `Could not load .../pkg/ocelli_wasm.js` |
| `release_wasm.py HEAD --out ...` | clone `m` at c32231f1, clean | now | 0 in 65 s, `publish this exact file` printed |
| `npm publish --dry-run <verified .tgz>` from `packages/wasm` | clone `ci-wp` | now | 0, no prepack, `public access`, shasum equals the file |
| `release_wasm.py HEAD~1` | `m` | now | 1, SHA refusal only, no output file |
| `release_wasm.py v9.9.9` | `m` | now | 1, cannot resolve |
| unstaged README edit, `release_wasm.py HEAD` | `m` | now | 1, "does not record a clean tree" |
| staged README edit, `gate packages` | `m` | now | 1, "@ocelli/wasm build id does not record a clean tree" |
| clean tree, `gate packages` | `m` | now | 0 |
| committed stage swap, `gate packages` | `m2` d537a7eb (scratch commit) | now | 1, smoke throw plus TS2339 and TS2578 |
| committed stage swap, `release_wasm.py HEAD` | `m2` d537a7eb | now | 1, two build id digest refusals and two tarball byte refusals |
| committed tampered render digest, `release_wasm.py HEAD` and `gate packages` | `m2` d16025d3 (scratch commit) | now | 1 and 1, digest refusals |
| `_IDENTITY_CALL_IN_MAIN` over three mutated `main()` bodies | `m2` | now | 0 for each, see S1 |
| `test_release_wasm.py` under four mutations of `release_wasm.py` | `m2` | now | SHA, tarball bytes and build id digest mutations red. `is not False` to `is True` on dirty stays OK (N2) |
| `guard_probe.py --only` the six new or touched `packages.*` probes | worktree | now | all red for their reason, 4 controls green |
| `guard_probe.py --profile deep` on the two `lint-policy.clean-*` probes | `m`, then `m2` with fragments set back to "2" | now | green, then red "nothing in the output contains '2 ...'" |
| `bin/ocelli.sh gate guards` | `m` at c32231f1 | now | 0 |
| `test_release_wasm.py`, `test_guard_catalogue.py`, `sync_agent_skills.py --check`, `prose_check.py` | worktree | now | OK, OK, 0, 0 |
| GLM cycle 3 full check list | c32231f1 | prior receipt, not re-run | recorded green |

Environment: macOS arm64, rustc 1.97.1, wasm-pack 0.15.0, node v24.13.0, npm from fnm. Scratch clones lived under the session scratchpad `f237-p2` with their own `CARGO_TARGET_DIR`s and were deleted with `shutil.rmtree`. The two scratch commits existed only in scratch clones.

## Defects

### D1, a staged tree can no longer pass `gate packages`, so `/verify` cannot record

**Where**: `packages/wasm/scripts/stage.mjs:63-68`, `scripts/package_check.py:261-266`
**What**: The stage records `dirty: true` whenever `git status --porcelain --untracked-files=no` is non-empty, which includes staged changes, and the gate refuses that id. `/verify` runs its profile on the final staged state before the commit exists (`.claude/commands/verify.md` "Final-state attempt", `.claude/commands/complete-feature.md:48`), and the feature profile is `--floor` plus corpus, which includes `packages`. So every story's `/verify` now fails `gate packages`, whatever it changed. A one-line README edit is enough.
**Why it is wrong**: the commit-msg hook writes the provenance trailer from a receipt keyed on `git write-tree`. A gate that cannot pass for any uncommitted index blocks every `/complete-feature`, including F-237's own. The S1 ruling meant to keep a dirty build out of a release, not to fail the pre-commit verification of a clean index.
**Evidence**: clone `m`, clean tree `gate packages` exit 0. `echo x >> README.md` and `git add README.md`, `gate packages` exit 1 with "@ocelli/wasm build id does not record a clean tree: dirty is True".
**Scope and regression**: present on HEAD, introduced by af9f69eb. GLM's commits carry no `Ocelli-Verify` trailer and its green gate runs were all on committed trees, which is why it never showed. This follows the S1 ruling literally, so it needs a lead re-ruling. One shape: record `git write-tree` beside the SHA and refuse only unstaged tracked changes in the gate, keeping the full clean-HEAD refusal in `release_wasm.py`. Regression: a probe or test that stages a change and requires the gate's identity check to accept it.

### D2, the cold-build comparison runs after the tag push and the crates.io publish

**Where**: `.claude/commands/release.md:43-49` (precondition 8) and `:79-89` (publish step 5), `docs/RELEASE.md:99-117` (item 6)
**What**: The Preconditions section opens "Refuse before any tag or push if one fails". Precondition 8 now defers its one refusing check, `release_wasm.py vX.Y.Z`, to publish step 5, "once the tag exists". Step 5 comes after step 1 (branch push), step 3 (tag push) and step 4 (crates.io, which has no unpublish). A refusal there, for example a toolchain on which the build stopped being reproducible, leaves the crates published at X.Y.Z, the tag pushed and nothing on npm.
**Why it is wrong**: lead decision Q2 says "/release refuses unless ...", and the procedure's own rule is that a precondition refuses before any tag or push. The script does not need the tag. Its `revision` argument is documented as "the release tag, or the commit it will be created at", and `release_wasm.py HEAD` ran green end to end in this pass.
**Evidence**: reading release.md's step order, plus the HEAD run in Evidence.
**Scope and regression**: release path, present on HEAD. Smallest fix: precondition 8 runs `python3 scripts/release_wasm.py HEAD` and keeps the verified `.tgz`, and step 5 publishes that file (re-running against the tag is optional, since step 2 tags that exact HEAD). The same sentence moves in RELEASE.md item 6 and the release-policy LLD. Regression: the AC1-style grep that precondition 8 names `release_wasm.py HEAD`.

### D3, the CI comment reports a failure that did not happen and misplaces where the job fails

**Where**: `.github/workflows/ci.yml:313-318`
**What**: "the first pack after integration turned this job red with 'wasm-pack is not installed' instead of skipping". F-237 has not been integrated and no CI run of this branch exists (`gh run list -b sprint/s46 -w CI` is empty). Pass 1 predicted the failure from a local run. In a fresh CI-shaped clone of HEAD the job fails at `npm run build`, before any pack, with or without wasm-pack.
**Why it is wrong**: a false sentence in prose, here a tracked comment that the next reader of a red CI job will trust.
**Evidence**: the two CI emulation rows in Evidence.
**Scope and regression**: comment only. Fix: "Without them the gate fails closed with 'wasm-pack is not installed'", which `bin/ocelli.sh` does print.

## Smells

### S1, the call-in-main probe is a substring pin that three unwirings pass

**Where**: `scripts/guards/catalogue.py` `_IDENTITY_CALL_IN_MAIN`, probe `packages.build-id-call-in-main`
**What**: The read passes whenever the text `wasm_identity_from_tarball` appears anywhere after `def main() -> int:`. Each of these leaves it at exit 0 and `gate packages` green on a correct tree: the two lines commented out, the call kept with its result discarded (`problems +=` removed), and the branch changed to `if name == MODULE_PACKAGE + 'x':`.
**Failure mechanism**: pass 1's S2 named "a refactor that drops or mis-indents the call". Only the exact deletion the probe performs goes red. A mis-gated or discarded call removes AC3 from the gate with every check green, which is checklist item 4.
**Fix**: make the probe behavioural. Run `package_check.main()` with `run`, `tarball_paths` and `check_consumer` stubbed so the `wasm` tarball is a synthetic one with a stale build id, and require the stale refusal in the output. Then the three mutations above, and the deletion, all go red.

## Nitpicks

- N1, `.github/workflows/ci.yml:309-311`: the pre-existing comment "`gate packages` runs scripts/package_check.py ... pinning it" now sits below `setup-python` and above the rust toolchain step it does not describe. It belongs above `setup-python`.
- N2, the `packages.build-id-dirty` note says "A build id with no dirty field takes the same refusal, which fails closed". True today in both `package_check.py` and `release_wasm.py`, and guarded in neither: changing `is not False` to `is True` leaves the dirty probe red and `test_release_wasm.py` OK. One input with the field absent would pin it.
- N3, `ci_floor_check.py` stays at exit 0 with the frontend job's `wasm-pack-action` step deleted. The ruling only asked it to stay green, so this is not a defect, but nothing keeps D1's provisioning in place.
- N4, the two `lint-policy.clean-*` fragments duplicate F-232's d514da30 on `sprint/s46`, which made the same "2" to "3" change with different note text. Integration will conflict mechanically, and the `sprint/s46` text should win.

## Rulings on edits outside the widened write set

- **`scripts/tests/test_guard_catalogue.py`**, `test_release_wasm.py` added to the expected suite list. Correct and needed. `test_the_guards_gate_names_its_python_suites_exactly` pins the `guards` arm's suites exactly, and the arm (in `bin/ocelli.sh`, which the design lists) now runs the new suite, so without this line the catalogue test is red. Wiring the release tests into the floor `guards` gate is the right home, because the `release-wasm` guard has no sandbox probes.
- **The two `lint-policy` count fragments, "2" to "3"**. Correct and needed. `held.rs`'s `#[path = "schedule.rs"]` came from F-216's ae7c9c9e, which is in this branch's base, so the tree follows two path modules and the planted one is the third. With the fragments set back to "2" both deep probes go red for "nothing in the output contains '2 ...'". Not F-237's scope, and see N4 for the merge.

## Verified clean

- The release script's three facts (SHA, clean flag, recorded digests) and the fourth (the tarball's own bytes) each refuse through the real script on real builds, and the refused runs leave no file to publish. The verified file is the one `npm publish` would upload, by shasum, and publishing a file runs no prepack.
- Byte reproducibility held again: the cold builds in `/var/folders`, in two scratch clones and in two CI-shaped clones all produced `9d2c7927...` and `07cf6db9...` for the render and decode modules.
- The swap is refused by the real consumer install, by both the runtime and type halves, as the packages limit's layering sentence says.
- The ten build id probes, two surface probes and one notice probe the LLD counts are all level 1 in `guard_probe.py --list`, matching "Ten level-1 probes ... eight over the parsed id and two over its wiring".
- `gate guards` is green on c32231f1, and the probe budget carries `packages: 49` and `release-wasm: 15`.
- README:170-175, README:452-457 and use.html:238 match the build id the stage writes. more.html names no distribution route and is not stale.
- No em-dash, en-dash or prose semicolon in the range (`prose_check.py` 0).

## Limitations and handoff

- No GitHub Actions run of this branch exists. CI was emulated locally on macOS, not on ubuntu-latest, and the `jetli/wasm-pack-action` install was emulated by a directory holding wasm-pack 0.15.0. The action installs `latest` by default, as the `wasm` job already does.
- The `frontend` job is red on `sprint/s46` without F-237, at `npm run build`, because the example viewer needs `crates/ocelli-wasm/pkg` and no step builds it. Candidates by path history are F-230's 68d98792 and F-217's 02c08048, unconfirmed. With wasm-pack now in that job, a `bin/ocelli.sh wasm` step before `gate packages` made the emulated job green. That needs a lead owner before integration, since F-237's CI change cannot be shown to work until it lands.
- Untracked files are outside the clean flag by design (`--untracked-files=no`). An untracked input that changes the bytes is caught only by the release script's cold build, not by the packages gate.
- No publish and no registry write took place. `npm publish --dry-run` was run once on the verified file.
- D1 and D2 need lead rulings. D3, S1 and the nitpicks are GLM's to remediate. Pass 3 must review the remediation.

## Self-review catch figure

Of the 4 blocking findings, GLM's recorded checks should have caught 3:

- D1: its own stage comment says "A staged-but-uncommitted change is dirty", and cycle 3 recorded `git status` empty on the commit it verified, with no `/verify` on a staged tree.
- D2: cycle 3 read "release.md preconditions 7 to 9 and publish steps 4 to 6 in full", which puts the deferred check after steps 3 and 4 on the page it read.
- S1: cycle 3 considered a call-site pin for the surface check and rejected it as "reformat-fragile", the same property that makes the existing identity pin weak, without mutating the one it kept.

D3 was out of reach of its recorded checks, which never emulated the CI job. Of the 4 nitpicks, N2 (a claim GLM wrote in the note, refuted by one mutation) and N4 (an edit it made knowingly) were within reach, so 5 of 8 findings overall.
