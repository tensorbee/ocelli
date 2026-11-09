# F-243 review, pass 6

**Reviewed**: `d3b240b4..bcaf384f` at tip `bcaf384f` on `work/f-243-xai`, worktree `ocelli-wt-f-243`. Five commits: `f0baab4a`, `09005b57`, `f4590583`, `5f0e5bbc` (implementer) and `bcaf384f` (lead erratum). No unstaged tracked paths. `.venv` is untracked and was not read.
**Author / reviewer**: commits by a Claude implementer agent and the sprint lead. Reviewed by an independent Claude reviewer agent that wrote none of the range and did not write the AC14 standard check.
**Result**: 4 defects, 0 smells, 2 nitpicks

## Acceptance challenged

- **AC14 remediation, the lead's ruling "after the AC14 standard check".** Every changed citation was read against ITU-T T.87 (06/1998) as published, not against the implementer's report or the AC14 record. Source: the zip `T-REC-T.87-199806-I!!ZPF-E` from `itu.int/rec/dologin_pub.asp`, from which only `software/T87/t87.pdf` was extracted into the session scratch directory. `Software.zip` was not extracted or opened. Clause titles were taken from the PDF's table of contents and body headings. Pages 33 and 47 were rendered as images to read the superscripts `2^P` and `2^J[RUNindex]` that text extraction drops.
- **The ruling's claim that no arithmetic or control flow changes.** Checked by a comment-filtered diff rather than accepted.
- **The new sentence in `PATCH-PROVENANCE.md:34` to `:35`**, "the upstream clause citations in the patched files were corrected against that text". Challenged by reading every clause citation still present in the five patched source files, changed or not. It is false, D2.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git diff -U0 d3b240b4..bcaf384f -- '*.rs' '*.py'`, lines not starting with `//` or `#` | tip `bcaf384f` | now | only `RITK_PROVENANCE_SHA256` and the five `RITK_PATCHED_HASHES` values in `scripts/pin_and_size_check.py`. Every other changed line in a `.rs` or `.py` file is a comment |
| T.87 TOC and headings, `pdftotext -layout` | `t87.pdf` from the itu.int zip | now | A.1 p. 16, A.2.1 pp. 16 to 17, A.3.1 to A.3.4, A.4.1 to A.4.5, A.5.1 to A.5.3, A.6.1, A.6.2, A.7.1.1, A.7.1.2 (A.15, A.16 on p. 24), A.7.2 (item 8 on p. 25), A.8, C.1.2, C.2.1, C.2.3, C.2.4.1.1 (Tables C.1 and C.2 on p. 33), D.2 p. 38, F.1 pp. 46 to 47, 3.3 symbols. Clause 6 "Encoder requirements" has no subclause |
| rendered pp. 33, 47 | same PDF, pages 39 and 53 of the file | now | Table C.1 MAXVAL "0, or 1 ≤ MAXVAL < 2^P". F.1 step 18.c.ii "If RUNindex>0, decrement RUNindex by one". F.1 step 14 holds the decoder's `RANGE*(2*NEAR+1)` wrap |
| SHA-256 of the six `RITK_PATCHED_HASHES` files and `PATCH-PROVENANCE.md` | tip | now | all seven match the pins, and each file pin also appears in `PATCH-PROVENANCE.md` |
| `git archive HEAD` into scratch, one byte `14495` to `14496` in each of `decoder.rs`, `context/mod.rs`, `scan.rs`, `bitstream.rs`, `encoder.rs`, `PATCH-PROVENANCE.md`, then `python3 scripts/pin_and_size_check.py` | scratch copy of tip | now | exit 1 each time, "has undeclared changes beyond its recorded patch" (or "from the exact reviewed patch record" for the provenance file). Restored: exit 0. `bin/ocelli.sh gate pins` on a copy with `decoder.rs` changed `C.2.3` to `C.2.4`: FAILED pins, exit 1. Copy removed with `shutil.rmtree` |
| `bin/ocelli.sh gate pins` | worktree tip | now | ALL GREEN, exit 0 |
| `python3 scripts/prose_check.py` | tip | now | OK over 970 files, exit 0 |
| `python3 scripts/source_provenance_check.py` | tip | now | OK over 1851 files, exit 0 |
| `python3 scripts/deviation_check.py` | tip | now | OK, 32 deviations, exit 0 |
| `cargo test -p ocelli-codec -p ocelli-dicom`, `CARGO_TARGET_DIR=$PWD/target` | tip | now | exit 0, 23 test binaries, 365 passed, 0 failed, 3 ignored |
| added-line scan for U+2013, U+2014, U+2212, other non-ASCII and `;` | `git diff -U0` of the range | now | no en-dash, em-dash or semicolon added. Three en-dashes removed. The em-dash on `scan.rs:17` and U+2212 on `context/mod.rs:48` were already on the edited lines (removed 1, added 1). U+2500 and `§` are upstream comment furniture. One `≤` (U+2264) added to the plan, inside a quotation of Table C.1, and prose_check accepts it |

