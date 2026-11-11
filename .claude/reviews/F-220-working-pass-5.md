# F-220 review, pass 5

**Reviewed**: staged tree `22fcce32fc43aebc04876992a7969e0c85e15e23` on `work/f-220-xai` (HEAD `c07ef382b174dce3f5e605877175310aa58307a3`), plus the unstaged diff over `.claude/plans/F-220-design.md`, `THIRD_PARTY_LICENSES`, `ci/guard-probe-budget.json`, `ci/notices/sources.toml`, `docs/DEVELOPER_SETUP.md`, `docs/runbooks/guard-verification.md`, `scripts/guards/catalogue.py`, `scripts/licence_check.py` and `scripts/tests/test_licence_check.py`. `git diff | shasum` gave `2fa681bb6637fe83bab6a5142eb912b87eaa081f` at the start and at the end of the pass, and `git write-tree` gave the same tree both times. Untracked `tools/oracle/out` and the pass-1 to pass-4 reports were ignored.
**Author / reviewer**: the pass-4 remediation was written by an external coding agent (its report is in the session scratchpad, not tracked). This pass was reviewed by a Claude Code subagent that wrote no F-220 code, design, amendment or earlier review.
**Result**: 1 defect, 1 smell, 10 nitpicks

F-220, pass 5: 1 defect, 1 smell, 10 nitpicks. Pass-4 D1 is closed. The gate is offline. What remains is one overbroad sentence in `sources.toml` and a lead decision that the plan does not record.

## Acceptance challenged

