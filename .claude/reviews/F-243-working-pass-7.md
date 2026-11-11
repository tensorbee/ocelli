# F-243 review, pass 7

**Reviewed**: `a399933d..89aa715a` at tip `89aa715a` on `work/f-243-xai`, worktree `ocelli-wt-f-243`. Two commits: `a1ea8692` (Claude implementer, remediation of pass 6 by the lead's ruling) and `89aa715a` (sprint lead, two upstream comments corrected, with re-pins). No unstaged tracked paths. `.venv` is untracked and was used only to run imagecodecs.
**Author / reviewer**: commits by a Claude implementer agent and the sprint lead. Reviewed by an independent Claude reviewer agent that wrote none of the range.
**Result**: 4 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **The lead's ruling after pass 6.** Each of pass 6's D1 to D4, N1 and N2 replayed against the tip and against ITU-T T.87 (06/1998).
- **"Only comments changed."** Checked by a comment-filtered diff, not taken from the commit message.
- **`PATCH-PROVENANCE.md:31` to `:35`**, "the upstream clause citations in the patched files were corrected against that text". Challenged by a sweep of every clause citation in `decoder.rs`, `context/mod.rs`, `scan.rs`, `bitstream.rs` and `encoder.rs`, plus every other comment in those files that makes a claim T.87 can contradict.
- **The run miss whose remainder ends exactly on the line end** (pass 6's unconfirmed concern, the ruling's last bullet). Reproduced independently with my own stream builder. A mutant that decrements RUNindex on that miss was built in a scratch copy, so the comparison is shown to discriminate.
- Standard source: the zip `T-REC-T.87-199806-I!!ZPF-E` fetched now from `itu.int/rec/dologin_pub.asp`. Only `software/T87/t87.pdf` was extracted (SHA-256 `4ca6f981...a8778`). `Software.zip` was not extracted or opened.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git diff -U0 a399933d..89aa715a -- '*.rs' '*.py'`, lines not starting with `//` or `#` | tip `89aa715a` | now | only `RITK_PROVENANCE_SHA256` and the `bitstream.rs`, `context/mod.rs` and `scan.rs` values in `RITK_PATCHED_HASHES`. Every other changed `.rs` or `.py` line is a comment. `PATCH-PROVENANCE.md` changed only its three SHA-256 lines |
| `pdftotext -layout` of `t87.pdf` | itu.int zip | now | A.2.1 step 1 p. 17: `LIMIT = 2 * (bpp + max(8,bpp))`, `bpp = max(2, ceil(log(MAXVAL+1)))`. A.2.1 step 2: indices `[0..364]` regular "(total of 365, see A.3)", `[365]` and `[366]` run interruption. A.3.4 NOTE p. 19: 729 vectors. A.5.1 Code segment A.10: `for(k=0; (N[Q]<<k)<A[Q]; k++)`. A.7.2 item 8 p. 25: `LG(k, glimit)`, `glimit = LIMIT - J[RUNindex] - 1`. F.1 step 14 p. 47: the `RANGE*(2*NEAR+1)` wrap |
| SHA-256 of the six `RITK_PATCHED_HASHES` files and `PATCH-PROVENANCE.md` | tip | now | all seven match their pins, and each file pin also appears in `PATCH-PROVENANCE.md` |
| `git archive HEAD` into scratch, one byte `JPEG` to `JPEH` in each of the six patched files and `PATCH-PROVENANCE.md`, backup kept outside the tree, `python3 scripts/pin_and_size_check.py` | scratch copy of tip | now | exit 1 each time, "has undeclared changes beyond its recorded patch" (or "from the exact reviewed patch record"). Restored: exit 0 |
| `bin/ocelli.sh gate pins` on the scratch copy with `bitstream.rs` `max(8, bpp)` to `max(9, bpp)` | scratch copy | now | FAILED pins, exit 1. Restored: ALL GREEN. Copy removed with `shutil.rmtree` |
| `bin/ocelli.sh gate pins` | worktree tip | now | ALL GREEN, exit 0 |
| `python3 scripts/prose_check.py` | tip | now | OK over 971 files, exit 0 |
| `python3 scripts/source_provenance_check.py` | tip | now | OK over 1852 files, exit 0 |
| `python3 scripts/deviation_check.py` | tip | now | OK, 32 deviations, exit 0 |
| `cargo test -p ocelli-codec -p ocelli-dicom`, `CARGO_TARGET_DIR=$PWD/target` | tip | now | exit 0, 365 passed, 0 failed, 3 ignored |
| Run miss at the line end: 4 prefix families, each followed by every bit string of length 1 to 12 (8,190 suffixes), zero padded, A.1 stuffing, 2 rows, P 8, NEAR 0, no LSE. Decoded by CharLS 2.4.4 through imagecodecs 2026.10.10 and by a scratch binary calling the vendored `decode_jpeg_ls_fragment` | prefixes `1111 0 1` (width 5, J 1, remainder 1), `11111 0 1` (width 7), `11111111 0 11` (width 15, J 2, remainder 3), `11111111 0 10` (width 14) | now | see the table under Verified clean. Tip: 1,600 streams both open, all sample for sample equal. No stream Ocelli opens and CharLS refuses. Mutant: 58 sample mismatches and 852 verdict flips |
| added-line scan for U+2013, U+2014 and `;` | `git diff -U0` of the range | now | `bitstream.rs:110` keeps its upstream em-dash and semicolon on the rewritten line (N3). No other |

## Defects

### D1, the `limit` argument is documented as the fixed LIMIT, and the run interruption passes glimit

**Where**: `vendor/ritk-codecs-0.6.0/src/jpeg_ls/bitstream.rs:110`, rewritten in `89aa715a`
**What**: the `limit` argument line says the value is `2*(bpp + max(8, bpp))`, citing T.87 A.2.1, with no qualification. `read_golomb` is called from two sites. `scan.rs:266` passes `limit`, which is LIMIT. `scan.rs:227` passes `limit - J[run_index] - 1`.
**Why it is wrong**: T.87 A.7.2 item 8, p. 25, codes the run interruption sample with `LG(k, glimit)`, `glimit = LIMIT - J[RUNindex] - 1`. The formula is now right for LIMIT, but the comment says the argument always has that value, and for every run interruption sample it does not. The correction replaced one false fixed value with another.
**Evidence**: `grep -n read_golomb scan.rs` gives `:227` and `:266`. A.7.2 item 8 text from the PDF.
**Scope and regression**: comment only. Introduced in this range. Fix: "`limit`: the code length limit, LIMIT, `2*(bpp + max(8, bpp))` (T.87 A.2.1), for a regular sample, or glimit, `LIMIT - J[RUNindex] - 1` (A.7.2 item 8), for a run interruption sample". Drop the em-dash and semicolon while the line is rewritten. Then pin `bitstream.rs` and `PATCH-PROVENANCE.md` again.

### D2, `compute_k`'s comment gives the floor of log2(A/N), and A.10 gives the ceiling

**Where**: `vendor/ritk-codecs-0.6.0/src/jpeg_ls/context/mod.rs:208`, `:210` and `:215`, unchanged upstream text in a patched file
**What**: ":208 k = floor(log₂(A/N)), clamped to [0, qbpp]", ":210 Invariant: N×2^k ≤ A < N×2^(k+1)", ":215 ... (equivalent to k = ceil(log2(A/N)) but floored)".
**Why it is wrong**: A.5.1 Code segment A.10, p. 21, is `for(k=0; (N[Q]<<k)<A[Q]; k++)`, the smallest k with `N·2^k >= A`. That is `ceil(log2(A/N))`, and its invariant is `N·2^(k-1) < A <= N·2^k`. The code follows A.10, so the comment contradicts both the standard and the code under it. A.10 also has no `qbpp` bound.
**Evidence**: executed a mirror of the loop. A 5, N 1: the code and A.10 give k 3. `floor(log2 5)` is 2, and the stated invariant would need `8 <= 5`. A 3, N 2: k 1 against a floor of 0.
**Scope and regression**: comment only. Not in this range, caught by the sweep step 6 asks for. Fix: ":208 Golomb parameter k, the smallest k with N·2^k >= A (T.87 A.5.1 Code segment A.10), bounded here by qbpp", delete `:210`, and at `:215` delete the parenthesis. Pin `context/mod.rs` and `PATCH-PROVENANCE.md` again.

### D3, `ContextModel::new` gives RANGE as MAXVAL + 1, and the decoder accepts NEAR > 0

**Where**: `vendor/ritk-codecs-0.6.0/src/jpeg_ls/context/mod.rs:70`
**What**: "`a_init = max(2, (RANGE + 32) / 64)` where `RANGE = MAXVAL + 1`".
**Why it is wrong**: A.2.1 step 1, p. 17, gives `RANGE = MAXVAL + 1` only for NEAR 0, and `floor((MAXVAL + 2*NEAR) / (2*NEAR + 1)) + 1` for NEAR > 0. This decoder serves `.81` (`decoder.rs:88` bounds NEAR, it does not refuse it). The value passed in is right, since `CodingParams::new` at `:346` computes the near-lossless RANGE, so only the comment is wrong.
**Evidence**: A.2.1 step 1 text. `CodingParams::new` read at the tip.
**Scope and regression**: comment only, upstream text. Fix: "where RANGE is A.2.1 step 1's, `MAXVAL + 1` for lossless". Pin again as above.

### D4, `ContextState::default` is called the T.87 initial state and sets A to 0

**Where**: `vendor/ritk-codecs-0.6.0/src/jpeg_ls/context/mod.rs:31`
**What**: "Returns the ISO 14495-1 initial state: n=1 ...", and the body sets `a: 0`.
**Why it is wrong**: A.2.1 step 2, p. 17, initialises every A to `max(2, floor((RANGE + 2^5) / 2^6))`, which is at least 2. A state with A 0 is not the T.87 initial state. Only `context/tests.rs:192` uses `default()`, so no decode path is affected.
**Evidence**: `grep -rn "ContextState::default" src` returns that test line only.
**Scope and regression**: comment only, upstream text. Fix: "A zeroed state with n = 1, used by tests. The T.87 initial state is `ContextModel::new`'s". Pin again as above.

## Smells

None.

## Nitpicks

- **N1**, pass 6's N2 is still open. The ruling says "N2 taken", but the plan's last change is `a399933d`, and `.claude/plans/F-243-design.md:70` still quotes Table C.1 with `≤` beside the paragraph's `<=`. It does not block.
- **N2**, `scan.rs:48` "(§A.4.1, default)". `:28` to `:29` now say T.87 defines one predictor and the SOS header selects none, so "default" implies a choice T.87 does not offer. Separately, `scan.rs:34` says `Predictor::None` gives "left for others" while `predict` returns 0 for it away from the edges. The enum is dead code, so this contradicts the code, not the standard, and falls outside step 6. Flagged for the lead, because the severity table would count a false comment as a defect if it were in scope.
- **N3**, `bitstream.rs:110` was rewritten and kept upstream's em-dash and semicolon. prose_check does not cover the vendored tree. The D1 rewrite removes both.

## Pass 6 findings replayed

| Pass 6 | Status | Observation at `89aa715a` |
|--------|--------|---------------------------|
| D1, `reconstruct` cites A.4.4 for the wrap | closed | `context/mod.rs:385` to `:388` cite A.4.4 "with the decoder's modulo wrap from F.1 step 14". F.1 step 14, p. 47, holds that wrap, and the body at `:391` to `:397` is it line for line. A.4.4 Code segment A.8 holds the quantisation and the `Px + SIGN*Errval*(2*NEAR+1)` reconstruction |
| D2, the predictor block in `scan.rs` | closed | `:26` to `:30` drop C.2.3 and the "All 8 modes" sentence. C.2.3, p. 32, gives NEAR and ILV where T.81 has Ss and Se, so "the SOS header selects none" is right. `:48` and `:52` cite A.4.1, which is Code segment A.5, the body of `predict_adaptive`. `:74` drops MAXVAL/2. A.2.1 sets absent b, c, d to 0 on the first line and a = Rb at a line start, which gives Px 0, b and a for the three cases, as `:74` to `:76` state. The ISO 14495-1 attribution of modes 3 to 6 is gone |
| D3, the pins script says A.15 | closed | `scripts/pin_and_size_check.py:119` to `:120` say "code segment A.16 mirror". `git grep -n 'A\.15'` finds no other stale encoder attribution in the scripts, vendor, crates or docs trees |
| D4, the page 15 erratum | closed | `.claude/plans/F-243-design.md:420` now says "Outside that record, no committed text carries page 15". `git grep -nE 'p\. 15\|page 15' HEAD` returns the record's five lines, the erratum, and pass 6's quotations of both, and no new claim |
| N1, the two context counts | closed | `:42` "(9×9×9 - 1) / 2 + 1 = 365" evaluates to 365, and A.3.4's NOTE gives the 729 vectors it starts from. `:55` "2 run-interrupt contexts" matches A.2.1 step 2's indices 365 and 366 and the struct's `run_int_diff` and `run_int_same` |
| N2, `≤` in the plan | open | N1 above |

## Verified clean

- **LIMIT in code**, `context/mod.rs:348`: `let limit = 2 * (bpp + bpp.max(8))`. Here bpp is the SOF precision P, and `decoder.rs:82` refuses P outside 2 to 16, so T.87's `max(2, ceil(log2(MAXVAL+1)))` equals P whenever MAXVAL is `2^P - 1`. A non-default MAXVAL arrives only by LSE, and the adapter refuses that by name (`crates/ocelli-codec/src/jpegls.rs:389`, plan AC8). The comment at `:335` matches. glimit is `limit - J[run_index] - 1` at `scan.rs:227` and `encoder.rs:232`, with the index read before the decrement at `scan.rs:242`, as A.7.2 item 8 requires.
- **The 365 regular contexts**, A.3.4 and A.2.1 step 2. **Two run-interrupt contexts**, A.2.1 step 2, with RItype from A.7.2 Code segment A.17: `scan.rs:221` picks `run_int_same` when `|Rb - Ra| <= NEAR`.
- **The predictor enum**: no comment attributes modes 0 to 6 to ISO 14495-1 or T.87 any more. T.81's predictor table was not checked, see limitations.
- **The clause citation sweep**, every citation in the five files against the PDF:
  - `decoder.rs:9` C.2.3, ILV 0, 1, 2 as the enum gives them.
  - `bitstream.rs:3`, `:28`, `:196`, `:270` A.1, stuffing and zero padding before a marker. `:106` A.5.3 and A.2.1. `:174` D.2, p. 38, "optional X'FF' fill bytes may precede the marker". The only problem is D1.
  - `scan.rs:3` to `:4`, `:121` to `:122` A.3 to A.7 and F.1 steps 2 to 17 and 18. `:17` A.2.1 step 3, J matches entry by entry. `:28`, `:48`, `:52` A.4.1. `:73` A.2.1. `:162` A.3.1, `D1 = Rd - Rb`, `D2 = Rb - Rc`, `D3 = Rc - Ra`, matching `:163` to `:165`. `:173`, `:180` F.1 step 18. `:251` A.3 to A.6. `:297` A.4.5, Code segment A.9 range. `:307` A.1.
  - `encoder.rs:14` A.4.4. `:110` A.5.2. `:152` A.7. `:183` to `:184` A.7.1, Code segment A.16 in A.7.1.2. `:203` A.7.2. `:244` A.3 to A.6.
  - `context/mod.rs:3` A.3. `:10`, `:20`, `:71` A.2.1 and A.6. `:42` A.3.4. `:46` Table C.2 and A.6.1, where A.6.1 notes RESET may be set by LSE, which the adapter refuses. `:49`, `:52` 3.3 and A.6.2. `:59`, `:61`, `:125` A.7.2, where `negative_maps_to_odd` is the negative-error half of Code segment A.21. `:63` A.2.1 step 3 and A.7.1. `:88` indices 365 and 366. `:161` A.6, where `update_context` is A.12 and A.13, and the arithmetic shift on a negative B equals A.12's `-((1-B)>>1)` for every odd and even B. `:211` A.5.1, a correct cite whose text is D2. `:226` A.3.3 Code segment A.4, operator for operator. `:253`, `:270` A.3.4. `:287`, `:304`, `:314` C.2.4.1.1.1. `:326` A.2.1. `:362` A.4.5. `:374` A.4.4 Code segment A.8 quantisation. `:385` to `:386` A.4.4 and F.1 step 14. `:404` A.5.2.
  - So the provenance sentence is true of the clause citations. D2 to D4 are claims that cite nothing or cite correctly and say the wrong thing. They are outside that sentence but inside step 6.
- **Run miss at the line end, CharLS against Ocelli.** Per family, 8,190 suffixes:

  | Family | Both open, equal samples | Both refuse | Ocelli refuses, data ran out, CharLS opens | Ocelli refuses, unread bits, CharLS opens | Ocelli opens, CharLS refuses | Both open, different samples |
  |---|---|---|---|---|---|---|
  | width 5, J 1, remainder 1 | 883 | 7,092 | 65 | 150 | 0 | 0 |
  | width 7, J 1, remainder 1 | 488 | 7,664 | 20 | 18 | 0 | 0 |
  | width 15, J 2, remainder 3 | 109 | 8,037 | 36 | 8 | 0 | 0 |
  | width 14, J 2, remainder 2 | 120 | 8,014 | 48 | 8 | 0 | 0 |

  Every disagreement is one of the end-of-scan refusals the lead already ruled on: CharLS opens truncated data, or a trailing zero byte before EOI. One of each was traced by hand. Width 5, suffix `101001001`: both put 251 at row 1 column 3, which needs RUNindex 4, not 3, at the start of row 1. Ocelli then runs out of bits in the regular sample at column 4. Width 15, suffix `11110`: bytes `ff 3f 00`, a whole zero byte before EOI. The scratch mutant, which decrements RUNindex on such a miss, gives 5, 44, 3 and 6 sample mismatches and 666, 22, 78 and 86 streams it opens that CharLS refuses. So CharLS neither decrements RUNindex nor decodes an interruption sample for a miss that ends the line, and Ocelli matches it on every stream both open. The implementer's report was not located in the tree, so this is my own reproduction of the claim, not a replay of their cases.
- **Hashes and the pins gate**, see Evidence.

## Limitations and handoff

- Only T.87 (06/1998) was used. T.81 was not obtained, so the arithmetic in `scan.rs:40` to `:47` for upstream's modes 3 to 6 was not checked against T.81 Table H.1. Those lines no longer cite any standard.
- PDF text extraction drops superscripts and floor brackets. A.2.1's `2^5` and `2^6` were read from context, and the earlier rendered pages 16 to 17 were not rendered again.
- The run-miss probe calls the vendored crate directly, not Ocelli's adapter in `ocelli-codec`. The adapter adds header checks only. It covers P 8, NEAR 0, two rows, first-row prefixes, and suffixes up to 12 bits. NEAR > 0, P 16 and a miss on a later row were not exercised. The probe projects, the mutant and the binaries were deleted after the run, and the generator is kept only in the session scratch directory.
- `compute_k`'s and `RunInterruptionContext::compute_k`'s `k < qbpp` bound has no counterpart in A.10 or A.20. Whether A/N can exceed `2^qbpp` with RESET fixed at 64 was not proved or measured. This concern is unconfirmed and was not opened as a finding.
- `cargo test -p ocelli-codec` does not run `ritk-codecs`' in-crate tests. Only comments changed in the vendored files in this range.
- No oracle, GPU or browser job was run. CharLS source, `Software.zip`, dwv, Horos and Grok were not opened.
- Next owner: the implementer fixes D1 to D4, which are all comments, pins `bitstream.rs`, `context/mod.rs` and `PATCH-PROVENANCE.md` again, and a fresh independent pass reviews the change. The lead rules on N2's dead-code comment and on the run-miss result above. AC14 is not satisfied until then.
