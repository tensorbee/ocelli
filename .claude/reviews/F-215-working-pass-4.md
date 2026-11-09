# F-215 review, pass 4

**Reviewed**: f7b0c7b3..2230f09a on `work/f-215-xai`, tip 2230f09a, one commit ("F-215, review pass 3 remediation"). The worktree was clean apart from the untracked `.venv` and `tools/oracle/out`, which are not part of the story.
**Author / reviewer**: a separate Claude implementer agent wrote 2230f09a. This pass is an independent Claude reviewer that wrote none of the reviewed material and none of passes 1 to 3. Both are Claude, so independence is per participant, not per model.
**Result**: 3 defects, 1 smell, 2 nitpicks

## Acceptance challenged

- **Pass 3 D3, the page step.** The new `observeDeleteAll` step in `tests/browser/marks.mjs` was run on the tip through `bin/ocelli.sh gate viewer-browser`, ALL GREEN. The same step was then run against f7b0c7b3's page and module in a scratch copy. That run went red: "Nothing was saved.", and the deleted mark came back. The first attempt at the f7b0c7b3 run was void, because `git archive` gives files their commit time and cargo reused the tip binary. The binary's strings showed no `sessionPartialStudies`, the sources were touched, and the rerun's binary held it.
- **D3, adjacent case through the page.** Report P holds a rectangle on slice 0 and a length on slice 5, and report W holds a rectangle on slice 5. Both were saved by the page from separate pages. A fresh page opened slices 1 to 11 with P and W: "Read 2 documents: 2 marks ... 1 item not in this folder was not placed." Every mark was deleted on every slice and Save marks pressed: "Saved 0 marks and 0 key images in 1 file. 1 study has no marks left, so its file holds no marks and replaces the earlier one." A fresh page on the whole study with all three files showed P's rectangle and P's length, and not W's rectangle: "Read 3 documents: 2 marks ... Replaced by a later document: report-w.dcm." This is what the pass 3 ruling asks for. P's length, which the user deleted, comes back, and nothing the user reads says so (D3 below).
- **D1, every set the docs name.** 34 files were written from one package SR, with each comment's bytes set through Python's own codecs. pydicom 3.0.2 does not know ISO_IR 203 and silently writes its default encoding, so raw bytes were needed. The sets were ISO_IR 6, 100, 101, 109, 110, 126, 127, 138, 144, 148, 166, 192 and 203, GB18030, GBK and no set. The refusal cases were ISO_IR 13, `ISO 2022 IR 100`, `\ISO 2022 IR 87`, `ISO 2022 IR 6\ISO 2022 IR 100` and `ISO_IR 999`. Each was read by `readTid1500Groups` in Node and in headless Chromium through the page's `/lib/ocelli-core/index.js`. The two hosts agree on all 34. Every named set returns its text unchanged, ISO_IR 203 included. Every refusal case has every group unread. The exceptions are D1 and D2 below.
- **D2, transfer syntaxes.** Big endian with no set, ISO_IR 100, 192 and 203 all read whole. Implicit VR with no set, ISO_IR 100, 144 and 203 all read whole. Deflated with no set reads whole. Deflated ISO_IR 100 has every group unread, as documented in `public-api.md`. Deflated ISO_IR 203 and deflated `\ISO 2022 IR 87` return null (D2).
- **Truncated files.** `parsed` now retries with `ignoreErrors` for every file the walk can read. dcmjs's `_read` returns a partial dict on any error under that flag, so a truncated SR could have become a partial report named as a predecessor. Four truncations, inside a group, between groups and at the tail, return null at both f7b0c7b3 and the tip. That risk does not occur.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npm run build -w @ocelli/core`, then `npx vitest run` over tid1500-groups, tid2010, secondary-capture, sr, tid1500 and mark-documents | 2230f09a | now | 0, 94 of 94 |
| tid1500-groups and mark-documents, 10 runs | 2230f09a | now | 10 of 10 green |
| `npx tsc --build`, `npx eslint examples/folder-viewer packages/core/src`, `python3 scripts/prose_check.py` | 2230f09a | now | 0, 0, 0 |
| `bin/ocelli.sh gate viewer-browser`, `OCELLI_PORT=0`, scratch `CARGO_TARGET_DIR`, after `pgrep` showed only idle Playwright MCP servers | 2230f09a | now | 0, ALL GREEN. Delete all: "Saved 0 marks ... replaces the earlier one.", `reopened` 0 |
| tip `marks.mjs` in a copy of f7b0c7b3, binary rebuilt and checked by its strings | `git archive f7b0c7b3` | now | 1. "Nothing was saved.", `reopened` 1 |
| adjacent probe, two reports, Playwright 1.62.1 Chromium | copy of 2230f09a, release binary | now | as under Acceptance challenged |
| Node reads of 34 files plus 4 truncations, tip and f7b0c7b3 readers | scratch copies of each commit, `tsc -p packages/core` | now | table below |
| Chromium reads of the same 38 files | tip binary's `/lib/` | now | identical to Node line for line |
| `parsed(bytes, true)` tried in a scratch copy | copy of 2230f09a | now | deflated ISO_IR 203 and `\ISO 2022 IR 87` become 3 unread, truncations stay null, 72 of 72 green. Reverted, the copy diffed equal to the worktree |
| PS3.5 2026d 7.1.2, PS3.5 2016b A.3, PS3.3 2026d C.12.1.1.2 Table C.12-2 | dicom.nema.org | now | see D1 and Verified clean |
| ISO-IR 166 registration | itscj.ipsj.or.jp/ir/166.pdf | now | "96-character graphic character set", "A set of 88 Thai characters", position 2/0 NO-BREAK SPACE, 5/11 to 5/14 and 7/12 to 7/15 "This position shall not be used" |
| ISO/IEC 8859-11 mapping | unicode.org/Public/MAPPINGS/ISO8859/8859-11.TXT | now | 88 entries from A0 to FF, `0xA0 0x00A0 NO-BREAK SPACE` |
| `TextDecoder("windows-874", { fatal: true })` | Node | now | A0 to U+00A0. DB to DE and FC to FF to U+F8C1 to U+F8C8 |
| mutation catalogue, 15 package mutants and 2 page mutants | copy of 2230f09a, each restored and the copy diffed equal to the worktree | now | 14 red, 3 survived. Table below |

