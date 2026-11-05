# F-220, `/microscope --working`, pass 6

F-220 pass 6: 3 defects, 0 smells, 13 nitpicks. Not clean.

**Reviewed**: worktree `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-220`, branch `work/f-220-xai`, staged and unstaged, `tools/oracle/out` ignored. Index tree `b55cea92684d35d14fb91f60145cb2fd81c9ec78`, unstaged diff `shasum` `14011d71d4963a76345c3e2816f1098720386b91`, the same at start and end.
**Reviewer**: an independent Claude subagent spawned by the sprint lead. **Author of the changes under review**: the sprint lead (Claude), remediating pass 5 D1 and S1. Implementation is an external coding agent per the operator's assignment.
**Scope**: what changed since pass 5. The index differs from pass 5's tree `22fcce32` only in `ci/notices/sources.toml`, three header lines. The unstaged plan gained "Lead amendments after review pass 4". Each new sentence was checked against `scripts/licence_check.py` and `docs/DEVELOPER_SETUP.md`, and the amendment was read against the plan's decisions, AC1 to AC15 and the pass-1 and pass-2 amendments.

## Pass 5 findings

- **D1, closed.** `ci/notices/sources.toml:4-6` now says verify-upstream re-fetches "every crate pin and every earlier pin" and "does not re-fetch the MIT, Apache-2.0, Zlib or rust-std rows". `upstream_pin_rows()` is `crate_pin_rows()` plus the `[[earlier]]` table, and `verify_upstream_problems` reads nothing else. The four `[[file]]` identifiers are exactly `MIT`, `Apache-2.0`, `Zlib` and `rust-std` (`sources.toml:53,59,65,71`). "It is not a gate" holds: no file in `bin/`, `scripts/guards/`, `.github/` or `ci/` names `verify-upstream` apart from this comment, and `FLOOR_SECTIONS` (`licence_check.py:84`) does not hold it.
- **S1, closed in form, and the new text carries D1 to D3 below.** The amendment exists and records the offline gate and the separate command. Its statement of the limit is wrong.

## Defects

### D1, the amendment says verify-upstream refuses a consistently re-pointed earlier row, and it does not

**Where**: `.claude/plans/F-220-design.md`, "Lead amendments after review pass 4", second bullet: "an earlier row re-pointed at another commit with its file and digest replaced consistently passes the gate, and only `verify-upstream` refuses it."
**What**: `verify_upstream_problems` calls `earlier_commit_problems(rows, blob_at=fetch_raw_pin)` (`licence_check.py:1890-1953`). That compares the bytes at the row's own URL with the row's own sha256. Nothing anywhere checks that the commit is the parent of `cfb19922`, which is what the pass-2 amendment requires ("pinned from the commit before it"). A re-point to any commit whose `LICENSE.txt` matches the replaced file passes verify-upstream as well as the gate.
**Reproduced**, in a scratch copy with its own git repository: the parent of `757cd841` is `03e253f597a79a83ea7f5a9d79c4a7a3433dcd4c`, and `git show 03e253f5:LICENSE.txt | shasum -a 256` is `e353f37b…`, the pinned digest. I replaced `757cd841…` with `03e253f5…` in all four `[[earlier]]` URL and `commit` fields and changed nothing else. `binaries` exited 1 ("THIRD_PARTY_LICENSES does not match the lockfile"). After `--write binaries`, `binaries` exited 0, and `verify-upstream` with the network up printed `OK: licence check (verify-upstream)` and exited 0. The four `reason` fields still say "This file is the parent of that commit", which is now false, and no check reads them. The code path is the same for a commit whose `LICENSE.txt` names a different holder, provided the local file and digest are replaced with those bytes. I did not reproduce that case.
**Why it is wrong**: `/microscope` section 3. Pass 5 tested only re-points to commits where the file is absent ("absent at commit" ×4) and generalised from them. The amendment repeats that generalisation as the design record, so the next reader will believe the parent-of-`cfb19922` rule is enforced somewhere.
**Smallest fix**: the lead's call. Either state the real limit (neither the gate nor verify-upstream checks which commit an earlier row names. verify-upstream refuses only when the bytes at that commit differ from the pin or are absent), or add the missing check. One way is a recorded `before` commit (`cfb19922`) with verify-upstream confirming through the GitHub API that the row's commit is its parent. A regression test would be the `03e253f5` re-point above, which must go red.

### D2, "`docs/DEVELOPER_SETUP.md` says so" is false

**Where**: the same bullet, last sentence.
**What**: `docs/DEVELOPER_SETUP.md:76-95` says that gate time does not fetch, that a crate pin's commit is checked against `.cargo_vcs_info.json`, and that verify-upstream refuses a missing file, "including an earlier row pointed at a commit that has no `LICENSE.txt`". It never says that the offline gate does not check an earlier row's commit, or that a consistent re-point passes it. `grep -n "earlier\|re-point\|consisten" docs/DEVELOPER_SETUP.md` finds only lines 79, 91 and 93, and none of them states the limit. A reader can infer it from what is not said, but that is not "says so".
**Smallest fix**: delete the sentence, or add the corrected limit from D1 to the setup doc and keep it.

