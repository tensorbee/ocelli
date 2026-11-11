# F-231 working pass 6

Scope: the merge `e47bbd05` of `sprint/s46` (at `3f098cd5`) into `work/f-231-xai`
(at `9ea4b7fc`, clean at pass 5). Merge base `308a0f06`. This pass reviews the
merge only and not F-231's own diff again.

**Result: 0 defects, 0 smells, 2 nitpicks.**

## What was checked

| Check | Observation |
|-------|-------------|
| Files both sides changed | `git diff --name-only` from the base on each side, intersected: `Cargo.lock`, `crates/ocelli-cache/src/image.rs`, `crates/ocelli-cache/src/lib.rs`, `docs/lld/cache.md`, `README.md`, `examples/folder-viewer/src/more.html`, `examples/folder-viewer/src/use.html` |
| Both sides survive | `git merge-file -p` of each file against the base, compared with the merged blob. Five merge clean and equal the merged tree byte for byte. `more.html` and `use.html` conflict once each, and the merged tree differs from the auto merge only inside those hunks |
| `use.html` resolution | The merged hunk is the conflict with its three markers removed, so F-231's `CommittedFrame` paragraph comes first and F-226's release, content id and study paragraphs follow it whole |
| `more.html` resolution | F-231's sentence is kept. F-226's two sentences, that no shipped session produces `cache.released` and that the cache and the release calls arrive with F-231, are gone. The new sentence replaces them |
| New `more.html` sentence | `releaseSeries` is exported from `packages/core/src/index.ts` line 252 and calls the wasm export `release_series` (`frame-cache.ts` line 150). The export `#[wasm_bindgen(js_name = release_series)]` in `crates/ocelli-wasm/src/session.rs` line 1526 calls `Session::release_series`, which releases from `self.frames`, the session cache, and pushes the events through `self.emit`. `release_series_reports_one_event_and_keeps_the_display` asserts the ring holds `KIND_RELEASED` then `KIND_PRESSURE` with series 5 and frames 3. `decodeNamedEvent` names that kind `cache.released` (`events.ts` line 476) |
| `releaseStudy` | Exists in `frame-cache.ts` line 159, exported from `index.ts` line 253, and README line 409 is the only prose naming it. No sentence names a `releaseStudy` that does not exist |
| Stale "arrives with F-231" or "not built" | `grep -i` over README, `use.html` and `more.html` for `F-231`, `shipped session`, `arrive`, `not yet`, `produces it yet` and `session cache`. The only F-231 mention left is `use.html` line 776, which describes what F-231 built. The `use.html` ring paragraph at line 366 says the session produces the cache kinds including a series release, which matches the session code. F-226's sentence that "the cache has no study" is about `ocelli_cache`, and is still true there |
| Same grep over `packages/core/src`, `crates` and `docs/lld` | No sentence says a release or a frame cache arrives later. The two `arrive with` hits are error code comments unrelated to the cache |
| F-232 doctest attribute | `python3 scripts/lint_policy_check.py` exits 0 and reports 16 library roots forbid unsafe code in their doctests |
| Compile and tests | `cargo test -p ocelli-wasm -p ocelli-cache` passes: `ocelli-cache` 36 unit, 24 bricks, 4 budget, 3 content id, 20 image cache, 2 invariants, 13 telemetry, and `ocelli-wasm` 51 unit, including both `release_series` tests |

Not exercised: the browser gate and the packages gate, per the brief, because a
verification run holds them. No mutation was run, since the merge changed no
F-231 code or test.

## Nitpicks

1. `examples/folder-viewer/src/more.html` line 422. "The session's
   `releaseSeries` releases a series from that cache and reports it as
   `cache.released`" is unqualified, and a series holding nothing emits no
   event. README line 410 carries the qualification, so nothing is false, and
   the sentence reads as the normal case.
2. Outside the merge. `crates/ocelli-wasm/src/session.rs` line 3172 justifies
   the Low reading with "2,097,152 < 4,194,304", while the quantity that
   decides the band is the used figure 1,048,576 against the half budget
   2,097,152. The assertions are right and the comment was in the tree pass 5
   accepted.
