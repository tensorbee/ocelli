# F-229 review, pass 14

**Reviewed**: `git diff HEAD~1..HEAD` on `work/f-229-xai`, HEAD `930862c4615c04aa840c344514da5f6dd8cd4207`, tree `0b52a457a40657acd36995c9e206e0abf2e65115`. The commit adds pass 13's review file and changes one line of `docs/lld/oracle.md`. I also reread every added sentence of the viewer and documentation half, `git diff 04442962..HEAD` over `README.md`, `docs/`, `examples/folder-viewer/src/` and the skill files, for counts. No unstaged tracked changes. The one untracked path, `tools/oracle/node_modules`, is not reviewed material.
**Author / reviewer**: the lead wrote 930862c4. This pass is an independent Claude reviewer. I wrote none of the reviewed material and none of passes 12 or 13.
**Result**: 0 defects, 0 smells, 1 nitpick

## Acceptance challenged

- **The one changed sentence.** I checked it against the artefact it names, not against the pass that asked for it. `tools/oracle/render-params.json` has one commit in its whole history (`git log --all --follow`), F-010's 787225d0, which is an ancestor of F-011's 2ffabe37. The blob is `5c81ae6c` at 2ffabe37 and at HEAD. The worktree `run.json` records `renderParamsSha256` `d771d96e...`, and `shasum -a 256` of the file on HEAD gives the same digest. The manifest and `unsupported.json` blobs at 2ffabe37 and at F-X007's b878c9c8 are identical (`226a7c3d` and `a926a834`), so when F-X007 wrote the sentence its stack frames were F-011's frames exactly.
- **Counts the diff states as present tense.** I recomputed them from the run and the files rather than reading them. A count true only of an older run would have passed a review that read the prose.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git log --all --follow -- tools/oracle/render-params.json`, `git rev-parse` of the blob at 2ffabe37 and HEAD, `git merge-base --is-ancestor 787225d0 2ffabe37` | history | now | one commit, 787225d0. Both blobs `5c81ae6c9df821a79ca646369a580e7eb0cd6e36`. Ancestor holds |
| `shasum -a 256 tools/oracle/render-params.json` against `renderParamsSha256` in `tools/oracle/out/run.json` and the main checkout's `run.json` | HEAD, the two local runs | now | all three `d771d96e42f0d6c0c5b96ed362d97642da27c33e04b8089828281aaf70593d4a` |
| `git rev-parse` of `corpus/manifest.tsv` and `tools/oracle/unsupported.json` at 2ffabe37 and b878c9c8 | history | now | identical pair at both commits |
| Python read of the worktree `run.json` | `tools/oracle/out/run.json` | now | `applicable` 100, `readBack` 93, `unsupported` 7. 16 stack rows and 6 reformats in `lowInformation`. The 7 refused are `us_ybr_full_422` (blank read-back), `deflated_explicit_vr_le` and the 5 JPEG XL rows |
| pydicom over the 93 rows with `ok` true, `uv run --with pydicom` | `corpus/data`, the run's rows | now | 7 colour, 85 monochrome with a top-level window, and `ct_multiframe_perframe.dcm` with `FrameVOILUTSequence` per frame, so 86 windowed. Two rows larger than 512 in both dimensions, `dx_varepop` and `us_cmb_crc` |
| `awk` over `corpus/manifest.tsv` | HEAD | now | 100 rows, 24 under `syntax/`, 5 of them `jpegxl_*` |
| `git grep` for "eighty" or "ninety" before "stack" or "rendered" in `docs`, `README.md`, `examples` | HEAD | now | only the four `oracle.md` lines this half edits, plus `DEVIATIONS.md:33`, which is outside the diff and says 89 stack frames for F-011, which agrees with history |
| `curl` of `sect_G.2`, `sect_G.3`, `sect_G.3.2`, `sect_G.4`, `sect_G.5` from dicom.nema.org, PS3.5 2026d | the standard | now | see N1 |
| `python3 scripts/prose_check.py` | HEAD worktree | now | exit 0, 919 files |

## Defects

None.

## Smells

None.

## Nitpicks

- **N1, carried from pass 13 and now confirmed**: `examples/folder-viewer/tests/preview.rs:628` cites "PS3.5 G.2 and G.3.1" for "a 64 byte header naming one segment at offset 64, then one literal run of count 3 ... padded to even length with one zero byte". In PS3.5 2026d, G.2 is "Byte Segments" and G.3.1 is "The RLE Encoder", which defines the literal run as `< count - 1 >` then the octets. The header is G.5, "RLE Header Format": 16 little-endian UL values, 64 bytes, "the offset of RLE Segment One is 64". G.4 places the header before the segments. The even-length padding rule is in G.3's opening paragraph, not G.3.1. So the citation covers the run and misses the section that defines the header. `crates/ocelli-codec/tests/rle.rs:74` cites "G.2 through G.5" for the same shape. The bytes are right under the cited and the uncited sections: one segment, offset 64, run byte 3, which G.3.2's decoder reads as "output the next n+1 bytes literally", four samples, one zero pad. The smallest fix is "PS3.5 G.3 to G.5". This is not a defect because the arithmetic and the expected greys are unaffected.

## Verified clean

- **Pass 13 D1.** Closed. The sentence now reads "the stack frames are the same artefact F-011 started against", with no count to go stale. It is true for the tree F-X007 wrote it in, because the manifest, `unsupported.json` and `render-params.json` there are byte-identical to F-011's. The digest has not moved since, so as a claim about render parameters it is still true today. The stack row set has grown since F-011, 89 rendered then and 93 now, and the sentence does not say otherwise once the count is gone.
- **Other counts in the half.** All true now. 93 rendered rows, 91 of them smaller than the canvas, 86 with their own window and 7 colour, 16 stack rows on `lowInformation` (15 under `syntax/` plus `ct_unsigned_16.dcm`), `syntax/` at 24 rows split 15, 3 and 6 as the parenthesis says, three HTJ2K and three JPEG-LS rows rendered, `readBack: 93`, five JPEG XL corpus rows, and the imagecodecs 2026.10.10 and libjxl 0.12.0 pins in `scripts/corpus_synth.py` `BUILT_WITH`. The "92 digests" sentence at line 49 lost its number, which removes a claim.
- **Historical counts.** I found no other sentence in the half that gives a count for a past commit. The dated measurements in `codecs.md` (1,129 bytes, 1,095,647,232, 2,260,992, 940,335) are measurements with dates, not counts of the tree. I did not rerun them, see Limitations.
- **Prose.** No em-dash, en-dash or prose semicolon in the changed line or in this file.

## Limitations and handoff

- No build, test, oracle, GPU or browser job. The commit changes one prose line, and the counts were checked against the existing worktree run, whose `unsupportedSha256` is the pre-edit file's, as pass 13 recorded.
- The `codecs.md` memory and size measurements were not reproduced.
- The section numbers come from the current PS3.5 edition, 2026d. I did not check an older edition.
- Nothing blocks. Zero defects and zero smells is a clean pass under the microscope severity rule, for tree `0b52a457`. N1 is the implementer's to take or leave. Any change to the tree, N1's fix included, makes this pass stale and needs a fresh independent pass over that change.
