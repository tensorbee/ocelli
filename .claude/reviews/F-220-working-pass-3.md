# F-220 review, pass 3

**Reviewed**: staged tree `22fcce32fc43aebc04876992a7969e0c85e15e23` on `work/f-220-xai` (HEAD `c07ef38`), plus one unstaged path, `.claude/plans/F-220-design.md`, which holds the lead amendments after passes 1 and 2. The unstaged diff hashed `0b2d4e6b…` at the start and at the end of the pass, and `git write-tree` gave the same tree at both points. Untracked `tools/oracle/out` and the pass-1 and pass-2 reports were ignored.
**Author / reviewer**: the remediation was written by an external coding agent (its report is in the session scratchpad, not tracked). This pass was reviewed by a Claude Code subagent that wrote no F-220 code, design, amendment or earlier review.
**Result**: 2 defects, 2 smells, 7 nitpicks

F-220, pass 3: 2 defects, 2 smells, 7 nitpicks. Pass-2 D1, D2 and S1 are closed. Pass-2 D3 regressed. Pass-2 D4 is fixed in the register, but the attribution it records does not reach the shipped file.

## Acceptance challenged

- **Pass-2 D1, objc2 family.** For each of the ten crates I read the published commit from `.cargo_vcs_info.json` in `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f`. I compared it with the pin URL, then compared the pinned bytes with `git show <commit>:LICENSE.md` in a fresh clone of madsmtm/objc2. All ten match. At `b4167b5`, `8852b42`, `7b1abfd` and `8d214f5` the only licence-named file is `LICENSE.md`, with sha256 `7f976f7e…` at all four. `git log -1 --format=%P cfb19922` prints only `757cd841d0293341dbefd9b99d7548dec244ffde`, so that commit is the sole parent. `cfb19922` ("Clarify the licensing", 2025-01-22) deletes `LICENSE.txt` and is an ancestor of all four published commits. `757cd841:LICENSE.txt` begins `MIT License` / `Copyright (c) Steven Sheldon` and hashes `e353f37b…`, which matches the pin. In `THIRD_PARTY_LICENSES`, each objc2-family section is `LICENSE.md`, then the elected template, then Sheldon's notice for the MIT-only crates. Each holder line is correct: `Mads Marquart <mads@marquart.dk>` for block2, objc2 and objc2-encode, `Mads Marquart <mads@marquart.dk>, Mary <mary@mary.zone>` for dispatch2, and `the <name> authors (https://github.com/madsmtm/objc2)` for objc2-foundation and the five framework crates, none of which declare authors. The Sheldon sentence appears for exactly block2, objc2, objc2-encode and objc2-foundation (lines 1647, 1650, 1653, 4875). `Copyright (c) Steven Sheldon` appears twice, once per text group. `Copyright 2026` appears nowhere in `THIRD_PARTY_LICENSES` or `ci/notices/`.
- **Pass-2 D2, Apache.** `ci/notices/spdx/Apache-2.0.txt` and a fresh fetch of `https://www.apache.org/licenses/LICENSE-2.0.txt` both hash `cfc7749b…`. Of the 21 Apache-like blocks in `THIRD_PARTY_LICENSES`, one is byte-identical to apache.org: the group for gl_generator, khronos_api, raw-window-metal, spirv and static_assertions. The other 20 are crates' own packaged `LICENSE-APACHE` files, which are reproduced as packaged and are not the pin. No fallback crate elects Apache today. I mutated `fill_copyright` so that it fills the appendix line, and the unit suite went red at `test_a_template_fallback_names_declared_authors_or_the_crate`.
- **Pass-2 D3, probe notes.** Replayed for both notes. See D2 below.
- **Pass-2 D4, choosealicense.** The register is corrected. I checked the CC BY 3.0 Unported legal code against where the shipped adaptation goes. See D1 below.
- **Pass-2 S1, published-commit check.** I worked in a scratch copy. Changing dispatch2's pin to `7b1abfd…`, a real objc2 commit whose `LICENSE.md` is byte-identical, made `licence_check.py binaries` and `--write binaries` both exit 1 with `dispatch2 0.3.1 pin URL commit 7b1abfd… differs from the published commit 8852b42…`. That is the case a digest check cannot catch. The probe `licences.binaries-published-commit` printed `red` on the unmodified tree. With `published_commit_problems` returning `[]`, the probe printed HARNESS "exited 0. The guard it aims at did not fire", as its note says, and two unit tests failed. Removing only the call in `render_closure` left the probe red (it calls the function directly), but `test_binaries_gate_reads_the_published_commit` failed, so the wiring is watched in `gate licences`. A 7-character prefix of the right commit is accepted on purpose (`_same_commit`).
- **Cross-story, F-216 mimalloc.** See the cross-story rule below. I simulated the change in a scratch copy with a cloned `CARGO_HOME`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `.cargo_vcs_info.json`, `Cargo.toml` authors, licence and repository for the 16 pinned crates | registry cache | now | every pin URL commit equals the published sha1. The full table is below |
| fresh blobless clone of madsmtm/objc2, `ls-tree`, `show`, `merge-base --is-ancestor`, `log --format=%P` | upstream, 2026-11-05 | now | as described above |
| every `sources.toml` URL fetched, sha256 compared with the row and the local file | upstream | now | the 16 crate rows, the 4 `[[earlier]]` rows and Apache-2.0 equal upstream. The MIT URL is a blob page and the template is compared by diff instead. Zlib is a template. rust-std was compared with `cmp` against the 1.97.1 sysroot file: identical |
| `diff` of the choosealicense raw `mit.txt` body and zlib v1.3.1 `LICENSE` against the templates | upstream | now | MIT: only the copyright line, plus the leading blank line after the front matter. Zlib: the holder line and sign-off are dropped and the terms are dedented |
| CC BY 3.0 legalcode.txt, sections 1(a), 4(a) and 4(b) | creativecommons.org | now | quoted in D1 |
| choosealicense `README.md` "## License" | gh-pages | now | content is CC BY 3.0, source code is MIT |
| `grep -c 'choosealicense\|Creative Commons\|CC BY'` in `THIRD_PARTY_LICENSES` and `THIRD_PARTY_NOTICES` | staged | now | 0 and 0 |
| scratch copy: dispatch2 pin URL changed to `7b1abfd…` / to a short prefix / to `main` | scratch | now | 1 refused / 0 accepted / 1 refused, but the message says "published commit absent" (N5) |
| scratch copy: `[[earlier]]` URL and `commit` field changed to `ee9a7ada…`, where `LICENSE.txt` does not exist | scratch | now | `binaries` exited 0 (S1) |
| scratch copy: one byte changed in the earlier file / objc2-foundation's earlier row dropped | scratch | now | 1 on the digest / 1 "does not match the lockfile" |
| scratch copy: `published_commit_problems` returns `[]` | scratch | now | probe HARNESS "did not fire", 2 unit failures |
| scratch copy: call removed in `render_closure` | scratch | now | probe red, 1 unit failure |
| scratch copy: `fill_copyright` fills the Apache appendix | scratch | now | 1 unit failure |
| scratch copy: `pinned_digest_problems()` replaced by `[]` in `binaries_problems`, `guard_probe.py --profile deep --only licences.binaries-pin` | scratch | now | HARNESS "got the exit status it wanted, but not at the declared refusal: nothing in the output contains 'sha256 is'", output "THIRD_PARTY_LICENSES does not match the lockfile" (D2). The unmutated probe is red |
| scratch copy: `rm THIRD_PARTY_LICENSES`, `--write binaries` | scratch | now | 0. `THIRD_PARTY_LICENSES` and `sources.toml` are byte-identical to the worktree |
| scratch copy, cloned `CARGO_HOME`: `mimalloc = "=0.1.52"` in the workspace and the folder viewer, unlocked `cargo metadata`, then `binaries` and `rust` | scratch | now | lock adds `mimalloc 0.1.52` and `libmimalloc-sys 0.1.49` (dep `cc`). `binaries` exited 0, `rust` exited 0 |
| `python3 scripts/licence_check.py binaries` | worktree | now | 0, OK |
| `python3 -B -m unittest discover -s scripts/tests -p test_licence_check.py` | worktree | now | 0, 67 tests OK |
| `bin/ocelli.sh gate licences` | worktree | now | 0, ALL GREEN |
| `bin/ocelli.sh gate packages` | worktree, existing dist | now | 0, ALL GREEN |
| `bin/ocelli.sh gate guards` | worktree | now | 0. "429 refusal probe(s) drove 37 guard(s) red … 93 control(s) green, 108.1s". `licences.binaries-published-commit` red |
| `python3 scripts/guard_census.py`, then with `--check-runbook` | worktree | now | 0 and 0. "1089 refusal(s) in 78 file(s), all claimed", "0 watched by nothing" |
| `bin/ocelli.sh gate ci` | worktree | now | 0, ALL GREEN |
| `python3 scripts/prose_check.py` | worktree | now | 0, 840 files |
| `python3 scripts/source_provenance_check.py` | worktree | now | 0, 1677 files |
| `gate guards-deep` | | implementer receipt only | not re-run in full. `binaries-pin` ran alone under `--profile deep` |
| `git write-tree`, `git diff \| shasum` at the start and the end | worktree | now | `22fcce32…` and `0b2d4e6b…` both times |

