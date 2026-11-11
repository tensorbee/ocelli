# F-220 review, pass 1

**Reviewed**: staged tree `f56f2165a532edc40949fc5ea7f0d101e8082018` on `work/f-220-xai`, base `c07ef382b174dce3f5e605877175310aa58307a3` (`git merge-base HEAD sprint/s46`). No unstaged tracked paths. Untracked `tools/oracle/out` ignored as instructed.
**Author / reviewer**: implementation by an external coding agent (its report is in the session scratchpad, not tracked). Reviewed independently by a Claude Code subagent that wrote none of the F-220 code or design.
**Result**: 4 defects, 2 smells, 4 nitpicks

## Acceptance challenged

- **AC2, closure.** I worked out the closure on my own with `cargo tree --locked --offline -e normal,build`, once for `ocelli-wasm` on `--target wasm32-unknown-unknown` and once for `ocelli-native` on `--target all`. Workspace members were removed and the union is 124 crates. The index lists 129. Every crate in my set is in the index, so nothing is missing. The index has five extras: `dispatch2`, `objc2-io-surface`, `termcolor`, `winapi-util` and `windows-sys 0.61.2`. They come from cargo metadata unifying features across the whole workspace. For example, `codespan-reporting -> termcolor` and `objc2-core-foundation -> dispatch2` are edges in the workspace-unified resolve. Listing too many crates is safe for compliance, and plan decision 2 accepts it. `proptest` is absent.
- **AC4, the election and the 16 crates with no licence file.** The election is correct. The texts it pulls in for those crates are wrong, see D1 and D2.
- **AC6 and AC7.** I bumped `log` 0.4.34 to 0.4.35 with `cargo update --precise --offline` and the gate went red: "THIRD_PARTY_LICENSES does not match the lockfile". I changed `rust-toolchain.toml` to 1.98.0 and the check refused with the rust-docs instruction. Both checks hold. The *repair* route that the docs name for a toolchain change is broken, see D3.
- **AC1 and AC8, macOS half.** I deleted the file in a scratch copy at a different absolute path and ran `--write` twice. Both runs were byte-identical to the tracked file. The file has no CR, no `/Users/`, no `/home/`, no `.cargo/registry`, no `ocelli-` crate and no Tensorbee.
- **AC12.** I replaced the wasm rule's `if wasm:` with `if False:` and the probe `packages.wasm-without-notices` went red. I then dropped `THIRD_PARTY_NOTICES` from the required pair and the probe went red again. After restoring, it was green.
- **AC5.** I removed the `pinned_digest_problems()` call from `binaries_problems`. The probe and all 49 unit tests stayed green, see S1.
- **Pinned-text provenance.** I fetched each upstream file again and recomputed its SHA-256. All four pins match their recorded digests, and each upstream file hashes to the recorded digest. The digests are not the problem. What the texts *say* is, see D1, D2 and D4.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `shasum -a 256 ci/notices/spdx/* ci/notices/rust-std/*` | staged tree | now | all four equal `sources.toml` |
| `curl` madler/zlib v1.3.1 LICENSE, apache.org LICENSE-2.0.txt, choosealicense `_licenses/mit.txt` | upstream, 2026-11-05 | now | `845efc77…`, `cfc7749b…`, and `002c2696…` for the MIT body after the front matter with the leading blank line stripped. All match |
| `curl` spdx/license-list-data v3.29.0 `text/Zlib.txt` and `LICENSE` | upstream | now | the SPDX Zlib text has no copyright line. Root `LICENSE` returns 404 |
| `cmp` pinned COPYRIGHT-library.html against `~/.rustup/toolchains/1.97.1-aarch64-apple-darwin/share/doc/rust/` | toolchain 1.97.1 | now | identical |
| independent closure, `cargo tree` union compared with the index | Cargo.lock at staged tree | now | 124 against 129, no crate missing, 5 over-included |
| hand check of 12 crates against `~/.cargo/registry/src/index.crates.io-*` (bytemuck, memchr, unicode-width COPYRIGHT, khronos-egl `/`, zerocopy BSD OR, naga `.MIT`, windows lowercase, cfg_aliases NOTICES.md, libloading ISC, unicode-ident AND, foldhash, rustc-hash `/`) | registry cache | now | every file present, normalised bytes equal, carriers correct |
| no-file crates spirv, ndk-sys, objc2-metal, profiling | index | now | Apache-2.0, MIT, Zlib, MIT. The election is correct |
| `python3 scripts/licence_check.py binaries` | staged | now | 0, OK |
| `python3 -B -m unittest discover -s scripts/tests -p test_licence_check.py` | staged | now | 49 tests OK |
| mutation: `elect_terms` allowlist removed | licence_check.py:1384 | now | unit test ERROR (red). Restored |
| mutation: digest call dropped from `binaries_problems` | :1695 | now | 49 OK, probe `licences.binaries-pin` green. Restored |
| mutation: "escapes the crate root", "no licence file and no licence expression", "has no allowlisted licence" changed to `return []`, "licence file absent", `--write` section guard | :1386, :1410, :1415, render, main | now | 49 OK, `binaries-no-text` green. Restored |
| `cargo update -p log --precise 0.4.35 --offline`, then `binaries` | Cargo.lock | now | 1, drift message. Restored |
| channel changed to 1.98.0, then `binaries` | rust-toolchain.toml | now | 1, rust-docs instruction. Restored |
| scratch copy, channel changed to 1.93.0 (rust-docs installed), `--write binaries` | scratch | now | 1, "block2 0.6.2 has no pinned text for MIT". `sources.toml` corrupted, D3 |
| package rule mutations, two of them | package_check.py | now | probe red both times. Restored |
| `bin/ocelli.sh gate licences` | staged | now | 0 ALL GREEN |
| `bin/ocelli.sh gate packages` | staged, built dist from implementer | now | 0 ALL GREEN |
| `bin/ocelli.sh gate guards` | staged | now | 0 |
| `bin/ocelli.sh gate ci` | staged | now | 0 ALL GREEN |
| `guard_census.py`, `--check-runbook` | staged | now | 0, "1081 refusal(s) in 78 file(s), all claimed" |
| `prose_check.py`, `source_provenance_check.py` | staged | now | 0, 840 files and 1667 files |
| `gate guards-deep` | | implementer receipt only | not re-run |
| `git write-tree` after every mutation | | now | `f56f2165…` each time, `git diff --stat` empty |