| File | Tip | f7b0c7b3 |
|---|---|---|
| ISO_IR 203 `€ Œ Ÿ`, explicit, big endian, implicit | read whole | null |
| ISO_IR 6 | read whole | 3 unread |
| no set, big endian or deflated | read whole | 3 unread |
| ISO_IR 100 or 192, big endian | read whole | 3 unread |
| ISO_IR 166, `A1 A0 A2` | **1 unread** (D1) | `ก ข`, read whole |
| ISO_IR 203 or `\ISO 2022 IR 87`, deflated | **null** (D2) | null |
| ISO_IR 100, deflated | 3 unread | 3 unread |
| every other named set, and implicit VR | read whole | read whole |
| ISO_IR 13, `ISO 2022 IR 100`, both multi-valued, `ISO_IR 999` | 3 unread | 3 unread |
| four truncations | null | null |

| Mutant | Result |
|---|---|
| T1 the retry only for a set the reader does not decode | red, the ISO_IR 203 test |
| T2a big endian not walked | red |
| T2b no default repertoire for a file the walk cannot read | red |
| T3 the C1 refusal removed | red |
| T4 a multi-valued set read by its first value | red |
| T5 implicit VR walked as explicit | red |
| T6 ISO_IR 6 not the default repertoire | red |
| T7 the TIS 620 refusal removed | red |
| T8 `String.fromCharCode(...bytes)` unchunked | red, the 200000 byte test |
| T9 a report placed in part named as a predecessor | red, 2 tests |
| T10 no empty report after a delete all | red, 3 tests |
| O1, own: big endian lengths read little endian | red |
| O2, own: a file the walk cannot read is always the default repertoire | **survived** (S1) |
| O3, own: the TIS 620 run stops at DD | **survived** (N1) |
| O4, own: each chunk drops one byte | red |
| P1 page, T10 in the module the binary serves | red in `marks.mjs`, "Nothing was saved." |
| P2 page, T9 in the module the binary serves | **survived** in the browser gate (N2) |

## Defects

### D1, ISO_IR 166 refuses byte A0, NO-BREAK SPACE, one of the set's 88 characters