### Pin table, URL commit against published commit and upstream bytes

| Crate | Version | Published sha1 (`.cargo_vcs_info.json`) | URL commit | Bytes vs upstream at that commit |
|---|---|---|---|---|
| block2 | 0.6.2 | `b4167b582b2f…` | same | equal |
| dispatch2 | 0.3.1 | `8852b424193c…` | same | equal |
| gl_generator | 0.14.0 | `ea503e8d5fb6…` | same | equal |
| jni-sys-macros | 0.4.1 | `64d77b7a5f11…` | same | equal |
| khronos_api | 3.1.0 | `f150967b1c44…` | same | equal |
| ndk-sys | 0.6.0+11769913 | `49bbbba16c58…` | same | equal |
| objc2 | 0.6.4 | `8852b424193c…` | same | equal |
| objc2-core-foundation | 0.3.2 | `7b1abfd750a2…` | same | equal |
| objc2-core-graphics | 0.3.2 | `7b1abfd750a2…` | same | equal |
| objc2-encode | 4.1.0 | `8d214f547736…` | same | equal |
| objc2-foundation | 0.3.2 | `7b1abfd750a2…` | same | equal |
| objc2-io-surface | 0.3.2 | `7b1abfd750a2…` | same | equal |
| objc2-metal | 0.3.2 | `7b1abfd750a2…` | same | equal |
| objc2-quartz-core | 0.3.2 | `7b1abfd750a2…` | same | equal |
| profiling | 1.0.18 | `8271551172eb…` | same | equal |
| spirv | 0.4.0+sdk-1.4.341.0 | `8afc3d0ac8e1…` | same | equal |
| `[[earlier]]` ×4 | | n/a, by design | `757cd841d029…` = `cfb19922^` | equal |

