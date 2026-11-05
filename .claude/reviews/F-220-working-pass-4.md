# F-220 review, pass 4

**Reviewed**: staged tree `22fcce32fc43aebc04876992a7969e0c85e15e23` on `work/f-220-xai` (HEAD `c07ef38`), plus the unstaged diff over `.claude/plans/F-220-design.md`, `THIRD_PARTY_LICENSES`, `ci/guard-probe-budget.json`, `docs/runbooks/guard-verification.md`, `scripts/guards/catalogue.py`, `scripts/licence_check.py` and `scripts/tests/test_licence_check.py`. That diff hashed `89d79461…` at the start and at the end of the pass. `git write-tree` gave the same tree at both points. The staged tree is the one pass 3 reviewed, so the pass-3 remediation is entirely unstaged. Untracked `tools/oracle/out` and the pass-1 to pass-3 reports were ignored.
**Author / reviewer**: the remediation was written by an external coding agent (its report is in the session scratchpad, not tracked). This pass was reviewed by a Claude Code subagent that wrote no F-220 code, design, amendment or earlier review.
**Result**: 1 defect, 0 smells, 10 nitpicks

F-220, pass 4: 1 defect, 0 smells, 10 nitpicks. Pass-3 D1, D2, S1 and S2 are closed. The S1 fix introduced a gate-time network fetch, which is the new defect.

## Acceptance challenged

