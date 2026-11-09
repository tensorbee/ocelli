# F-237 review, pass 1

**Reviewed**: `622ce1b5..e063f5e1` on `work/f-237-xai` (one commit, e063f5e1, F-237 only). F-228's commits below 622ce1b5 were not judged. The worktree was clean apart from untracked `.venv` and `tools/oracle/out`, which are not part of the review.
**Author / reviewer**: implementation and self-review by GLM in opencode. Reviewer is Claude, independent of both. The reviewer did not author any F-237 material.
**Result**: 5 defects, 3 smells, 2 nitpicks

F-237 pass 1: 5 defects, 3 smells, 2 nitpicks.

## Acceptance challenged

- **AC1** (RELEASE.md names the place). Executed the design's grep, which is empty in `docs/RELEASE.md`. Widened to the whole tree with `git grep`. The procedure `/release` actually runs, `.claude/commands/release.md`, still carries the old rule (D2).
- **AC2** (the tarball carries both modules, glue and both notices). `npm pack --dry-run -w packages/wasm` lists 16 files: both `.wasm`, both glue `.js` and `.d.ts` pairs, `THIRD_PARTY_LICENSES` and `THIRD_PARTY_NOTICES` at the root, `dist/build-id.json`, the licences, `NOTICE`, `README.md` and `package.json`. Removing `THIRD_PARTY_NOTICES` from `files` is refused (M4).
- **AC3** (build identity refused when stale). The function refusals are real: a stage that writes `HEAD~1` is refused as stale (M7) and a stage that hashes the wrong file is refused on digest (M8). Three things the probes do not see: the wiring into `main()` (S2), a dirty tree recorded as HEAD (S1), and the release-time comparison against a fresh build, which exists only as prose (D3).
- **AC4** (the gate covers the new place). `PACKAGES` includes `wasm`, `check_tarball` runs on it, and the publish dry run includes it. The gate cannot tell the render module from the decode module (S3).
- **AC5** (a consumer outside the workspace). The smoke resolves bytes through `import.meta.resolve` inside the installed package, runs `loadCore`, and compares both modules' `ocelli_version()` to `VERSION`. Removing the peer dependency, or widening it to `^0.1.0`, is refused (M5, M6).
- **Lead decision Q2, byte reproducibility.** I measured it independently (see Evidence). The claim holds on this machine.
- **Focus 4, CI.** The gate does not skip in CI. It fails, and that turns the TypeScript job red (D1).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh gate packages` | worktree at e063f5e1 | now | 0. `OK: 3 package(s) at 0.1.0`. The build id afterwards records `e063f5e1`, with digests `9d2c7927...` (render) and `07cf6db9...` (decode) |
| `bin/ocelli.sh gate wasm` | worktree | now | 0, size within budget |
| `bin/ocelli.sh gate guards` | worktree | now | 0, ALL GREEN |
| `npm run lint`, `npm run typecheck`, `python3 scripts/prose_check.py` | worktree | now | 0, 0, 0 |
| Cold build A, `bin/ocelli.sh wasm` then `wasm-decode` | `git archive HEAD` into scratch `a`, empty `CARGO_TARGET_DIR=target-a` | now | 0. render `9d2c7927...`, decode `07cf6db9...` |
| Cold build B, run in parallel with A | `git archive HEAD` into scratch `b`, empty `target-b` | now | 0. Digests identical to A |
| Warm rebuild, a no-op and then a rebuild after `touch` of two crate sources | scratch `a`, `target-a` | now | Same digests. All eight wasm-pack outputs (both `.wasm`, `.js`, `.d.ts`, `_bg.wasm.d.ts`) agree byte for byte across A, B and the worktree, so one distinct hash per file |
| Clone build in a third path (`m`), reusing `target-a` | scratch clone of the branch | now | Same digests again |
| M1, delete `problems += wasm_identity_from_tarball(...)` from `main()` | scratch clone `m` | now | `guard_probe.py` with all 18 `packages.*` ids: exit 0, all caught, 12 controls green. **Survives** |
| M2, swap the render and decode copies in `stage.mjs` | `m` | now | `package_check.py --no-publish-dry-run` exit 0, root `dist/ocelli_wasm_bg.wasm` is the decode module `07cf6db9...`. **Survives** |
| M3, uncommitted `#[wasm_bindgen]` fn added to `crates/ocelli-wasm/src/lib.rs` | `m` | now | exit 0. Build id `git_sha` is `e063f5e1`, digests are `d3471dff...` and `529b33df...`, not HEAD's. **Survives** |
| M4, drop `THIRD_PARTY_NOTICES` from `files` | `m` | now | exit 1, "carries dist/decode/ocelli_wasm_bg.wasm and has no THIRD_PARTY_NOTICES" |
| M5, peer `^0.1.0`. M6, peer removed | `m` | now | exit 1 each, peer refusal |
| M7, stage writes `rev-parse HEAD~1` | `m` | now | exit 1, "stale artefact from another tree" |
| M8, stage hashes the render module for the decode entry | `m` | now | exit 1, digest refusal |
| `npm pack` of `packages/wasm` with `wasm-pack` hidden (cargo, rustc and rustup still on PATH) | `m` | now | prepack fails. `package_check.py` prints `FAIL: npm packaging`, `@ocelli/wasm: npm pack failed`, `wasm-pack is not installed`, exit 1 |
| `npm pack --dry-run --ignore-scripts -w packages/wasm` after deleting `dist` | `m` | now | 7 files, no module and no build id |
| CI run 41951682234 (main, S45), TypeScript job | GitHub | prior | success, `OK: 2 package(s)` |

