# F-237, The wasm module ships outside the @ocelli/core tarball

**Status**: approved
**Epic ref**: E19.6
**Sprint**: S47
**Estimate**: 1w

Story row, `docs/sprints/BACKLOG.md`: "The wasm module ships outside the @ocelli/core tarball: docs/RELEASE.md preflight item 6 and the published list name where the module and its licence files ship and how a release proves the module is the one its HEAD builds, with the packages gate checking that place". Layer Build, depends on F-220 and F-228, milestone M20.

`allocation.json` notes: "Found while designing F-220. Answered by the operator on 2026-11-06: change the release rule, the module ships outside the @ocelli/core tarball and a story in S47 decides where."

## Normative source, transcribed

The HLD does not say how the module is distributed. The nearest lines are `docs/hld/01-purpose-and-scope.md:30`, "published packages use the @ocelli npm scope", `docs/hld/12-workspace-and-build.md:58`, "core/ # @ocelli/core (TypeScript shell)", and `docs/hld/03-architecture-and-crates.md:41`, the `ocelli-wasm` row, "The only crate that may import wasm-bindgen".

### `docs/RELEASE.md`, the rule this story rewrites

Lines 42 and 43: "`ocelli-wasm` is `publish = false`. It is a `cdylib` whose artefact ships inside `@ocelli/core`, so publishing it to crates.io would offer a crate nobody can use from Rust."

Lines 53 and 54: "npm publishes **after** crates.io, because the npm package embeds the wasm module and a failed crate publish should stop the release before anything reaches npm."

Item 6: "`npm pack` for each published package, with its contents listed. The wasm module must be present in `@ocelli/core` and must be the one this HEAD builds, not a stale artefact from a previous build. The tarball that carries that module must also carry `THIRD_PARTY_LICENSES` and `THIRD_PARTY_NOTICES`. `gate packages` refuses a `.wasm` file without both."

### The operator's answer, `docs/sprints/CURRENT_SPRINT.md`

"`npm pack --dry-run -w packages/core` lists 281 entries on c775931 and none ends in `.wasm`. ... **Answered by the operator on 2026-11-06:** change the release rule. The module ships outside the `@ocelli/core` tarball and `docs/RELEASE.md` item 6 says where. A story in S47 decides where and rewrites the rule."

## Facts measured for this plan

Read on 2026-11-06 on `sprint/s46` at `1ba258e`.

1. `packages/core/package.json` has `"files": ["dist", "LICENSE-MIT", "LICENSE-APACHE", "NOTICE", "README.md"]`, no `.wasm` and no `THIRD_PARTY_*` file.
2. `bin/ocelli.sh wasm` runs `wasm-pack build crates/ocelli-wasm --target web --out-dir pkg "$@"`, and `wasm-decode` writes `pkg-decode` with `--no-default-features --features decode-worker`. Both directories are ignored by git. The generated `pkg-decode/package.json` is named `ocelli-wasm`.
3. `loadCore` (`packages/core/src/load.ts:19`) is `export function loadCore(initialise: () => Promise<unknown>): Promise<true>`. The caller imports and initialises the module itself, so `@ocelli/core` does not locate the module today.
4. `examples/viewer-react/vite.config.ts` reaches the module by repository path, `../../crates/ocelli-wasm/pkg/ocelli_wasm.js`. `examples/folder-viewer/src/use.html:280` shows `import init, * as wasm from "./pkg/ocelli_wasm.js";`.
5. `scripts/package_check.py` asks for the two notice files only when a tarball holds a `.wasm` file.
6. `README.md` lines 166 to 168: "`bin/ocelli.sh wasm` writes the module to `crates/ocelli-wasm/pkg`. That directory is a build artefact and is not committed".

## What the specification does not cover

Where the module ships, and how a release proves it is the one HEAD builds. Options under Open questions.

## Problem and reproducer

`/release` preflight item 6 requires the module inside `@ocelli/core`, which neither ships it nor can by the operator's answer. So `/release` cannot pass item 6 as written, and nothing says where a consumer gets the module.

## Scope, API and compatibility

In scope: `docs/RELEASE.md` lines 42 and 43, 53 and 54, item 6, the place chosen, `scripts/package_check.py` for that place, `docs/lld/typescript-packaging.md`, `docs/lld/release-policy.md`, and the user-facing pages. Non-goals: the module's content, a CDN, and `@ocelli/core`'s `loadCore` signature, which fact 3 already leaves to the caller.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `/release` item 6 | `npm pack` of the chosen package, or the chosen artefact's listing | release HEAD | the module, its two notice files, its build identity |
| `gate packages` | `package_check.py` | every published npm package | refusal for a module without notices or identity |
| a consumer | the chosen place | `loadCore(initialise)` | a module whose version matches `@ocelli/core` |