- **Pass-3 D1, CC BY credit in the shipped file.** `THIRD_PARTY_LICENSES` line 6 reads: "Attribution. choosealicense.com file _licenses/mit.txt (https://github.com/github/choosealicense.com/blob/gh-pages/_licenses/mit.txt), published by GitHub, is licensed under the Creative Commons Attribution 3.0 Unported licence (https://creativecommons.org/licenses/by/3.0/). The text was adapted into a template whose copyright line is filled per crate." I checked it against CC BY 3.0 Unported `legalcode.txt` fetched from creativecommons.org. Section 4(a) wants a copy or the URI of the licence: present. Section 4(b)(i) wants the Original Author or Attribution Parties: "GitHub" (see N8). 4(b)(ii) wants the title if supplied: choosealicense.com and the file name. 4(b)(iii) wants the URI: present. 4(b)(iv) wants a credit identifying the adaptation: the last sentence. The choosealicense project has no copyright notice on its content for 4(b) to keep intact. Its `LICENSE.md` notice, "Copyright (c) GitHub, Inc. and contributors", covers the source code. The README "## License" section says the content is CC BY 3.0 and the source code is MIT, so the licence claim is true. A diff of the upstream `_licenses/mit.txt` body against `ci/notices/spdx/MIT.txt` shows only the leading blank line and `[year] [fullname]` replaced by the marker, so "adapted … copyright line" is accurate.
- **Is the Zlib template CC BY content?** No. `ci/notices/spdx/Zlib.txt` keeps two spaces after `warranty.`, which madler/zlib v1.3.1 `LICENSE` has and choosealicense `_licenses/zlib.txt` does not (one space, a `zlib License` title and a `(C) [year] [fullname]` line). The template is the zlib text, dedented, with the holder and sign-off dropped. It needs no CC BY credit, and the paragraph correctly gives none. Apache-2.0 is apache.org verbatim (pass 3).
- **Removing the paragraph goes red.** In a scratch copy I deleted the `f"{_CC_BY_ATTRIBUTION}\n"` header line from `render_notices`. `licence_check.py binaries` exited 1 with "CC BY attribution paragraph is absent" and "does not match the lockfile". `guard_probe.py --profile deep --only licences.binaries-attribution` went HARNESS-red on its control ("exited 1 … on an UNMUTATED sandbox"). Three unit tests failed, including `test_the_attribution_paragraph_is_in_the_header`. The guard watches the emission through its control and the unit suite, and watches `attribution_problems` itself through the refusal probe.
- **Pass-3 D2, the `binaries-pin` note.** In a scratch git copy I replaced `pinned_digest_problems()` with `[]` in `binaries_problems` and ran the probe. The harness printed HARNESS "got the exit status it wanted, but not at the declared refusal: nothing in the output contains 'sha256 is'. A run that went the right way for another reason proves nothing about this guard." The guard output was "FAIL: licence check" and "THIRD_PARTY_LICENSES does not match the lockfile". That is the note, word for word.
- **Pass-3 S1, `[[earlier]]` rows.** In a scratch copy I pointed all four URLs and `commit` fields at `ee9a7ada2131f5944b8750428e265c15632f2a19`, a commit whose tree holds `LICENSE-APACHE.txt`, `LICENSE-MIT.txt`, `LICENSE-ZLIB.txt` and `LICENSE.md` and no `LICENSE.txt`. `binaries` and `--write binaries` both exited 1 with "earlier pin does not match the commit it names" and "block2 0.6.2 LICENSE.txt is absent at commit ee9a7ada…" for all four crates. `--write` left `THIRD_PARTY_LICENSES` unchanged. Changing only one row's URL commit, or only its `commit` field, was refused with "earlier pin URL commit … differs from the commit the row claims …". When I replaced the commit `pin_repository_and_commit` returns with `cfb19922`, the gate went red ("does not match the lockfile") and two unit tests failed, so the shipped sentence is generated from the row. When I removed the `earlier_commit_problems()` call in `render_closure`, `test_binaries_gate_reads_the_earlier_commit` failed, so the wiring is watched. The refusal depends on reading GitHub at gate time, which is D1.
- **Pass-3 S2.** Every one of the ten objc2-family carrier sentences names `https://github.com/madsmtm/objc2` and a full 40-character commit. I compared each with the sha1 in `.cargo_vcs_info.json` in the registry cache: block2 `b4167b58…`, objc2 and dispatch2 `8852b424…`, objc2-encode `8d214f54…`, and the six 0.3.2 crates `7b1abfd7…`. All ten match. The four earlier sentences name `757cd841d0293341dbefd9b99d7548dec244ffde`. `cfb19922` and "at that commit" occur nowhere in the file.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git write-tree`, `git diff \| shasum`, start and end | worktree | now | `22fcce32…` and `89d79461…` both times |
| fetched choosealicense `_licenses/mit.txt`, `_licenses/zlib.txt`, `README.md`, `LICENSE.md`, CC BY 3.0 `legalcode.txt`, zlib v1.3.1 `LICENSE` | upstream, 2026-11-05 | now | as described above |
| pass-3 objc2 clone: `ls-tree` at the four published commits, `show 757cd841:LICENSE.txt` | upstream clone | now | only `LICENSE.md` (sha256 `7f976f7e…`) at each. The earlier file is byte-identical to the pin. No licence-named file in the six registry packages checked |
| scratch: the paragraph's emission removed | scratch copy with its own git | now | gate 1, attribution probe control HARNESS-red, 3 unit failures |
| scratch: `pinned_digest_problems()` dropped, `--only licences.binaries-pin` | scratch | now | HARNESS text identical to the note |
| scratch: earlier rows to `ee9a7ada…` / URL only / commit field only | scratch | now | 1 "is absent at commit" ×4 / 1 differs / 1 differs. `--write` 1, file unchanged |
| scratch: `earlier_commit_problems()` call removed | scratch | now | 1 unit failure |
| scratch: sentence commit forced to `cfb19922` | scratch | now | gate 1, 2 unit failures |
| `https_proxy=http://127.0.0.1:9 python3 scripts/licence_check.py binaries` | worktree | now | **1**, "pin URL https://raw.githubusercontent.com/madsmtm/objc2/757cd841…/LICENSE.txt could not be read: [Errno 61] Connection refused" (D1) |
| the same proxy, unit suite | worktree | now | **FAILED (failures=2, errors=2)**: closure, objc2 holder, tracked-file regeneration, channel round trip (D1) |
| scratch: `rm THIRD_PARTY_LICENSES`, `--write binaries` | scratch | now | 0. `THIRD_PARTY_LICENSES` and `sources.toml` byte-identical to the worktree |
| `python3 scripts/licence_check.py binaries` | worktree | now | 0, OK |
| `python3 -B -m unittest discover -s scripts/tests -p test_licence_check.py` | worktree | now | 0, 70 tests OK |
| `bin/ocelli.sh gate licences` | worktree | now | 0, ALL GREEN |
| `bin/ocelli.sh gate packages` | worktree, existing dist | now | 0, ALL GREEN |
| `bin/ocelli.sh gate guards`, twice | worktree | now | 0. "431 refusal probe(s) drove 37 guard(s) red … 95 control(s) green". `binaries-published-commit`, `binaries-attribution` and `binaries-earlier-commit` red |
| `python3 scripts/guard_census.py`, then with `--check-runbook` | worktree | now | 0 and 0. "1102 refusal(s) in 78 file(s), all claimed", "0 watched by nothing" |
| `bin/ocelli.sh gate ci` | worktree | now | 0, ALL GREEN |
| `python3 scripts/prose_check.py` | worktree | now | 0, 840 files |
| `python3 scripts/source_provenance_check.py` | worktree | now | 0, 1677 files |
| `gate guards-deep` | | implementer receipt only | not re-run in full. `binaries-pin` ran alone under `--profile deep` in scratch |

## Defects

### D1, the binaries gate now fetches from GitHub at gate time

