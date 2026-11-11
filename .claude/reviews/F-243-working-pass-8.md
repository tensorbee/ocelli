# F-243 review, pass 8

**Reviewed**: `867c3eaa..1b2a1464` at tip `1b2a1464` on `work/f-243-xai`, worktree `ocelli-wt-f-243`. One commit, `1b2a1464`, remediation of pass 7 by the lead's ruling. No unstaged tracked paths. `.venv` is untracked and was used only to run imagecodecs.
**Author / reviewer**: the commit is by a Claude implementer agent. Reviewed by an independent Claude reviewer agent that wrote none of the range.
**Result**: 0 defects, 0 smells, 2 nitpicks

## Acceptance challenged

- **The lead's ruling after pass 7.** Pass 7's D1 to D4 and N1 to N3 replayed at the tip against ITU-T T.87 (06/1998).
- **"Only comments and pins changed."** Checked by a comment-filtered diff.
- **Every clause citation in the five patched files** (`decoder.rs`, `bitstream.rs`, `scan.rs`, `encoder.rs`, `context/mod.rs`) listed by grep at the tip and checked against the PDF text.
- **The cap on k.** The claim that `compute_k`'s and `RunInterruptionContext::compute_k`'s `k < qbpp` bound never stops the loop before A.10 would. Derived step by step from A.2.1, A.4.5, A.6.1, A.7.2 and A.10 below, then measured with counters in a scratch build over 437,874 streams, with two mutants to show the counter discriminates and one to show which premise carries the argument. The implementer's own write-up of the argument is not in the tree (not in the plan, the provenance record, the commit message or the ignored progress note), so the argument checked is the one the lead's brief states.
- Standard source: the zip `T-REC-T.87-199806-I!!ZPF-E` fetched now from `itu.int/rec/dologin_pub.asp` (SHA-256 `c54f20d1...ea41a`). Only `software/T87/t87.pdf` was extracted (SHA-256 `4ca6f981...a8778`, the same file pass 7 used). `Software.zip` was not extracted or opened.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git diff -U0 867c3eaa..1b2a1464 -- '*.rs' '*.py'`, lines not starting with `//` or `#` | tip `1b2a1464` | now | only `RITK_PROVENANCE_SHA256` and the `bitstream.rs`, `context/mod.rs` and `scan.rs` values in `RITK_PATCHED_HASHES`. `PATCH-PROVENANCE.md` changed only three SHA-256 lines. The plan changed one `≤` to `<=` |
| SHA-256 of the six `RITK_PATCHED_HASHES` files and `PATCH-PROVENANCE.md` | tip | now | all seven match their pins, and each file pin also appears in `PATCH-PROVENANCE.md` |
| `git archive HEAD` into scratch, one byte case-flipped inside each rewritten comment of `bitstream.rs`, `context/mod.rs`, `scan.rs`, and in `PATCH-PROVENANCE.md`, then `bin/ocelli.sh gate pins` | scratch copy of tip | now | exit 1 each time, "has undeclared changes beyond its recorded patch" (or "from the exact reviewed patch record"). Restored: exit 0. Copy removed with `shutil.rmtree` |
| `bin/ocelli.sh gate pins` | worktree tip | now | ALL GREEN, exit 0 |
| added-line scan for U+2013, U+2014 and `;` (Python, 22 added lines) | `git diff -U0` of the range | now | none |
| `python3 scripts/prose_check.py` | tip | now | OK over 972 files, exit 0 |
| `python3 scripts/source_provenance_check.py` | tip | now | OK over 1853 files, exit 0 |
| `python3 scripts/deviation_check.py` | tip | now | OK, 32 deviations, exit 0 |
| `cargo test -p ocelli-codec -p ocelli-dicom`, `CARGO_TARGET_DIR=$PWD/target` | tip | now | 365 passed, 0 failed, 3 ignored. Build exit 0, no FAILED line |
| k-cap probe, scratch copy of `vendor/ritk-codecs-0.6.0` with a counter in both `compute_k` functions, a header-and-scan entry point, release build, `CARGO_TARGET_DIR` in the session scratch | 172 committed streams (`jpegls_charls_streams.tsv` and every `*.jls` fixture), 954 CharLS 2.4.4 encodes through imagecodecs (P 2, 3, 4, 5, 6, 7, 8, 10, 12, 16, NEAR 0, 1, 2, 3, 7 and `min(255, MAXVAL/2)`, six image kinds, 64x64, 9x300 and 1x500), 425,752 single-bit flips in the scan, 3,378 dropped final bytes, 3,378 inserted bytes before EOI, 4,240 random A.1-stuffed scans after a real header | now | see the table under the k verdict. Tip: 0 binds in 598 million calls. Mutants bind |