## Defects

### D1, the pinned "Zlib" text attributes six crates to Jean-loup Gailly and Mark Adler

**Where**: `ci/notices/spdx/Zlib.txt:1-22`, and `THIRD_PARTY_LICENSES:2156` and `:2174`.
**What**: the pin is zlib's own `LICENSE` file. That file is the zlib *library's* grant, and its copyright line reads " (C) 1995-2022 Jean-loup Gailly and Mark Adler". It ends with their sign-off and email addresses. `dispatch2`, `objc2-core-foundation`, `objc2-core-graphics`, `objc2-io-surface`, `objc2-metal` and `objc2-quartz-core` elect Zlib, so the shipped notice states that those six crates are copyright Gailly and Adler. That is a false attribution in a legal notice. Four of the six also print an empty `authors:` line, so no other holder appears anywhere in their sections.
**Why it is wrong**: plan decision 5 says a crate with no licence file gets "the standard text of the licence it declares", and the generated sentence beside the text says the same. The SPDX `Zlib` text (license-list-data v3.29.0 `text/Zlib.txt`, fetched now) starts "zlib License" and has no copyright line at all. The pinned file is not the licence the identifier means. It is one licensee's instance of it. `sources.toml` describes it as "each steward's" text, which is also false for this file.
**Evidence**: `grep -n Jean-loup THIRD_PARTY_LICENSES` gives 2156 and 2174, inside the sha256 `845efc77…` block whose carriers are the six crates above.
**Scope and regression**: present on the staged tree. The fix is a Zlib text with no holder line. Its provenance must pass SOURCE-POLICY, see D4. Smallest regression: a unit test asserting that no pinned standard text names a copyright holder, plus a hash change caught by the existing digest check.

### D2, MIT and Zlib standard texts ship unfilled placeholders, and six crates get no copyright holder at all

