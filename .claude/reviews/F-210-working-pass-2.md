# F-210 review, pass 2

**Reviewed**: `git diff e5dcf15 HEAD` on `work/f-210-xai`, HEAD `bb0419f`, tree `dd3d7428c719f33b487b00a77d62e7ced3035e2d`. That covers `9ce0719` (the pass 1 fixes) and `bb0419f` (the claim covers the longest unit). Nothing was staged or unstaged before or after the pass. The only untracked paths are `.claude/reviews/F-210-working-pass-1.md` and this report.
**Author / reviewer**: The remediation is by a Claude implementer agent. This review is by a separate Claude reviewer agent that wrote none of F-210, its plan, its remediation or the pass 1 review. One limit on independence: all of them are the same model family.
**Result**: 1 defect, 3 smells, 4 nitpicks

## Acceptance challenged

- **Pass 1 closure, through the real route.** Each pass 1 trigger was replayed as a mutation in a scratch copy of HEAD. D1, D3, S1, S2, S3 and S4 now go red. See Evidence, M0 to M20.
- **The lead ruling at HEAD, "the claim covers the longest unit".** I checked it against the claim routes in `CacheState::claim_buffer`, and not only the fresh route that the new test drives. There are three routes: a parked buffer of the same length, a fresh buffer, and a buffer stolen from the oldest frame of the same length. The steal route makes no room for the difference between the stolen frame's charge and the new frame's charge. A discriminating probe shows the store then evicting a frame that a reply is reading. With the same layout and a unit from the rule, the frame is kept. See D1.
- **The Presentation LUT Shape `BOGUS` case.** The assignment calls this "a refusal in both preview and value". On HEAD that is **not** what happens, and per the lead ruling it should not be. `value_ignores_attributes_stage_one_never_reads` pins `/value` answering `40 HU` and the preview, with a centre 40 and width 400 override, refusing `window`. The ruling says "a window, presentation, lossy or burned-in attribute the value never reads cannot refuse it", so the case refuses in the preview alone. M19 makes the value refuse a `BOGUS` shape and goes red, so the test pins it. I judge this behaviour correct.
- **The standard, checked myself against PS3.3 2026d and PS3.5 2026d** (fetched from dicom.nema.org `current/output/chtml` during this pass, edition string "DICOM PS3.3 2026d"):
  - Multi-energy CT Acquisition (0018,9361) is Type 3 at the top level of the CT Image Module, PS3.3 C.8.2.1 Table C.8-3. It is also Type 3 at the top level of the Enhanced CT Image Module, C.8.15.2 Table C.8-114. Its Enumerated Values are YES and NO. So the top-level read in `value_facts` is the right place for both single-frame and enhanced CT.
  - C.8.2.1, Rescale Type: "Required if the Rescale Type is not HU (Hounsfield Units), or Multi-energy CT Acquisition (0018,9361) is YES. May be present otherwise." Rescale Intercept: "If Image Type (0008,0008) Value 1 is ORIGINAL and Value 3 is not LOCALIZER, and Multi-energy CT Acquisition (0018,9361) is either absent or NO, output units shall be Hounsfield Units (HU)." Both match the code comment, the test comment and the plan's ruling.
  - C.8.9.4 PET Image Module, Rescale Intercept: "The value b in relationship between stored values (SV) and pixel value units (U) defined in Units (0054,1001): U = m*SV+b." Units (0054,1001) is Type 1 in the PET Series Module, C.8.9.1. The citations in `value.rs` and `tests/value.rs` are correct.
  - PS3.5 8.1.1 says the receiver "may not assume anything about the contents of unused bits", and "The sign bit shall be the High Bit". PS3.3 C.7.6.3 says "High Bit (0028,0102) shall be one less than Bits Stored (0028,0101)". The corrected citations are right.
  - PS3.5 Table 6.2-1: LO is "64 chars maximum (see Note in Section 6.2)" and CS is "16 bytes maximum". The Section 6.2 note says that lengths of VRs whose repertoire can be extended "are expressly specified in characters rather than bytes ... because the mapping from a character to the number of bytes ... may be dependent on the character set". So LO counts characters, and `chars().count()` is the right measure. 64 times `char::MAX_LEN_UTF8` = 256 is the correct owned bound for a Rust `Box<str>`.
  - PS3.3 C.7.6.1 Table C.7-9: Burned In Annotation has Enumerated Values YES and NO, as the new `ocelli-dicom` test says.