**Where**: `scripts/licence_check.py:1856-1883` (`fetch_raw_pin`, `urllib.request.urlopen` at 1868), called from `earlier_commit_problems` (1886) through `render_closure` (2248). False prose: `ci/notices/sources.toml:3`, `.github/workflows/ci.yml:137-138`, `docs/lld/guards.md:628-629`.
**What**: to close pass-3 S1, `earlier_commit_problems` reads each `[[earlier]]` URL from raw.githubusercontent.com every time `binaries` runs. That covers `gate licences` (a floor gate), `--write binaries`, every sandboxed deep probe that runs the entry point, and four unit tests that call `binaries_problems()` or `generate_third_party_licenses()` with nothing patched. With no route to GitHub, the floor gate and the unit suite go red on an unchanged tree. A GitHub outage or a rate limit (any non-404 status is a `BinariesError`) does the same in CI. A 404 is treated as "absent at commit", so an outage that returns 404 would report a false provenance failure.
**Why it is wrong**: the lead amendment after pass 1 says "Fetching happens only when a pin is added, never at gate time." AC11's rejected failure mode is "a silent network fetch at gate time". This fetch is not silent, but it is still a gate-time fetch, and no amendment after pass 3 changed that rule. `sources.toml:3` still says "Gate time reads these files. It does not fetch.". The CI comment and the guards LLD give "the binaries section reads the registry cache offline" as the reason for `cargo fetch`, and both now describe only half of what that section reads. `/microscope` section 3 treats those sentences as false claims.
**Evidence**: the two proxy rows in the Evidence table. The same commands exit 0 with the network.
**Scope and regression**: present only in the unstaged remediation. It affects `gate licences` locally and in CI, `--write binaries`, the unit suite and the deep probes. The S1 refusal is correct, and the probes inject `blob_at` and never fetch. Two ways to fix it, and choosing between them is the lead's decision. (a) Keep the gate offline. Check the URL commit against the `commit` field, which needs no network and is already the first branch. Move the fetch and digest-at-commit check to the pin-adding path (`--write binaries` or a separate pin-verification mode), and keep a unit test that proves `binaries` opens no socket. (b) Accept a network-dependent floor gate as a recorded amendment, and correct the three sentences above. Under (b) a non-404 failure should say the network is the cause, and the 404 case needs more thought. Smallest regression either way: a unit test that runs `binaries_problems()` with `urllib.request.urlopen` patched to raise, asserting the documented behaviour.

## Smells

None.

## Nitpicks

- N1, carried. Six `authors: ` lines still end in a trailing space (the 0.3.2 group).
- N2, carried. `</html>-----END RUST STANDARD LIBRARY NOTICE-----` still shares one line.
- N3, carried, not re-examined. The closure-count test is not independent of `_rust_shipped`.
- N4, carried. "reachable on a normal or build edge" omits crates added only through workspace-unified features.
- N5, carried, not re-run. A branch URL in a crate pin reports "differs from the published commit absent".
- N6, carried and grown. `docs/lld/guards.md:618-624` lists the binaries probes and omits `binaries-published-commit`, `binaries-attribution` and `binaries-earlier-commit`. The runbook rows 51 to 53 do list them.
- N7, carried. The header does not say what the `sha256:` line digests.
- N8. The paragraph names "GitHub". The project's only notice names "GitHub, Inc. and contributors". This is reasonable under 4(b)'s "reasonable to the medium", but the notice's own wording is the safer choice.
- N9. The `[[earlier]]` `reason` field says "This file is the parent of that commit". A file is not a commit's parent. It does not ship.
- N10. `statement_sentence` hard-codes "the repository root" before the earlier filename. That is true for the one URL today (`…/757cd841…/LICENSE.txt`), but `raw_pin` already parses the path, so a row whose path is in a subdirectory would make the sentence false.

## Verified clean

- The whole header (lines 1 to 9) and all ten objc2-family sections, read end to end. Every sentence is true against upstream: the attribution paragraph, the closure lines, each "neither … carries licence text" (only `LICENSE.md` at each published commit, with no licence file in the packages), each "LICENSE.md from commit X of R" (the digest equals upstream), the earlier sentence (`757cd841:LICENSE.txt` is byte-identical to the pin and names Steven Sheldon), each holder line, and the Sheldon text for exactly the four MIT-only crates.
- Pass-3 D1 is closed. The credit ships, it meets 4(a) and 4(b) as reviewed above, the Zlib template rightly carries no credit, and removing the credit goes red in the gate, a probe control and the unit suite.
- Pass-3 D2 is closed. The note matches the observed harness output word for word.
- Pass-3 S1 is closed as to behaviour. The rows are validated for commit agreement, file presence and digest at the commit, and the sentence comes from the row. The refusal works at `ee9a7ad` and on a one-field change. Its network dependency is D1.
- Pass-3 S2 is closed. The repository and the full commit are in every carrier sentence and match `.cargo_vcs_info.json`.
- Regeneration round trip is byte-identical. All ten requested commands are green with the network.

## Limitations and handoff

- All mutations ran in a scratch copy with its own git repository, under the scratchpad. The worktree's tree hash and unstaged diff are unchanged.
- I simulated no network with an unroutable HTTPS proxy, not with a real offline host. `urllib` honours `https_proxy`, so the failure is the same code path.
- `gate guards-deep` was not run in full. The Linux half of AC8 still needs CI on the pushed head. The Apple SDK question in objc2's `LICENSE.md` is still the operator's.
- No change from pass 3 alters the F-216 cross-story rule.
- Next owner: the lead, to choose (a) or (b) for D1, then implementation, then a fresh independent pass 5.
