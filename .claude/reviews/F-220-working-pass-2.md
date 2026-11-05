# F-220 review, pass 2

**Reviewed**: staged tree `8f0823e0a44e271b988e8ab7ffb14f7c036b0d74` on `work/f-220-xai`, plus one unstaged path, `.claude/plans/F-220-design.md`. That path holds the lead's amendments, and the unstaged diff hashed `ccacae1b…` at the start and at the end of the pass. Untracked `tools/oracle/out` and the pass-1 report were ignored.
**Author / reviewer**: the remediation was written by an external coding agent (its report is in the session scratchpad, not tracked). This pass was reviewed independently by a Claude Code subagent that wrote no F-220 code, design or amendment. It did not write pass 1 either.
**Result**: 4 defects, 1 smell, 4 nitpicks

F-220, pass 2: 4 defects, 1 smell, 4 nitpicks. Every pass-1 finding is closed. The new findings come from the remediation. The main one is the objc2 notice source, which is the departure the lead asked this pass to rule on.

## Ruling on the later-commit objc2 pins

**Pinning the `ee9a7ad` grant files is not an honest notice for the locked versions.** The upstream evidence, all checked during this pass:

- **Published commits.** `.cargo_vcs_info.json` in each locked package gives the commit it was published from. block2 0.6.2 is `b4167b5` (2025-10-04). objc2 0.6.4 and dispatch2 0.3.1 are `8852b42` (2026-02-26). The five framework crates and objc2-foundation 0.3.2 are `7b1abfd` (2025-10-04, tag `frameworks-0.3.2`). objc2-encode 4.1.0 is `8d214f5` (2025-01-23). The tags `block2-0.6.2`, `objc2-0.6.4`, `dispatch2-0.3.1` and `objc2-encode-4.1.0` resolve to those same commits.
- **What those commits hold.** All four have `LICENSE.md` at the root and no other licence-named file. The text of `LICENSE.md` is identical across all four. It says objc2, block2, objc2-foundation and objc2-encode are "[currently] licensed under the MIT license", and that every other crate is "trio-licensed under the Zlib, Apache-2.0 or MIT license, at your option". It links to opensource.org, zlib.net and apache.org for the texts and names no copyright holder. It then says the crates are derived from Apple SDKs and that "it is unclear whether distributing derived works such as these crates are allowed". The packages ship no licence file at all.
- **The declared licences.** These match `LICENSE.md`. block2, objc2, objc2-encode and objc2-foundation declare `MIT`. dispatch2 and the five framework crates declare `Zlib OR Apache-2.0 OR MIT`.
- **What `ee9a7ad` changed.** It is dated 2026-03-18, after every locked commit, and its title is "Include license text in the repository". It adds `LICENSE-MIT.txt`, `LICENSE-ZLIB.txt` and `LICENSE-APACHE.txt`. The MIT and Zlib files open with `Copyright 2026 Mads Marquart`. It also repoints the three links in `LICENSE.md` at those files and drops `license-file = "LICENSE.md"` from the workspace manifest. The commit says nothing about earlier releases.
- **What the maintainer said about earlier releases.** On issue #826, asked about exactly these versions, the maintainer answered on 2026-10-10 that "The license of the old crate versions are as stated in the `license` field". On 2026-10-11 he was asked whether the copyright notices in the added files are sufficient for those versions. He answered: "Dunno how copyright law works, so can't really answer that. Probably not though, there's other contributors than me."
- **The earlier holder.** Until `cfb19922` (2025-01-22, an ancestor of every locked commit), the repository's root `LICENSE.txt` read `MIT License` / `Copyright (c) Steven Sheldon`. objc2, block2, objc2-encode and objc2-foundation are MIT-only because of that origin (`LICENSE.md` cites issue #23).

The upstream author says the later files' copyright line is probably insufficient. The repository at every locked commit carried no such line. So shipping `Copyright 2026 Mads Marquart` under the sentence "The text below is the licence file from its repository" states, for releases made before that file existed, a single holder and a year that upstream never attached to them. That is D1 below.

**What the notice must say instead**, within the lead's own amendment:

1. For each objc2-family crate, pin the licence file its repository actually carried at the published commit: `LICENSE.md` at `8852b42`, `7b1abfd`, `b4167b5` or `8d214f5`. The four blobs are byte-identical. Decision 4's file rule counts it, because the name begins with `LICENSE`. It is upstream's licence statement for those versions, and it carries the Apple SDK caveat that pass 1 sent to the operator.
2. `LICENSE.md` holds no terms. Add the elected template (MIT, or Zlib for the trio) through the amendment's fallback rule, with the holder from the declared authors, or "the <name> authors (https://github.com/madsmtm/objc2)" when none are declared. Record that reason in `sources.toml`. No year.
3. The section sentence must say what is true: the package carries no licence file, the repository at the published commit carries `LICENSE.md` and no licence text, and the text below is the declared licence's terms. It must not say "the licence file from its repository".
4. **Lead decision needed.** Should the four MIT-only crates also carry the repository's earlier notice, `Copyright (c) Steven Sheldon`? It can be pinned from `cfb19922^:LICENSE.txt` (sha256 `e353f37b12aefbb9f9b29490e837cfee05d9bda70804b3562839a3285c1df1e5`). The MIT condition is to keep "the above copyright notice", and this was the repository's only explicit one for that code until January 2025. I recommend including it, labelled as the repository's earlier notice. This pass does not count it as a defect, because the amendment does not cover it.

The `ee9a7ad` files may appear in addition, labelled as added upstream after the locked release, but they must not be presented as the notice of the locked version.

**The other per-crate pins are clean.** Each URL's commit equals the package's `.cargo_vcs_info.json` `sha1`. gl_generator 0.14.0 is `ea503e8`, khronos_api 3.1.0 is `f150967`, jni-sys-macros 0.4.1 is `64d77b7`, ndk-sys 0.6.0+11769913 is `49bbbba`, profiling 1.0.18 is `8271551` and spirv 0.4.0+sdk-1.4.341.0 is `8afc3d0`. Each refetched file hashes to its recorded digest. At `ea503e8` the gl-rs manifests read gl_generator 0.14.0 and khronos_api 3.1.0. At `f150967`, khronos_api reads 3.1.0. GitHub's reported licences match the policy rows: Apache-2.0 for those five repositories, and NOASSERTION ("Other") for madsmtm/objc2 and madler/zlib.

## Acceptance challenged