- **Open decision 4** (Image Type ORIGINAL and not LOCALIZER is not read). I agree with leaving it out. For a conformant object, Rescale Type's own 1C condition means an absent Rescale Type is HU whatever the Image Type. The Rescale Intercept clause states when the output is HU, not when Rescale Type may be omitted.
- **AC12 on the corpus.** I rebuilt the reviewer probe independently and ran every manifest row through `decode_held`, `recipe`, `value` and `value_json` at the centre and at (0, 0). The result: 86 rows with both and 0 stage 1 mismatches (now equal by construction, see N4), 0 rows where the recipe holds and the value is refused, and 9 refused at decode (8 `not-monochrome`, 1 `attribute`). Units: 67 `HU rescale-type`, 18 `unspecified none`, 1 `unspecified rescale-type`. These match pass 1 and the implementer.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo fmt --all --check` | worktree, tree dd3d7428 | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | same, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel -p ocelli-dicom` | same, no `corpus/data` in worktree | now | 0. Folder viewer lib 69, `tests/value.rs` 21, `tests/preview.rs` 59, `ocelli-dicom` `tests/frame_open.rs` 24, all ok |
| `borrowed_bytes_match_the_parsed_pixels_for_every_accepted_corpus_file`, `recycled_grey_matches_a_fresh_decode_for_every_accepted_corpus_file`, `value_and_preview_agree_on_stage_one` | scratch `git archive HEAD` under `target/review210p2/ws`, `corpus/data` symlinked to the main checkout | now | 0, all 3 ok |
| reviewer corpus probe (scratch test, not kept) | same scratch | now | as under AC12 above |
| mutations M0 to M21, below | same scratch, each applied alone, restored and touched | now | 19 red, 3 survived (M10, M16, M21) |
| steal-route probe, below | same scratch | now | red with a 64-letter unit, green with the rule unit. D1 |
| HEAD's `tests/value.rs` and `serve.rs` tests on `9ce0719` | scratch `git archive 9ce0719` under `target/review210p2/ws9`, sources touched before the build | now | `a_long_file_unit_does_not_evict_a_frame_being_read` red ("the frame being read answered 410 from None"). `value_unit_is_bounded_by_the_lo_length` red (65 letters answered) |
| `npm run lint`, `npm run typecheck` | worktree | now | 0, 0 |
| `bin/ocelli.sh gate guards`, `gate bench` | worktree | now | 0 ALL GREEN, 0 ALL GREEN |
| `guard_census.py`, `--check-runbook` | worktree | now | 0, 0. The four LIMIT lines are the existing bench and panic-probe limits |
| `prose_check.py`, `source_provenance_check.py` | worktree | now | 0 (878 files), 0 (1734 files) |
| implementer mutations and corpus runs | `.claude/scratch/F-210-progress.md` | prior receipt | not relied on. Re-derived above |

One measurement trap is worth recording. The first `9ce0719` run reused a target directory cloned from the HEAD scratch, and `git archive` gives the files the commit's mtime. The result was wrong: the multi-energy test failed and the long-unit test passed. After touching every source file, the result was the one in the table. Anyone repeating this should touch the sources or use a fresh target.

Mutations. "Red" means `cargo test -p ocelli-folder-viewer` failed, with `-p ocelli-dicom` added where the site is in that crate:

