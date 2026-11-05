# F-220, `/microscope --working`, pass 7

F-220 pass 7: 1 defect, 0 smells, 1 nitpick. Not clean.

**Reviewed**: worktree `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-220`, branch `work/f-220-xai`, staged and unstaged, `tools/oracle/out` ignored. Index tree `b020b15c96cac19b44e4baa0224aaeb4472415ee`, unstaged diff `shasum` `2e7a828214fdadc34dd60ad516bcdc5fcd0e407b`, the same at start and end. The index differs from pass 6's index `b55cea92` only in `.claude/plans/F-220-design.md` (`git diff b55cea92 b020b15c --stat`), which is now fully staged with the rewritten section.
**Reviewer**: an independent Claude subagent spawned by the sprint lead. **Author of the change under review**: the sprint lead (Claude), rewriting "Lead amendments after review pass 4" after pass 6. Implementation is an external coding agent per the operator's assignment. This reviewer wrote none of it.
**Scope**: the rewritten section's three bullets, executed against `scripts/licence_check.py` in the working tree, its consistency with the rest of the plan, `docs/DEVELOPER_SETUP.md` and the `ci/notices/sources.toml` header, and whether the rewrite touched anything else.

## Pass 6 findings

- **D1, closed.** The re-point claim is gone. The third bullet now states the limit, and it holds (reproduced below).
- **D2, closed.** The sentence "`docs/DEVELOPER_SETUP.md` says so" is gone. Nothing in the section now cites the setup doc.
- **D3, closed in its own terms.** The "reads only the files pinned under `ci/notices/`" sentence is gone and `binaries` is named as the section that checks digests. Its replacement, "No floor section fetches", is false, which is D1 below.

## Defects

### D1, "The gate stays offline. No floor section fetches." is false

**Where**: `.claude/plans/F-220-design.md:270`, the first bullet's heading and first sentence.
**What**: `FLOOR_SECTIONS` (`licence_check.py:84`) includes `rust`. `rust_problems` (`licence_check.py:1118-1123`) runs `cargo deny --locked --format json check licenses` with no `--offline`. cargo-deny 0.19.0 resolves metadata through cargo, and with a cold registry cache it downloads the crates. Only `binaries` uses `cargo metadata --locked --offline` (`licence_check.py:1158-1167`).
**Evidence**, executed now:
- `CARGO_HOME=<scratch>/coldcargo cargo deny --locked check licenses` exited 0, printed `licenses ok`, and left 340M of `registry/{cache,index,src}` in the empty directory.
- With a second empty `CARGO_HOME`: `licence_check.py binaries` alone exited 1, naming `cargo fetch --locked`, as AC11 asks. Then `licence_check.py` with no section, which is what `gate licences` runs, exited 0 with `OK: licence check (rust, npm, python, notices, holder, binaries)` and the directory held 340M. Then `binaries` alone exited 0. The `rust` section fetched the registry and `binaries` passed on what it fetched.
**Why it is wrong**: `/microscope` section 3. It is also the wrong behaviour AC11 names, "a silent network fetch at gate time". AC11's test covers `binaries` alone, so it does not see this route through `gate licences`. In CI the `guards` job runs `cargo fetch --locked` first (`.github/workflows/ci.yml:139`), so CI does not see it either. `test_the_binaries_gate_does_not_fetch` patches only Python `urlopen` and `fetch_raw_pin`, and a subprocess does not go through either.
**Scope and regression**: the command `cargo deny --locked` is unchanged since HEAD (`git show HEAD:scripts/licence_check.py`, line 1044), so the behaviour is not new. What is new is the claim. A reader of the design record will believe `gate licences` cannot touch the network. The smallest fix is the lead's call. Either narrow the sentence to what holds ("No floor section fetches a pin. `binaries` runs `cargo metadata --offline`. The `rust` section's `cargo deny` is not offline and fills a cold registry cache"), or pass `--offline` to `cargo deny` (its `--help` lists `--offline`, "Run without accessing the network") and add a regression that runs the whole floor with an empty `CARGO_HOME` and expects exit 1. `docs/DEVELOPER_SETUP.md:81` ("Gate time reads those files and checks each sha256. It does not fetch.") and the `sources.toml:3` header ("Gate time reads these files. It does not fetch.") are about the pinned files in their own context, and are true read that way. They would be false read as the gate as a whole.

## Smells

None.

## Nitpicks

- N14, new. The third bullet's "passes both the gate and `verify-upstream`" leaves out that `THIRD_PARTY_LICENSES` must be regenerated, because the shipped sentence now names the commit. That is pass 6's N13, still there. Not counted.
- N1 to N13, carried and not re-examined. N9 (the `reason` text is unchecked) still applies. `sources.toml:246` says "This file is the parent of that commit", and after the re-point below it is false and nothing reads it.

## Acceptance challenged