## Defects

### D1, the shipped adaptation of choosealicense's CC BY 3.0 content carries no credit

**Where**: `THIRD_PARTY_LICENSES`, in the text groups for block2, objc2 and objc2-encode (lines 1688-1708) and for objc2-foundation (lines 4910-4930). The generator header is in `render_notices`, `scripts/licence_check.py`. `docs/SOURCE-POLICY.md` third answer. The `sources.toml` header.
**What**: pass 2's remediation routed the four MIT-only objc2 crates through the MIT template. So `THIRD_PARTY_LICENSES` now distributes `ci/notices/spdx/MIT.txt`, filled, in two text groups. In pass 2 no rendered output contained a template, which is why pass 2 rated its D4 as register-only. That template is an Adaptation of `_licenses/mit.txt`, whose content the register now correctly records as CC BY 3.0 Unported. The credit is in `sources.toml` comments and `docs/SOURCE-POLICY.md`, and those are repository files. Neither ships to a recipient of the wasm tarball or the planned native archive. Neither `THIRD_PARTY_LICENSES` nor `THIRD_PARTY_NOTICES` names choosealicense, Creative Commons or CC BY (`grep -c` gives 0 for both). The section sentence says only "the standard text of the licence the crate declares".
**Why it is wrong**: CC BY 3.0 section 4(b) says: "If You Distribute … the Work or any Adaptations … You must … provide, reasonable to the medium or means You are utilizing", the Attribution Parties, the title, the URI and, "in the case of an Adaptation, a credit identifying the use of the Work in the Adaptation". Section 4(a) requires a copy or the URI of the licence with every copy of the Work distributed. The lead amendment after pass 2 requires "the attribution that licence requires". `docs/SOURCE-POLICY.md` answer 3 says CC BY "permits that adaptation when the attribution in the answer above is kept". As built, the attribution is kept only where no recipient sees it, so that answer's condition is not met.
**Evidence**: the grep above, the legal code fetched from creativecommons.org, and the choosealicense README "## License" section.
**Scope and regression**: present on the reviewed tree. It affects every recipient of `THIRD_PARTY_LICENSES`. Uncertainty: whether GitHub holds any copyright in a verbatim MIT text is doubtful. The amendment, however, chose the CC BY treatment, and this file is the legal artefact. Fix by emitting the credit in `THIRD_PARTY_LICENSES` itself: a header or per-template line naming GitHub, choosealicense.com, `_licenses/mit.txt`, its URI, CC BY 3.0 and its URI, and the change. The alternative is to take the MIT terms from a source not under CC BY, recorded with its own register row. Smallest regression: a unit test asserting that any rendered output containing the MIT template also contains the CC BY credit, plus a probe for it.