- M0, the multi-energy guard always true: red (`value_multi_energy_ct_has_no_assumed_unit`). M1, `NO` not accepted: red. M2, the CT arm reads no Multi-energy CT Acquisition: red.
- M3, the PT `Units` arm moved after the `US` arm (S4): red (`value_pet_units_decide_over_rescale_type`).
- M4, Rescale Type read at the top level (S2): red (`value_reads_the_per_frame_rescale_type`).
- M5, the 24-bit guard at 64, and M6, the guard as `>=` (S3): both red (`value_refuses_padding_above_24_stored_bits`).
- M7, `charged_bytes` without the facts: red. M8, `heap_bytes` without the unit: red.
- M9, the claim without `UNIT_MAX_BYTES`: red (`a_long_file_unit_does_not_evict_a_frame_being_read`).
- **M10**, the claim without `size_of::<ValueFacts>()`: **survived** (S1).
- M11, byte bound equal to the character bound: red. M12, no length refusal: red. M13, character bound 63: red. M14, length counted in bytes: red.
- M15, `read_padding` returns `Ok(None)`: red (`padding_is_read_alone_where_frame_open_refuses_a_flag`).
- **M16**, `value_facts` ignores a `read_padding` error (`.unwrap_or(None)`): **survived** (S2).
- M17, MONOCHROME1 padding order flipped in `padding_of`: red in two `ocelli-dicom` tests.
- M18, the value refuses a non-positive Window Width, and M19, the value refuses a `BOGUS` shape (D3 regressions): both red.
- M20, the F-216 fixture with 0 comment bytes: red (`single_frame_bytes_drop_and_preview_stays`).
- **M21**, the non-finite value refused as `window` again: **survived** (N3).

`ValueFacts` measured 120 bytes and `HeldFrame` 376 bytes in this build, from a scratch `eprintln!` that has since been removed.

The scratch tree `target/review210p2`, with `ws`, `ws9`, `tgt` and `tgt9`, was removed with `shutil.rmtree` (it does not follow the corpus symlink). `git status --short` shows only the two review files, and `git write-tree` is still `dd3d7428`.

## Defects

### D1, the steal route of the claim makes no room for the new frame's larger charge, so a frame being read is evicted

**Where**: `examples/folder-viewer/src/held.rs`, `CacheState::claim_buffer`, the branch `if let Some(buffer) = self.take_oldest_frame_buffer(len) { self.park_if_fits(buffer); if let Some(buffer) = self.pop_free(len) { self.note_reused(); return buffer; } }` (about lines 1511 to 1517), together with the doc comment on `frame_sample_cost` at about 1476 to 1480.
**What**: When a claim does not fit, `claim_buffer` steals the buffer of the oldest unread frame of the same sample length and returns it. The other two routes call `make_room_dropping`, but this one does not. Removing the stolen frame frees exactly that frame's charge. Since F-210 that charge is no longer the same for every frame of one length. A frame whose unit comes from the rule owns 0 unit bytes. A frame whose file names its unit owns up to 256. A frame whose value was refused owns no `ValueFacts` at all. When the new frame's charge is larger than the stolen one's, `reserve_for_insert` at store time evicts the oldest entry. That can be a frame whose reply is still reading it. This is the failure the lead's ruling at HEAD set out to prevent, reached through the route the new test does not drive. The `frame_sample_cost` doc says "so the charge when the frame is stored never exceeds this claim", which is false on this route.
**Trigger and evidence**: I added a scratch test next to `a_long_file_unit_does_not_evict_a_frame_being_read`, built with that test's helpers:
  1. Frame X (slice 2, unit from the rule, nobody reading) is cached first.
  2. Frame A (slice 0) is cached next and held through `preview_source` as the reading `Arc`.
  3. File B is slice 1 with a 64-letter Rescale Type and 1024 bytes of Image Comments, so file B is big enough that A's own claim does not steal X.
  4. The budget is `2 * frame_used + file_b_cap + 63`, which is the HEAD test's formula.
  5. B is previewed, then A is previewed again.

  HEAD prints `b 200 again 410 None used 25136` (B charged 25072 + 64), so A was evicted while it was read. The **control**, the same test with B as plain slice 1 plus the comments (unit from the rule), prints `again 200 Some("held") used 50144`. Only the unit's 64 bytes separate the two runs. Before F-210 every frame of one length charged `len + size_of::<HeldFrame>()`, so a stolen frame always freed exactly the new frame's charge. This route gap is new with `9ce0719` and was not closed by `bb0419f`.