## State transitions and invariants

Not applicable. The invariant: a released `@ocelli/core` version has exactly one module released beside it, at the same version, built from the same HEAD.

## Ownership, commit and recovery

Not applicable beyond release ordering, which `/release` owns. Under O1 the new package publishes before `@ocelli/core`, so a failed module publish stops the release before the shell that needs it.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | `docs/RELEASE.md` names the place in the published list, in the ordering paragraph and in item 6, and no line says the module ships inside `@ocelli/core` | review, `grep -n 'inside .@ocelli/core' docs/RELEASE.md` empty | Item 6 rewritten and lines 42 and 53 left | Implementation |
| AC2 | The chosen package or artefact carries the `.wasm`, its JS glue, `THIRD_PARTY_LICENSES` and `THIRD_PARTY_NOTICES` | `npm pack --dry-run` listing, or the artefact listing | Notices left in `@ocelli/core` only | Implementation |
| AC3 | Build identity. The module carries the HEAD it was built from in a form `/release` compares to the release HEAD, and a stale build is refused | `package_check.py` refusal and a probe with a stale identity | A presence check that a stale module passes | guards |
| AC4 | `gate packages` covers the new place, including F-221's test-file refusal and F-218's advisory checks if it is an npm package | `gate packages` | A package outside the gate | Implementation |
| AC5 | A consumer outside the workspace installs `@ocelli/core` and the module at one version and `loadCore` resolves | the packages gate's consumer proof, extended | A proof that imports from the repository path | Implementation |

## Approach

Depends on Q1.

## Dependencies, write ownership and resources

Declared: F-220, which wrote the notices and their `package_check.py` rule, and F-228, which documents the module's path under a Content Security Policy and writes `docs/lld/typescript-packaging.md`. Writes `docs/RELEASE.md`, `scripts/package_check.py`, `bin/ocelli.sh` if the build moves, the guard catalogue, budget and runbook, and the two LLDs.

## Verification and evidence reuse

```bash
bin/ocelli.sh wasm
bin/ocelli.sh gate packages guards
```

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a, packaging
- Tier B (WebGL2): n/a, packaging
- Tier C (CPU): n/a, packaging

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| guard probe | AC3, a stale or notice-less module refused | `scripts/guards/catalogue.py` |
| integration | AC2, AC4, AC5 | `gate packages` |

## Parity surface covered

None.

## Deviations

None expected. The HLD names no distribution route.

## LLD impact

`docs/lld/typescript-packaging.md`, "What is in the `@ocelli/core` tarball today" and the module's place. `docs/lld/release-policy.md`, the published set.

## User-facing documentation impact

`README.md` lines 166 to 168 and the third-party paragraph near line 406, `examples/folder-viewer/src/use.html` line 238's table row and line 280's import, and `more.html` if it names how the module is obtained.

## Open questions

- **Q1, where the module ships.** Affects every row. Decision owner: the lead in the S47 design round, under the operator's answer. Options:
  - **O1, recommended.** A separate npm package in the @ocelli scope, for example `@ocelli/wasm`, built by `wasm-pack` from `crates/ocelli-wasm`, carrying the two notice files, versioned with `@ocelli/core` and declared by it as a peer dependency at the same version. HLD 1 already puts published packages in that scope, and `gate packages` already proves npm packages.
  - **O2.** A release asset attached to the GitHub release for the tag, with a digest in the release notes. Not an npm install, and `gate packages` would not see it.
  - **O3.** No binary shipped, and consumers build the module from the tagged source. Leaves every consumer a Rust toolchain.
  Deferral leaves `/release` refusing at item 6.
- **Q2, the build identity form.** Affects AC3. Options: a version and git SHA written into the package by the build, or a digest of the `.wasm` compared to a fresh build at release. Owner: the lead.
- **Q3, the decode module.** `pkg-decode` is a second module (fact 2). Whether it ships, and where. Owner: the lead.

## Lead decisions, pulled forward into S46

Made by the sprint lead on 2026-11-07 under the operator's answer of 2026-11-06 (change the release rule) and the operator's decision of 2026-11-07 to pull bounded S47 work forward into S46 while implementer slots are idle.