### D3, "Every floor section reads only the files pinned under `ci/notices/` and the registry cache, and checks each pinned file against its recorded SHA-256" is false

**Where**: the same amendment, first bullet, second sentence.
**What**: `FLOOR_SECTIONS` is `rust, npm, python, notices, holder, binaries`. `rust_problems` runs `cargo deny --locked … check licenses` over `Cargo.lock` and `deny.toml` (`licence_check.py:1118-1123`). `npm_problems` reads the npm lockfiles, and `notices_problems` reads `THIRD_PARTY_NOTICES`. None of these reads only `ci/notices/` and the registry cache. Only `binaries` checks the pinned digests, through `pinned_digest_problems` in `generate_third_party_licenses`. What is true is narrower: no floor section opens the network, and `binaries` reads the pins and checks each digest. Pass 5's socket-stub runs support the first half.
**Smallest fix**: "No floor section fetches. The `binaries` section reads the pinned files under `ci/notices/` and the registry cache, and checks each pinned file against its recorded SHA-256."

## Smells

None.

## Nitpicks

- N12, new. Plan line 186, "Network once, at implementation, for the pinned texts", is now superseded by the pass-4 amendment's developer-time command. That matches the plan's convention, where amendments supersede the body as the pass-1 amendment does for decision 6's SPDX source, so it is not counted.
- N13, new. "passes the gate" in the amendment leaves out that `THIRD_PARTY_LICENSES` must also be regenerated. The un-regenerated re-point is refused as drift (reproduced above). That is the safe direction, so it is not counted.
- N1 to N11, carried from pass 5 and not re-examined, except N9. The D1 reproduction shows a concrete effect of N9: the `reason` text is unchecked and goes false on a re-point.

## Composition and consistency

- The pass-4 amendment does not contradict the pass-1 amendment ("Fetching happens only when a pin is added, never at gate time"). verify-upstream "runs when a pin is added or refreshed" and is part of no gate. It does not contradict AC5 (a changed byte in a pinned file is refused on its digest), AC11 (offline, `cargo fetch --locked` named), or decision 9.
- It contradicts the pass-2 amendment only through D1. Pass 2 fixes the commit, and pass 4 claims a check for it that does not exist.

## Generated files and digests

- `THIRD_PARTY_LICENSES` is unchanged in the index between pass 5's tree `22fcce32` and `b55cea92` (`git diff --stat 22fcce32 b55cea92` lists only `sources.toml`). The working copy equals the generator's output: `binaries` exit 0 in the worktree and in a fresh scratch copy.
- Nothing hashes `sources.toml` itself. `grep` finds no reference in `ci/guard-probe-budget.json` or `docs/runbooks/guard-verification.md`, and `THIRD_PARTY_LICENSES` names no `sources.toml` digest. The header comment changes no generated byte.

## Evidence

| Command | Input | Now / prior | Exit and observation |
|---------|-------|-------------|----------------------|
| `git write-tree`, `git diff \| shasum`, start and end | worktree | now | `b55cea92…`, `14011d71…` both times |
| `python3 scripts/licence_check.py binaries` | worktree, `CARGO_TARGET_DIR=<wt>/target` | now | 0, OK |
| `python3 -B -m unittest discover -s scripts/tests -p test_licence_check.py` | worktree | now | 0, 72 OK |
| `bin/ocelli.sh gate licences` | worktree | now | 0, ALL GREEN |
| `python3 scripts/guard_census.py` | worktree | now | 0, "1102 refusal(s) in 78 file(s), all claimed", "0 watched by nothing" |
| `python3 scripts/guard_census.py --check-runbook` | worktree | now | 0 |
| `python3 scripts/prose_check.py` | worktree | now | 0, 840 files |
| `python3 scripts/source_provenance_check.py` | worktree | now | 0, 1677 files |
| `python3 scripts/licence_check.py verify-upstream`, network up | worktree | now | 0, OK |
| objc2 blobless clone: `git rev-parse 757cd841^`, `git show 757cd841^:LICENSE.txt \| shasum -a 256` | upstream | now | `03e253f5…`, `e353f37b…` |
| scratch: four earlier rows re-pointed to `03e253f5`, `binaries`, `--write binaries`, `binaries`, `verify-upstream` | scratch copy, own git | now | 1 drift / 0 / 0 / 0 |
| `gate guards`, `gate guards-deep`, `gate ci` | | pass 5 receipts only | not re-run, since no code, probe or budget changed since pass 5 |

## Limitations and handoff

- All mutations ran in a scratch copy under the session scratchpad. The worktree's index tree and unstaged diff are unchanged, and only this report was written.
- The different-holder variant of D1 follows from the code path and was not reproduced.
- Next owner: the lead, for the amendment text (D1 to D3) and for deciding whether D1 is closed by prose or by a check. Then an independent pass 7.