## Defects

### D1, `reconstruct` cites A.4.4 for a wrap that A.4.4 does not contain

**Where**: `vendor/ritk-codecs-0.6.0/src/jpeg_ls/context/mod.rs:384` to `:386`
**What**: the comment reads "Dequantize and reconstruct a sample (ISO 14495-1 §A.4.4): wrap into `[−NEAR, MAXVAL + NEAR]` by RANGE·(2·NEAR+1) steps, then clamp". `5f0e5bbc` changed "§A.4.4 / §A.8.2" to "§A.4.4". Dropping the nonexistent A.8.2 was right, but what remains attributes the wrap to A.4.4.
**Why it is wrong**: A.4.4, Code segment A.8, p. 20, computes `Rx = Px + SIGN * Errval * (2 * NEAR + 1)` and clamps to `[0..MAXVAL]`. It has no modulo wrap. The wrap, `if (Rx < −NEAR) Rx = Rx + RANGE*(2*NEAR+1) else if (Rx > MAXVAL + NEAR) ...`, is F.1 step 14, p. 47, and the function's body is that procedure line for line. Under the lead's ruling a wrong citation is a false sentence.
**Evidence**: rendered p. 47 and `pdftotext` of p. 20, against `context/mod.rs:388` to `:395`.
**Scope and regression**: comment only, no behaviour. Introduced in this range. Fix: "(ISO 14495-1 §A.4.4, with the decoder's modulo wrap from F.1 step 14)", then pin `context/mod.rs` and `PATCH-PROVENANCE.md` again.

### D2, the predictor block in `scan.rs` still cites clauses T.87 does not have or that say otherwise, so the new provenance sentence is false

**Where**: `vendor/ritk-codecs-0.6.0/src/jpeg_ls/scan.rs:26`, `:48`, `:52`, `:73` to `:74`, and the claim at `vendor/ritk-codecs-0.6.0/PATCH-PROVENANCE.md:34` to `:35`
**What**:
- `:26` was changed in this range from §C.2.4 to §C.2.3 and now reads "JPEG-LS predictor modes specified in the SOS header (ISO 14495-1 §C.2.3)", followed by "All 8 modes are defined by the ISO 14495-1 standard".
- `:48` and `:52`, unchanged, cite "§6.3.1" for the edge-detecting predictor.
- `:73` was changed in this range from §A.3 to §A.2.1 and introduces a list whose first item says "standard uses MAXVAL/2".
- `PATCH-PROVENANCE.md:34` to `:35`, added in this range, says "the upstream clause citations in the patched files were corrected against that text".
**Why it is wrong**: C.2.3, p. 32, replaces T.81's Ss with NEAR and Se with ILV. JPEG-LS has no predictor selection, and T.87 defines one predictor, A.4.1 Code segment A.5, p. 20. T.87 has no clause 6.3.1, because clause 6, "Encoder requirements", p. 11, has no subclauses. A.2.1, p. 16, defines the absent neighbours on the first line as zero, and at a line start as Rb. It does not use MAXVAL/2. So the newly attached citations at `:26` and `:73` now cite clauses that contradict the sentences they sit on, `:48` and `:52` cite a clause that does not exist, and the provenance sentence certifying the patched files is false.
**Evidence**: T.87 TOC lines for clauses 5 to 8, C.2.3 text, A.2.1 text and A.4.1 text, all from the PDF. `predict` is live: `scan.rs:261` and `encoder.rs:251` call it with `Predictor::Adaptive`, so these comments describe the decode path.
**Scope and regression**: comments only. Behaviour agrees with the standard, because `Adaptive` is A.4.1 and the boundary overrides give the same Px as A.2.1's neighbour rule. Smallest fix: `:48` and `:52` cite §A.4.1. At `:26`, delete the C.2.3 citation and the "All 8 modes are defined by the ISO 14495-1 standard ... for standard compliance" sentence, or say that T.87 defines only the A.4.1 predictor and the enum is upstream's. At `:74`, delete "standard uses MAXVAL/2 but test data matches 0", because A.2.1 gives zero. Then pin `scan.rs` and `PATCH-PROVENANCE.md` again. Narrowing the provenance sentence instead would be the other way out, but it would leave citations to a nonexistent clause in a file the record calls reviewed.

