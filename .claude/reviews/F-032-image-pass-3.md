# F-032 review, pass 3

**Reviewed**: working tree, uncommitted. Scope `Cargo.lock`, `crates/ocelli-cache/Cargo.toml`, `crates/ocelli-cache/src/lib.rs`, `crates/ocelli-cache/src/image.rs`, `crates/ocelli-cache/tests/image_cache.rs`, `docs/sprints/BACKLOG.md`, `.claude/plans/F-032-design.md`, `.claude/reviews/F-032-image-pass-1.md` and `.claude/reviews/F-032-image-pass-2.md`. Pass 1 and pass 2 were read in full before any check. The reviewer was independent of the author.
**Result**: 0 defects, 0 smells, 0 nitpicks

Every remediation from pass 2 was verified against the current tree. Every mutation below was reverted exactly and read back. The full suite and clippy were run again green after the last revert.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

**Pass 2 smell S1, the tier the cache fixes.** `ImageCache::new` fixes the tier at `crates/ocelli-cache/src/image.rs:221` to `CacheTier::Decoded`. The read side test at `crates/ocelli-cache/tests/image_cache.rs:296` now asserts `pressure().tier` equals `CacheTier::Decoded` alongside `used` and `budget`. Changing `Decoded` to `Encoded` at line 221 and running `bin/ocelli.sh test ocelli-cache --test image_cache` gave 10 passed and 1 failed with exit 101. The failure was `read_side_and_accessors_are_executed` at line 296 with left `Encoded` and right `Decoded`. Pass 2 ran the same mutation green at 11 passed and exit 0. The mutation was reverted exactly and line 221 was read back as `Decoded`. The smell is closed.

**Pass 2 nitpick 1, the property reaches the refused arm.** The property at `crates/ocelli-cache/tests/image_cache.rs:348` now draws byte counts from `1_usize..300` against a fixed budget of 250 at line 350. The range holds 299 values and 49 of them sit above the budget, so a draw above 250 is refused by `would_admit` in the `Lru`. The refused arm at lines 371 to 375 is therefore reachable inside the property and no longer needs the deterministic test alone to pin it. The deterministic refused case at 251 bytes remains in `displaced_and_refused_are_not_evicted`.

**Pass 2 nitpick 2, `CacheEvents` length and emptiness in both directions.** `is_empty` is asserted true on the setup inserts and false at `crates/ocelli-cache/tests/image_cache.rs:169` with `assert!(!events.is_empty())`. `len` is asserted at 1 on line 170 and at 2 on line 325 in the combined order test. Both wrappers now have a true and a false direction on record.

**Pass 1 remediation held.** The kind numbers stay pinned at `crates/ocelli-cache/src/image.rs:130` to `crates/ocelli-cache/src/image.rs:134` as 1 and 2 and 3 and re-exported at `crates/ocelli-cache/src/lib.rs:27` to `crates/ocelli-cache/src/lib.rs:28`. The test at `crates/ocelli-cache/tests/image_cache.rs:240` to `crates/ocelli-cache/tests/image_cache.rs:242` asserts each constant against the literal. The randomised accounting property exists at line 347 and passes. The plan sentence about a fixed capacity value is gone. `CacheEvents` wraps a `Vec` at `crates/ocelli-cache/src/image.rs:190` and the insert loop pushes without a cap. The raw payload at byte 16 is asserted by `raw_payload_starts_at_byte_16` at line 273. The header at lines 3 to 7 cites Rows and Columns in DICOM PS3.3 C.7.6.3.1 for the 512 by 512 by 2 product.