Environment: macOS arm64, rustc 1.97.1, wasm-pack 0.15.0, node v24.13.0. Scratch material lived under the session scratchpad `f237-p1` with its own target directories and was deleted with `shutil.rmtree`.

## Defects

### D1, the CI TypeScript job goes red, it does not skip

**Where**: `packages/wasm/package.json` `scripts.prepack`, `packages/wasm/scripts/stage.mjs:30-35`, `.github/workflows/ci.yml:297-327`
**What**: `gate packages` runs `npm pack` in `packages/wasm`. That runs prepack, which runs `bin/ocelli.sh wasm`. The CI `frontend` job installs node and python and nothing else: no Rust target, no `wasm32-unknown-unknown`, no wasm-pack. `bin/ocelli.sh wasm` exits 1 with "wasm-pack is not installed", and `package_check.py` reports `FAIL: npm packaging` and exits 1. It fails closed, so this is no silent skip. It does mean the TypeScript job on the first push after integration is red.
**Why it is wrong**: CLAUDE.md, `gate --floor` is what CI runs. A story that leaves the floor CI job red has not finished, whatever the reason.
**Evidence**: the hidden-wasm-pack row in Evidence. The `wasm` job, which does have wasm-pack, sets up no node and runs no `npm ci`, so the gate cannot simply move there either.
**Scope and regression**: present on HEAD. `.github/workflows/ci.yml` is outside F-237's write set, so this needs a lead ruling: either widen the write set so the frontend job adds the toolchain, the wasm32 target and wasm-pack, or have the gate stage from a supplied build. Smallest regression: the CI job itself, or a `ci_floor_check.py` assertion that the job running `gate packages` provisions wasm-pack. Separate and not F-237's: in a clean scratch clone of the base 622ce1b5, `npm run build` already fails at `@ocelli/example-viewer` (vite cannot load `crates/ocelli-wasm/pkg/ocelli_wasm.js`) before any pack runs. I could not reconcile that with CI's green S45 run, so it is recorded under Limitations rather than charged to this story.

### D2, `/release` still runs the old rule and never publishes `@ocelli/wasm`

**Where**: `.claude/commands/release.md:43-46` and `:76-77`, and `docs/lld/release-policy.md:45-49`
**What**: The procedure `/release` executes still says "The wasm module must be present in `@ocelli/core`" and publishes "`@ocelli/core` then `@ocelli/react`" because "the npm package embeds the wasm module". Followed as written, preflight 8 cannot pass. If it is waved through, step 5 publishes `@ocelli/core` with a peer dependency on `@ocelli/wasm@0.1.0`, a version that was never published. The LLD sentence "`/release` publishes it before `@ocelli/core` ... and refuses a release whose build id names another SHA than the tag's" is false against that file.
**Why it is wrong**: the lead's Q1 and Q2 decisions say "/release refuses unless that SHA is the tag's" and give a publication order. Design AC1 rejects "Item 6 rewritten and lines 42 and 53 left", and the same defect has moved to the second copy of the rule.
**Evidence**: the `git grep` for `inside .@ocelli/core|embeds the wasm|present in .@ocelli/core` returns exactly `.claude/commands/release.md:44,76,77` outside plans and the HLD.
**Scope and regression**: present on HEAD. `.claude/commands/release.md` is not in the design's write set, so the lead must widen it. Regression: the AC1 grep run over both release documents.

### D3, the release-time fresh-build comparison is not fresh and has no executable