## Defects

None.

## Smells

None.

## Nitpicks

- **N1**, `vendor/ritk-codecs-0.6.0/PATCH-PROVENANCE.md:33` to `:35` says the upstream clause citations were corrected against T.87. This round and earlier rounds also rewrote upstream comments that are not citations: `ContextState::default` at `context/mod.rs:31`, the floor invariant removed at `:211`, and `Predictor::None` at `scan.rs:34`. The sentence is true and the pins hold the exact bytes, so nothing is false. A reader comparing against the archive would find those edits easier to account for if the sentence said "and the upstream comments found false against it were corrected".
- **N2**, `bitstream.rs:110` now uses ` - ` after `limit`, while the `k` and `qbpp` lines beside it keep upstream's em-dash. The rewrite rightly drops the em-dash, so this is only list style.

## Pass 7 findings replayed

| Pass 7 | Status | Observation at `1b2a1464` |
|--------|--------|---------------------------|
| D1, `read_golomb`'s `limit` | closed | `bitstream.rs:110` to `:112` name LIMIT, `2*(bpp + max(8, bpp))`, from A.2.1 step 1, and glimit, `LIMIT - J[RUNindex] - 1`, from A.7.2 item 8, p. 25. The two callers are `scan.rs:267` (`limit`) and `scan.rs:228` (`limit - J[run_index] - 1`), and `run_index` is read at `:221`, before the decrement at `:242`, as item 8 requires. The em-dash and semicolon are gone |
| D2, `compute_k`'s floor | closed | `context/mod.rs:211` to `:212` say the smallest k with `N * 2^k >= A`, A.5.1 Code segment A.10, `for(k=0; (N[Q]<<k)<A[Q]; k++)`, p. 21, "bounded here by qbpp". The invariant line and the "ceil but floored" parenthesis are deleted. `:216` matches the loop |
| D3, RANGE for NEAR > 0 | closed | `:71` to `:73` give `MAXVAL + 1` for lossless and `floor((MAXVAL + 2*NEAR) / (2*NEAR + 1)) + 1` for NEAR > 0, which is A.2.1 step 1, p. 17, and what `CodingParams::new` computes at `:347` |
| D4, `ContextState::default` | closed | `:31` to `:32` call it a zeroed state with n = 1, used by tests, and point to `ContextModel::new`. `grep` finds only `context/tests.rs:192` calling it. `ContextModel::new` at `:75` to `:88` sets A to `a_init`, B and C to 0, N to 1 and Nn to 0, which is A.2.1 steps 2 and 4 |
| N1, `≤` in the plan | closed | `.claude/plans/F-243-design.md:70` now reads "1 <= MAXVAL < 2^P". `grep -n '≤'` finds no other in the plan |
| N2, `Predictor` comments | closed | `scan.rs:49` drops "default". `scan.rs:34` to `:35` say Px = 0 and that `predict` gives 0 at the first sample, b in the first column and a in the first row, which is `predict` at `:88` to `:96` |
| N3, upstream em-dash and semicolon on `bitstream.rs:110` | closed | gone, see D1 |

## The cap on k, verdict

**The argument holds at the tip, and the cap never binds.** The steps, with δ = floor(RANGE/2) and c = A's initial value:

1. **The error bound.** A.4.5 Code segment A.9 maps every coded error into `[-floor(RANGE/2), ceil(RANGE/2) - 1]`, so |Errval| <= δ. In the decoder this premise is not automatic, because the error comes from the bits. It is supplied by F-243's `in_coded_range` (`scan.rs:302` to `:304`), which refuses before the A update in both paths (`:272` before `:282`, and `:230` before `:233`).
2. **The start.** A.2.1 step 2 gives c = `max(2, floor((RANGE + 32) / 64))` and N = 1, so A <= c + δ(N - 1) holds with equality. `CodingParams::new` at `context/mod.rs:350` computes c, and RANGE >= 2 always, since MAXVAL >= 1 and NEAR <= MAXVAL/2, so c <= RANGE.
3. **The update, A.6.1 Code segment A.12.** A grows by |Errval| <= δ and N by 1, which keeps A <= c + δ(N - 1).
4. **The RESET halving.** RESET is the constant 64 at `context/mod.rs:48`, so the halving happens at N = 64, which is even. Before it A <= c + 64δ, after it A <= floor(c/2) + 32δ <= c + 32δ, and N becomes 32 + 1 = 33, where the bound is c + 32δ. The invariant survives. (An odd RESET would break this exact form, but a weaker invariant, A <= N·2^qbpp, still survives an odd halving, and the adapter refuses LSE in any case.)
5. **The k bound.** qbpp = ceil(log2 RANGE) (`:348`, bit length of RANGE - 1), so 2^qbpp >= RANGE >= 2δ. Then N·2^qbpp >= N·RANGE >= RANGE + (RANGE/2)(N - 1) >= c + δ(N - 1) >= A, the middle step being RANGE(N - 1)/2 >= 0. So A.10's loop stops at some k <= qbpp, and `k < qbpp` never stops it first.
6. **Run interruption, A.7.2.** A.23 adds `(EMErrval + 1 - RItype) >> 1`, which with A.22's EMErrval is |Errval| for RItype 0 and |Errval| - 1 for RItype 1, where |Errval| >= 1. `RunInterruptionContext::update` at `:147` to `:158` is A.23 line for line, with the same RESET. RItype 0 uses TEMP = A, which is step 5. **RItype 1** uses A.20's TEMP = A + (N >> 1), with A <= c + (δ - 1)(N - 1). At N = 1, TEMP = c <= RANGE <= 2^qbpp. For N >= 2, N·RANGE - TEMP >= RANGE(N - 1)/2 + N/2 - 1 > 0. So TEMP < N·2^qbpp there.
7. **Tight only at RANGE 2.** Equality in step 5 needs c = RANGE and 2^qbpp = RANGE and N = 1. c = RANGE has the one solution RANGE 2, where c = 2. At RANGE 2, N = 1 and A = 2 (or TEMP = 2), A.10 and the capped loop both stop at k = 1 = qbpp. RANGE 2 is reachable only near-lossless, for example P 2 with NEAR 1, P 3 with NEAR 3 or P 8 with NEAR 127, because the decoder refuses P below 2 and so lossless RANGE is at least 4.

**Measured**, counting a bind as an exit with `k == cap` while `N << k < A` still holds, and a tight case as `A == N << qbpp`:

| Build | Regular calls | Regular binds | RItype 0 binds | RItype 1 binds | Tight cases |
|---|---|---|---|---|---|
| tip | 558,910,463 | 0 | 0 | 0 | 239,076 regular, 15,128 RItype 0, 51,842 RItype 1 |
| cap at qbpp - 1 | 517,206,390 | 309 | 1 | 51,978 | n/a |
| cap at qbpp - 3 | 369,273,148 | 7,594,008 | 23,360 | 409,648 | n/a (18,697 regular binds on undamaged streams alone) |
| tip with `in_coded_range` disabled | 743,167,406 | 2,082,303 | 894 | 18,299 | n/a |

The tip's zero has no exception in any group: undamaged, flipped bit, dropped bytes, inserted byte or random scan. The maximum observed A/(N·2^qbpp) is exactly 1, and **every one of the streams with a tight case is a RANGE 2 stream** (51,842 streams, all RANGE 2 rows of the generated set and their damaged variants). The other 36 RANGE values, from 3 to 65536, and the committed fixtures, never reach equality. The two capped mutants show the counter fires, and the last row shows **the argument rests on `in_coded_range`**: without that refusal a damaged stream decodes |Errval| above δ and the cap binds about two million times. That refusal is F-243's own and is refused before any update, so the argument holds at the tip. The dependence is not written anywhere, which is worth a line if the cap is ever discussed again, and it is not a finding.