- **Lead amendment 2, "at the tag or commit matching its locked version".** The independent check was to read the published commit from each package's `.cargo_vcs_info.json` and compare it with the commit in each pin URL. Six crates match. Ten objc2-family crates do not. See D1 and S1.
- **Lead amendment 1, templates.** `ci/notices/spdx/MIT.txt` is choosealicense's `mit.txt` body with only the copyright line changed. `Zlib.txt` keeps zlib's terms and drops its holder line and sign-off. No crate takes the fallback today, so no rendered output contains a template.
- **The Apache-2.0 template on a fallback.** `fill_copyright` on the pinned Apache text rewrites the appendix line. The comment says that text stays as published. See D2.
- **D3 replay, through the public CLI.** In a scratch copy, setting the channel to 1.93.0 made `binaries` refuse with the rust-docs instruction. `--write binaries` exited 0, `sources.toml` parsed with four `file` rows (MIT, Apache-2.0, Zlib, rust-std at 1.93.0) and 16 `crate` rows, and `binaries` then exited 0. Setting the channel back to 1.97.1 and writing again made `sources.toml`, the HTML and `THIRD_PARTY_LICENSES` byte-identical to the tracked tree. An adjacent failure case: with the block2 pin removed and the channel at 1.93.0, `--write` exited 1 with "is absent" and all three files were unchanged.
- **The holder skip for `ci/notices/`.** Tensorbee's own `LICENSE-MIT` files are `LICENSE-MIT`, `crates/ocelli-wasm/LICENSE-MIT` (a symlink to it), `packages/core/LICENSE-MIT` and `packages/react/LICENSE-MIT`, and none is under `ci/notices/` or `vendor/`. Replacing "Tensorbee Limited" in each one in a scratch copy turned `licence_check.py holder` red, naming the file. The skip is a prefix on tracked paths, so it cannot hide those files.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `.cargo_vcs_info.json`, `Cargo.toml`, `Cargo.toml.orig` and directory listing of the 16 no-file crates | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f` | now | no licence file in any package. The published commits and declared licences are listed above |
| `git clone --filter=blob:none` of madsmtm/objc2, then `ls-tree` at `b4167b5`, `8852b42`, `7b1abfd`, `8d214f5` and `ee9a7ad` | upstream, 2026-11-05 | now | only `LICENSE.md` at the four published commits. `ee9a7ad` adds three `LICENSE-*.txt` files |
| `git show ee9a7ad`, the `LICENSE.md` diff across commits, licence-file history | objc2 clone | now | the commit changes only links and the manifest. `LICENSE.md` is identical at all four published commits. `cfb19922` deleted `LICENSE.txt` (Steven Sheldon) |
| issue #826 HTML, comment bodies extracted from the embedded JSON | github.com, 2026-11-05 | now | maintainer comments quoted above, dated 2026-10-10 and 2026-10-11 |
| every `sources.toml` URL refetched, sha256 compared with the row and the local file | upstream | now | 16 crate rows and Apache-2.0 equal upstream. MIT and Zlib are templates and differ from upstream as described |
| `diff` of the upstream choosealicense `mit.txt` and zlib v1.3.1 `LICENSE` against the templates | upstream | now | the changes are the ones `sources.toml` describes |
| the GitHub repository page for 9 sources, `"license":{"spdxId"…}` | github.com | now | matches the policy rows |
| choosealicense `README.md` "License" section | upstream gh-pages | now | "The content of this project itself is licensed under the Creative Commons Attribution 3.0 Unported license", see D4 |
| `fill_copyright(Apache-2.0.txt, holder)` diff | staged `licence_check.py:1597` | now | the appendix line `Copyright [yyyy] [name of copyright owner]` is replaced |
| `grep -c -i 'gailly\|Mark Adler' THIRD_PARTY_LICENSES` | staged | now | 0 |
| `grep` for `<copyright-holder>`, `[year]`, `[fullname]` | staged | now | none |
| scratch copy, `rm THIRD_PARTY_LICENSES`, `--write binaries` | scratch copy of the staged and unstaged tree | now | 0. `THIRD_PARTY_LICENSES` and `sources.toml` byte-identical |
| scratch copy, channel 1.93.0, check, `--write`, check, channel back, `--write` | scratch | now | 1, 0, 0, 0. Round trip byte-identical |
| scratch copy, block2 pin removed, channel 1.93.0, `--write` | scratch | now | 1 "is absent". The three files are unchanged |
| S1: `pinned_digest_problems()` replaced by `[]` in `binaries_problems` | scratch `licence_check.py` | now | unit suite FAILED (1). `guard_probe.py --profile deep --only licences.binaries-pin` printed HARNESS "exited 0. The guard it aims at did not fire". It printed OK before the mutation. Restored, `cmp` equal |
| S2 and the pass-1 D1/D2 guards: the refusal raise or test at elect, escape, absent, no-expression, the `--write` section, foreign holder and placeholder each neutralised | scratch | now | the unit suite FAILED and the matching `licences.binaries-*` probe printed FAIL for all 7. Restored after each, `cmp` equal |
| holder mutation on each Tensorbee `LICENSE-MIT` | scratch | now | red, naming each file |
| `python3 scripts/licence_check.py binaries` | worktree | now | 0, OK |
| `python3 -B -m unittest discover -s scripts/tests -p test_licence_check.py` | worktree | now | 0, 63 tests OK |
| `bin/ocelli.sh gate licences` | worktree | now | 0, ALL GREEN |
| `bin/ocelli.sh gate packages` | worktree, existing dist | now | 0, ALL GREEN |
| `bin/ocelli.sh gate guards` | worktree | now | 0. "428 refusal probe(s) drove 37 guard(s) red … 92 control(s) green, 108.7s", ALL GREEN |
| `python3 scripts/guard_census.py`, then with `--check-runbook` | worktree | now | 0 and 0. "1088 refusal(s) in 78 file(s), all claimed", "0 watched by nothing" |
| `bin/ocelli.sh gate ci` | worktree | now | 0. "all 28 floor gate(s) are invoked by CI" |
| `python3 scripts/prose_check.py` | worktree | now | 0, 840 files |
| `python3 scripts/source_provenance_check.py` | worktree | now | 0, 1667 files |
| `gate guards-deep` | | implementer receipt only | not re-run. Its probe `binaries-pin` was run alone, above |
| `git write-tree` and `git diff \| shasum` at the start and the end | worktree | now | `8f0823e0…` and `ccacae1b…` both times. All mutations were made in a scratch copy |

## Defects

### D1, the ten objc2-family notices attribute locked releases to a copyright line added after them