**Where**: `docs/RELEASE.md:99-113` (item 6), `packages/wasm/scripts/stage.mjs:30-35`
**What**: Item 6 has the releaser build both modules, then pack. Prepack rebuilds into the same `crates/ocelli-wasm/pkg` and `pkg-decode`, copies those files, and hashes what it copied. Executed literally, "recorded digests match a fresh build of that SHA" compares a build with itself and cannot differ. `npm publish` then runs prepack again, so the bytes that reach the registry come from a third build that nothing compares to the listing item 6 inspected. No script implements the comparison, and `gate packages` only checks that the build id agrees with the tarball it sits in.
**Why it is wrong**: lead decision Q2 says "/release refuses unless ... the .wasm digest equals a fresh build of that SHA" and "if the measured build is not byte-reproducible, the story stops". A comparison that is self-referential by construction cannot refuse.
**Evidence**: reading the stage, plus the gate run, which builds once for pack and once for the publish dry run. An independent comparison does work: cold builds from `git archive HEAD` in two other paths with empty target directories matched the worktree's digests. So the rule can be met, but nothing directs or performs it.
**Scope and regression**: release path only, present on HEAD. Smallest fix: a release mode of `package_check.py` that builds the tag cold in a temporary checkout and target directory and compares digests to the packed build id, with `/release` publishing that exact `.tgz` (`npm publish <file>` does not run prepack). Regression: a probe with a tarball whose build id digests differ from the cold build.

### D4, the published README misstates the decode module's exports

**Where**: `packages/wasm/README.md:27`
**What**: "It carries the panic surface exports and nothing else." `crates/ocelli-wasm/pkg-decode/ocelli_wasm.d.ts` also exports `ocelli_version`, and the gate's own smoke calls `ocelli_version` from `@ocelli/wasm/decode`.
**Why it is wrong**: a false sentence in a file every consumer receives.
**Evidence**: `grep 'export function' pkg-decode/ocelli_wasm.d.ts` lists `install_panic_hook`, `ocelli_version`, `panic_record_len` and `panic_record_ptr`.
**Scope and regression**: the tarball README. Fix: "the panic surface exports and `ocelli_version`".

### D5, "cannot be packed without building the module" is false

**Where**: `docs/lld/typescript-packaging.md:34-35`, `packages/wasm/scripts/stage.mjs:4-5`
**What**: Both say a tarball cannot be packed without building the module. `npm pack --ignore-scripts`, or `ignore-scripts=true` in an npmrc (a common hardening setting), skips prepack. With `dist` absent it packed 7 files and no module.
**Why it is wrong**: a false claim, and the identity rule leans on it. The gate fails closed in that case (MODULE_REQUIRED, or the stale check against an old `dist`), but `npm publish` under the same setting would publish whatever `dist` holds.
**Evidence**: the `--ignore-scripts` row in Evidence.
**Scope and regression**: prose only. Fix: "npm runs it on every pack and publish unless scripts are ignored". This also feeds D3's fix of publishing the inspected `.tgz`.

## Smells

### S1, a dirty-tree build is recorded as HEAD

**Where**: `packages/wasm/scripts/stage.mjs:63`
**What**: The stage writes `git rev-parse HEAD` and never checks for tracked modifications. M3 built an uncommitted crate change, and the build id named `e063f5e1` with digests that build of `e063f5e1` never produces. The gate passed. README:173, README:454 and use.html:238 all say the build id names "the HEAD it was built from", which is false for such a build. Only `/release`'s clean-tree precondition keeps this out of a release. The tree under review showed it in practice: at the start of the pass, `packages/wasm/dist/build-id.json` recorded `622ce1b5` and had been staged while F-237's diff was still uncommitted.
**Failure mechanism**: any non-release use of the build id, such as a CI artefact, a bug report or a local tarball handed to an integrator, carries a SHA its bytes do not come from.
**Fix**: refuse, or record `"dirty": true` and have the gate refuse it, when `git status --porcelain --untracked-files=no` is non-empty. Add one probe for it.

### S2, the identity check's wiring is guarded by nothing

**Where**: `scripts/package_check.py:705-706`
**What**: All seven build-id probes call `wasm_identity_problems` directly. None goes through `wasm_identity_from_tarball` or `main()`. M1 deleted the call, and all 18 packages probes stayed green. `gate packages` also stays green on a correct tree. The guard's `limit` text declares only the MODULE_REQUIRED gap, so this one is undeclared.
**Failure mechanism**: a refactor that drops or mis-indents the call removes the whole of AC3 from the gate with every check green. That is review checklist item 4.
**Fix**: a level-1 probe that builds a small `.tgz` with a stale build id and calls `wasm_identity_from_tarball`, plus a structural pin that `main()` calls it for `MODULE_PACKAGE`. Alternatively, declare it in `limit`.

### S3, the gate cannot tell the render module from the decode module