**Why it is wrong**: The lead ruling recorded for HEAD is that "the pre-decode claim covers the longest legal unit", and the code comment states the invariant for every claim. F-216's design (`.claude/plans/F-216-design.md:151`, `:279`) requires that the claim and the store agree, so that a store never evicts behind a reader. The visible effect is a 410 on the next `/preview` or `/value` for A, which makes the page re-post the file. No pixel or value is wrong, but the cache promise the ruling made is broken.
**Scope and regression**: `/preview` and `/value` on any cache tight enough to steal, with files that name their unit, or with a refused-value frame stolen for a grey frame that has facts. The smallest fix: on the steal route, call `self.make_room_dropping(frame_sample_cost(buffer.capacity()))` before returning, as the parked-buffer route already does. Better still, make every route's room come from that one cost. Regression: the probe above as a test, with the steal shown happening, which is the HEAD test's shape with an unread older frame of the same length.

## Smells

### S1, the claim's `ValueFacts` term is unguarded

**Where**: `examples/folder-viewer/src/held.rs`, `frame_sample_cost`, the line `.saturating_add(size_of::<crate::value::ValueFacts>())`.
**What**: M10 deletes that line and every test still passes. `ValueFacts` is 120 bytes, so the 256-byte unit bound absorbs it for every unit up to 136 bytes. The 64-letter unit in `a_long_file_unit_does_not_evict_a_frame_being_read` owns 64.
**Failure mechanism**: A refactor that drops the term, or that grows `ValueFacts`, gives a claim that is short whenever the unit owns more than 256 minus 120 bytes. That happens with a legal 64-character LO in a multi-byte character set, such as 64 characters of 3 or 4 UTF-8 bytes each. In that case the store evicts behind a reader, as in D1, and no test notices.
**Regression**: Drive the HEAD eviction test with a 64-character unit of 4-byte characters (the `an_owned_unit_is_charged_and_a_rule_unit_is_not` string `"\u{10000}".repeat(64)`), whose 256 owned bytes leave no slack, and set the budget from that charge.

### S2, nothing tests that a malformed padding range refuses the value

**Where**: `examples/folder-viewer/src/value.rs`, `value_facts`, the line `read_padding(...).map_err(|_| PreviewError::Attribute)?`.
**What**: M16 replaces the refusal with `.unwrap_or(None)` and every test passes. The `ocelli-dicom` test proves the reader's own errors, but nothing proves that the value route keeps the refusal.
**Failure mechanism**: Under the mutant, a file with a Pixel Padding Range Limit and no Pixel Padding Value, or with a range in the wrong order for its photometric interpretation (PS3.3 C.7.5.1.1.2), is answered as if it had no padding. Padded pixels then show as numbers with a unit, which is the quietly wrong value. Before `9ce0719` this refusal came from `open_frame` and was implicit. Now it is one line that a refactor can lose.
**Regression**: Add to `tests/value.rs` a limit without a value and a reversed MONOCHROME2 range, each refused `attribute`.

### S3, the ruling at HEAD is not in the plan, and neither is the refusal it adds

**Where**: `.claude/plans/F-210-design.md`, "Lead rulings and corrections after review pass 1". `bb0419f` does not touch the plan.
**What**: The plan's last ruling says only that the facts' heap bytes are charged. HEAD adds two behaviours that the plan, which "holds where it differs", never states. The first is that the pre-decode claim includes the longest unit. The second is that a Rescale Type or Units longer than 64 characters refuses the value `attribute`. The second is a user-visible refusal policy, recorded only in `.claude/scratch/F-210-progress.md` and the commit subject.
**Failure mechanism**: `/complete-feature` writes AS_BUILT and the LLD from the plan and the ledgers. A later reviewer checks refusals against the plan. A 65-character Rescale Type withholding the value would then read as an unplanned refusal, or be removed as one. It is the same AC20 rule pass 1 applied: corrections land in the plan.
**Regression**: One ruling bullet naming the claim bound and the 64-character refusal, with the PS3.5 Table 6.2-1 and Section 6.2 note citation.

## Nitpicks