### D2, the `licences.binaries-pin` note is false again

**Where**: `scripts/guards/catalogue.py:7760-7766`.
**What**: the note says "Dropping pinned_digest_problems from binaries_problems makes this probe exit 0. The harness then reports that the guard it aims at did not fire." On this tree, after that mutation the probe exits 1 with `THIRD_PARTY_LICENSES does not match the lockfile`. The harness then prints HARNESS: "got the exit status it wanted, but not at the declared refusal: nothing in the output contains 'sha256 is'". The note was true at pass 2, because no crate rendered the MIT template then. Now four crates do, so the appended newline in `MIT.txt` changes the rendered bytes and a second refusal fires.
**Why it is wrong**: `/microscope` section 3. A guard note is a claim about observed behaviour, and the remediation that made the objc2 crates use the template falsified it without the note being re-measured.
**Evidence**: the scratch mutation row in the Evidence table. The probe is red unmutated.
**Scope and regression**: prose only. The guard still goes HARNESS-red, so detection holds. Re-measure and state the observed outcome, or describe the outcome without naming the exit status. The `licences.binaries-published-commit` note was re-measured and is true.

## Smells

### S1, the `[[earlier]]` provenance is unchecked, and the shipped sentence hard-codes its commit

**Where**: `_STATEMENT_EARLIER_SENTENCE` (`scripts/licence_check.py:105-111`), `earlier_files`, and the four `[[earlier]]` rows in `ci/notices/sources.toml`.
**Mechanism**: the generator reads only `path` and `sha256` from an `[[earlier]]` row. `grep` finds no reader of its `commit` or `reason` fields. The shipped sentence names "the parent of commit cfb19922" from a string constant, not from the row. In a scratch copy I changed all four earlier URLs and `commit` fields to `ee9a7ada…`, a commit with no `LICENSE.txt` at all, and `binaries` exited 0. The shipped text kept claiming the parent of `cfb19922`. This is the failure class pass-2 S1 closed for crate rows. The source of an earlier pin can drift from what the notice says, with every gate green. The `commit` field looks authoritative and nothing reads it. A second crate family needing an earlier notice would also get objc2's commit in its sentence.
**Smallest fix**: derive the sentence's commit from the row, and refuse a row whose URL commit differs from its `commit` field. A unit test and a probe should change one of them.

### S2, the shipped sentence refers to "that commit" and never names it

**Where**: `_STATEMENT_SENTENCE` and `_STATEMENT_EARLIER_SENTENCE`. Rendered at `THIRD_PARTY_LICENSES:1647`, `:1650`, `:1653`, `:2182`, `:4757` and the five other objc2-family carriers.
**Mechanism**: "Neither the published package nor its repository at that commit carries licence text. The text below is LICENSE.md from that commit…" No commit or repository is named anywhere in the section. `grep -c` for any of the five commit prefixes in `THIRD_PARTY_LICENSES` gives 0. Where authors are declared, the holder line does not name the repository either. Three carriers with three different published commits share one text group. A recipient cannot tell which file at which commit is being reproduced. A later reviewer has to use `sources.toml`, which does not ship. The provenance statement is the claim this story exists to make, and in the shipped artefact it cannot be checked.
**Smallest fix**: render the repository and the published commit, taken from the pin URL, into each carrier's sentence. S1's fix supplies the earlier commit the same way.