**Where**: `ci/notices/sources.toml:31-35` and its ten `madsmtm/objc2` crate rows. `ci/notices/crates/{block2,dispatch2,objc2,objc2-*}/`. `THIRD_PARTY_LICENSES:1645-1660` and `:2129-2149`. `docs/SOURCE-POLICY.md:121` and `:143-147`. `scripts/tests/test_licence_check.py:942-960`.
**What**: the sections for block2 0.6.2, objc2 0.6.4, objc2-encode 4.1.0, objc2-foundation 0.3.2, dispatch2 0.3.1 and the five framework crates 0.3.2 print "The text below is the licence file from its repository" and then `Copyright 2026 Mads Marquart`. That text comes from `ee9a7ad`. The commit is dated 2026-03-18, after every locked release, and its file was never in the repository at any locked commit. When asked about these exact versions, upstream's maintainer said its copyright notice is "Probably not" sufficient for them, because "there's other contributors than me". Five of the ten sections have an empty `authors:` line, so this line is the only holder they name. Three prose claims are also false. `sources.toml:34` says "LICENSE.md is not a grant", but `LICENSE.md` is the statement that the crates are MIT or trio-licensed, and the maintainer confirmed that the declared licence governs. The policy row's licence column says "grant files", which is not a licence. The generated sentence says the file is from the repository, which is false for the locked version.
**Why it is wrong**: lead amendment 2 requires "a licence file pinned from the crate's own repository at the tag or commit matching its locked version". It falls back to the template "only when the upstream repository carries no licence file". This crate family meets neither condition as implemented. MIT's condition is the "above copyright notice", and a notice that names a holder and year upstream did not attach to that release is a false attribution in a legal file.
**Evidence**: see the ruling above. `ls-tree` at the four published commits, `git show ee9a7ad`, and the #826 comment bodies dated 2026-10-10 and 2026-10-11. `test_objc2_crates_name_their_holder` asserts `"Copyright 2026 Mads Marquart"`, so the test pins the implementation's choice rather than the amendment.
**Scope and regression**: present on the reviewed tree. It affects every shipped binary that includes the Apple-target closure. Fix as the ruling describes: pin `LICENSE.md` at the published commit, apply the template fallback with the recorded reason, correct the sentence, and get a lead decision on Steven Sheldon. Smallest regression: a test asserting that each crate row's URL commit equals the package's published `sha1`, or that the row records a fallback reason (see S1). Also a test that no objc2-family section contains `Copyright 2026`.

### D2, a fallback Apache-2.0 text is modified, and the comment says it is not

**Where**: `scripts/licence_check.py:95-97` and `:1597`. Also the sentence at `:87-90`.
**What**: the comment says "Apache-2.0 stays the published text, whose appendix placeholder is part of that licence". `fill_copyright` replaces `Copyright [yyyy] [name of copyright owner]` in the appendix with `Copyright (c) <holder>`, and `test_a_template_fallback_names_declared_authors_or_the_crate` asserts that replacement. A crate that falls back to Apache-2.0 would ship an Apache text whose "How to apply" appendix has been edited. Its section would still claim "the standard text of the licence it declares", and `sources.toml` would still say "This file is that text". No crate reaches this today.
**Why it is wrong**: a doc comment that is false about the code it describes is a defect under `/microscope` section 3. The sentence beside the text is a claim in the shipped legal file.
**Evidence**: calling `fill_copyright` on the pinned Apache text gives a one-line diff at the appendix line.
**Scope and regression**: latent, because no Apache-only fallback crate exists in the closure today. Choose one behaviour and make the comment, the sentence and the test agree. Either keep the Apache text as published and put the holder line above it, or keep the replacement and drop the "stays the published text" claim.

### D3, the `binaries-pin` probe note states the opposite of what the harness does

**Where**: `scripts/guards/catalogue.py:7760-7763`.
**What**: the note says "dropping pinned_digest_problems from binaries_problems leaves this probe red". In this harness's vocabulary a passing probe is listed as `red` and "drove … guard(s) red". After that mutation the harness instead printed `HARNESS licences.binaries-pin … exited 0. The guard it aims at did not fire` (measured).
**Why it is wrong**: a guard note is a claim about the guard, and `/microscope` section 3 asks for it to be executed. This one reads as the reverse of the behaviour observed.
**Evidence**: the S1 mutation row in the Evidence table.
**Scope and regression**: prose only. Say "turns this probe's harness run red", or describe the observed HARNESS outcome.