**Where**: `scripts/package_check.py:508-540` (SMOKE_SCRIPT) and the `check.ts` lines added near :627
**What**: M2 swapped the two copies in `stage.mjs`. `@ocelli/wasm`'s root entry became the 16 kB decode module, and `package_check.py` reported OK. The smoke and the type check use only `init` and `ocelli_version`, which both modules export, and the build id hashes whatever was staged.
**Failure mechanism**: a staging edit, or a later third module, ships a root entry with no renderer and passes every check. The LLD sentence "It carries the render module as its root entry" would then go unproven.
**Fix**: assert a render-only export on the root (for example `typeof wasm.apply_commands === "function"`) and its absence on `./decode`, in both the smoke and `check.ts`.

## Nitpicks

- N1, `docs/RELEASE.md:55-57`: "because the module is built from the published crates". The module is built from the workspace path sources, not from the published crates, so the ordering reason is now weaker than the sentence says.
- N2, `docs/lld/typescript-packaging.md:73`: "It initialises `@ocelli/wasm/decode` the same way". The smoke initialises it directly rather than through `loadCore`, which the same sentence goes on to explain. "Reads its bytes the same way" would be exact.

## Verified clean

- Byte reproducibility on one machine and one toolchain: two cold builds in different paths with empty target directories, a no-op warm rebuild, a warm rebuild after touching sources, a third path, and the worktree build. All eight wasm-pack outputs are identical. This confirms the dated claim in RELEASE.md item 6 and both LLDs on macOS arm64. Other machines and operating systems were not measured, which matches the "on one toolchain" scoping.
- The build id SHA is the real `git rev-parse HEAD` of the repository the stage runs in (M7 proves the gate compares it). It equals HEAD for a clean tree (gate run).
- The notice rule (M4), the peer exact-version rule (M5, M6), the stale SHA (M7) and the digest (M8) each go red through the real gate.
- The exports map: `.` and `./decode` with `types` and `default`, all advertised paths present in the tarball, `publishConfig.access` public, version 0.1.0 equal to the workspace.
- The consumer uses `import.meta.resolve` inside the installed package and no repository path. `loadCore` flips `coreAvailable()`, and both modules report `VERSION`.
- The published set: `grep -rn '^publish' crates/*/Cargo.toml tools/oracle/Cargo.toml` prints `ocelli-native`, `ocelli-oracle` and `ocelli-wasm`. `@ocelli/example-viewer` is private, and the three public packages are at 0.1.0. `NOTICE` ships in all three.
- README:170-173, README:434-454 and README:550-551, use.html:231 and :238, use.html:262 and :279-280: every API and path named exists (`init`, `install_panic_hook`, `loadCore`, `@ocelli/wasm/decode`, glue fetch via `new URL('ocelli_wasm_bg.wasm', import.meta.url)`). The README snippet's `init()` runs in a browser, as its comment says. more.html names no distribution route and is not stale.
- `decode_graph_check` and the size budget still pass (`gate wasm`). The guard census, the runbook and the probe budget agree (`gate guards`). There are no em-dashes or prose semicolons in the added text.
- Composition: F-233's SIMD change does not add a shipped module, so `@ocelli/wasm`'s two-module list stays complete after integration. The probe budget and the runbook will conflict mechanically with F-233 at merge.

## Limitations and handoff

- No GitHub Actions run of this branch exists. D1 was reproduced locally by hiding wasm-pack. The pre-existing clean-clone `npm run build` failure at the example viewer, seen on 622ce1b5 and on main with this branch's `node_modules`, was not reconciled with CI's green S45 TypeScript job. That needs its own look before CI is trusted after integration.
- No publish and no registry interaction took place beyond `npm publish --dry-run` inside the gate.
- Lead rulings needed: the write set for `.github/workflows/ci.yml` (D1) and for `.claude/commands/release.md` (D2), and the shape of the release-time cold-build comparison (D3).
- This is a bounded review of F-237's commit only. It is not acceptance, and pass 2 must review the remediation.

## Self-review catch figure

GLM's notes (`.claude/scratch/F-237-self-review-1.md`) record checks that should have exposed 5 of the 10 findings here:

- D4: cycle 1 called both modules' `ocelli_version()` under node.
- D5: cycle 1 checked that prepack runs on pack.
- S1: its own staged build id recorded `622ce1b5` over a dirty tree.
- S2: its seven-mutant harness never mutated the call site.
- D2: AC1's grep, widened from one file to the tree.

D1 was raised by GLM as an open point outside the notes, so it counts as noticed but not ruled. D3, S3, N1 and N2 were plausibly out of reach of the checks it recorded.