## Nitpicks

- N1, carried. Six `authors: ` lines still have a trailing space.
- N2, carried. `</html>-----END RUST STANDARD LIBRARY NOTICE-----` still shares one line.
- N3, carried, not re-examined. The closure-count test is not independent of `_rust_shipped`.
- N4, carried. The header says "reachable on a normal or build edge", and some crates are included only through workspace-unified features.
- N5. A pin URL with no commit (a branch such as `main`) is refused with "differs from the published commit absent". The published commit is present. It was never looked up. The refusal is correct, but the diagnostic misleads.
- N6. `docs/lld/guards.md:618-624` lists the binaries level-1 probes and omits `licences.binaries-published-commit`.
- N7. For the six text groups whose source file lacks a trailing newline, the `sha256:` line is the digest of the text without the newline the generator appends. The header does not say what the digest covers.

## Cross-story rule for F-216 (mimalloc)

- **The folder viewer is not in this file's closure.** `_rust_shipped` walks only from `ocelli-wasm` (wasm32) and `ocelli-native` (unfiltered). `ocelli-folder-viewer` is a workspace member with `publish = false`. Plan line 235 says it "is a local example that is not distributed". It already links crates outside the closure, such as `ocelli-codec` and `dicom-object`.
- **Measured.** I added `mimalloc = "=0.1.52"` to `[workspace.dependencies]` and to the folder viewer, default features only, as F-216's amendment says. The lock gained `mimalloc 0.1.52` and `libmimalloc-sys 0.1.49`, whose only dependency is `cc`, and changed no existing version. `licence_check.py binaries` and `rust` both stayed green. `cc` is not in the closure today.
- **The rule.** AC7 refuses ("does not match the lockfile") exactly when the regenerated `THIRD_PARTY_LICENSES` differs. That happens when a lock change alters any package reachable on a normal or build edge from `ocelli-wasm` (wasm32) or `ocelli-native` (any platform), including an edge added through workspace-wide feature unification, or alters that package's version, licence, authors or files. A change that only adds packages reachable from the folder viewer does not refuse. So integration order does not matter for F-216 as designed. Whichever story integrates second must still run `python3 scripts/licence_check.py binaries` on the merged tree. If it refuses with "does not match the lockfile", that story runs `--write binaries` and commits the regenerated file. If F-216 enables a mimalloc feature, or bumps `cc` or `libc` in a way that touches a closure node, the measured "no change" no longer holds. F-220 does not touch `Cargo.lock`. Both branches modify `ci/guard-probe-budget.json`, so expect a textual merge there.

## Verified clean

- Pass-2 D1 is closed as to the pins, the holder lines, the Sheldon notice and its scope, and the absence of any 2026 line. The shipped sentence's referent is S2.
- Pass-2 D2 is closed. The Apache pin and its rendered group are byte-identical to apache.org, and a test watches the appendix line.
- The `licences.binaries-published-commit` note is true. Pass-2 D3 regressed for `binaries-pin` only (D2).
- The register half of pass-2 D4 is closed. The choosealicense row reads "CC BY 3.0 for the copied content", and the answers give the attribution elements section 4(b) lists. Shipping that credit is D1.
- Pass-2 S1 is closed for `[[crate]]` rows, through the entry point, the probe and the unit suite.
- Every pin, the regeneration round trip, and all ten requested gate commands are green on the reviewed tree.

## Limitations and handoff

- All mutations ran in a scratch copy, detached from the worktree's git directory. The worktree's tree hash and unstaged diff are unchanged.
- The F-216 simulation used a cloned `CARGO_HOME` under the scratchpad and assumed version `0.1.52`, the current release. F-216's worktree has not added mimalloc yet, and it was only read.
- `gate guards-deep` was not run in full. The Linux half of AC8 still needs CI on the pushed head.
- The Apple SDK question in objc2's `LICENSE.md` is still the operator's.
- Next owner: implementation, for D1, D2, S1 and S2. Then a fresh independent pass 4.