- N1. README step 7 and the page's cross sentence say "HU on a CT that names no other unit and is not multi-energy". A CT with Rescale Type `US` arguably names no unit, and it shows `unspecified`, not HU. "a CT with no Rescale Type that is not multi-energy" would be exact. The sentence is true for every other case I ran: Rescale Type HU, HU_MOD, Z_EFF, multi-energy YES and NO, and PT with and without Units.
- N2. `ocelli_dicom::read_padding` is a public wrapper whose whole body is `padding_of(metadata, photometric, signed)`. It is the same reader `open_frame` calls at `frame_open.rs:349`, with no second copy, and `docs/lld/frame-open.md` and `use.html` describe it truly. Making `padding_of` itself public under that name would remove the forwarding layer.
- N3. The N1 fix from pass 1 (a non-finite value is `attribute`) is untested, because M21 survives. Slope `1e38` on stored 65535 reaches it.
- N4. `value_and_preview_agree_on_stage_one` (AC12) now compares `value.modality` with a clone of `recipe.modality`, so it is true by construction. It still guards a future split, but it no longer tests independent readers. The plan's AC12 wording could say so.
- Carried from pass 1, unchanged and not counted again: N2 (the third decode-charge-store copy in `decode_for_value`), N3 (a duplicate request on pointer re-entry) and N4 (the test name `value_fractional_rescale_keeps_two_decimals`).

## Verified clean

- **Pass 1 findings closed.**
  - D1: multi-energy is read at the top level for CT only, and HU only for absent or NO.
  - D2: citations corrected in `value.rs`, `tests/value.rs` and the plan ruling.
  - D3: the value takes stage 1 from the recipe's `ModalityTransform` after the same Modality LUT, rescale and transform refusals, and padding from `read_padding`. `open_frame` is no longer called. All five pass 1 rows answer `40 HU`.
  - S1: `charged_bytes` is used by both `Budgeted::bytes` and `reserve_for_insert`.
  - S2, S3 and S4: each now has a red-on-mutation test.
- **Implementer open decisions, judged.**
  - From `9ce0719`: 1 (public `read_padding`), fine. 2, superseded by HEAD. 3 (an unreadable or non-YES/NO Multi-energy value gives no HU default), fine and conservative. 4, agreed, see above. 5, fine. 6 (`FrameFacts` and `Recipe` heap not charged), carried from F-216. 7, fine.
  - From HEAD: 1 (the claim overstates by up to 256 bytes), acceptable. 2 (count characters), correct per the PS3.5 Section 6.2 note. 3 (Units above CS's 16 bytes and within 64 characters accepted), harmless, because the bound covers it and the text is the file's own.
- **`read_padding`** is the reader `open_frame` uses. The signedness passed from `value_facts` is `description.bits.pixel_representation()`, which has the same source as `open_frame`'s `pixel_representation(&metadata)`. The doc's error list matches `one_stored` and `padding_of`. The LLD signature block and sentence and the `use.html` sentence are true.
- **Budget.** `ValueFacts::heap_bytes` is the boxed struct plus the `Box<str>` length. `ModalityTransform` here is always the `Rescale` variant, with no heap, because the recipe refuses a Modality LUT first. `UnitText::Rule` owns nothing. The fresh-route and parked-route claims cover the charge. The steal route does not (D1).
- **The F-216 fixture change.** 1024 bytes of LT Image Comments go in before (0028,0002), in tag order, and change no pixel or chain input (the body is still `[64]`). Every original check stays meaningful: `files == 0`, `frames == 1`, `used > 0`, `used < file.len()` (a regression that held the file as well would have `used` above the file's length), the 410 after the frame, and `held` on re-preview. M20 is red.
- **Docs executed.** README step 7, the page `toggle-cross` sentence (pinned by `the_page_offers_a_folder_picker`), the `use.html` `read_padding` sentence and the `frame-open.md` paragraph were each checked against the code and the tests above. `more.html` has no sentence that this diff makes stale.

## Limitations and handoff

- Not exercised: a browser (AC16 is for sprint close), pydicom (AC17), a live `/value` over HTTP for the five D3 rows (`value_frame` and the route share `decode_held` and `prepare_held`, and AC14 covers the route), and multi-energy, PET or 32-bit objects from real devices, because the corpus has none.
- D1 was reproduced only with a synthetic three-file layout. Its prevalence on real folders depends on cache pressure and same-length frames, which is the usual case within one series.
- Next owner: the implementer fixes D1 and S1 to S3. A fresh independent pass 3 then reviews the change.