### D3, the pins script still calls the encoder fix the A.15 mirror

**Where**: `scripts/pin_and_size_check.py:118` to `:120`
**What**: "with the encoder's A.15 mirror". Every other site in the range moved to A.16: the plan at `:139` and `:161`, `docs/lld/codecs.md:110`, `PATCH-PROVENANCE.md:77` to `:78`, `encoder.rs:183` to `:184` and `tests/jpegls.rs:1414`. This comment is three lines above the hash lines this range rewrote.
**Why it is wrong**: the ruling's first bullet says A.15 cited "for the end-of-line hit that is A.16's branch" is corrected. The encoder change is the `else if (RUNcnt > 0)` branch of Code segment A.16, p. 24, and A.8 step 17.c, p. 27, also says "see Code segment A.16".
**Evidence**: `git grep -n 'A\.15' bcaf384f`.
**Scope and regression**: comment only, and no pin covers this file. Fix: "with the encoder's code segment A.16 mirror".

### D4, the erratum says no committed text carries page 15, and the committed AC14 record does

**Where**: `.claude/plans/F-243-design.md:420`
**What**: "Erratum to the AC14 record: A.1 is on printed page 16 of T.87 (06/1998), not page 15. No committed text carries page 15."
**Why it is wrong**: the first sentence is right, since the TOC and the body both place A.1 on p. 16. The second sentence is false, because `.claude/reviews/F-243-ac14-standard-check.md` is tracked and carries "A.1, p. 15" at `:27`, `:81`, `:82`, `:83` and `:111`.
**Evidence**: `git ls-files .claude/reviews` lists the record. `git grep -nE 'p\. 15|page 15' bcaf384f` returns those five lines and the erratum.
**Scope and regression**: plan prose. Fix: "Outside that record, no committed text carries page 15", which `git grep` confirms. Alternatively correct the five sites in the record.

## Smells

None.

## Nitpicks

- **N1**, `context/mod.rs:42` says "5×9×9 / 2 + 1 = 365", which evaluates to 203. The A.3.4 NOTE, p. 19, gives 729 vectors, and (729 − 1) / 2 + 1 = 365. `:54` says "1 run-interrupt context", but the struct holds two, 365 and 366. Both predate F-243 and are not citations, so they are outside the ruling. Listed only because the file is now recorded as reviewed.
- **N2**, plan `:70` quotes Table C.1 with `≤` while the rest of the paragraph uses ASCII `<=`. It is a faithful quotation and prose_check accepts it.

## Plausible concern, unconfirmed, outside this pass

`scan.rs:197` to `:209`: a run miss whose remainder lands exactly on the line end (`run_len == remaining`) is accepted. It decodes no interruption sample and skips F.1 step 18.c.ii's decrement, because the decrement sits inside `if run_len < remaining`. A compliant encoder never writes that stream, since A.16 writes a miss only for a sample that interrupts inside the line, and clause 7's NOTE leaves non-compliant data unspecified. So this is not a standard violation. It is a stream class the story's "refuse rather than draw" policy (AC7) does not name. Pass 1's notes cover `run_len == remaining` only for the hit branch. No stream was built and CharLS's verdict was not measured. This is for the lead to decide.

## Verified clean

Each site below was read against the PDF clause named, and agrees.

- **decoder.rs**, the only change: `:9` "Interleave mode from the SOS header §C.2.3". ILV is defined in C.2.3, p. 32.
- **context/mod.rs**, every changed site:
  - `:3` A.3, context determination.
  - `:10` A.2.1 and A.6: initial A in A.2.1 step 2, update in A.6.
  - `:20` A.2.1 and A.6.1.
  - `:45` RESET 64 is Table C.2, p. 33, applied in A.6.1 Code segment A.12.
  - `:48` and `:51` MIN_C −128 and MAX_C 127 are defined in 3.3, p. 5, and used in A.6.2 Code segment A.13, which says "defined in 3.3".
  - `:58` and `:60` RItype is in A.7.2 Code segment A.17. RItype 1 is `|Ra − Rb| <= NEAR`, matching `run_int_same`.
  - `:62` RUNindex comes from A.2.1 step 3 and A.7.1.
  - `:70` the initial A is A.2.1 step 2.
  - `:160` `update_context` does A.6.1 and A.6.2, which is A.6.
  - `:210` k is A.5.1 Code segment A.10.
  - `:252` and `:269` sign merging and the Q mapping are A.3.4.
  - `:402` the inverse of A.5.2's mapping.
  - Unchanged sites also agree: `:225` A.3.3 Code segment A.4, `:286`, `:303` and `:313` C.2.4.1.1.1, `:325` A.2.1, `:361` A.4.5, `:373` A.4.4. The only exception is D1.
