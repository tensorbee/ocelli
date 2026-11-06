# F-210 review, pass 4

**Reviewed**: `git diff 222edbe HEAD` on `work/f-210-xai`, HEAD `354b73a5fbaa970e94f19c0a2ffcc738a171e29c`, tree `56ec237a07be5ee121996264abc17a774e38bd6c`. That is the pass 3 remediation commit `354b73a`. This pass also swept the whole story once, `git diff 198e197 HEAD`, for claims and untested branches earlier passes missed. Nothing was staged or unstaged before or after the pass. The only untracked paths are the pass 1 to 3 reports and this report.
**Author / reviewer**: The remediation is by a Claude implementer agent and the plan sections by the sprint lead. This review is by a separate Claude reviewer agent that wrote none of F-210, its plan, its remediations or the earlier reviews. All are the same model family, which limits independence.
**Result**: 0 defects, 2 smells, 2 nitpicks

## Acceptance challenged

- **Pass 3 closure.**
  - D1: the `claim_buffer` doc comment now says "a buffer stolen from the oldest frame of the same length when no reply reads it". I checked it against `take_oldest_frame_buffer`. It takes the first same-length entry in `frame_order` and returns `None` when a reply holds that entry. The comment is true. Closed.
  - S1: `a_claim_passes_over_every_frame_being_read` goes red under pass 3's M6 (`for _ in 0..1`) with `R2 was evicted while a reply read it`. It also goes red under M14, where a claim evicts readers, with `R1 was evicted`. Closed.
  - S2: the steal test now requires reused +1 and fresh unchanged. Under pass 3's M3 (stealing disabled) it goes red with `B's claim went from reused 0 fresh 2 to reused 0 fresh 3, so it did not steal X's buffer`. Closed.
  - N2: both remaining open-coded copies now call `forget_evicted`.
- **N2 keeps behaviour.**
  - I put one probe test into a scratch copy, first with HEAD's `held.rs` and then with `222edbe`'s. It runs 400 seeded steps at each of four budgets (600, 1500, 2600, 4000). The steps mix `insert_file`, claimed-buffer stores, plain stores, held and dropped readers, and lookups. After every step it records `files`, `frames`, `lru.len()`, `used` and `free`, and it requires `files + frames == lru.len()`.
  - The 1600-line logs are byte-identical between the two versions. With instrumentation, the probe reached `evict_oldest` 306 times, the claim's unread eviction 525 times, the claim's reader re-insert 473 times and the steal route's reader branch 153 times.
  - The two arms N2 changed are `absorb`'s evicted loop and the evicted loop inside the steal route's reader re-insert. Neither fired, because `reserve_for_insert` makes room before every `Lru::insert`, so both arms are defensive.
  - In each changed arm, the replaced lines were exactly `forget_frame(&key)` followed by the `HeldKey` decrement match. That is the body of `forget_evicted`, so behaviour is the same by construction.
- **Each AC the plan assigns to implementation or integration maps to a named test that exists and passes:**

  | AC | Test |
  |----|------|
  | AC1 | `value_signed_12_bit_ct_is_minus_1025` |
  | AC2 | `value_masks_bits_above_bits_stored` |
  | AC3 | `value_mr_slope_two_is_minus_400_unspecified` |
  | AC4 | `value_fractional_rescale_keeps_two_decimals` |
  | AC5 | `value_without_rescale_is_the_stored_value` |
  | AC6 | `value_unit_follows_rescale_type`, with `value_pet_units_decide_over_rescale_type`, `value_multi_energy_ct_has_no_assumed_unit` and `value_unit_is_bounded_by_the_lo_length` for the lead's rulings |
  | AC7 | `value_uses_the_per_frame_rescale` and `value_reads_the_per_frame_rescale_type` |
  | AC8 | `value_reports_padding_without_a_number` |
  | AC9 | `value_refuses_a_modality_lut_sequence` |
  | AC10 | `value_closed_right_edge_and_outside_refusal` |
  | AC11 | `value_route_reads_the_synthetic_named_pixels` |
  | AC12 | `value_and_preview_agree_on_stage_one` (in `src/value.rs`) |
  | AC13 | `value_points_header_is_bounded` |
  | AC14 | `value_ignores_window_headers` |
  | AC15 | `the_page_offers_a_folder_picker` |
  | AC18 | `value_route_takes_only_a_held_id` |
  | AC19 | `value_on_a_held_frame_does_not_decode_again` |

  AC20 was the reviewer's transcription check, done in pass 1 against PS3.3 2026d and recorded in the lead rulings. AC9 and AC10 are mapped but are checked at the function and not at the HTTP route (S1).