**Where**: `packages/core/src/tid1500.ts:779` to `:781`, with the comment at `:776` and the test at `packages/core/src/tid1500-groups.test.ts:820` to `:827`
**What**: `tis620Undefined` returns true for 0xA0, so any ISO_IR 166 string holding a no-break space makes its group unread. f7b0c7b3 read it correctly. The code is also inconsistent with itself: the comment at `:749` says the G1 is 88 characters, and the refusal leaves 87.
**Why it is wrong**: PS3.3 Table C.12-2 defines ISO_IR 166 as ISO-IR 166 with 88 characters in G1, designated by ESC 02/13 05/04, a 96-set. The ISO-IR 166 registration lists 2/0 as NO-BREAK SPACE, which is byte A0 in G1, and marks only 5/11 to 5/14 and 7/12 to 7/15 "shall not be used", which are DB to DE and FC to FF. ISO/IEC 8859-11 agrees. Thai has 87 letters, signs and digits, and NBSP is the 88th. Python's `tis_620` codec leaves A0 out, but that follows the national standard, not the registration Table C.12-2 cites. Pass 3's N3 named A0 as undefined, and the remediation took that sentence as given.
**Evidence**: the file holding `A1 A0 A2` reads as `{"groups":2,"unread":1}` at the tip in Node and in Chromium, and as `ก ข` with 0 unread at f7b0c7b3.
**Scope and regression**: new in this range. It affects any Thai report with a no-break space. Fix: drop `byte === 0xa0` and correct the comment at `:776` to "DB to DE and FC to FF, which ISO-IR 166 says shall not be used". In the test, expect `[0xa0, 0x41]` to read `" A"`, and loop the refusal over all eight of DB, DC, DD, DE, FC, FD, FE and FF, which also closes N1.

### D2, a deflated report whose set dcmjs refuses is dropped whole, though the docs say its groups are counted unread

**Where**: `packages/core/src/tid1500.ts:1062`. The claims are at `docs/lld/public-api.md:298`, "A file whose set uses code extensions or is not one of those is read, and every group in it is counted unread", and `examples/folder-viewer/src/use.html:882`, "It counts a group unread when that set uses code extensions or is another one".
**What**: `parsed` retries with `ignoreErrors` only when the walk found a set. A deflated file cannot be walked. When it carries ISO_IR 203, or more than one value such as `\ISO 2022 IR 87`, dcmjs throws, `readTid1500Groups` returns null, and `readDocument` reports the file as no marks document at all. Its predecessors are not read either. `use.html` also lists ISO_IR 100 and the rest as decoded with no qualifier, while a deflated ISO_IR 100 file has every group unread, which `public-api.md` does state.
**Why it is wrong**: these are false claims a user of the library reads. The reader never uses dcmjs's decode of a content-tree string, so the comment at `:1044` already gives the reason the retry is safe for every file.
**Evidence**: the deflated ISO_IR 203 and `\ISO 2022 IR 87` files return null at the tip in Node and in Chromium. With `parsed(bytes, true)` in a scratch copy both read as 3 unread, the four truncated files still return null, and 72 of 72 tests stay green.
**Scope and regression**: present at f7b0c7b3 too, so it is not a regression. The sentences were rewritten in this range, and the D2 remediation added the deflated path they describe. Fix: always retry, `parsed(bytes, true)`, or remove the parameter. In `use.html` add that a file the reader cannot walk, such as a deflated one, is decoded only when it has no Specific Character Set. Regression: the existing `deflated()` helper on a recoded ISO_IR 203 file, expecting `{ text: null, unread: 1 }` rather than null.

### D3, README says deleting every mark of a study is saved, and in a study with a report placed in part the deleted marks come back

**Where**: `README.md:394` to `:396`, and the status sentence at `examples/folder-viewer/src/page.html:5225`
**What**: in the adjacent probe the user deleted every mark the page showed and saved. The status line said the new file "replaces the earlier one". On reload the partial report's length on slice 5, which the user had deleted, came back beside its rectangle. README says "Deleting every mark of a study is saved too, as an SR with no marks that replaces the earlier one." The behaviour is what the pass 3 ruling chose: a report placed in part is never a predecessor. Neither README nor the status line tells the user that this report is kept, so their deletion of its marks is not saved.
**Why it is wrong**: a false sentence in README is a defect under `/microscope` section 3. Pass 3's D3 named this README line. The behaviour it described is fixed, and the sentence is still false for pass 3's second trigger. The pass 2 D3 rule says the status line says why something is not saved.
**Evidence**: the adjacent probe above, reproduced on the tip binary.
**Scope and regression**: in this range the mixed case changed from "Nothing was saved." to a save, so the misleading status sentence is new here. Fix, docs and status only, with no behaviour change: README adds that a report placed only in part is not replaced, so its marks come back even if they were deleted. The status line adds a count, for example "1 report placed only in part is kept, so its marks come back", whenever the save's study read a report not placed whole. Recording that count needs the placement result the page already gets from `placeDocuments`. Regression: assert that sentence in the adjacent case, in `mark-documents.test.ts` or in `observeDeleteAll`.