- **bitstream.rs**:
  - `:3`, `:28`, `:196` and `:270` stuffing and padding are A.1, p. 16.
  - `:106` LG(k, LIMIT) is A.5.3 and LIMIT is A.2.1 step 1.
  - `:174` T.87 D.2, p. 38: "optional X'FF' fill bytes may precede the marker".
- **encoder.rs**:
  - `:110` forward mapping is A.5.2, where the old A.5.3 was wrong.
  - `:183` to `:184` the end-of-line one-bit is A.16's `else if (RUNcnt > 0)`, confirmed by A.8 step 17.c.
  - `:244` regular mode A.3 to A.6.
- **scan.rs**:
  - `:3` to `:4` and `:121` to `:122`. F.1 steps 2 to 17 are the regular mode and step 18 is run mode, as step 3 says.
  - `:17` J is A.2.1 step 3.
  - `:162` gradients are A.3.1.
  - `:173` and `:180` cite F.1 step 18, and Annex F is marked informative, p. 46.
  - `:251` regular mode, and `:307` padding is A.1.
  - The run loop's comment matches its code. `glimit` uses the index before the decrement, as A.7.2 item 8, p. 25, requires.
- **sample_interleaved.rs:20**: J is A.2.1 step 3.
- **tests/jpegls.rs**:
  - `:868` and `:1438` F.1 step 18, J from A.2.1 step 3.
  - `:911` F.1 step 18.b, the hit branch.
  - `:1347` D.2.
  - `:1414` A.7.1 Code segment A.16, since A.16 is in A.7.1.2.
- **generate_jpegls.py:151** to `:154`: F.1 step 18, A.2.1 step 3, A.1.
- **Plan**:
  - `:41` A.15 and A.16 are encoder segments in A.7.1.2.
  - `:43` F.1 step 18 is on p. 47, and the Annex F quote is a truncation of the p. 46 heading.
  - `:54` and `:58`: the new decrement line sits after the remainder read, in F.1 18.c order, and the quote is verbatim. A.7.2 item 8 is on p. 25, quoted verbatim.
  - `:62` J is on A.2.1 step 3 p. 17 and F.1 step 1.e p. 46.
  - `:70` Table C.1's MAXVAL range is verbatim from the rendered p. 33.
  - `:96` A.1 and the C.1.2 NOTE.
  - The AC1, AC7 and AC14 rows, items 2 and 4, and `:389` D.2.
- **codecs.md**: `:97` to `:99` the record exists and is tracked. Also `:101`, `:110`, `:134`, `:156` C.2.4.1.1.1 and `:165` C.2.4.1.1.
- **PATCH-PROVENANCE.md**: `:39`, `:50`, `:68` and `:77`, except the `:34` to `:35` claim (D2).
- **Not stale**: the remaining "T.87 A.7.1" run-mode cites are A.7.1.2's normative statement that RUNindex rises for each full rg. They sit at `generate_jpegls.py:29`, `corpus_synth.py:672`, `docs/lld/corpus.md:123`, `CURRENT_SPRINT.md:256` and `pin_and_size_check.py:118`. No T.81 B.1.1.2 cite remains. D.2 does not refer to T.81, so dropping it entirely was right. README, `use.html` and `more.html` carry no T.87 clause and were not touched.
- **Hashes and the pins gate**: see Evidence.

## Limitations and handoff

- Only T.87 (06/1998) was checked. T.81 was not, and no corrigenda or later edition was looked for. Pages other than 33 and 47 were read as extracted text, where superscripts and floor brackets are lost. Pages 16, 17 and 34 were rendered by the AC14 record and not again here.
- The "comment only" result rests on a line filter: lines whose first non-space characters are `//` or `#`. No Rust or Python change hides behind a comment marker in this range, and every changed line was also read in the full diff.
- `cargo test -p ocelli-codec` does not run `ritk-codecs`' in-crate unit tests. Only comments changed there.
- The sample covers every changed line in the range plus every clause citation still present in the five patched source files, so more than the 20 asked for. Unpatched vendored files were not read, per the ruling.
- No oracle, GPU or browser job was run. CharLS source, `Software.zip`, dwv, Horos and Grok were not opened.
- Next owner: the implementer fixes D1 to D4, pins `context/mod.rs`, `scan.rs` and `PATCH-PROVENANCE.md` again, and a fresh independent pass reviews the change. AC14 is not satisfied until then.