**Casts and arithmetic.** A search for an `as` cast in `crates/ocelli-cache/src/image.rs` found only the word inside doc comments. No cast lint allowance exists in either F-032 file. The one numeric conversion is `usize` to `u64` for the payload byte count at `crates/ocelli-cache/src/image.rs:175` using `try_from` with saturation to `u64::MAX`. The conversion cannot fail where `usize` is at most 64 bits. Payload writes are four fixed slices into a zeroed 32 byte array and the last eight bytes stay zero. `DecodedFrame::bytes` at lines 92 to 94 returns the buffer length and performs no rows by columns arithmetic. No LUT and no VOI and no geometry formula appears in this diff. `ImagePlane` and `StoredPixelDescription` are carried as opaque validated values. Budget admission at exactly the budget is inherited from `would_admit` in the `Lru`.

**Tests would fail if the code were wrong.** Four mutations were applied and run and reverted exactly with the file read back after each revert. Swapping `KIND_EVICTED` and `KIND_DISPLACED` made `kind_numbers_are_pinned_to_the_wire_values` fail with left 2 against right 1 and exit 101. Setting the viewport sentinel to zero made two tests fail with left 0 against right 4294967295 and exit 101. Returning zero from `DecodedFrame::bytes` made six tests fail with exit 101 and the property reported a minimal failing input of one 1 byte insert. Changing the fixed tier to `Encoded` made the read side test fail as recorded above. Fixture values are hand computed in the test comments and the layout numbers come from section 17.3 in `docs/hld/14-the-boundary-in-code.md` rather than from implementation output.

**Prose claims, executed.** No ring is built and nothing reaches JavaScript. The working tree adds no `ocelli-wasm` path and `crates/ocelli-wasm/src` still holds only `lib.rs` and `panic.rs`. The claim that `bytes` follows the buffer when description and buffer disagree is the malformed pair in `frame_bytes_is_the_allocated_buffer` and it passes. The claim that a displaced value comes before any eviction is the combined order test and it passes. The claim that `remove` emits no event holds by construction because `remove` returns an owned frame with no event type and the read side test confirms pressure falls to zero after it. The claim that reading is a use and `pressure` is not a use is delegated to the `Lru` and its own tests cover tick behaviour. The range comment beside the kind constants records a reservation for F-034. A search found no other kind constants in the crates today and the backlog still shows F-034 pending.

**Dead and unreached code.** Each `Admission` field maps to exactly one `CacheEvent` variant and each variant is constructed in the deterministic tests and encoded in the kind test. Every arm of the kind mapping in `CacheEvent::parts` is encoded. `FrameKey::new` and `SeriesId` and `CacheEvents::events` and `is_empty` and `len` and the three accessors and `get` and `remove` and `pressure` are all executed by committed tests. The reserved payload tail is asserted zero. No public surface in `image.rs` is left unexecuted after the tier assertion landed.

**Boundary and tier.** A search for `unsafe` and for `wasm-bindgen` across the new and modified cache files returned no hits. The event payload carries series and instance and frame and byte count only. No sample crosses in an event. No view over wasm memory is built and no ring exists to drain. Allocation happens on the insert path through the vector in `CacheEvents` and the map in the `Lru`. That is the decode path and not the render loop. `ImageCache` always builds its `Lru` over the decoded tier and holds no tier conditional so the cache behaves identically on tiers A and B and C. The crate keeps its `no_std` attribute with `extern crate alloc` at `crates/ocelli-cache/src/lib.rs:18`. `Cargo.lock` adds exactly the `bytemuck` and `ocelli-pixel` edges to `ocelli-cache`. The backlog change is the single status word from pending to in-progress for F-032.

**Structure.** No new trait is declared. The diff implements the existing `Budgeted` trait for one new value type and that trait already has other implementers. No new generic parameter and no dynamic dispatch wrapper and no feature flag appears. `ImageCache` fixes the tier and the key type and the value type and translates an admission into reportable events so it removes choices a caller of the raw `Lru` would otherwise face.

**Suite and lints.** `bin/ocelli.sh test ocelli-cache` passed with exit 0 after all mutations were reverted. The run held 21 unit tests and 4 budget tests and 11 image cache tests and 2 invariant tests for 38 in total. `bin/ocelli.sh clippy ocelli-cache` passed with exit 0. Exit codes were read from the commands directly with no pipe in either invocation.
