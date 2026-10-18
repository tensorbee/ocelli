# F-047 review, pass 1

**Reviewed**: working tree on `sprint/s15` against `2b7a84b` (`F-046, window presets, invert and colormap`)
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`git diff --name-only 2b7a84b` is eight tracked paths. `git diff 2b7a84b --stat` is 77 insertions and 7 deletions. `crates/ocelli-viewport/src/scroll.rs` and `crates/ocelli-viewport/tests/scroll.rs` are untracked, so a path diff does not print them. `git diff 2b7a84b -- crates/ocelli-render/src/graph.rs` is empty. No shader path is in the diff. `presentation.rs` and `view.rs` are not in the diff. F-046 code was not edited.

**Stored order.** `StackSequence` stores `keys.to_vec()` and a `u32` at `scroll.rs:35`. There is no `sort`, no position read, and no use of `FrameKey`'s `Ord`. `stored_order_is_not_sorted_by_key` scrolls instance numbers `2, 0, 1` as indices `0, 1, 2`. Sorting inside `new` made that test fail. `bin/ocelli.sh test ocelli-viewport --test scroll stored_order_is_not_sorted_by_key` exited 101. The panic was `tests/scroll.rs:40`. Left was `FrameKey` instance `0`. Right was instance `2`. The sort was removed.

**Prefetch radius.** `prefetch_keys` at `scroll.rs:63` is `checked_sub(1)` and `checked_add(1)`. The selected index is not a slot. `prefetch_radius_is_one` covers index `0`, the middle, and the last of three. Index `0` is `[None, Some(key(20))]`. The middle is `[Some(key(10)), Some(key(30))]` while the displayed key is `key(20)`. The last is `[Some(key(20)), None]`.

**Upload samples.** `samples_for_upload` at `scroll.rs:92` calls `ImageCache::get` only with `displayed_key`. Empty returns `Empty` before `get`. A miss returns `NotResident` and the error carries no bytes. Replacing that match so a failed `contains` does `get` on the first prefetch key and returns those samples made `a_miss_returns_no_neighbour_bytes` fail. The command was `bin/ocelli.sh test ocelli-viewport --test scroll a_miss_returns_no_neighbour_bytes`. It exited 101. The panic was `tests/scroll.rs:74`, `matches!(samples, Err(ScrollError::NotResident))`. The original function was restored. `scroll_by` at `scroll.rs:73` clamps to `0..=last` and does not wrap. The empty path returns before the clamp. `scroll_by_clamps_and_an_empty_sequence_is_empty` covers both ends and an empty sequence after `scroll_by(3)`.

**`contains` and a miss.** `ImageCache::contains` at `image.rs:397` takes `&self` and calls `Lru::contains_key`. `contains_key` at `lru.rs:203` is `BTreeMap::contains_key` and does not call `tick`. `Lru::get` at `lru.rs:185` does call `tick` before the lookup. On a miss the `?` returns before any `used_at` write, which is what `lru.rs:326` says. A temporary `cache.get` of a missing key, inserted above `contains` in `contains_does_not_refresh_the_oldest_key`, left that test green. The oldest key was still the eviction. A miss ticks the clock and does not change eviction order. Changing `contains` to `&mut self` and `self.get(key).is_some()` made the same test fail. It exited 101 at `tests/image_cache.rs:151`. Left was `Evicted` instance `2`. Right was `Evicted` instance `1`, the oldest key. That is the red result the plan requires if `contains` refreshes. The method was restored to the `&self` forward.

**Conversions.** `usize::from(u32)` is not implemented on rustc 1.97.1. A host compile and a `wasm32-unknown-unknown` compile of `usize::from(1u32)` both failed with E0277. `key_at` uses `usize::try_from`. On this host `usize::try_from(u32::MAX)` is `Ok(4294967295)`. On wasm32 the core impl is `impl_try_from_unbounded`, whose body is `Ok(value as Self)`, so it cannot fail. The only pointer width where `TryFrom<u32> for usize` can fail is 16-bit. This sequence never stores such an index. `scroll_by` writes an index only after `u32::try_from(len)` and a clamp into `0..=last`. On 16-bit, `len` already fits in `usize`, so the stored index does too. The `?` in `key_at` cannot drop a real stored index. `u32::try_from(len)` fails only when `len` is above `u32::MAX`. That arm returns without writing `index`. The index stays `0`, `index()` stays `Some(0)`, and the sequence is not reported `Empty`. HLD section 17.4's `slice_index` is a `u32`, so frames past that range have no index this story stores. `u32::try_from(clamped).unwrap_or(0)` cannot fail. `clamped` is inside `0..=last` and `last` is a `u32`. Host `u32::try_from(i64::from(u32::MAX - 1))` was `Ok`. `unwrap_or` is not `unwrap`, `expect`, or `panic`.

**Boundary.** `scroll.rs` has no `unsafe`, no `unwrap(`, no `expect(`, no `panic!`, and no `as` cast. `bin/ocelli.sh clippy ocelli-viewport` exited 0, which includes `--all-targets` and `-D warnings`. The crate stays `#![cfg_attr(not(test), no_std)]` with `extern crate alloc`. `python3 scripts/no_std_check.py` printed `OK: 7 no_std crate(s) reach no std feature`. `Cargo.toml` depends on `ocelli-cache` and does not depend on `ocelli-render` or wgpu. `cargo tree -p ocelli-viewport -e normal` lists neither wgpu nor `ocelli-render`. `cargo tree -p ocelli-cache -e normal` does not reach viewport or render. `cargo tree -i ocelli-viewport -e normal` shows render, compute, native, and wasm depending on viewport. That edge already existed. Cache under viewport does not close it. There is no second cache. `StackSequence` holds the key list and reads `ImageCache`.

**LLD.** `docs/lld/viewport.md` lines 93 to 108 match the code. Stored order, clamp, no wrap, empty `Empty`, one `get` of the selected key, `NotResident` with no neighbour bytes, radius one, no decode and no upload, no render-graph edit. `contains` is the non-refreshing probe. A prefetch insert is not in this module, and nothing in the diff sets `Dirty.slice`. Tier C uses the same host lookup. There is no second result behind a tier check. The crate does not depend on wgpu. `docs/lld/cache.md:124` matches the forward to `contains_key`. The README rows for `viewport.md` and `cache.md` name F-047, and the file headers name the same set. The new sentences have no prose semicolon and no em dash.

**Restored tree.** After the mutations were removed, `shasum -c` matched `scroll.rs`, `image.rs`, and `image_cache.rs` to the pre-mutation hashes. `git diff -- crates/ocelli-viewport/src/scroll.rs` printed nothing. `git diff -- crates/ocelli-cache/src/image.rs` is only the seven-line `contains` addition. `bin/ocelli.sh test ocelli-viewport --test scroll -- --test-threads=1` exited 0. All 5 tests passed. `bin/ocelli.sh test ocelli-cache --test image_cache contains_does_not_refresh_the_oldest_key` exited 0.