- **Bullet 1, second sentence.** `binaries` checks every pinned file's digest: `digest_rows` is the `file`, `crate` and `earlier` tables, and every file under `ci/notices/` except `sources.toml` is some row's `path` (checked by a loop over `find ci/notices -type f`). One appended byte in `crates/profiling-1.0.18/LICENSE-MIT` was refused on its sha256, and so was one in `crates/objc2-earlier/LICENSE.txt` (the message is printed twice, once by `pinned_digest_problems` and once by `earlier_commit_problems`). Changing the profiling pin's URL commit to `00000001…` was refused: "differs from the published commit 82715511…". Holds.
- **Bullet 2.** `verify_upstream_problems` runs `earlier_commit_problems(upstream_pin_rows(), blob_at=fetch_raw_pin)`, and `upstream_pin_rows` is the crate pins plus the `[[earlier]]` table. It fetches the recorded URL, whose commit must equal the recorded `commit`. A changed byte with the digest updated to match and `--write` run passed `binaries`. `verify-upstream` then exited 1 with "LICENSE-MIT at commit 82715511… sha256 is c8167fde…, expected a826b544…". A URL path renamed to `LICENSE-NOPE` passed `binaries`, and `verify-upstream` exited 1 with "LICENSE-NOPE is absent at commit 82715511…". `binaries verify-upstream` together exits 1 with "verify-upstream runs alone". Outside `licence_check.py` and its tests, `verify-upstream` is named only in `sources.toml` and `docs/DEVELOPER_SETUP.md`, and it is not in `FLOOR_SECTIONS`, `bin/`, `scripts/guards/`, `.github/` or `.claude/commands/`. "Part of no gate" holds.
- **Bullet 3.** In a fresh blobless clone of `madsmtm/objc2`, `cfb19922^` is `757cd841`, so today's pin is the parent. `757cd841^` is `03e253f5`, and its `LICENSE.txt` digest is `e353f37b…`, the same as the pin. In a scratch copy with its own git repository I replaced `757cd841…` with `03e253f5…` throughout `sources.toml`, eight lines, and nothing else. Results: `binaries` exit 1 (drift), `--write binaries` exit 0, `binaries` exit 0, `verify-upstream` with the network up exit 0, the whole floor exit 0, the 72 unit tests OK, and `bin/ocelli.sh gate licences` ALL GREEN. `THIRD_PARTY_LICENSES` now names `03e253f5` four times. The only `cfb19922` references in code are two `assertNotIn` lines in the tests, and no code compares commits against `cfb19922` or its parent. Holds.

## Composition and consistency

- **Rewrite scope.** `git diff HEAD -- .claude/plans/F-220-design.md` adds only lines 247 to 272: the pass-1, pass-2 and pass-4 sections. Nothing above line 247 changed. The pass-1 and pass-2 bullets read the same as the text pass 6 quoted ("Fetching happens only when a pin is added, never at gate time", "pinned from the commit before it"). The pass-4 header now says "and corrected after pass 6".
- **Against other amendments.** Bullet 3 sits beside the pass-2 rule ("pinned from the commit before it") and records that nothing enforces that rule. It does not contradict it. Bullets 1 and 2 agree with pass 1 ("never at gate time" for pins) and with pass 2 (the gate checks the published commit offline).
- **Against the body.** Decision 6 ("The gate verifies every digest"), decision 9, AC5 and AC14 agree. AC11 is contradicted only through D1. Plan line 186 is superseded by bullet 2, as pass 6's N12 says.
- **`docs/DEVELOPER_SETUP.md:76-95`** agrees with bullets 2 and 3: every crate pin and earlier pin, recorded URL and commit, refuses a difference or a missing file, not part of `gate licences`. It does not state bullet 3's limit, and the plan no longer says it does. **`sources.toml:1-6`** agrees: every crate pin and earlier pin, not the four `[[file]]` rows, not a gate.

## Evidence

| Command | Input | Now / prior | Exit and observation |
|---------|-------|-------------|----------------------|
| `git write-tree`, `git diff \| shasum`, start and end | worktree | now | `b020b15c…`, `2e7a8282…` both times |
| `python3 scripts/licence_check.py binaries` | worktree, `CARGO_TARGET_DIR=<wt>/target` | now | 0, OK |
| `python3 -B -m unittest discover -s scripts/tests -p test_licence_check.py` | worktree | now | 0, 72 OK |
| `bin/ocelli.sh gate licences` | worktree | now | 0, ALL GREEN |
| `python3 scripts/prose_check.py` | worktree | now | 0, 840 files |
| `python3 scripts/source_provenance_check.py` | worktree | now | 0, 1677 files |
| `cargo deny --locked check licenses`, empty `CARGO_HOME` | worktree | now | 0, 340M downloaded |
| `binaries`, then the floor, then `binaries`, second empty `CARGO_HOME` | worktree | now | 1 / 0 / 0, 340M downloaded by the floor |
| objc2 blobless clone, `rev-parse 757cd841^ cfb19922^`, `show …:LICENSE.txt \| shasum` | upstream | now | `03e253f5…`, `757cd841…`, both `e353f37b…` |
| scratch re-point to `03e253f5`: `binaries`, `--write`, `binaries`, `verify-upstream`, floor, unit tests, `gate licences` | scratch copy, own git | now | 1 / 0 / 0 / 0 / 0 / OK / green |
| scratch mutations: crate pin byte, earlier byte, crate pin commit, consistent byte change, missing upstream path, `binaries verify-upstream` | scratch copy | now | each refused as described above |
| `gate guards`, `guards-deep`, `ci`, `guard_census` | | pass 5 and 6 receipts only | not re-run. No code, probe or budget changed since pass 6 |

## Verified clean

All three bullets were executed rather than read. Bullet 1's second sentence, bullet 2 and bullet 3 hold. The rewrite replaced only the pass-4 section. The section contradicts no other amendment or AC except AC11, through D1. The setup doc and the `sources.toml` header agree with bullets 2 and 3. The section contains no em-dash and no semicolon.

## Limitations and handoff

- All mutations and cold-cache runs used scratch directories under the session scratchpad. The two empty `CARGO_HOME` directories were deleted afterwards. The worktree's index tree and unstaged diff are unchanged, and only this report was written.
- I did not watch whether a warm-cache `cargo deny` opens a connection. D1 rests on the cold-cache download, which was reproduced.
- Next owner: the lead, for D1. Narrow the sentence or make the `rust` section offline. Then an independent pass 8.