- **Pass-4 D1, the gate is offline.** I ran it three ways. (1) `https_proxy`, `http_proxy`, `HTTPS_PROXY` and `HTTP_PROXY` all set to `http://127.0.0.1:9`. (2) A reviewer `sitecustomize.py` on `PYTHONPATH` that replaces `socket.getaddrinfo`, `socket.create_connection`, `socket.socket.connect` and `connect_ex` with a function that writes `NONET` to stderr and raises. I checked the stub first: `urlopen('https://example.com')` raised `URLError … NONET`. (3) Both together. With the stub, `licence_check.py binaries`, the default floor set, the 72-test unit suite and `bin/ocelli.sh gate licences` all exited 0. No `NONET` line appeared anywhere. The 15 `licences.binaries-*` probes under `--profile deep`, run with the stub and the proxy, gave "15 refusal probe(s) … 13 control(s) green", exit 0. `--write binaries` in a scratch copy with the stub exited 0, so plan line 95 ("never reaches the network itself") holds.
- **Search of the gate path.** In `scripts/licence_check.py`, `urllib` appears only in `fetch_raw_pin` (1859-1887). Its only caller is `verify_upstream_problems` (1983). `render_closure` calls `earlier_commit_problems()` with no `blob_at`, so that branch is skipped. `socket`, `http.client`, `requests` and `curl` do not appear. The `subprocess` calls are `git ls-files` (npm), the interpreter site report (python), `cargo deny --version` and `cargo deny … check licenses` (rust, both pre-existing), `cargo metadata --locked --offline` and `rustc --print sysroot`. None of them is a git fetch or a curl. `scripts/guards/catalogue.py`, `scripts/guard_probe.py` and `scripts/package_check.py` contain no `urllib`, `socket`, `curl`, `verify-upstream` or `fetch_raw_pin`. `verify-upstream` is not in `FLOOR_SECTIONS`, and the default run prints `(rust, npm, python, notices, holder, binaries)`.
- **Earlier rows are still checked offline.** In a scratch copy with its own git, all under the stub. One byte appended to `objc2-earlier/LICENSE.txt`: exit 1, "sha256 is 2d7fd012…, expected e353f37b…" ×4. The file deleted: exit 1, "is absent" ×4. One row's URL commit changed to `ee9a7ada…` and its `commit` field left alone: exit 1, "earlier pin URL commit ee9a7ada… differs from the commit the row claims 757cd841…".
- **Where a consistent re-point is refused.** All four rows had URL and `commit` both changed to `ee9a7ada2131f5944b8750428e265c15632f2a19`. `binaries` exited 1 with only "THIRD_PARTY_LICENSES does not match the lockfile". `--write binaries` then exited 0 and rewrote four sentences to say LICENSE.txt is "from commit ee9a7ada…". After that, `binaries` exited 0. So the offline gate cannot detect this, and a regenerate makes it pass. `verify-upstream` with the network up refused: "block2 0.6.2 LICENSE.txt is absent at commit ee9a7ada…", ×4, exit 1. `docs/DEVELOPER_SETUP.md` says which command refuses it, and says that command is not in any floor gate. The plan says nothing on the point (S1).
- **verify-upstream.** On the worktree with the network up: "OK: licence check (verify-upstream)", exit 0. It checks 20 rows: 16 crate pins and 4 earlier pins. In scratch: (a) one byte appended to `block2-0.6.2/LICENSE.md`: exit 1, local "sha256 is 5018805a…, expected 7f976f7e…". (b) The same byte, with every matching `sha256` in `sources.toml` rewritten to the new digest, so the local check passes: exit 1, with ten lines of the form "LICENSE.md at commit <c> sha256 is 7f976f7e…, expected 5018805a…", one per objc2-family crate at its own commit. (c) The earlier `LICENSE.txt` changed and its digest rewritten: `binaries` 1 and `verify-upstream` 1, "LICENSE.txt at commit 757cd841… sha256 is e353f37b…" ×4. (d) Under the stub, `verify-upstream` exited 1 with "pin URL … could not be read: NONET …", so a missing network is a refusal, not a pass. `--write verify-upstream`, `verify-upstream binaries` and `installed verify-upstream` each exit 1 with a named refusal.
- **Upstream identity of the earlier pin.** In the full objc2 clone (not shallow) from pass 3, `git log -1 --format='%H %P' cfb19922` gives parent `757cd841d0293341dbefd9b99d7548dec244ffde`. `git show cfb19922^:LICENSE.txt` hashes to `e353f37b…`, the recorded digest. `cfb19922` has no `LICENSE.txt`. The `sources.toml` comment "from the parent of commit cfb19922" is true.
- **Would the new tests fail if the code were wrong.** Each mutation was made in scratch and then reverted. M1 passed `blob_at=fetch_raw_pin` in `render_closure`, and `test_the_binaries_gate_does_not_fetch` failed. M3 disabled the URL-vs-`commit` comparison, and `test_an_earlier_pin_must_match_the_commit_it_names` failed. M2 disabled the local-digest branch in `earlier_commit_problems`, and all 72 tests stayed green (N11).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git write-tree`, `git diff \| shasum`, start and end | worktree | now | `22fcce32…` and `2fa681bb…` both times |
| `licence_check.py binaries`, proxy only | worktree | now | 0, OK |
| `licence_check.py binaries`, socket stub | worktree | now | 0, OK, no `NONET` line |
| unit suite, proxy only / socket stub | worktree | now | 72 OK / 72 OK, 0 `NONET` lines |
| `bin/ocelli.sh gate licences`, stub and proxy | worktree | now | 0, ALL GREEN |
| `licence_check.py` default floor set, stub | worktree | now | 0, all six sections |
| `guard_probe.py --profile deep` with every `--only licences.binaries-*`, stub and proxy | worktree | now | 0, 15 refusal probes, 13 controls |
| scratch: earlier bytes / file deleted / one URL changed | scratch copy, own git | now | 1 / 1 / 1, messages above |
| scratch: four rows re-pointed consistently, `binaries`, `--write`, `binaries`, `verify-upstream` | scratch | now | 1 (lockfile mismatch only) / 0 / 0 / 1 "absent at commit" ×4 |
| `licence_check.py verify-upstream`, network up | worktree | now | 0, OK |
| scratch: crate pin byte changed, digest kept / digest rewritten | scratch | now | 1 local mismatch / 1 upstream mismatch ×10 |
| scratch: earlier byte changed, digest rewritten | scratch | now | `binaries` 1, `verify-upstream` 1 ×4 |
| `verify-upstream`, stub | scratch | now | 1, "could not be read: NONET" |
| scratch mutations M1, M2, M3 | scratch | now | 1 failure / 72 OK / 1 failure |
| objc2 clone: parent of `cfb19922`, digest of `cfb19922^:LICENSE.txt` | upstream clone | now | `757cd841…`, `e353f37b…` |
| scratch: `rm THIRD_PARTY_LICENSES`, `--write binaries`, stub | scratch | now | 0. `cmp` shows `THIRD_PARTY_LICENSES` and `sources.toml` byte-identical to the worktree |
| `python3 -B -m unittest discover -s scripts/tests -p test_licence_check.py` | worktree | now | 0, 72 OK |
| `bin/ocelli.sh gate licences` | worktree | now | 0, ALL GREEN |
| `bin/ocelli.sh gate packages` | worktree, existing dist | now | 0, ALL GREEN |
| `bin/ocelli.sh gate guards`, twice | worktree | now | 0, "431 refusal probe(s) drove 37 guard(s) red … 44 accept … 95 control(s) green" |
| `python3 scripts/guard_census.py`, then `--check-runbook` | worktree | now | 0 and 0, "1102 refusal(s) in 78 file(s), all claimed", "0 watched by nothing" |
| `bin/ocelli.sh gate ci` | worktree | now | 0, ALL GREEN |
| `python3 scripts/prose_check.py` | worktree | now | 0, 840 files |
| `python3 scripts/source_provenance_check.py` | worktree | now | 0, 1677 files |
| `gate guards-deep` in full | | implementer receipt only | not re-run. The 15 binaries probes ran under `--profile deep` |

## Defects

### D1, `sources.toml` says verify-upstream re-fetches every pin

**Where**: `ci/notices/sources.toml:4-5`, "python3 scripts/licence_check.py verify-upstream re-fetches every pin from its recorded URL at its recorded commit."
**What**: `verify-upstream` reads `upstream_pin_rows()`, which is the 16 `[[crate]]` rows and the 4 `[[earlier]]` rows. It does not re-fetch the `[[file]]` rows: the MIT, Apache-2.0 and Zlib templates and the `rust-std` notice. The repository calls those pins too. Line 1 of the same file says "Pinned inputs". Decision 6 of the plan says "Standard texts are pinned". `render_notices` takes them as `pins`, and `rewrite_rust_std_pin` and `pinned_digest_problems` use the word. Apache-2.0 has a recorded URL and is verbatim, so a reader of line 4 will believe its bytes are upstream-checked, and they are not.
**Why it is wrong**: `/microscope` section 3. A false claim in prose is a defect. `docs/DEVELOPER_SETUP.md:90` has the accurate scope: "every crate pin and every earlier pin".
**Evidence**: `upstream_pin_rows()` returned 20 rows, all `[[crate]]` or `[[earlier]]` and none from `[[file]]`. `verify_upstream_problems` reads nothing else.
**Scope and regression**: present only in the unstaged remediation, in a comment. The smallest fix is to use the setup doc's wording ("re-fetches every crate pin and every earlier pin"). No code change is needed. Re-fetching the Apache text would be a scope change, and that is the lead's call.

## Smells

### S1, the plan does not record the pass-4 decision or where the earlier-commit refusal lives

**Where**: `.claude/plans/F-220-design.md`. The amendments stop at "Lead amendments after review pass 2". The plan never mentions `verify-upstream`.
**What**: the brief says the lead chose option (a) from pass 4: an offline gate, plus a separate network command outside every floor section. That decision is not in the tree. The plan's pass-2 amendment pins the four earlier rows to "the commit before" `cfb19922`, and the plan nowhere says the offline gate cannot verify it. Pointing all four rows at another commit and running `--write binaries` passes every gate and ships a false provenance sentence (Acceptance challenged, fourth bullet). Only the manual `verify-upstream` catches that. `docs/DEVELOPER_SETUP.md` documents it, but the plan does not. The plan's environment line 186 ("Network once, at implementation") also no longer describes the network use, which now includes a developer-time command.
**Why it is a smell**: the plan is the normative design, and `/complete-feature` writes the LLD and as-built record from it. An unrecorded lead decision invites the next reviewer to relitigate it, or a later change to "fix" the gap by wiring the fetch back into the floor. That would bring back pass-4 D1, and no amendment would say why it is wrong.
**Evidence**: `grep -n "verify-upstream\|amendments after review pass [34]" .claude/plans/F-220-design.md` finds nothing. Scratch re-point run: `binaries` 1, then `--write` 0, then `binaries` 0, then `verify-upstream` 1.
**Scope and regression**: the plan only. Owner: the lead. The smallest fix is a "Lead amendments after review pass 4" entry that says three things: the gate is offline, `verify-upstream` re-fetches crate and earlier pins and is run when a pin is added, and a consistent re-point of an earlier row is refused only there. When F-220 completes, `docs/lld/guards.md` or `docs/lld/release-policy.md` should carry the same sentence.

## Nitpicks

- N1, carried. Six `authors: ` lines still end in a trailing space (the 0.3.2 group).
- N2, carried. `</html>-----END RUST STANDARD LIBRARY NOTICE-----` still shares one line.
- N3, carried, not re-examined. The closure-count test is not independent of `_rust_shipped`.
- N4, carried. "reachable on a normal or build edge" omits crates added only through workspace-unified features.
- N5, carried, not re-run. A branch URL in a crate pin reports "differs from the published commit absent".
- N6, carried. `docs/lld/guards.md:618-624` still omits `binaries-published-commit`, `binaries-attribution` and `binaries-earlier-commit`.
- N7, carried. The header does not say what the `sha256:` line digests.
- N9, carried. The `[[earlier]]` `reason` still says "This file is the parent of that commit". A file is not a commit's parent. The comment at lines 46-47 says it correctly.
- N10, carried. `statement_sentence` still hard-codes "the repository root" for the earlier file.
- N11, new. In `earlier_commit_problems`, the local-digest branch (`local_digest != expected`) can be disabled with all 72 tests green (M2). The gate still refuses through `digest_rows()` in `pinned_digest_problems`, so the floor behaviour is unchanged. For `verify-upstream`, which reads the same branch, no test covers a local file that differs from its row while the upstream bytes match it. That case is also refused by `binaries`, so this is a test gap and not a hole.

N8 from pass 4 (the "GitHub" credit wording) is not re-counted, since nothing touching it changed. A crate-row failure inside `verify-upstream` would be labelled "earlier pin URL commit …". It is reachable only for a crate row whose URL is not a raw GitHub URL, which `published_commit_problems` already refuses, so it is not counted.

## Verified clean

- Pass-4 D1 is closed. The floor gate, the unit suite and the binaries probes pass with the network blocked at the socket layer as well as through an unroutable proxy. Nothing in the gate path opens a socket, and wiring the fetch back in turns `test_the_binaries_gate_does_not_fetch` red.
- `[[earlier]]` rows are still checked offline: local presence, local digest, and agreement between the URL commit and the `commit` field. A consistent re-point is refused by `verify-upstream`, and only there.
- `verify-upstream` passes on the real pins with the network up. It refuses changed local bytes, changed bytes with a rewritten digest (against upstream), a commit with the file missing, and no network. It refuses to run with other sections or with `--write`.
- `ci/notices/sources.toml:3` ("Gate time reads these files. It does not fetch.") is true. `.github/workflows/ci.yml:137-138` and `docs/lld/guards.md:628-629` ("the binaries section reads the registry cache offline") are true. `docs/DEVELOPER_SETUP.md:76-95` is true sentence by sentence: the crate-pin commit is checked against `.cargo_vcs_info.json`, the earlier `commit` field is what the sentence names, gate time does not fetch, and `verify-upstream` scope, refusals and non-floor status are as stated.
- The parent of `cfb19922` is `757cd841`, and the pinned bytes are that commit's `LICENSE.txt`.
- Regeneration under the socket stub is byte-identical for `THIRD_PARTY_LICENSES` and `sources.toml`. All the requested gates and scripts are green.

## Limitations and handoff

- All mutations ran in a scratch copy with its own git repository, under the scratchpad. At the end, the worktree's tree hash and unstaged diff are what they were at the start. Only this report was written.
- The no-network run is a Python-level socket stub plus proxy variables, not a host with networking disabled. It covers every Python process that inherits `PYTHONPATH`. It does not cover `cargo`, which runs `--offline`. I did not confirm that the probe harness passes `PYTHONPATH` to its sandboxed children, so the probe row is weaker evidence than the direct runs.
- `gate guards-deep` was not run in full. The Linux half of AC8 still needs CI on the pushed head. The Apple SDK question in objc2's `LICENSE.md` is still the operator's.
- Next owner: the lead for S1, implementation for D1 (one comment), then a fresh independent pass 6 over those changes.
