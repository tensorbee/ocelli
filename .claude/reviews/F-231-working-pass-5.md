# F-231 review, pass 5

**Reviewed**: `git diff ebcb156a HEAD`, the pass 4 remediation. HEAD `9ea4b7fcf5bab55488c3ccd17e6b43720a4087a9`, tree `7c10ad647614ca6898ddb789275e7eb7f450dc1f`, worktree `ocelli-wt-f-231` on `work/f-231-xai`. Before this report the only untracked files were the pass 1 to pass 4 reports. Pass 4 swept the whole story, so this pass covers only the three changed files.
**Author / reviewer**: the brief says the sprint lead, a Claude agent, wrote the remediation. The commit's git author is the operator's identity, so that cannot be confirmed from the tree. The reviewer is an independent Claude agent. It did not write the remediation, the plan or any earlier pass.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

- **No code changed.** Every `+` and `-` line in `git diff ebcb156a HEAD` is a `//` comment in `session.rs` (test `a_burst_over_the_ring_capacity_counts_the_dropped`), a `//` comment inside the `use.html` code sample, or a `*` line in the `frame-cache.test.ts` module doc. That is 3 files, 9 insertions and 7 deletions. No assertion, statement or markup outside those comments moved.
- **Pass 4 D1, the AC11 session comment, by probe.** The test was run in a scratch copy of `git archive HEAD` with its own target directory. A `panic!` placed after the test's own asserts printed the held kinds: `first Some(4) pressure 1 displaced 255 held 256 header_held 256 dropped 44`. `KIND_PRESSURE` is 4 and `KIND_DISPLACED` is 2 (`ocelli-cache/src/image.rs:402`, `:409`). Each clause of the new comment holds:
  - "300 commits ... push 300 events": 256 held plus 44 dropped.
  - "the first is an insert whose pressure crossing is one event": the first held kind is pressure, and exactly one pressure event is held. The band stays Critical after that, so none of the 44 dropped can be a second pressure event.
  - "the other 299 displace": 255 displaced events are held and 44 are dropped, which is 299.
  - "256 are held and 44 are counted in dropped, which drain_events reports in its header": header words at offsets 0 and 12 read 256 and 44.
  The probe would reject the old wording, which says 300 displacements, and a wording that left out the pressure event. Pass 4 D1 is closed.
- **Pass 4 N1, the `use.html` sample sentence, against `show_resident`** (`session.rs:461` to `:517`) and `FrameCache::lookup` (`image.rs:681` to `:705`). The sample's slot carries the current epoch. At the selected index, a held entry whose content or extent does not match is `FrameLookup::Stale`, `lookup` removes it (`image.rs:697` to `:699`), and `show_resident` returns `Ok(0)`, which `showResident` maps to `false`. Away from the selected index under the current epoch the call is refused with `Selection` before the lookup, so it throws and keeps the entry. That is pass 4's P3. A wrong epoch returns `false` before the selection check and removes nothing. The sentence a few lines earlier already names that case on its own ("A wrong epoch returns false at any index"), so "mismatched" here can only mean a stale entry, and the qualified sentence is true. Pass 4 N1 is closed.
- **Pass 4 N2, the `frame-cache.test.ts` module doc.** Plan row AC13 (`.claude/plans/F-231-design.md:268`) is the bigint row, and its "what it catches" column is "A `u64` argument truncated to `u32` at the export". AC14 (`:269`) is `releaseStudy` deduplication and AC15 (`:270`) is refusals. The file's passthrough cases use `0x1_0000_0001n` (`:81`, `:93`) and `0x1_0000_0005n` (`:120`), and both are above `u32`. So "The bigint passthrough of AC13" names the right row. AC13's own test lives on the built module (`boundary.wasm.test.ts`). The stub test covers the `@ocelli/core` half of the same truncation risk, and "the other half" says that. Pass 4 N2 is closed.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git diff ebcb156a HEAD` | HEAD `9ea4b7fc` | now | 3 files, comment lines only |
| `cargo fmt --all --check` | HEAD, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo test -p ocelli-wasm` | HEAD, `CARGO_TARGET_DIR=<wt>/target` | now | 0, lib 51 passed |
| AC11 ring probe, `cargo test -p ocelli-wasm --lib a_burst_over_the_ring_capacity` | scratch copy of `git archive HEAD` with a `panic!` after the asserts, own target dir | now | failed by design, printed `first Some(4) pressure 1 displaced 255 held 256 header_held 256 dropped 44` |
| `npx vitest run packages/core` | HEAD | now | 0, 458 passed in 49 files |
| `npm run lint` | HEAD | now | 0 |
| `python3 scripts/prose_check.py` | HEAD | now | 0 |
| `python3 scripts/source_provenance_check.py` | HEAD | now | 0 |
| Wasm size, boundary suite, clippy, census | pass 4 receipts on `ebcb156a` | prior | not rerun. No code changed, so a comment-only diff cannot move them |

## Defects

None.

## Smells

None.

## Nitpicks

None new.
- Carried, not re-raised by this diff: pass 3 N2 / pass 4 N3 (the boundary test title "answers false only for a miss") and pass 2 N1 (`evicted_bytes` dropped by F-034's 32-byte payload, outside F-231). Neither is touched by this remediation.

## Verified clean

- All three changed sentences were checked against what was executed, not against the text around them. The AC11 comment was checked against the probe's held-kind counts and header words. The `use.html` sentence was checked against the `show_resident` and `lookup` control flow, with pass 4's executed P2, P3 and P5 for the off-selection and wrong-epoch cases. The `frame-cache.test.ts` sentence was checked against plan rows AC13 to AC15 and the literals in the file.
- No new `as` cast, `#[allow]`, `unsafe` or behaviour change, because the diff has no non-comment line.
- Prose rules: no em-dash or prose semicolon in the new sentences, and `prose_check.py` exited 0.

## Limitations and handoff

- Bounded to `ebcb156a..9ea4b7fc`. The whole-story sweep is pass 4's. No GPU, oracle, browser or wasm build was run, as the brief directed.
- The scratch copy and its target directory were deleted. After cleanup, `git status --short` showed only the pass 1 to pass 4 reports, and `git write-tree` was `7c10ad647614ca6898ddb789275e7eb7f450dc1f`, equal to `HEAD^{tree}`. Nothing tracked was edited. This report is the only new file.
- This is a clean bounded review of the remediation. It is not completed acceptance or sprint readiness. Next owner: the sprint lead, for `/verify` and `/complete-feature`.