### D4, the choosealicense policy row records the code licence, not the licence of the content copied

**Where**: `docs/SOURCE-POLICY.md:117`, `:135-138` and `:156`, and the third answer at `:163-167`.
**What**: the row and answers record github/choosealicense.com as MIT, citing `LICENSE.md` and GitHub's metadata. The repository's own README says "The content of this project itself is licensed under the Creative Commons Attribution 3.0 Unported license, and the underlying source code … is licensed under the MIT license". The file taken from it, `_licenses/mit.txt`, is content.
**Why it is wrong**: SOURCE-POLICY question 3 asks whether the licence permits the specific use, and the answer recorded is for the wrong licence. Lead amendment 3 requires that a source be called only what it is. CC BY 3.0 probably permits the copying with attribution, and the MIT terms themselves are not choosealicense's work, but the register should record which licence applies.
**Evidence**: `curl` of the gh-pages `README.md`, its "## License" section.
**Scope and regression**: a policy-register claim, and no shipped output is affected today, because no crate uses the MIT template. Correct the row's licence column and the three answers. Attribution is already recorded in `sources.toml`.

## Smells

### S1, the amendment's "matching its locked version" rule has no check, and D1 passed every gate

**Where**: `upstream_files` and `crate_pin_rows` in `scripts/licence_check.py`, and the `licences` guard.
**Mechanism**: a pin is keyed on name and version only. The commit in its URL is never compared with anything. The registry cache already holds each package's published `sha1` in `.cargo_vcs_info.json`, offline. The ten objc2-family pins come from a commit that matches none of their releases, and every gate stayed green. The next pin added by hand can repeat this silently. Suggested check: refuse a crate row whose URL commit differs from the package's published `sha1` unless the row records a reason. Add a probe for it.

## Nitpicks

- N1, carried from pass 1. Six `authors: ` lines still have a trailing space (`THIRD_PARTY_LICENSES:1655`, `:2134`, `:2137`, `:2140`, `:2143`, `:2146`).
- N2, carried. `</html>-----END RUST STANDARD LIBRARY NOTICE-----` still shares one line.
- N3, carried. The closure-count test is still not independent of `_rust_shipped`.
- N4, carried. The header still says "reachable on a normal or build edge", and five crates are included only through workspace-unified features.

## Verified clean

- **Pass-1 D1** is closed. No Gailly or Adler line remains. The only holders in no-file sections come from each crate's own repository, apart from the objc2 timing problem in D1. jni-sys-macros's "rust-jni-sys Developers" comes from its own repository, jni-rs/jni-sys.
- **Pass-1 D2** is closed. No unfilled marker remains. Every no-file crate names a holder through its pinned text or its `authors:` line. ndk-sys's upstream `LICENSE-MIT` has no copyright line, so its holder is the authors line "The Rust Windowing contributors", which is upstream's own file faithfully copied.
- **Pass-1 D3** is closed, through the documented CLI route, the round trip and the failure-path replay.
- **Pass-1 D4** is closed as to existence and format. The rows exist in the five-column form dated 2026-11-05, spdx/license-list-data is Read yes and Depend no, and choosealicense is no longer called a steward. The content issues are this pass's D1 and D4.
- **Pass-1 S1** is closed. The probe now runs the entry point, and removing the call turns it and one unit test red.
- **Pass-1 S2** is closed. All five sites, plus the foreign-holder and placeholder refusals, go red in both the unit suite and their probe.
- The holder skip cannot hide a Tensorbee file. The regeneration is byte-identical. All the gates named in the brief are green on the reviewed tree.

## Limitations and handoff

- All mutations ran in a scratch copy of the worktree (rsync with `target` excluded), not the worktree itself. The worktree's tree hash and unstaged diff were unchanged at the end.
- `gate guards-deep` was not run in full. `binaries-pin` ran alone under `--profile deep`.
- The Linux half of AC8 still needs the CI run on the pushed head.
- The Apple SDK redistribution question in objc2's `LICENSE.md` is still an operator matter that the notice cannot settle. The ruling's fix would at least ship upstream's statement of it.
- Next owner: implementation, for D1 to D4 and S1, after a lead decision on whether to include the Steven Sheldon notice. Then a fresh independent pass 3.