**Where**: `ci/notices/spdx/MIT.txt:3` ("Copyright (c) [year] [fullname]"), `THIRD_PARTY_LICENSES:1647-1669`, and `_authors_line` in `scripts/licence_check.py`.
**What**: the MIT copyright line is left as the choosealicense template. MIT's only condition is that "the above copyright notice and this permission notice shall be included". For seven crates the "above copyright notice" in the shipped file is literally `[year] [fullname]`. The holder appears only on a separate `authors:` line drawn from Cargo metadata. `objc2-foundation` and four Zlib crates have no `authors`, so their sections name nobody. Upstream does carry a notice. `madsmtm/objc2` `LICENSE-MIT.txt` (fetched now) reads "Copyright 2026 Mads Marquart", and the crate README's licence badge links `../../LICENSE.md`. The generator's own sentence calls the result "the standard text of the licence it declares", and a template with placeholders is not that.
**Why it is wrong**: the MIT condition quoted in the plan's "licence conditions being met" section. README.md:398 claims "the full licence texts", which is false for these sections.
**Evidence**: `sed -n 1645,1670p THIRD_PARTY_LICENSES`. Also `grep -n 'authors: $' THIRD_PARTY_LICENSES` lists `objc2-foundation`, `objc2-core-foundation`, `objc2-core-graphics`, `objc2-io-surface`, `objc2-metal` and `objc2-quartz-core`. The last five carry the Zlib text from D1.
**Scope and regression**: present. This is a decision the plan left implicit, and question 3's answer ("with the declared authors") does not cover empty authors. A fix needs a lead decision. Two options are pinning the upstream repository's licence file per crate family, with provenance recorded, or refusing a no-file crate whose authors are empty. Regression: a unit test with a no-file crate and `authors=()` that must refuse, or must render a recorded holder.

### D3, `--write binaries` after a toolchain change corrupts `ci/notices/sources.toml`

**Where**: `scripts/licence_check.py:1740`, `_SOURCES.write_text(head + "[[file]]".join(rewritten), ...)`.
**What**: `head, *blocks = text.split("[[file]]")` and then `head + "[[file]]".join(blocks)` loses the first `[[file]]` header. The MIT row's keys fall into the top-level table, so the MIT pin disappears from the `file` array, and with it the MIT digest check. Generation then fails, and the corrupted `sources.toml` and the copied new HTML are left on disk.
**Why it is wrong**: this is the exact route `docs/DEVELOPER_SETUP.md` ("Shipped binary notices"), README.md:400 and the refusal message tell a developer to run after a channel change. Plan decision 7 and the "Scope" section say `--write` regenerates after a toolchain change.
**Evidence**: in a scratch copy of the staged tree, with the channel changed to 1.93.0 (rust-docs installed), `python3 scripts/licence_check.py --write binaries` exits 1 with "shipped binary notices: block2 0.6.2 has no pinned text for MIT". `diff` shows line 20's `[[file]]` replaced by an empty line, and `_standard_texts()` returns `['apache-2.0', 'zlib']`.
**Scope and regression**: present, on the refresh path only. The check path is unaffected. No test or probe exercises `refresh_stale_rust_std` or `_update_rust_std_pin`, which is why this survived. Fix: `"[[file]]".join([head, *rewritten])`, or a targeted substitution. Write nothing until generation has succeeded. Regression: a unit test that calls `_update_rust_std_pin` on a copy and asserts that `tomllib` still yields four `file` rows.

### D4, five newly assessed sources were taken without the rows `docs/SOURCE-POLICY.md` requires

**Where**: `docs/SOURCE-POLICY.md` "Extensions to the table" (line 97), which is unchanged, and `ci/notices/sources.toml:4-18`.
**What**: the policy is normative. It says "Before taking anything from a source not in the table above, check three things and record the answer", and "A source assessed after this policy was written gets a row here, in the same form as the table above, with the date it was decided." F-220 assessed SPDX license-list-data and refused it. It copied text from github/choosealicense.com, apache.org and madler/zlib, and copied the rust-docs `COPYRIGHT-library.html`. None of these has a row. The implementer's report says "`docs/SOURCE-POLICY.md` was not edited". `sources.toml` comments are not the register the policy names. They also describe choosealicense as a steward, which it is not. The MIT licence has no steward, and choosealicense is GitHub's presentation of it with its own placeholders.
**Why it is wrong**: `docs/SOURCE-POLICY.md` "The rule for anything not yet listed" and "Extensions to the table". F-217 recorded its sources there, for example the openjph-core notice row dated 2026-11-05.
**Evidence**: `git diff --cached --stat` does not include `docs/SOURCE-POLICY.md`. `grep -n 'choosealicense\|madler\|license-list-data' docs/SOURCE-POLICY.md` returns nothing.
**Scope and regression**: present. `source_provenance_check.py` does not catch it, because it checks dependencies and blocked names rather than this register. Fix: add the rows, with the three answers, in the same change that replaces the D1 and D2 texts.

## Smells

### S1, the AC5 digest refusal is not watched at the gate entry point

**Where**: `binaries_problems`, `scripts/licence_check.py:1695`, probe `licences.binaries-pin`, and `test_a_changed_pin_byte_is_refused_on_its_sha256`.
**Mechanism**: the probe and the test both call `pinned_digest_problems(overrides)` directly through a seam that exists only for them. Deleting the call from `binaries_problems` leaves all 49 tests and the probe green (measured). Today a changed pin still goes red through drift, because the text is embedded. A pin that the current closure does not use, or a later refactor that drops the call, would pass silently. The probe should mutate the file in the sandbox and run `licence_check.py binaries`, the way `binaries-drift` does.