## Audit backlog, not blocking

False or doubtful upstream comments that are not citations, on lines F-243 did not write or rewrite, for the sprint-close audit story the ruling names:

- `context/mod.rs:1`, "context model for JPEG-LS lossless decoding". The module serves near-lossless decoding (`CodingParams::new` takes NEAR) and the encoder too.
- `context/mod.rs:15`, "`n` ... initialized to 1 to avoid division by zero". A.2.1 step 2 initialises N to 1, and A.10 has no division, so the reason given does not match the code it describes.
- `context/mod.rs:295` to `:296`, "DICOM data is ≥ 8-bit so the high branch is the production path". The decoder accepts P from 2 (`decoder.rs:82`), and DICOM Bits Stored below 8 gives MAXVAL below 128, which takes the low branch.
- `scan.rs:164` to `:166`, the names "vertical", "horizontal" and "diagonal" on D1 = Rd - Rb, D2 = Rb - Rc and D3 = Rc - Ra. A.3.1 names them only local gradients. D1 and D2 are both differences along the row above and D3 is a difference down a column, so the names do not match the parenthesised positions beside them. Doubtful rather than plainly false, since "vertical gradient" is sometimes used for the response to a vertical edge.

## Verified clean

- **Every changed line**, 22 added and 18 removed, read in full. Outside comments only pin values changed. Each new comment was checked against the code below it and against T.87, see the replay table.
- **The clause citations in the five patched files**, listed at the tip by grep. Every one names the clause pass 7 verified, and the only new or rewritten ones are `bitstream.rs:111` A.2.1 and `:112` A.7.2 item 8, `context/mod.rs:71` A.2.1 step 1, `:211` to `:212` A.5.1 Code segment A.10, and `scan.rs:49` A.4.1 (Code segment A.5, p. 20). All agree. Spot-checked again against the text: A.1 stuffing, A.3.1, A.3.3 Code segment A.4, A.3.4, A.4.5 Code segment A.9, A.5.2, A.6.1 Code segment A.12, A.7.1 Code segment A.16, Table C.2 RESET 64, `MAX_C` 127, and D.2's "optional X'FF' fill bytes may precede the marker".
- **Hashes and the pins gate**, see Evidence. Every pinned hash matches, and the gate goes red on a one-byte edit in each of the four re-pinned files.
- **The required commands**, all exit 0, see Evidence.
- **The cap on k**, proved and measured, see the verdict.

## Limitations and handoff

- Only T.87 (06/1998) was used. PDF text extraction drops floor brackets and superscripts, so A.2.1's `2^5` and `2^6` and step 1's floor and ceiling were read from context, as in pass 7.
- The probe calls the vendored header parser and `decode_fragment` directly, not Ocelli's adapter in `ocelli-codec`. The adapter adds checks such as the LSE refusal, so the probe is a superset of what Ocelli feeds the decoder. Streams with LSE that CharLS writes for some low-P or high-NEAR cases were counted too.
- The generated set is CharLS's output and damaged variants of it. No stream was crafted to drive |Errval| to δ on every sample, so the measured maximum ratio of 1 comes from the initial state at RANGE 2, not from a worst-case run. The bound above N = 1 rests on the proof.
- Random scans and flips mostly decode a short prefix before refusal. Counts include compute_k calls on streams later refused, which is stricter than counting opened streams only.
- `cargo test -p ocelli-codec` does not run `ritk-codecs`' in-crate tests. Only comments changed in the vendored sources in this range.
- No oracle, GPU or browser job was run. CharLS source, `Software.zip`, dwv, Horos and Grok were not opened. The scratch crate copies, mutants and binaries were deleted with `shutil.rmtree`.
- Next owner: the lead. This pass has no blocking finding, so on this range the F-243 review loop is clean. The nitpicks and the audit-backlog list are optional. AC13 and AC18 from the progress note are outside this review.
