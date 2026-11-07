# F-226 review, pass 10

**Reviewed**: commit `9566b11b` on `work/f-226-xai`, range `f2aea2c1..9566b11b`, "F-226, lifetime clauses deleted", closing pass 9 D1. HEAD tree `1e5827cf`, matching `git write-tree`. No unstaged tracked paths. Untracked at start and end: `.venv` and `tools/oracle/out`, neither touched. Then a sweep of `git diff d9c2d477..HEAD`, review files excluded, for any sentence claiming the posted bytes stay in the cache for a decode.
**Author / reviewer**: the commit is the lead's. This pass is an independent Claude reviewer that wrote none of the reviewed material. The reviewer added only this report.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

- **Pass 9 D1, the `PreviewSource` type doc**, now "A preview's view of one id." It makes no lifetime claim. The variant docs below it carry the meaning, and nothing in the impl block or in `SharedBytes` leans on the deleted "so a second preview or `/headers` still finds them" clause.
- **Pass 9 D1, the `Shared` variant**, now "Posted bytes, shared with the cache." `preview_source` builds `Shared` only in the arm where `state.lru.get(&HeldKey::File(id))` returns `Held::File(bytes)`, and hands out `Arc::clone(bytes)` (`held.rs` lines 1278 to 1285). The bytes are therefore the cache's own `Arc` at handout. The sentence says nothing about how long the cache keeps it, so eviction and forget do not contradict it.
- **The plan's ruling, `.claude/plans/F-226-design.md` line 329**, now "`preview_source` hands out shared posted bytes. The in-flight count is taken where `preview_source` hands out the shared bytes, and only `store_frame_with` checks the doomed set." `take_off` is called in that same arm (line 1281). `doomed` is read with `contains` only at line 1319 in `store_frame_with`. The other references insert in `forget_ids` (line 1636), remove in `land` (line 1704), and the test assertion at line 2351. The closing "restated on that basis" still reads correctly, since the basis is now the shared handout and the count's placement, and the "Where these differ from the text above, these win" preface still supersedes decision 6 and the state table row that name `restore_file`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer` | worktree at HEAD `9566b11b` | now | 0. 81, 0, 1, 14, 79, 1, 1, 24 and 0 passed across the binaries, none failed or ignored |
| `cargo clippy -p ocelli-folder-viewer --all-targets -- -D warnings` | same | now | 0, no warnings |
| `git diff d9c2d477..HEAD -- . ':!.claude/reviews'`, added lines grepped for stay, remain, keep, kept, until, in the cache, shared with the cache, evict, during | same | now | every hit read in context, see Verified clean |
| Code reading | `held.rs` lines 140 to 240, 1262 to 1295, 1319, 1636, 1704. `tests/preview.rs` lines 3769 to 3783. Plan lines 84, 93, 168, 329 | now | as cited |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- `docs/lld/cache.md` new section. "`preview_source` counts each decode it hands posted bytes to" and "A flight lasts from the lookup that hands out the posted bytes until after the store" describe the flight, not the bytes' residence. No sentence says the bytes stay in the cache.
- `README.md`, `use.html` and `more.html` additions. None states a lifetime for posted bytes during a decode. `more.html` already names eviction as a reason a file is posted again.
- `held.rs` `SharedBytes::take_flight`, `Flight`, `PreviewSource::take_flight` and the `Lookup::Miss` doc ("shared with the cache so a decode does not copy them"). Each is about sharing or the count, not about the cache keeping the bytes.
- `serve.rs` comments at lines 1029, 1043 and 1058 keep the flight, not the bytes, to the end of the round.
- `tests/preview.rs` line 3773, "Frame 2 is not decoded, so the posted bytes stay beside them." It is a claim about that test's one state on an 8 MiB viewer with two small files, no budget pressure and no forget, and the test's `files 2` expectation passes in the run above.

## Limitations and handoff

- `held.rs` line 1269, the `preview_source` doc "Frame or shared posted bytes. The file stays in the cache.", is present at base `d9c2d477` (line 1165 there) and is not in the story's diff, so it is not counted. Read as a statement about the call, it is true, since `preview_source` no longer removes the file. Read as a lifetime it is false in the way pass 9 D1 was. Worth deleting the second sentence when the file is next touched.
- Plan lines 84, 93 and 168 still name `restore_file`. They are outside the story's diff and superseded by the 2026-11-07 ruling's preface.
- Not run: `prose_check.py`, any gate, the browser job. The reviewed commit is comment and plan prose only.
- This pass is clean for the reviewed commit and the sweep. It is not completed acceptance or sprint readiness.