- **Q1, O1.** The module ships as a separate npm package, `@ocelli/wasm`, built by `wasm-pack` from `crates/ocelli-wasm`, carrying the two notice files, versioned with `@ocelli/core` and declared by it as a peer dependency at the same version. `gate packages` proves it the way it proves the others.
- **Q2.** The build writes the package version and the git SHA of HEAD into the package, and `/release` refuses unless that SHA is the tag's and the `.wasm` digest equals a fresh build of that SHA. If the measured build is not byte-reproducible, the story stops and reports rather than relaxing the check.
- **Q3.** `pkg-decode` ships in the same package as a second module file, under the same identity rule.
- **Order.** This story depends on F-228, which is not integrated. It is built on F-228's branch and integrates after it.

### Lead rulings after review pass 1

Made by the sprint lead on 2026-11-07 after `.claude/reviews/F-237-working-pass-1.md`.

- **D1, the write set widens to CI.** The CI job that runs `gate packages` installs `wasm-pack` and the `wasm32-unknown-unknown` target the way the `wasm` job does, in `.github/workflows/ci.yml`, and `scripts/ci_floor_check.py` stays green.
- **D2, the write set widens to `/release`.** `.claude/commands/release.md` requires the module in `@ocelli/wasm`, publishes `@ocelli/wasm` before `@ocelli/core` and `@ocelli/react`, and the adapters are regenerated with `scripts/sync_agent_skills.py`. `docs/lld/release-policy.md` says the same.
- **D3, the release compares a cold build with what is published.** A script builds the tag cold in a temporary checkout, compares its module digests with the staged package's recorded digests, and produces the exact `.tgz` that `npm publish` is then given, so the published bytes are the compared bytes. Item 6 names the script and the command. Tests drive a matching build, a mismatched digest and a wrong SHA.
- **S1, a dirty tree is never recorded as HEAD.** The build id records whether the tree was clean, and both the packages gate and the release script refuse an id built from a dirty tree.
- **S2 and S3** each gain a catalogue probe: removing the identity check's call, and swapping the render and decode modules, both go red.
- **D4, D5, N1, N2.** The sentences are corrected to what the files and commands show.
- **Not F-237's:** a clean clone's `npm run build` failing at the example viewer on the base commit is recorded for the lead and investigated separately.

### Lead rulings after review pass 2

Made by the sprint lead on 2026-11-07 after `.claude/reviews/F-237-working-pass-2.md`.

- **D1, the build id records the tree, not only HEAD.** It records the git SHA of HEAD and the `git write-tree` of the staged index, and whether the working tree has unstaged tracked changes. `gate packages` refuses only unstaged tracked changes, so `/verify` on a staged tree records, and `release_wasm.py` keeps refusing anything but a clean checkout of the tag. A test stages a change and shows the gate green, and leaves one unstaged and shows it red.
- **D2, the comparison is a precondition.** `release_wasm.py` runs before any tag, push or crates.io publish, as a numbered preflight item in `.claude/commands/release.md` and `docs/RELEASE.md`, and the publish step uses the exact file it verified.
- **D3, the CI comment says only what is true.** No CI run of this branch exists, so the comment describes the provisioning, not a failure it never saw.
- **S1, the call-in-main probe is behavioural.** It runs `main()` with the npm steps stubbed over a tarball with a stale build id and requires the refusal, red under each of the three unwirings the review lists.
- **N2, N3 and N4** are taken. For N4 the sprint's F-232 note text wins.
- **The fresh-clone `npm run build` failure is not F-237's.** It reproduces on the sprint head and not on main, and the lead investigates it separately.

### Lead ruling after review pass 3

- **D1.** The two lint-policy probe notes are copied from `sprint/s46` byte for byte, which the lead did, and both probe blocks now equal `sprint/s46`'s.
- **N2 taken,** by the lead: the release refusal says the release revision's tree. N1 and N3 are not taken. Precondition 2 covers N1's gap and the procedure cannot reach N3.
- **Owed acceptance.** AC4's `frontend` job on `ubuntu-latest` after integration, the feature `/verify` receipt with the corpus on the operator's machine, AC3 end to end at the real release, and AC5 against the registry after publish. AC3 and AC5 are release-time rows and are named as such in the completion record.

### Lead ruling after review pass 4

- **D1.** The "(F-210)" attribution is deleted from both notes, identically on `sprint/s46` and here, so the blocks stay byte-identical. `held.rs` came with F-216.
- **N1 taken.** The `compare_problems` docstring says the tree of that commit.