### S2, several new refusal sites have no test or probe, including one whose removal produces exactly AC4's rejected behaviour

**Where**: `elect_terms` "has no allowlisted licence in" (:1386), `collect_licence_files` "escapes the crate root" (:1410) and "licence file … is absent" (:1415), `render_notices` "has no licence file and no licence expression", and `main`'s "--write regenerates the binaries section only".
**Mechanism**: each was mutated away (the first to `return []`) and every unit test plus `binaries-no-text` stayed green. The `return []` mutation drops the crate's text while keeping it in the index, which is the "silently omitting a crate with no file" that AC4 rejects. AC14's rejected implementation is "a refusal site with no probe". `claims=("*",)` keeps the census quiet, but it does not watch these sites.

## Nitpicks

- N1. An empty authors list renders as `authors: ` with a trailing space (`THIRD_PARTY_LICENSES:1665` and others). D2 is the substantive part.
- N2. The pinned HTML has no final newline, so the end marker lands on the same line as the HTML: `</html>-----END RUST STANDARD LIBRARY NOTICE-----` (line 11491). The bytes are correct. It just reads oddly.
- N3. `test_closure_lists_each_shipped_crate_once` hard-codes `129`, and it goes red on every legitimate lockfile change. It is also not independent, because it compares the render with `_rust_shipped`, the same provider. An independent `cargo tree` comparison would be the discriminating test.
- N4. The header says "reachable on a normal or build edge from … the native entry point". Five listed crates are reachable only through metadata's workspace-unified features (see AC2 above). Plan decision 2 accepts this, so a clause saying so would keep the header literally true.

## Verified clean

- All four pinned digests match the files and their upstream sources. The std notice is byte-identical to the 1.97.1 toolchain's `COPYRIGHT-library.html`, and the toolchain in `sources.toml` equals `rust-toolchain.toml`'s channel.
- The Apache-2.0 pin is the ASF canonical text. Its appendix placeholder is part of the licence as published.
- The closure has no under-inclusion against an independent `cargo tree` union. Workspace members, proptest and Tensorbee are absent.
- The file rule, checked against the registry for 12 crates: `LICENSE-*`, `LICENSE.*`, lowercase `license-*`, `COPYING`, `UNLICENSE`, `COPYRIGHT` and `NOTICES.md` are all collected, with normalised bytes equal to the source and carriers correct. No crate in the closure has a `LICENSES/` directory, so that branch is covered only by the synthetic AC3 test.
- The election: `/` read as OR (khronos-egl, rustc-hash, version_check), left-most allowlisted disjunct, and the implementer's mutation replayed red.
- Determinism and path independence, by regenerating in a copy at another path.
- AC6 and AC7 refusals on real inputs. AC11 was not re-run, but the implementer's receipt and the unit test exist, and the unit suite passed.
- The `package_check.py` wasm rule and its level-1 probe, red under two mutations. `gate packages` is green on real tarballs.
- CI: `cargo fetch --locked` sits in the `guards` job after the toolchain install and immediately before `gate licences`, and before `gate guards` and `gate guards-deep`. No other CI job runs `gate licences`. `gate ci` is green.
- Docs: `LICENSE`, `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE` and `THIRD_PARTY_NOTICES` all exist as RELEASE.md names them. The guards LLD probe levels match the catalogue. The README and DEVELOPER_SETUP commands exist, but the toolchain half is broken by D3 and "full licence texts" is qualified by D2. `use.html` and `more.html` need no change.

## Limitations and handoff

- I did not re-run `gate guards-deep`. The two deep probes were exercised by direct mutation instead.
- AC8's Linux half needs the CI `guards` run on the pushed head. It does not exist yet.
- The GitHub API licence lookup for spdx/license-list-data returned no JSON (likely rate-limited), so "GitHub reports no licence" was not independently confirmed. The 404 for the root `LICENSE` at v3.29.0 was confirmed.
- Not in F-220's scope, but worth raising with the operator: objc2's upstream `LICENSE.md` states that the crates are derived from Apple SDKs and that redistribution under the Xcode licence is unclear. That is a licensing question the notice file cannot answer.
- Next owner: implementation, to fix D1 to D4 and S1 to S2. D2 needs a lead decision on how to handle crates with no file and no authors. Then a fresh independent pass 2.