- **External ACs.** AC16 (a browser hover reading `C 65  R 49  40 HU`) is for sprint close. AC17 (pydicom `apply_modality_lut` at three pixels on three corpus CT rows) is open until someone runs it by hand. Neither was run.
- **Independent challenge of the route.** These are probes through the real HTTP caller on both the direct and the process-server viewer.
  - At HEAD, a point outside the image is 400 `point`. Frame 3 of single-frame slice 0, asked before any decode, is 400 `frame`. `GET /held` reports `inflight` 0 after both the refusal and a 200.
  - If frame 0 is decoded first, the single-frame file's bytes drop, and then frame 3 answers 410 `file-gone`. That is F-216's existing rule and is the same for `/preview`, so it is not counted here.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | HEAD tree above | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | HEAD, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel -p ocelli-dicom` | HEAD | now | 0. Every binary `ok`, folder viewer lib 72 passed, `tests/value.rs` 23, `tests/preview.rs` 59. The `ocelli-dicom` corpus binary has 3 ignored |
| `npm run lint`, `npm run typecheck` | HEAD | now | 0, 0 |
| `bin/ocelli.sh gate guards`, `bin/ocelli.sh gate bench` | HEAD | now | 0 ALL GREEN, 0 ALL GREEN |
| `python3 scripts/guard_census.py`, `--check-runbook` | HEAD | now | 0, 0. "0 watched by nothing" |
| `python3 scripts/prose_check.py`, `scripts/source_provenance_check.py` | HEAD | now | 0 "voice rules clean over 878 file(s)", 0 "clean over 1734 files" |
| Corpus: `borrowed_bytes_match_the_parsed_pixels_for_every_accepted_corpus_file`, `recycled_grey_matches_a_fresh_decode_for_every_accepted_corpus_file`, AC12 `value_and_preview_agree_on_stage_one` | scratch `git archive HEAD` under `target/f210-p4`, own target dir, `corpus/data` symlinked to the main checkout | now | 3 passed in 2.80 s (the corpus loops ran, since without the symlink the tests return at once) |
| M6 scan cut to one look | scratch, HEAD `held.rs` | now | red: `a_claim_passes_over_every_frame_being_read` only |
| M3 `take_oldest_frame_buffer` never steals | same | now | red: `the_steal_route_makes_room_for_a_long_unit` (the new claim count check) and F-216's `cached_frames_and_free_buffers_share_one_budget` |
| M14 the claim's reader predicate `> 99` (readers evicted) | same | now | red: the new held test (`R1 was evicted`) and the steal test |
| M1 steal route makes no room | same | now | red: the steal test |
| N2 probe, HEAD against `222edbe` `held.rs` | same, 4 budgets x 400 steps | now | identical logs, no count drift |
| MR1 decode refusal in `value_route` answers `gone()` | scratch `serve.rs` | now | **green, whole folder viewer suite**. A probe shows frame 3 then answers 410 `file-gone` on both viewers instead of 400 `frame` (S1) |
| MR2 `value_json` refusal in `value_route` answers `gone()` | same | now | **green, whole suite** (S1) |
| MR3 scheduled failed decode does not release inflight | same | now | green, whole suite. My probe's decode refusal (`frame`) comes before any sample charge, so it does not reach this arm either (S2) |
| MR4 direct path never leaves inflight | same | now | **green, whole suite**. A probe shows `GET /held` `inflight` 24576 after one 200 `/value` on the direct viewer, and it stays there (S2) |
| MR5 a hit labelled `decoded` | same | now | red: `value_on_a_held_frame_does_not_decode_again` |
| `bin/ocelli.sh bench viewer.resident_memory viewer.open` | HEAD, load 3.67 at the time | now | exit 0 without measuring: "The benchmark harness's browser is not installed" in this worktree's `tools/bench`. I did not install it, because the pass may not modify the worktree |

The scratch copy lived in `target/f210-p4/` with its own `CARGO_TARGET_DIR`. Every mutated file was restored from the saved original and `cmp`'d against the worktree before the next step. After the corpus symlink was unlinked, the copy was deleted with `shutil.rmtree`. The main checkout's `corpus/data` is intact. I then touched the worktree's `held.rs`, `serve.rs` and `tests/value.rs`. At the end, `git status` showed only the untracked review reports, HEAD was still `354b73a`, and `git write-tree` was still `56ec237`.

## Defects

None.

## Smells

### S1, the `/value` route's two refusal arms are unexercised, so AC9's and AC10's "400" is untested at the route

**Where**: `examples/folder-viewer/src/serve.rs:1317` (`Err(error) => return error_reply(error)`, a decode refusal from `decode_for_value`) and `serve.rs:1326` (`Err(error) => error_reply(error)`, a `value_json` refusal). Both have been there since `e5dcf15`.
**What**: Changing either arm to answer `gone()` leaves the whole suite green (MR1, MR2). The plan's AC9 says a Modality LUT Sequence "returns 400 code `window`", and AC10 says an outside point "is 400 code `point`". Their named tests call `value_frame` and compare `PreviewError` variants, so no test observes an HTTP status for any refusal after `points` and `file-gone`.
**Failure mechanism**: The page treats 410 as "re-post the file and retry once" (`fetchValue`). A refactor that sends a refusal down the 410 path would re-upload the file on every pointer cell of a refused frame, such as a Modality LUT object, a point at the edge or a malformed padding range. The readout would still show only `C` and `R`, so nothing on screen changes, and the suite stays green. MR1 shows this for a decode refusal: frame 3 answered 410 `file-gone` on both viewers.
**Regression**: One route test on `both_viewers()`. Post slice 0 and ask frame 3 before anything decodes, which must be 400 `frame`. Then ask a point past the right edge, which must be 400 `point`. Optionally ask a Modality LUT object, which must be 400 `window`. The test must go red under MR1 and MR2.

### S2, the value route's inflight bookkeeping has no test, so pass 1's N2 hazard is now concrete

**Where**: `serve.rs:1394`, `1405` (scheduled path) and `serve.rs:1436` (direct path) in `decode_for_value`.
**What**: Deleting the direct path's `leave_inflight` (MR4) leaves the suite green. With MR4, one 200 `/value` on the direct viewer leaves `GET /held` reporting `inflight` 24576 permanently. The scheduled path's failure release (MR3) is unreached by any test as well. F-216's census tests check that `inflight` returns to 0 after previews (`tests/preview.rs:2078`, `2109`, `2129`, `2370`), but none of them sends `/value`.
**Failure mechanism**: Pass 1's N2 named `decode_for_value` as the third copy of the decode, charge, store and release sequence, and noted that a later fix to the accounting must reach all three copies. These mutations show that nothing would notice if it did not. The resident memory accounting that `use.html` describes and `GET /held` reports would then drift by one frame's sample bytes per value decode.
**Regression**: Extend the S1 test, or `value_on_a_held_frame_does_not_decode_again`, to read `GET /held` after a decoded `/value` and after a refused one, on both viewers, and require `inflight` 0. It must go red under MR4. A refusal that happens after the sample charge would cover MR3, for example truncated pixel data, if the fixture can be built cheaply. Otherwise note that arm as unreached.

## Nitpicks

- **N1.** The reworded `claim_buffer` comment was not rewrapped. `held.rs:1502` runs to 88 columns while its neighbours stop at 77 to 78. rustfmt does not wrap comments, so nothing will fix it.
- **N2.** In the steal test, the "reused +1" check cannot tell a stolen buffer from one already parked of the same length. The fixture has no parked buffer today, and M3 shows the check discriminates as written. I note it only so that a fixture change that parks a buffer is not mistaken for a steal.
- Carried and not counted again:
  - pass 3 N1 (the full-cost room costs one entry on an exactly full cache), for the lead
  - pass 3 N3 (the plan's superseded Alternatives sentence)
  - pass 2 N2 (`read_padding` forwards)
  - pass 1 N2 to N4. Pass 1's N2 is the duplication itself, and S2 here is the missing guard, not the duplication.

## Verified clean

- **The pass 3 remediation diff, line by line.**
  - The `claim_buffer` comment is true against `take_oldest_frame_buffer`.
  - The new held test's comment is true. Every claim in it was checked against the arithmetic below.
    - Both readers' `Arc`s are held by the test, and U is dropped.
    - 4 bytes matches no cached length.
    - The budget is `2 x reader_charge + frame_sample_cost(4)`, so evicting U alone is enough.
    - The 4-byte frame's real charge is at most `frame_sample_cost(4)`.
  - Under M6, the claim moves R1 to the newest end and gives up, and `reserve_for_insert` then evicts R2. That is the observed red.
  - The steal test's renamed comment and messages are true.
  - The rest of `354b73a` is the N2 substitution.
- **The whole-story sweep.**
  - `value.rs`:
    - The cell rule: `floor`, `x == extent` is the last cell, non-finite or outside is `None`. The f32 narrowing is exact below 2^16.
    - `stored_at` reuses `unpack` over one sample, and its `bits_allocated / 8 == 0` refusal is in place.
    - `stored_integer`'s checked conversions are in place.
    - The over-24-bit padding refusal is tested by `value_refuses_padding_above_24_stored_bits`.
    - The non-finite value refusal is in place.
    - The `Display` comment is true: Rust's float `Display` is the shortest round-trip text and never uses an exponent.
    - Both claims in the `value_json` refusal-order doc hold: points are checked before the facts, and no partial list is returned.
  - `modality_unit` matches the plan table and the lead rulings, for PT Units over Rescale Type, `US`, multi-energy `YES`, and the 64-character bound.
  - `prepare_held`:
    - The value is `Err` on every early refusal, with the same code as the recipe.
    - It is built from the same `modality` the recipe holds.
    - Colour is refused earlier, by `SampleLayout::new(1, ..)` at `preview.rs:618`, so `stored_at`'s one-sample indexing is never reached for a colour frame.
  - `parse_points`:
    - 64 is accepted and 65 refused.
    - Empty pairs from doubled or trailing spaces are refused.
    - `inf` and `NaN` are refused, and a repeated header is refused.
  - The page:
    - The column is clamped to `[1, width]` and `imagePoint` already refuses points outside `[0, width]`, so the page and server cells agree, including at `x == width`.
    - The generation and cell check is pinned by the contract test.
    - A 410 re-posts once.
    - Colour images send nothing.
  - `README.md` step 7, the page sentence, the `use.html` `read_padding` sentence and `docs/lld/frame-open.md` match the code. The `use.html` cache paragraph at line 270 is still true, since the claim's pass over readers does not contradict "evicts to make room".
  - `frame_sample_cost` covers the struct, the boxed facts and `UNIT_MAX_BYTES`, so a stored frame never charges more than its claim.

## Limitations and handoff

- **Bench.** `viewer.resident_memory` and `viewer.open` were not measured. The load condition held (3.67), but the worktree has no bench browser install, and installing one would modify the worktree. There is no new figure against the baseline.
- **Not exercised.**
  - A browser (AC16) and pydicom (AC17).
  - The scheduled path's charged-decode failure (MR3). No cheap trigger was built.
  - The two defensive arms N2 changed, which no realistic sequence reaches.
- **Probes.** The N2 count probe and the route probes were scratch tests and were deleted with the copy. Their shapes are given above so the S1 and S2 regressions can be rebuilt.
- **Next owner.** The implementer adds the S1 and S2 route tests. These add test code only and no behaviour change. A fresh independent pass 5 then reviews them. No finding has survived three passes. S1 and S2 are new, found by the whole-story sweep, and both have been in the code since `e5dcf15`.