## Smells

### S1, nothing tests that a file the walk cannot read, carrying a Specific Character Set, is counted unread

**Where**: `packages/core/src/tid1500.ts:1067`
**What**: mutant O2, which applies the default repertoire whenever the walk cannot run, survives. Under it, a deflated ISO_IR 13 file, whose G0 is JIS X 0201 Romaji, or a deflated ASCII-only file in a set the reader refuses would be shown as ISO-IR 6 text, which is quietly wrong text rather than unread. The guard is correct today and nothing would notice its removal.
**Fix**: one vitest case: a deflated ISO_IR 100 or ISO_IR 13 report of ASCII text, expecting `{ text: null, unread: 1 }`, red under O2.

## Nitpicks

- N1. The TIS 620 test samples only A0, DB and FC, so mutant O3 survives. It is folded into D1's regression.
- N2. Mutant P2, a report placed in part named as a predecessor in the module the binary serves, survives the browser gate. The rule is covered at module level, where T9 is red, and the adjacent probe shows the page right today. This is carried from pass 3.

## Verified clean

Pass 3 findings replayed through the real caller:

| Pass 3 | State | How |
|---|---|---|
| D1 | closed | ISO_IR 203 reads `€ Œ Ÿ` in explicit, big endian and implicit VR, in Node and Chromium. T1 red |
| D2 | closed | big endian and deflated ASCII with no set read whole. Big endian ISO_IR 100, 192 and 203 read whole. T2a, T2b and O1 red. The deflated-with-a-set gaps are D2 and S1 above |
| D3 | closed in behaviour | the page step goes red on f7b0c7b3 and green on the tip. The adjacent case keeps only the partial report's marks. P1 and T9/T10 red. The README claim is D3 above |
| S1 | closed | T3, T4 and T5 red |
| N1 | closed | README's paragraph is at most 79 columns and the reflow changes no word |
| N2 | closed | ISO_IR 6 reads whole. T6 red |
| N3 | **open** as D1 above | the 88 comment is right, but the A0 refusal is wrong |
| N4 | closed | 200000 bytes decode. T8 and O4 red |

**The big endian walk** matches PS3.5 2016b A.3, where tags, lengths and values are big endian under 7.3, and 2026d 7.1.2, where the tag is two 16-bit numbers, then the VR, then a 16-bit length or two reserved bytes and a 32-bit length. The file meta stays explicit VR little endian, PS3.10 7.1. The CS value is a byte string, read unswapped. Implicit VR always reads little endian lengths, which is right because implicit VR has no big endian form.

**The D2 fallback's premise.** dcmjs 0.52.2's `_read` adds (0008,0005) to its dict only when it reads that tag, at `dcmjs.es.js` 12859 to 12880, so `SpecificCharacterSet === undefined` means the file has none.

**TIS 620 bytes DB to DE and FC to FF.** These are refused correctly. ISO-IR 166 says those positions shall not be used, and windows-874 maps them to private use code points.

**The page step's own claims.** "slices 1 to 11" matches `synthetic.rs` `SLICES = 12`. `CLEARED` is a prefix of the sentence at `page.html:5225`. The `fetch` global added to `eslint.config.js` is used only by `marks.mjs` under Node.

**Removed state.** `partialStudies`, `placed.partial` and `sessionPartialStudies` have no remaining reader or writer in the tree, and the `.d.mts` matches the module.

**User docs.** `public-api.md:298` and `use.html:882` match the code apart from D2 and the A0 sentence under D1. `README.md` is correct apart from D3. `more.html` is unchanged in this range and makes no character set claim.

## Limitations and handoff

- AC18 in real Chrome and AC19 stay with the operator at sprint close, as ruled. dciodvfy is not installed.
- The character set files are byte-level recodes of one package-written SR, not vendor files. The ISO_IR 203 file was read by `readTid1500Groups` in Chromium, not loaded through Load marks onto matching images.
- The adjacent probe used the page's synthetic CT and Playwright's directory upload.
- Process notes. All mutants and probes ran in scratch copies made with `git archive`, with node_modules linked from the worktree and their own `CARGO_TARGET_DIR`. Each copy was diffed against the worktree after restoring. Scratch files were removed with Python. No tracked file was edited apart from this review, which is left uncommitted.
- Next: implementation remediates D1 to D3 and S1, then a fresh independent pass 5 reviews the changed material.

F-215 pass 4: 3 defects, 1 smell, 2 nitpicks.
