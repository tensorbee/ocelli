# F-244, Encapsulated Pixel Data with an empty Basic Offset Table opens

**Status**: approved
**Epic ref**: E3.10
**Sprint**: S46
**Estimate**: 2w

Proposed story row, for the lead to file in `docs/sprints/BACKLOG.md` under `### M20, Local folder viewer`: "Encapsulated Pixel Data with an empty Basic Offset Table opens per PS3.5 A.4: the zero-length first Item is the table and every later Item a Fragment, at the top level and inside an Icon Image Sequence, for every encapsulated syntax the registry decodes, single-frame and multi-frame, with frames that span Fragments delimited by their codestream start and end markers where A.4 lets a frame span Fragments, a present Extended Offset Table applied through EncapsulatedFrameIndex::from_extended, and an empty-table corpus row per decodable syntax". Layer Rust, depends on F-019, milestone M20, phase P1.

Proposed `allocation.json` notes: "Approved by the operator on 2026-11-08 to be fixed in S46. dicom-object 0.10.0 collector.rs build_encapsulated_data clears its first-item flag only when an Item value is read, so a zero-length Basic Offset Table leaves the flag set and the first Fragment is read as the table. 0.10.1 and upstream master carry the same code. Found while F-213 was reviewed, AS_BUILT F-213 notes."

**Why F-244 and E3.10.** F-243 is the highest F-ID in `docs/sprints/BACKLOG.md`, `docs/sprints/allocation.json`, every `ocelli-wt-*` worktree's backlog and plans, every local branch name and every commit subject, measured on 2026-11-08. E3 ends at E3.9 (F-234) in the backlog, the allocation and every worktree. The S46 close drafts (`scratchpad/S46-close-backlog-drafts.md`) numbered this item F-245 and gave F-244 to the built-viewer `/@fs` item. Neither draft is filed, and this story takes the next free number, so the lead renumbers those drafts when filing them.

**Why S46.** The operator approved fixing it in S46 on 2026-11-08 and chose the route-agnostic answer "fix in S46". The draft's S47 is superseded. Its only declared dependency, F-019, is done.

Every path and line below was read on `sprint/s46` at 3c37ecd0. Third-party lines are from `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`.

## Normative source, transcribed

The HLD gives nothing on encapsulation. `docs/hld/` has no text on offset tables or Fragments, and §15.2 (`docs/hld/12-workspace-and-build.md:93`) names dicom-rs only for its features. The normative source is DICOM PS3.5, edition 2026d, fetched from dicom.nema.org on 2026-11-08, and PS3.3 2026d for the Extended Offset Table. The quotations below are verbatim.

### PS3.5 2026d A.4, the encapsulated Pixel Data element

"If Encapsulated Format, it has the Value Representation OB and is an octet-stream resulting from one of the encoding processes. It contains the encoded Pixel Data Stream fragmented into one or more Item(s). This Pixel Data Stream may represent a Single-frame or Multi-frame Image. See Table A.4-1 and Table A.4-2."

"Each Data Stream Fragment encoded according to the specific encoding process shall be encapsulated as a DICOM Item with a specific Data Element Tag of Value (FFFE,E000). The Item Tag is followed by a 4 byte Value (Item) Length Field encoding the explicit number of bytes of the Item."

"Note. Whether more than one Fragment per Frame is permitted or not is defined per Transfer Syntax."

"All items containing an encoded Fragment shall be made of an even number of bytes greater or equal to two. The last Fragment of a Frame may be padded, if necessary, to meet the Sequence Item format requirements of the DICOM Standard."

"Note. Any necessary padding may be added in the JPEG or JPEG-LS compressed data stream as per [ISO/IEC 10918-1] and [ISO/IEC 14495-1] such that the End of Image (EOI) marker ends on an even byte boundary, or may be appended after the EOI marker, depending on the implementation. [ISO/IEC 10918-1] and [ISO/IEC 14495-1] define the ability to add any number of padding bytes FFH before any marker (all of which also begin with FFH). It is strongly recommended that FFH padding bytes not be added before the Start of Image (SOI) marker."

### PS3.5 2026d A.4, the Basic Offset Table

"The first Item in the Sequence of Items before the encoded Pixel Data Stream shall be a Basic Offset Table item. The Basic Offset Table Item Value, however, is not required to be present:

- When the Item Value is not present, the Item Length shall be zero (00000000H) (see Table A.4-1).
- When the Item Value is present, the Basic Offset Table Item Value shall contain concatenated 32-bit unsigned integer values that are byte offsets to the first byte of the Item Tag of the first Fragment for each Frame in the Sequence of Items. These offsets are measured from the first byte of the first Item Tag following the Basic Offset Table item (see Table A.4-2)."

"Note. For a Multi-frame Image containing only one Frame or a Single-frame Image, the Basic Offset Table Item Value may be present or not. If present it will contain a single 00000000H value. **Decoders of encapsulated Pixel Data, whether Single-frame or Multi-frame, need to accept both an empty Basic Offset Table (zero length) and a Basic Offset Table filled with 32 bit offset values.** A Basic Offset Table Item Value is not permitted (i.e., the Item Length of the first Item will be zero) if Extended Offset Table (7FE0,0001) is present. If the Basic Offset Table Item Value is not zero length, the first entry will always be 00000000H. The offset is to the first byte of the Item Tag itself, not the first byte of the Value Field within the Item."

"This Sequence of Items is terminated by a Sequence Delimitation Item with the Tag (FFFE,E0DD) and an Value (Item) Length Field of Value (00000000H) (i.e., no Value Field shall be present)."

"Note. The decoder of a Data Set will need to parse the Value Field processing the Item Elements to determine the end of the Value Field. It may not scan for a Sequence Delimitation Item since the series of bytes of which it is composed may be a valid value within the Value Field, and there is no escape mechanism."

Table A.4-1 is titled "Example for Elements of an Encoded Single-frame Image Defined as a Sequence of Three Fragments Without Basic Offset Table Item Value": Pixel Data `(7FE0,0010)` OB, `0000H` reserved, `FFFFFFFFH` undefined length, then a Basic Offset Table Item `(FFFE,E000)` of length `00000000H` with no value, then three Fragment Items `(FFFE,E000)` each with its length and value, then `(FFFE,E0DD)` of length `00000000H`. Table A.4-2 is titled "Examples of Elements for an Encoded Two-Frame Image Defined as a Sequence of Three Fragments with Basic Table Item Values".

### PS3.5 2026d 8.2, fragmented frames

"Depending on the Fragmentable Encapsulated Transfer Syntax, a frame may be entirely contained within a single Fragment, or may span multiple Fragments to support buffering during compression or to avoid exceeding the maximum size of a fixed length Fragment. A recipient can detect fragmentation of Frames by comparing the number of Fragments (the number of Items minus one for the Basic Offset Table) with the number of Frames. Some performance optimizations may be available to a recipient in the absence of fragmentation of Frames, but **an implementation that fails to support such fragmentation does not conform to the Standard.**"

### PS3.5 2026d, the per-syntax Fragment rules

| Section | Syntaxes | Rule, verbatim |
|---------|----------|----------------|
| A.4.1 JPEG | `.50`, `.51`, `.57`, `.70` | "If the object allows Multi-frame Images in Pixel Data (7FE0,0010), then each Frame shall be encoded separately. Each Fragment shall contain encoded data from a single Frame. Note. Though a Fragment may not contain encoded data from more than one Frame, the encoded data from one Frame may span multiple Fragments." |
| A.4.2 RLE | `.5` | "If the object allows Multi-frame Images in Pixel Data (7FE0,0010), then each Frame shall be encoded separately. Each Frame shall be encoded in one and only one Fragment (see Section 8.2)." |
| A.4.3 JPEG-LS | `.80`, `.81` | "each Frame shall be encoded separately. Each Fragment shall contain encoded data from a single Frame. Note. Though a Fragment may not contain encoded data from more than one Frame, the encoded data from one Frame may span multiple Fragments." |
| A.4.4 JPEG 2000 and HTJ2K | `.90`, `.91`, `.201`, `.202`, `.203` | "Each Fragment shall contain encoded data from a single Frame. ... Though a Fragment may not contain encoded data from more than one Frame, the encoded data from one Frame may span multiple Fragments for JPEG 2000 Transfer Syntaxes. See note in Section 8.2. For HTJ2K encoding, each Frame shall be encoded separately as a single Fragment." |
| A.4.12 JPEG XL | `.110`, `.111`, `.112` | "For JPEG XL encoding, each Frame shall be encoded separately as a single Fragment." |

### PS3.3 2026d C.7.6.3, Extended Offset Table

Attribute row, Extended Offset Table `(7FE0,0001)`, Type 3: "Byte offsets of the Item Tags of the Frames in the Sequence of Items in Encapsulated Pixel Data encoded in Pixel Data (7FE0,0010). See Section C.7.6.3.1.8. May only be present when: Pixel Data (7FE0,0010) is present, and the Transfer Syntax uses Encapsulated Format for the Pixel Data (7FE0,0010), and the Transfer Syntax encodes Frames in separate Fragments, and the Basic Offset Table is not present (i.e., the first Item of Pixel Data (7FE0,0010) has zero length), and each Frame is entirely contained within one Fragment. Note. Unlike a Basic Offset Table, an Extended Offset Table, if the Attribute is present, is not permitted to be empty." Extended Offset Table Lengths `(7FE0,0002)`, Type 1C: "Byte lengths of the Frames in the Sequence of Items in Encapsulated Pixel Data encoded in Pixel Data (7FE0,0010). See Section C.7.6.3.1.8. Required if Extended Offset Table (7FE0,0001) is present."

C.7.6.3.1.8: "The Extended Offset Table (7FE0,0001) Value shall contain byte offsets to the first byte of the Item Tag of the one and only Fragment for every Frame in the Pixel Data (7FE0,0010) Sequence of this Instance. The byte offsets are measured from the first byte of the first Item Tag following the empty (zero length) Basic Offset Table Item, i.e., the Item Tag of the one and only Fragment of the first Frame. ... If present, the first entry will always be 0000000000000000H."

### The frame-boundary rule this gives, when the table is empty

1. **The first Item is the table, whatever its length.** A zero-length first Item is an empty table, and the next Item is the first Fragment. That is the rule dicom-object breaks.
2. **One frame.** Every Fragment belongs to it, in order (Table A.4-1).
3. **Several frames, as many Fragments as frames.** One Fragment per frame, by 8.2's count comparison and the rule that a Fragment holds data from a single frame.
4. **Several frames with an Extended Offset Table.** Each frame is its one Fragment at the offset and length the table gives (C.7.6.3.1.8), and the Basic Offset Table is empty by A.4.
5. **Several frames, more Fragments than frames.** PS3.5 gives no algorithm. It says only that a frame may span Fragments under A.4.1, A.4.3 and A.4.4 Part 1, and that failing to support that does not conform (8.2). Under A.4.2, A.4.4 HTJ2K and A.4.12 such a file is itself non-conformant. Section "What the specification does not cover", item 2, gives this plan's rule.

### Transcription flag

The A.4 and C.7.6.3 text was fetched from `dicom.nema.org/medical/dicom/current/output/chtml/` and reduced from HTML to text for this plan. The edition banner on every fetched page reads 2026d. AC16 makes the check of every quoted line against the published pages a reviewer row.

## Facts measured for this plan

From a scratch copy of the tracked tree (`git archive` of 3c37ecd0) with its own `CARGO_TARGET_DIR`, on 2026-11-08. The generator and probe are kept in the session scratchpad under `f244/`, `gen.py` and `bot_probe.rs`, with their outputs `probe.tsv` (unpatched) and `probe-patched.tsv`.

**Inputs.** 126 synthetic files derived from the 14 corpus syntax rows of the decodable encapsulated syntaxes, each re-encapsulated by hand per Table A.4-1 and A.4-2 with its codestream unchanged: single frame in one Fragment (`s1`), single frame in three (`s3`), three frames in one Fragment each (`m1`), three frames in two Fragments each (`m2`), each with an empty table and with a populated one, and `m1` with an Extended Offset Table built by pydicom's `encapsulate_extended` (`m1_eot`). No patient data. The probe runs `parse_part10`, then `EncapsulatedFrameIndex::from_basic` on what dicom-object returns, then a registry decode of every frame, then the production `preview_frame` for every frame. It compares each extracted frame with the populated-table control's.

**Controls.** All 56 populated-table files open, every frame decodes, and every monochrome preview draws. The 14 `.50` previews refuse `NotMonochrome`, which is the preview's colour rule and not this defect.

**Reproduction, unpatched dicom-object 0.10.0.** All 70 empty-table files fail. "Table" is what dicom-object hands back as the offset table.

| Syntax | First Fragment length, mod 4 | `s1` | `s3` | `m1` | `m2` | `m1_eot` |
|--------|-----------------------------|------|------|------|------|----------|
| `.5` RLE | 6784, 0 | index `NoFragments`, table of 1696 values `[2, 64, 576, ...]` | index `OffsetCount { declared: 1, actual: 565 }` | index `OffsetCount { declared: 3, actual: 1696 }` | index `OffsetCount { declared: 3, actual: 848 }` | as `m1` |
| `.50` JPEG Baseline | 936, 0 | index `NoFragments` | index `OffsetCount { 1, 78 }` | index `OffsetCount { 3, 234 }` | index `OffsetCount { 3, 117 }` | as `m1` |
| `.51` JPEG Extended | 764, 0 | index `NoFragments` | parse `InvalidDataSet` | index `OffsetCount { 3, 191 }` | parse `InvalidDataSet` | as `m1` |
| `.57` JPEG Lossless | 8482, 2 | parse `InvalidDataSet` | parse `InvalidDataSet` | parse `InvalidDataSet` | index `OffsetCount { 3, 1060 }` | parse `InvalidDataSet` |
| `.70` JPEG Lossless SV1 | 3972, 0 | index `NoFragments` | index `OffsetCount { 1, 331 }` | index `OffsetCount { 3, 993 }` | parse `InvalidDataSet` | as `m1` |
| `.80` JPEG-LS | 1354, 2 | parse `InvalidDataSet` | parse `InvalidDataSet` | parse `InvalidDataSet` | index `OffsetCount { 3, 169 }` | parse `InvalidDataSet` |
| `.81` JPEG-LS near | 1052, 0 | index `NoFragments` | parse `InvalidDataSet` | index `OffsetCount { 3, 263 }` | parse `InvalidDataSet` | as `m1` |
| `.90` JPEG 2000 lossless | 872, 0 | index `NoFragments` | parse `InvalidDataSet` | index `OffsetCount { 3, 218 }` | index `OffsetCount { 3, 109 }` | as `m1` |
| `.91` JPEG 2000 | 628, 0 | index `NoFragments` | index `OffsetCount { 1, 52 }` | index `OffsetCount { 3, 157 }` | parse `InvalidDataSet` | as `m1` |
| `.201` HTJ2K | 1508, 0 | index `NoFragments` | parse `InvalidDataSet` | index `OffsetCount { 3, 377 }` | parse `InvalidDataSet` | as `m1` |
| `.202` HTJ2K RPCL | 1508, 0 | index `NoFragments` | parse `InvalidDataSet` | index `OffsetCount { 3, 377 }` | parse `InvalidDataSet` | as `m1` |
| `.203` HTJ2K | 558, 2 | parse `InvalidDataSet` | parse `InvalidDataSet` | parse `InvalidDataSet` | parse `InvalidDataSet` | parse `InvalidDataSet` |
| `.110` JPEG XL lossless | 1612, 0 | index `NoFragments` | index `OffsetCount { 1, 134 }` | index `OffsetCount { 3, 403 }` | parse `InvalidDataSet` | as `m1` |
| `.112` JPEG XL | 292, 0 | index `NoFragments` | index `OffsetCount { 1, 24 }` | index `OffsetCount { 3, 73 }` | parse `InvalidDataSet` | as `m1` |

The layer is decided by the first Fragment's length alone. A length that is a multiple of four parses, the Fragment comes back as the table, and `EncapsulatedFrameIndex::from_basic` refuses with `NoFragments` or `OffsetCount`, which `preview.rs:1540` maps to `PreviewError::Decode`. A length of 2 mod 4 is refused by `parse_part10` as `InvalidDataSet` before any index or decoder runs. No decoder is ever reached, so **no frame decodes and no pixel is quietly wrong**: every failure is a loud refusal. The `s3` and `m2` columns are mixed because their first Fragment is a split of the frame and its length varies.

**A compressed Icon Image is a second trigger.** A file whose top-level Pixel Data carries a populated table and decodes, with an Icon Image Sequence `(0088,0200)` item holding encapsulated Pixel Data with an empty table and a 6-byte Fragment, is refused whole by `parse_part10` as `InvalidDataSet`. With a 4-byte icon Fragment it opens, and the icon's Fragment is silently its table. PS3.5 A.4 permits the icon's Pixel Data to be encapsulated.

**The cause, in dicom-object 0.10.0** (`Cargo.lock:296` to `:299`, checksum `0504b879...6b85`, equal to the SHA-256 of the cached `.crate`). `src/collector.rs` `build_encapsulated_data`, `:1214` to `:1274`. `let mut first = true` at `:1227` decides whether an Item value is the table. It is cleared only at `:1239`, inside the `LazyItemValue` arm, which a zero-length Item never produces. The `ItemEnd` arm at `:1246` to `:1253` sets an empty table and leaves `first` true, so the next Item, the first real Fragment, is read as the table at `:1233` to `:1238`.

**Why a length of 2 mod 4 becomes `InvalidDataSet`.** `dicom-parser` 0.10.0 `src/stateful/decode.rs:1089` to `:1091`, `read_u32_to_vec` reads `length >> 2` words, so 1 to 3 bytes of the Item are left unread. `src/dataset/lazy_read.rs:408` to `:441` then emits the same Item's value again because the position is short of its end. The collector, now with `first` false, reads the full length a second time from the wrong offset and overruns the Item. `update_seq_delimiters` at `:263` to `:298` raises `InconsistentSequenceEnd` (`:285` to `:290`), and Ocelli's `map_collector_error` (`crates/ocelli-dicom/src/parse.rs:604`) maps it to `InvalidDataSet`. The existing test `encapsulated_pixel_data_accepts_a_two_byte_fragment` passes by that same re-emission: zero words are read, then the 2 bytes come back as a Fragment.

**Ocelli's own preflight is right.** `validate_data_set_structure` (`parse.rs:349` onwards) walks the same Items and treats the first Pixel Item as the table whatever its length (`:446` to `:457`). The defect is only in collection.

**The cause, confirmed by a one-line patch.** In a scratch copy of dicom-object 0.10.0 with `first = false;` added at the end of the `ItemEnd` arm, wired in through `[patch.crates-io]`, 56 of the 70 empty-table files open. Every extracted frame is byte-identical to its populated-table control, and every registry decode and preview equals the control's, by the preview's grey-byte sum. Both icon files open with their Fragment intact. All 14 `m2` files are then refused by Ocelli's own rule, `BoundaryUnavailable { declared: 3, fragments: 6 }` (`frame_index.rs:118` to `:127`). `cargo test -p ocelli-dicom` passes in full against the patched copy.

**No published fix.** dicom-object 0.10.1, the newest on crates.io (`cargo info dicom-object`), has the identical function, diffed. Upstream `master` `object/src/collector.rs`, fetched 2026-11-08, has the identical function. dicom-rs issue 745, "Collector API ignores offset table", is the opposite defect, a populated table ignored. Its fix is what introduced `first`. No issue or pull request for the empty table was found by `gh search`. The eager reader in `src/mem.rs:2250` to `:2297` receives an `OffsetTable` token from the parser and is not affected, but Ocelli does not use it.

**What else reaches dicom-object.** `cargo tree -i dicom-object -e normal` lists `ocelli-dicom`, `ocelli-folder-viewer`, `ocelli-example-server` and `ocelli-oracle`, nothing outside the workspace. `cargo tree -p ocelli-wasm -e normal --target wasm32-unknown-unknown` reaches no dicom crate, in the default build and in `--no-default-features --features decode-worker`. The package is 432 KB with seven source files, no `unsafe`, `license = "MIT OR Apache-2.0"`, and `.cargo_vcs_info.json` names upstream commit `39603cef65da125e47daffbb32310c6826f2f62b`, path `object`. Its dev-dependency `dicom-test-files` fetches test files over the network.

**Codestream markers in the corpus frames.** Every `.50`, `.51`, `.57`, `.70`, `.80` and `.81` frame begins `FF D8` and ends `FF D9` or `FF D9 00`. Every `.90`, `.91`, `.201`, `.202` and `.203` frame begins `FF 4F` and ends `FF D9` or `FF D9 00`. RLE frames begin with their segment count and JPEG XL frames with a box or `FF 0A`, and neither carries an end marker.

**Consumers.** Two production callers build a frame index from dicom-object's value: `examples/folder-viewer/src/preview.rs:1529` to `:1563` (`decode_encapsulated`) and `tools/oracle/src/produce.rs:687` to `:720` (`encapsulated_bytes`). Neither reads the Extended Offset Table. `EncapsulatedFrameIndex::from_extended` (`frame_index.rs:156`) exists, is tested, and has no production caller.

## What the specification does not cover

1. **Where the fix lives.** PS3.5 says what the Items mean, not which library reads them. The route is the lead's to confirm, under "Route, for the lead to confirm". The rest of this plan holds under any route, except the write set and deviation rows named there.
2. **Frames that span Fragments when the table is empty.** PS3.5 gives no algorithm (rule 5 above). This plan's rule, for a multi-frame object with an empty table, no Extended Offset Table, and more Fragments than frames:
   - The syntax decides whether a frame is delimited by markers. JPEG and JPEG-LS frames use SOI `FF D8 FF` as start and EOI `FF D9` as end. JPEG 2000 Part 1 and HTJ2K frames use SOC then SIZ, `FF 4F FF 51`, as start and EOC `FF D9` as end. RLE and JPEG XL have no end marker and keep today's refusal.
   - A Fragment **starts a frame** when the logical prefix from that Fragment through later Fragment Values begins with the syntax's start bytes. It **ends a frame** when the logical suffix through its Value ends with `FF D9`, or with `FF D9 00`, the one pad byte A.4 allows for the last Fragment of a frame. Either marker may cross an even Fragment boundary.
   - Frame 0 starts at Fragment 0. Each frame ends at the first Fragment at or after its start that ends a frame. The next frame starts at the following Fragment, which must start a frame. Exactly `Number of Frames` frames must be found and the last must end at the last Fragment. Anything else is `FrameIndexError::BoundaryUnavailable`, unchanged.
   - Markers are read only in this case. The one-Fragment-per-frame case and every populated-table case keep today's behaviour, because tightening them could refuse files that open today.
   - Why the start check. JPEG stuffs a zero after every `FF` in entropy-coded data, JPEG-LS keeps the top bit of the byte after `FF` clear, and JPEG 2000 forbids `FF` followed by a byte above `8F` in packet data, so `FF D9` at a Fragment end is EOI or EOC except inside header segment payloads such as APPn or COM. A split after such bytes would end a frame early. Requiring the next Fragment to start a frame, and the count to come out exactly, refuses that file rather than handing a decoder the wrong bytes. cornerstone3D 5.8.2 checks only the end marker and would hand them over. AC10 is that case.
3. **HTJ2K split across Fragments.** A.4.4 forbids it, so such a file is non-conformant. cornerstone3D 5.8.2 opens it by its end-marker scan, so refusing it would put Ocelli below the floor. HTJ2K uses the JPEG 2000 Part 1 codestream markers, so the same proven boundaries apply, and the plan reads it leniently. The operator may veto at review without changing another row.
4. **RLE and JPEG XL split across Fragments.** Non-conformant under A.4.2 and A.4.12, and with no in-band end marker there is nothing to prove a boundary. They stay refused as `BoundaryUnavailable`. cornerstone3D 5.8.2 concatenates every Fragment as frame 0 and throws for later frames. The floor table records this as a named departure on non-conformant input.
5. **The Extended Offset Table.** When `(7FE0,0001)` is present, the index is built by `from_extended` from it and `(7FE0,0002)`, whose existing checks then hold: one Fragment per frame, offsets on Item boundaries, lengths matching with the odd-length pad. An Extended Offset Table beside a populated Basic Offset Table breaks A.4's "not permitted" and is refused as `ExtendedWithBasicOffsets`, a new variant. A malformed or absent `(7FE0,0002)`, or a value not readable as 64-bit unsigned, is refused as `ExtendedOffsetTableUnreadable`, a new variant. cornerstone3D 5.8.2 ignores the Extended Offset Table. This plan reads it because a table that disagrees with the Fragments means one of the two is wrong and Ocelli cannot tell which. Those refusals are a departure on non-conformant input, recorded in the floor table.
6. **One provider for both consumers.** The preview and the oracle producer each repeat the same five lines of `offset_table()`, `fragments()` and `from_basic`. The table choice, the marker rule and the Extended Offset Table read go into one constructor in `ocelli-dicom`, and both call it, so the rule exists once. It reduces cases for both callers, and it is not a forwarding wrapper, because it owns the selection.
7. **The Icon Image Sequence.** No Ocelli code reads icon pixels. The fix is in scope only because a compressed icon with an empty table refuses the whole file today. Nothing new reads or draws an icon.
8. **Not a quirk.** `corpus/quirks.json` and `scripts/quirk_check.py` hold field quirks, and the checker is written around the one sigmoid record. An empty table is conformant per A.4, so the new corpus rows are ordinary syntax-layer rows, not quirk records.

## Problem and reproducer

**Trigger.** Any Part 10 file under an encapsulated transfer syntax whose Basic Offset Table Item has length zero. A.4 allows that form for every syntax, and many writers emit it, for multi-frame objects especially. The same applies to encapsulated Pixel Data inside an Icon Image Sequence item.

**Observed.** The file does not open. Either `parse_part10` refuses it as `InvalidDataSet`, or the first Fragment comes back as the table and `EncapsulatedFrameIndex::from_basic` refuses with `NoFragments` or `OffsetCount`. In the folder viewer that reads "the frame could not be decoded" (`preview.rs:245`). The oracle producer stops the row as `frame-index-refused`.

**Expected.** The file opens, and each frame's bytes and pixels equal those of the same file with a populated table.

**Reproducer.** `scratchpad/f244/gen.py` writes the 126 files. `bot_probe.rs` is built as an example of `ocelli-folder-viewer` in the scratch copy and run over that directory. The draft finding's own reproducer is `corpus/data/syntax/rle_lossless.dcm` with its 4-byte table Item rewritten to length 0, which gives `Err(Decode)` from `preview_first_frame`.

**Why a story.** PS3.5 A.4 requires decoders to accept an empty table, and every corpus encapsulated row carries a populated one, which is why nothing caught this. `docs/lld/corpus.md:367` to `:370` already lists "an empty Basic Offset Table" as untested.

## Scope, API and compatibility

In scope:

- The dicom-object fix, by the confirmed route.
- `crates/ocelli-dicom/src/frame_index.rs`: the marker rule of item 2, the Extended Offset Table route of item 5, and the one constructor of item 6.
- `examples/folder-viewer/src/preview.rs` `decode_encapsulated` and `tools/oracle/src/produce.rs` `encapsulated_bytes` call that constructor.
- Fixtures, corpus rows and documentation below.

Public signatures, in `ocelli-dicom`:

```rust
/// How a transfer syntax lets one frame's encoded data span Fragments,
/// DICOM PS3.5 A.4.1 to A.4.12.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FrameDelimiter {
    /// No in-band end of frame. RLE A.4.2 and JPEG XL A.4.12.
    Unmarked,
    /// SOI `FF D8 FF` starts a frame and EOI `FF D9` ends it. JPEG A.4.1 and
    /// JPEG-LS A.4.3.
    JpegMarkers,
    /// SOC and SIZ `FF 4F FF 51` start a frame and EOC `FF D9` ends it.
    /// JPEG 2000 Part 1 and HTJ2K, A.4.4.
    Jpeg2000Markers,
}

impl FrameDelimiter {
    /// The delimiter for an encapsulated transfer syntax UID. A UID with no
    /// row here is `Unmarked`, which never infers a boundary.
    pub fn for_transfer_syntax(uid: &str) -> Self;
}

impl<'a> EncapsulatedFrameIndex<'a> {
    /// Build an index from a Basic Offset Table. An empty table maps one
    /// frame to every Fragment, equal counts to one Fragment per frame, and
    /// more Fragments than frames through `delimiter`.
    pub fn from_basic(
        frame_count: usize,
        offsets: &[u32],
        fragments: &'a [&'a [u8]],
        delimiter: FrameDelimiter,
    ) -> Result<Self, FrameIndexError>;

    /// Build the index for a parsed object's top-level Pixel Data: the
    /// Extended Offset Table when present, the Basic Offset Table otherwise,
    /// and the delimiter of the object's transfer syntax.
    pub fn from_parsed(
        parsed: &'a ParsedDicom,
        frame_count: usize,
    ) -> Result<Self, FrameIndexError>;
}

pub enum FrameIndexError {
    // ... existing variants unchanged ...
    /// Pixel Data is absent or is not encapsulated.
    NotEncapsulated,
    /// An Extended Offset Table is present beside a populated Basic Offset
    /// Table, which PS3.5 A.4 does not permit.
    ExtendedWithBasicOffsets,
    /// Extended Offset Table or its Lengths are absent, malformed or not
    /// 64-bit unsigned values.
    ExtendedOffsetTableUnreadable,
}
```

`from_extended` keeps its signature. `from_basic` gains `delimiter`, which breaks its three call sites: the two production callers, both moving to `from_parsed`, and `crates/ocelli-dicom/tests/frame_index.rs`. The crate is 0.1.0 and unpublished while D-22 blocks publication, and `git grep -n from_basic` finds no other caller. `FrameIndexError` is already `#[non_exhaustive]`, so the new variants break no exhaustive match. Each new variant names a failure class and carries no value from the file.

`from_parsed` borrows the Fragment values from the parsed object and allocates the vector of slices and of ranges once, as `from_basic` does today. The decoders and `Decoder::decode` are untouched.

Refusal policy. Every refusal that exists today keeps its variant and its precedence. The preview keeps mapping every index refusal to `PreviewError::Decode`, and the oracle to `frame-index-refused`.

Non-goals:

- Reading or drawing icon pixels.
- Tightening the populated-table path or the equal-count path with marker checks, item 2.
- Encapsulated Uncompressed `.98` and Deflated Image Frame `.8.1`, which the registry does not know.
- Any decoder change. The decoders are correct on every frame the fix delivers, measured.
- `frame_open.rs`, which refuses encapsulated Fragments as `UnsupportedPixelFragments` by design and does not index them.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `parse_part10`, every caller | dicom-object collector `build_encapsulated_data`, fixed | unchanged, one collection, no fallback parse | `offset_table()` empty and every Fragment present for an empty table, at the top level and inside an icon item |
| Folder viewer preview and value endpoints, `preview.rs:1529` | `EncapsulatedFrameIndex::from_parsed` | `frame_count` from Number of Frames, refusal is `PreviewError::Decode` | the frame draws where it was refused |
| Oracle producer, `produce.rs:687` | `EncapsulatedFrameIndex::from_parsed` | `frame_count` from the sidecar, frame 0 only, refusal is `frame-index-refused` | the new corpus rows produce a candidate |
| `crates/ocelli-dicom/tests/corpus.rs:173` | dicom-object fragments | reads the `rle_lossless.dcm` Fragment directly | unchanged |
| Folder viewer `pixel_note`, `preview.rs:3654` | dicom-object fragments | sums Fragment lengths for the header view | now counts every Fragment of an empty-table file. Before, it omitted the first |
| `ocelli-example-server` | parse only, through `ocelli-dicom` | unchanged | opens empty-table files |
| `ocelli-wasm`, `ocelli-native` | none, they reach no dicom crate | n/a | unchanged modules |

The delimiter table is owned once, by `FrameDelimiter::for_transfer_syntax`. Selecting between the two offset tables is owned once, by `from_parsed`. `frame_count` stays with each caller, as today, because the preview reads Number of Frames and the oracle reads its sidecar.

## State transitions and invariants

Stateless. `parse_part10` and the index are pure functions of their bytes. The invariant across consumers: for any file, the frames returned under an empty table are byte for byte the frames of the same codestreams under a correct populated table, or the call is refused. A refusal never yields frame bytes.

## Ownership, commit and recovery

Not applicable. Nothing is staged, cached or committed. The index borrows from the parsed object, and the decoder writes into the caller's buffer only after the existing checks, unchanged.

## Numbered acceptance inventory

**RED rule.** Every row that claims a failure today is run on the base tree first, and its refusal or its wrong value is recorded in the handoff. A test that does not go red on the base tree is replaced, not kept. Each GREEN is recorded on the tip.

**Corpus derivation rule.** An empty-table row derived from an existing row keeps that row's codestream byte for byte, so its expected decode is the source row's entry in `corpus/decode-digests.tsv`. Python writes that digest by decoding with pydicom, which reads empty tables correctly, independent of Ocelli. A multi-frame row encodes three distinct frames: the syntax reference image, that image reversed along columns, and reversed along rows. A frame swap or a wrong boundary then changes the decoded bytes.

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Table A.4-1 by hand, JPEG Baseline UID: Basic Offset Table Item `FE FF 00 E0 00 00 00 00`, one Fragment Item of length 6 with value `01 02 03 04 05 06`, then `FE FF DD E0 00 00 00 00`. `parse_part10` succeeds, `offset_table()` is empty and `fragments()` is exactly `[[1, 2, 3, 4, 5, 6]]`. Base tree: `InvalidDataSet` | `empty_table_then_a_six_byte_fragment` in `crates/ocelli-dicom/tests/part10.rs` | The present collector. A fix that clears the flag only for a 4-byte-aligned Item | implementation |
| AC2 | The same with an 8-byte Fragment `01` to `08`. Table empty, `fragments()` is `[[1, ..., 8]]`. Base tree: table `[0x04030201, 0x08070605]` and no Fragment, which is the quiet misread at parse level | `empty_table_then_an_eight_byte_fragment` | A test that only asserts parse success, which the base tree passes | implementation |
| AC3 | Table A.4-1's shape, three Fragments of lengths 4, 6 and 2, values `0A 0B 0C 0D`, `01` to `06`, `EE FF`. Table empty, three Fragments in order. Base tree: table `[0x0D0C0B0A]` and two Fragments | `empty_table_then_three_fragments_in_order` | Dropping or reordering a Fragment | implementation |
| AC4 | Table A.4-2's shape, unchanged. Table `[0x00000000, 0x00000010]`, Fragment A of 8 bytes, B of 4 and C of 2. `0x10` is 8 bytes of Item header plus 8 of value. Table and Fragments returned exactly, on base and tip | `populated_table_is_unchanged` | A fix that also drops the first value of a populated table | implementation |
| AC5 | A compressed icon. Top-level Pixel Data with table `[0]` and Fragment `01 02`. An Icon Image Sequence item whose Pixel Data has an empty table and a 6-byte Fragment `01` to `06`. Parses, and the icon's Fragments are `[[1, ..., 6]]`. Base tree: `InvalidDataSet` | `an_icon_with_an_empty_table_does_not_refuse_the_file` | A fix only in Ocelli's top-level path, route (a), which leaves this red | implementation |
| AC6 | The existing `encapsulated_pixel_data_accepts_a_two_byte_fragment` is strengthened to assert table empty and Fragments `[[1, 2]]`, true on base and tip. The two-byte table refusal and the no-Fragment refusal keep passing unchanged | `crates/ocelli-dicom/tests/part10.rs` | A fix that weakens the preflight | implementation |
| AC7 | Consumer, folder viewer. The hand RLE frame of `every_registration_in_codecs_decodes_a_monochrome_frame` (PS3.5 G.2 and G.3.1: header of one segment at 64, literal run of 4 samples 10, 20, 30, 40, one pad byte, 70 bytes in all) wrapped with an empty table. `preview_first_frame` gives grey `[0, 85, 170, 255]`, which is the full-range mapping of 10 to 40. Base tree: `Decode` | `an_empty_table_rle_frame_draws` in `examples/folder-viewer/tests/preview.rs` | Any route that fixes parse but not the preview's index call | implementation |
| AC8 | Marker rule, JPEG. Two frames, three Fragments, empty table: `FF D8 FF E0 11 22`, `33 44 FF D9`, `FF D8 FF DB 55 FF D9 00`. Frame 0 is Fragments 0 and 1, frame 1 is Fragment 2, with the pad byte kept, as A.4 permits. The same bytes under `Jpeg2000Markers` and under `Unmarked` give `BoundaryUnavailable { declared: 2, fragments: 3 }` | `jpeg_frames_spanning_fragments_split_at_their_markers` in `crates/ocelli-dicom/tests/frame_index.rs` | Today's unconditional refusal. A delimiter applied to the wrong syntax | implementation |
| AC9 | Marker rule, JPEG 2000. `FF 4F FF 51 00 29`, `AA BB FF D9`, `FF 4F FF 51 CC FF D9 00` give frames `[0..2]` and `[2..3]` | `jpeg2000_frames_spanning_fragments_split_at_their_markers` | Reading SOI where SOC is meant | implementation |
| AC10 | Marker rule refusals, each `BoundaryUnavailable`. (i) No Fragment but the last ends a frame, so one frame is found for two declared. (ii) Fragment 2 begins `00 00` instead of `FF D8 FF`. (iii) A false end: `FF D8 FF E0 FF D9`, `11 22 33 44`, `FF D8 FF DB FF D9`, where Fragment 0 ends `FF D9` but Fragment 1 does not start a frame. cornerstone3D's end-marker-only scan takes this as frame 0 = Fragment 0 and frame 1 = Fragments 1 and 2. (iv) The last Fragment does not end a frame. (v) Three frames found for two declared | `marker_rule_refuses_unproven_boundaries` | An end-marker-only scan, which accepts (iii). A scan that tolerates a count mismatch | implementation |
| AC11 | Equal counts are unchanged. Three frames in three Fragments with an empty table map one per Fragment under every delimiter, with no marker read, even when a Fragment carries no start marker | `equal_counts_need_no_markers` | Tightening the equal-count path, which could refuse files that open today | implementation |
| AC12 | Extended Offset Table through `from_parsed`. An `m1_eot` shape built by hand: empty table, three Fragments, an Extended Offset Table with the three hand-computed Item offsets and its Lengths. The index uses `from_extended`. With a wrong offset it refuses `OffsetNotFragmentBoundary`, and with a wrong length `InvalidExtendedLength`, which the Basic path would never see. A populated Basic table beside an Extended table gives `ExtendedWithBasicOffsets`. A missing Lengths element gives `ExtendedOffsetTableUnreadable`. A file with no Extended table takes the Basic path | `from_parsed_prefers_the_extended_offset_table` | Ignoring the Extended table, which cannot fail on a wrong one | implementation |
| AC13 | Delimiter table. `FrameDelimiter::for_transfer_syntax` gives `JpegMarkers` for `.50`, `.51`, `.57`, `.70`, `.80` and `.81`, `Jpeg2000Markers` for `.90`, `.91`, `.201`, `.202` and `.203`, and `Unmarked` for `.5`, `.110`, `.111`, `.112` and an unknown UID. Each row cites its A.4 subsection in the test | `delimiter_follows_ps3_5_a_4` | A JPEG XL or RLE file whose boundaries are guessed | implementation |
| AC14 | Corpus single-frame rows. `scripts/corpus_synth.py` writes one empty-table row per decodable encapsulated syntax by the derivation rule: `syntax/rle_lossless_empty_bot.dcm`, `jpeg_baseline_rgb8_empty_bot.dcm`, `jpeg_extended_12_empty_bot.dcm`, `jpeg_lossless_p14_empty_bot.dcm`, `jpeg_lossless_p14_sv1_empty_bot.dcm`, `jpegls_lossless_empty_bot.dcm`, `jpegls_near_lossless_empty_bot.dcm`, `j2k_lossless_empty_bot.dcm`, `j2k_lossy_empty_bot.dcm`, `htj2k_lossless_empty_bot.dcm`, `htj2k_lossless_rpcl_empty_bot.dcm`, `htj2k_lossy_empty_bot.dcm`, `jpegxl_lossless_empty_bot.dcm`, `jpegxl_lossy_empty_bot.dcm`. Each has a new SOP Instance UID from `fixed_uid`, an empty table, one Fragment byte-identical to its source row's, and a `decode-digests.tsv` entry equal to its source's. `corpus/manifest.tsv` gains the 14 rows through `scripts/corpus_check.py --add`. An ignored corpus test parses each one, indexes it through `from_parsed`, decodes it, and compares with its source row's decode. Base tree, all 14 refuse, with the layer recorded: `InvalidDataSet` for `.57`, `.80` and `.203`, `NoFragments` for the rest, measured | `manifest_backed_empty_table_rows_decode_as_their_sources` in `crates/ocelli-dicom/tests/corpus.rs`, `scripts/tests/test_corpus_synth.py` | A corpus that still cannot see the defect | implementation |
| AC15 | Corpus shape rows, by the derivation rule for multi-frame content: `syntax/rle_multiframe_empty_bot.dcm`, three frames in one Fragment each. `syntax/jpegls_lossless_fragmented_empty_bot.dcm`, one frame in three Fragments with the first Fragment's length 2 mod 4. `syntax/jpeg_lossless_p14_sv1_multiframe_fragmented_empty_bot.dcm` and `syntax/j2k_lossless_multiframe_fragmented_empty_bot.dcm`, three frames in two Fragments each, the first Fragment of each frame 2 mod 4. `syntax/j2k_lossless_multiframe_eot.dcm`, three frames, one Fragment each, empty table, Extended Offset Table and Lengths. Each comes from the syntax's existing producer in `EXTERNAL_ENCODERS` or the internal RLE encoder, with a multi-frame SOP class whose IOD allows Number of Frames, cited from PS3.3. The generator records each split offset. Each frame decodes equal to its distinct reference frame, and frames are compared one by one so a swap is caught. Base tree, each refuses, layer recorded | same tests, `scripts/tests/test_corpus_synth.py` | One frame repeated three times, which cannot catch a swap. Splits that never reach the 2 mod 4 path | implementation |
| AC16 | The standard. A reviewer checks every line under "Normative source, transcribed" against the published PS3.5 2026d A.4, A.4.1 to A.4.4, A.4.12 and 8.2, and PS3.3 2026d C.7.6.3 and C.7.6.3.1.8, and checks AC1 to AC13's hand bytes against them | review record | A transcription error implemented faithfully | review |
| AC17 | The route's own evidence. For route (b), `vendor/dicom-object-0.10.0/` holds the published package, digest `0504b879...6b85`, with a `PACKAGE-INVENTORY.sha256` of every published file, the upstream licence files from commit `39603cef`, and exactly one source patch in `src/collector.rs`, the `ItemEnd` arm clearing `first`. `PATCH-PROVENANCE.md` records the archive digest, the commit, the patched file's SHA-256 and the PS3.5 A.4 clause. `scripts/pin_and_size_check.py` gains `check_dicom_object_vendor` in the shape of `check_ritk_vendor`, recording the inventory digest, the patched digest and the provenance digest. `bin/ocelli.sh gate pins` passes, and a planted byte in any vendored file fails it with the file named. The guard catalogue gains that probe. The workspace dependency becomes `dicom-object = { version = "=0.10.0", path = "vendor/dicom-object-0.10.0", default-features = false, features = ["deflate"] }`, and the vendor path joins the workspace `exclude` list. `scripts/unsafe_allowlist_check.py` records `"dicom-object-0.10.0": 0` | `scripts/tests/test_pin_and_size_check.py`, guard probe | An unrecorded change in the vendored package. A second patch hidden beside the first | implementation |
| AC18 | D-18 amended in the same change, for route (b). It stops implying the published component is used unchanged and says `dicom-object` is the published 0.10.0 package vendored with one recorded patch under F-244, enforced by the pins gate, until a dicom-object release carries the fix. Its F-ID column gains F-244. `python3 scripts/deviation_check.py` passes | DEVIATIONS diff | A register that says the published crate is used while it is not | implementation |
| AC19 | Wasm size. `bin/ocelli.sh gate wasm` passes with `ci/wasm-size-budget.json` and `ci/wasm-decode-size.json` unedited and `--accept-size` never run. Both release module sizes are measured on base and tip and the deltas recorded, expected zero. `cargo tree -p ocelli-wasm -e normal --target wasm32-unknown-unknown --prefix none`, and again with `--no-default-features --features decode-worker`, shows no dicom crate | handoff, `docs/lld/build-targets.md` unchanged | A new edge from the shipped module to `ocelli-dicom` | implementation |
| AC20 | Cross-target. `bin/ocelli.sh gate native` passes, so `ocelli-dicom` with the fixed dicom-object still builds for `wasm32-unknown-unknown` and native with identical features | gate record | A vendored copy that builds only natively | implementation |
| AC21 | Existing behaviour. Every existing test in `crates/ocelli-dicom`, `examples/folder-viewer` and `tools/oracle` passes, and every existing corpus row's file digest and decode digest is unchanged | `cargo test --workspace`, `gate corpus`, `gate corpus-tests` | A fix that moves a value that was right | implementation |
| AC22 | The oracle sees the new rows. On the operator's machine `bin/ocelli.sh gate oracle` renders the 19 new rows through cornerstone3D 5.8.2, except the JPEG XL rows, which cornerstone3D cannot decode, as for the existing JPEG XL rows. `tools/oracle/compare-expectations.json` and any other census file the run changes record the rows with the qualifiers the run reports, in the same change | oracle run record | A row the census does not account for | implementation, on the operator's machine |
| AC23 | Provenance policy. The fix follows PS3.5 and reads dicom-rs (MIT or Apache-2.0) and cornerstone3D (MIT). dwv, Horos and Grok are never opened. `scripts/source_provenance_check.py` and `bin/ocelli.sh gate licences` pass | review, gate | Translating an excluded source | review |
| AC24 | JPEG and JPEG 2000 start and end markers crossing even Fragment boundaries, including a two-byte start Fragment and a final D9 with Item pad. Empty-table ranges equal populated-table controls through from_basic and from_parsed | `split_markers_across_even_fragments` | Looking for a marker only inside one Fragment | implementation |
| AC25 | Two Fragments with declared counts 3 and usize::MAX return BoundaryUnavailable before reservation. A divided EOI without a valid next start remains refused | `impossible_frame_count_is_refused_before_reservation`, `split_end_without_a_valid_next_start_is_refused` | A capacity-overflow panic or an end-marker-only split | implementation |

## Approach

Order, smallest first, each step with its RED recorded before its GREEN.

1. **Fixtures first.** AC1 to AC7 and AC13 are written against hand bytes and seen red on the base tree. AC8 to AC12 cannot compile against today's API, so their semantic RED is the base tree's `BoundaryUnavailable`, or the ignored Extended table, measured with the same bytes through today's `from_basic`.
2. **The dicom-object fix**, by the confirmed route. For route (b): vendor the published 0.10.0 package byte for byte, record its inventory, then add `first = false;` as the last statement of the `ItemEnd` arm of `build_encapsulated_data`. This is the one-line change measured above. AC1 to AC7 go green.
3. **`FrameDelimiter` and the marker rule** in `frame_index.rs`, inside the empty-table branch of `from_basic` where it returns `BoundaryUnavailable` today. AC8 to AC11 and AC13.
4. **`from_parsed`**, reading Pixel Data, its two tables and the transfer syntax, then dispatching to `from_extended` or `from_basic`. AC12.
5. **Consumers.** `preview.rs:1537` to `:1541` and `produce.rs:696` to `:715` replace their table and Fragment reads with `from_parsed`. AC7.
6. **Corpus.** AC14 and AC15, then the oracle run, AC22.
7. **Provenance, pins and D-18.** AC17 and AC18.
8. **Size and targets.** AC19 and AC20.

Rejected: decoding frame boundaries inside each decoder. The decoder receives one frame by contract (HLD §21, `Decoder::decode` takes one frame's bytes), and the index is the boundary owner today. Rejected: marker checks on the equal-count path, item 2. Rejected: the dicom-object quirk registry, item 8.

## Route, for the lead to confirm

The operator chose "fix in S46" without naming a route. Three were weighed.

**(a) Work around it in `ocelli-dicom`.** Ocelli would read the encapsulated Items itself. The collector offers `read_dataset_up_to_pixeldata`, then `read_basic_offset_table` and `read_next_fragment`, and both treat a zero-length Item correctly (`collector.rs:937`, `:1030`). Ocelli's preflight already walks every Item with its position. Against it:

- **It cannot fix the icon case.** A nested pixel sequence is collected by `collect_sequence` through the same `build_encapsulated_data`, so AC5 stays red unless Ocelli also reimplements sequence collection.
- Ocelli would have to resume collection after the Pixel Data for the trailing elements and the completion sentinel it appends (`parse.rs:230`). The collector documents no resume after `InPixelData`, so that is unmeasured.
- It adds a second reader of encapsulated Items beside dicom-rs's, which is the "exactly once" defect class.
- Its cost is real code in `parse.rs`, a file F-234 (pending, E3.9) also changes.

**(b) Vendor and patch dicom-object 0.10.0**, as the repository does for `ritk-codecs` (D-20, D-31) and `openjph-core` (D-22). One line in one function, measured above to fix every top-level and nested case with no test regression. It costs a 432 KB vendored package, a pins-gate section, a `PATCH-PROVENANCE.md`, and a D-18 amendment. Its compatibility cost: a path dependency is replaced by the registry version on `cargo publish`, so a published `ocelli-dicom` would lose the patch. Publication is blocked by D-22 today, and `ocelli-codec` already carries the same property through `ritk-codecs`. D-18's amendment states it, and `/release` must not publish `ocelli-dicom` until route (c) lands or the patch is upstream.

**(c) Upgrade dicom-object.** Not available. 0.10.1 is the newest release and upstream `master` has the same code, both measured on 2026-11-08.

**Recommendation: (b), with (c) as its exit.** It is the smallest change that makes every measured case right, it fixes the library at the one place the defect lives, and the repository already has the machinery to hold a recorded patch. The exit: when a dicom-object release carries the fix, a follow-up story removes the vendor directory, returns the workspace dependency to the registry, and drops the D-18 amendment. Optionally, and as an operator decision because it publishes outside the repository: report the defect upstream with the one-line fix and an empty-table test, which would make (c) arrive sooner. The plan does not depend on it.

If the lead chooses (a) instead, AC5 becomes a named gap, AC17 and AC18 drop, `crates/ocelli-dicom/src/parse.rs` joins the write set, and F-234's ordering against this story must be set. Every other row stands.

## Dependencies, write ownership and resources

Declared dependency F-019, done. Source-level overlaps on `sprint/s46`:

- F-243 (`work/f-243-xai`) and F-239's codec part (`work/f-239-codec`) both change `scripts/pin_and_size_check.py`, `scripts/corpus_synth.py`, `corpus/manifest.tsv`, `corpus/decode-digests.tsv`, `docs/lld/corpus.md` and `docs/hld/DEVIATIONS.md`. F-239 also changes `crates/ocelli-dicom/tests/corpus.rs`. This story touches none of their rows or functions. It adds a `check_dicom_object_vendor` section beside theirs, new syntax rows, and an amendment of D-18, a row neither touches. Whichever lands later merges as a union by three-way merge, never a hand-picked file list. The pins gate's provenance digest constants are per vendored crate, so no shared constant is recomputed.
- F-234 (pending, E3.9) changes `parse.rs`. Route (b) does not touch `parse.rs`.
- `examples/folder-viewer/src/preview.rs` is shared with most S46 folder-viewer stories. This story changes only `decode_encapsulated`.

Files this story writes, for route (b):

- `vendor/dicom-object-0.10.0/` (new), with `PACKAGE-INVENTORY.sha256`, `PATCH-PROVENANCE.md` and the upstream licence files
- `Cargo.toml` (workspace dependency and `exclude`), `Cargo.lock`
- `crates/ocelli-dicom/src/frame_index.rs`, `src/lib.rs` (exports)
- `crates/ocelli-dicom/tests/part10.rs`, `tests/frame_index.rs`, `tests/corpus.rs`
- `examples/folder-viewer/src/preview.rs`, `examples/folder-viewer/tests/preview.rs`
- `tools/oracle/src/produce.rs`
- `scripts/pin_and_size_check.py`, `scripts/tests/test_pin_and_size_check.py`, `scripts/unsafe_allowlist_check.py`, `scripts/guards/catalogue.py`
- `scripts/corpus_synth.py`, `scripts/tests/test_corpus_synth.py`, `corpus/manifest.tsv`, `corpus/decode-digests.tsv`, `corpus/README.md` if it lists rows
- `tools/oracle/compare-expectations.json` and any oracle census file the run changes
- `docs/hld/DEVIATIONS.md` D-18
- `docs/lld/dicom-ingest.md`, `docs/lld/corpus.md`, `docs/lld/oracle.md` if it names the producer's index call
- `README.md`, `examples/folder-viewer/src/use.html`, `examples/folder-viewer/src/more.html`

Exclusive resources: corpus regeneration and the oracle run need the ignored corpus and the operator's GPU host. Nothing else needs a GPU.

## Verification and evidence reuse

Focused: `bin/ocelli.sh check ocelli-dicom`, `cargo test -p ocelli-dicom`, `cargo test -p ocelli-folder-viewer --test preview`, `cargo test -p ocelli-oracle`, `bin/ocelli.sh gate pins`, `gate unsafe`, `gate guards`, `gate corpus`, `gate corpus-tests`, `gate native`, `gate wasm`, `gate licences`, `gate provenance`, `python3 scripts/deviation_check.py`, `python3 scripts/prose_check.py`. Then one feature profile run for the story, and `gate --sprint` once at sprint end, per the standing rule.

Evidence identities: dicom-object 0.10.0 archive digest `0504b879...6b85` and commit `39603cef`, pydicom 3.0.2 as the corpus decode-digest reader, each syntax producer's recorded version, the base commit each RED was measured on, and the tip each GREEN was measured on. A corpus regenerated with a different producer version invalidates AC14 and AC15's records. A missing oracle observation stays an open row and does not become a pass. Performance: the populated-table path gains one assignment per pixel sequence, and the marker rule reads at most 4 bytes at each end of each Fragment, only for multi-frame empty-table files with more Fragments than frames. No benchmark subject in `tools/bench/subjects.json` measures `parse_part10` alone. `viewer.open` and `viewer.first_image` stay inside their recorded bands where their runners exist, with `ci/bench-baseline.json` unedited.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Parsing and frame indexing run before any render loop, and the index's two vectors are allocated once per call, as today
- unsafe: none. dicom-object 0.10.0 has none, and its recorded external count is 0
- Tier A (WebGPU): n/a. Parsing and indexing run on the host CPU before any tier is involved, and they hand the same bytes to the decoder on every tier
- Tier B (WebGL2): n/a, for the same reason
- Tier C (CPU): full. The work is CPU code, and the CPU tier draws what the decoder returns

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | A.4 empty table at the top level, one, eight-byte and three Fragments, hand bytes, AC1 to AC3 | `crates/ocelli-dicom/tests/part10.rs` |
| fixture | A.4-2 populated table unchanged, AC4, and the strengthened two-byte case, AC6 | same |
| fixture | A.4 empty table in an Icon Image Sequence item, AC5 | same |
| fixture | 8.2 and A.4.1, A.4.3, A.4.4 frames spanning Fragments, marker rule, hand bytes, AC8, AC9 | `crates/ocelli-dicom/tests/frame_index.rs` |
| refusal | Unproven boundaries refused, including the false end marker, AC10 | same |
| regression | Equal counts unchanged, AC11 | same |
| fixture | PS3.3 C.7.6.3.1.8 Extended Offset Table through `from_parsed`, hand offsets, and its refusals, AC12 | same |
| unit | The delimiter table against A.4.1 to A.4.12, AC13 | same |
| fixture | PS3.5 G.2 and G.3.1 hand RLE frame under an empty table, through the production preview, AC7 | `examples/folder-viewer/tests/preview.rs` |
| corpus | 14 single-frame rows equal to their sources, AC14, and 5 shape rows frame by frame, AC15 | `crates/ocelli-dicom/tests/corpus.rs`, `scripts/tests/test_corpus_synth.py` |
| guard | Planted byte in the vendored package, AC17 | `scripts/tests/test_pin_and_size_check.py`, guard catalogue |
| golden | The new rows through cornerstone3D 5.8.2, AC22 | `tools/oracle/` |

No pixel arithmetic changes, so no LUT fixture is owed. The pixel claim is byte identity with an independently decoded source, AC7, AC14 and AC15.

## cornerstone3D 5.8.2 is the floor

Read from `tools/oracle/node_modules`, `@cornerstonejs/dicom-image-loader` 5.8.2 and `dicom-parser` 1.8.21, both MIT. dicom-parser reads the table Item by its length, so an empty table is `[]` and every later Item a Fragment (`dicomParser.js:1711` onwards, `findEndOfEncapsulatedElement`). `getEncapsulatedImageFrame.js` uses a populated table when there is one. Otherwise, when Number of Frames differs from the Fragment count, which includes a single-frame file with no Number of Frames, it builds a table with `createJPEGBasicOffsetTable` (`dicomParser.js:3457`). That function starts a frame at Fragment 0 and ends each frame at the first Fragment whose last 2 or 3 bytes hold `FF D9`. When the counts are equal it takes Fragment `i` for frame `i`. It never reads the Extended Offset Table.

| Behaviour | cornerstone3D 5.8.2 | Ocelli today | Ocelli after this story | Evidence |
|-----------|---------------------|--------------|-------------------------|----------|
| Empty table, one frame, one or several Fragments, any syntax | opens, all Fragments | refused, `NoFragments`, `OffsetCount` or `InvalidDataSet` | opens, equal to the populated-table decode | AC1 to AC3, AC7, AC14, AC15 |
| Empty table, several frames, one Fragment each | Fragment `i` | refused | Fragment `i` | AC11, AC15 |
| Extended Offset Table, empty Basic table | ignores it, Fragment `i` | refused | opens through `from_extended`, offsets and lengths checked | AC12, AC15 |
| Frames spanning Fragments, JPEG, JPEG-LS, JPEG 2000 | splits at end markers | refused | splits at proven start and end markers, count checked | AC8, AC9, AC15 |
| Same, HTJ2K, non-conformant under A.4.4 | splits at end markers | refused | splits at proven markers, item 3 | AC9, AC13 |
| An `FF D9` at a Fragment end that is not the frame's end | wrong bytes to the decoder | refused | refused, `BoundaryUnavailable` | AC10 (iii) |
| Frames spanning Fragments, RLE, non-conformant under A.4.2 | frame 0 from every Fragment, later frames throw | refused | **named departure on non-conformant input.** Refused, `BoundaryUnavailable` | AC8, AC13 |
| Same, JPEG XL | cannot decode JPEG XL | refused | refused, `BoundaryUnavailable` | AC13 |
| Extended table that disagrees with the Fragments, or beside a populated Basic table | ignored | refused or opened by the Basic table | **named departure on non-conformant input.** Refused by name | AC12 |
| Compressed icon with an empty table | main image opens | whole file refused when the icon Fragment is 2 mod 4 | opens | AC5 |
| Better than the floor | n/a | n/a | boundaries proven by both markers and the frame count, a present Extended table checked rather than ignored | AC10, AC12 |

## Parity surface covered

`docs/hld/B-parity-surface.md:28`, "Transfer syntaxes": every encapsulated syntax the registry decodes now opens with an empty Basic Offset Table, single-frame and multi-frame, as cornerstone3D's dicom-image-loader does, measured against corpus rows under the oracle in AC22. RLE and JPEG XL frames split across Fragments remain refused and named, which is non-conformant input.

## Deviations

- **D-18**, amended by this story under route (b), AC18. Its statement that the 0.10 component crates are used directly gains that `dicom-object` is the published 0.10.0 package vendored with one recorded patch under F-244, enforced by the pins gate, and that a published `ocelli-dicom` would lose the patch, so publication waits for an upstream fix.

No new deviation row. The next register number is already reserved by an unintegrated row on `work/f-238-xai`, so a new row here would collide, and the change is a qualification of D-18's own subject. The row is amended in the implementing change, not now, because writing that the crate is patched before it is would make the register false the other way.

## LLD impact

- `docs/lld/dicom-ingest.md`: "Dependencies and targets" (`:338` onwards) records the vendored dicom-object, its one patch and its exit. The `EncapsulatedFrameIndex` paragraphs (`:291` to `:309`) gain the marker rule, the delimiter table, `from_parsed` and its Extended-table preference, and the three new refusals. "Verification" gains the empty-table and icon fixtures.
- `docs/lld/corpus.md`: the 19 new rows and why they exist, and the "No encapsulation edge cases" limitation at `:367` to `:370` is removed or narrowed to what stays untested.
- `docs/lld/oracle.md`, if it names the producer's index call: the producer now calls `from_parsed`.

## User-facing documentation impact

- `README.md:178` to `:187`, the folder viewer paragraph: compressed files with an empty Basic Offset Table open, including multi-frame files whose frames span Fragments under JPEG, JPEG-LS and JPEG 2000.
- `examples/folder-viewer/src/use.html`, chapter 7, "Rust, parse" (`:563` onwards): one paragraph that an empty Basic Offset Table is read per PS3.5 A.4, and that `EncapsulatedFrameIndex::from_parsed` builds a frame index from the Extended Offset Table when present, from the Basic table otherwise, splitting spanning frames at their codestream markers and refusing what it cannot prove as `FrameIndexError::BoundaryUnavailable`.
- `examples/folder-viewer/src/more.html:394`, the transfer syntaxes paragraph: one sentence on the empty table and the frame-boundary rule.

## Decided by the operator on 2026-11-08

1. The empty Basic Offset Table finding is fixed in S46, not S47.
2. The answer is route-agnostic, "fix in S46". The route is the lead's to confirm, above.

Lead choices in this plan, each open to the operator's veto at review without changing another row: the marker rule for frames spanning Fragments (item 2), HTJ2K read leniently (item 3), the Extended Offset Table applied and its non-conformant forms refused (item 5).

## Open questions

None that block approval. The route is the lead's confirmation, with a recommendation and the consequence of each alternative stated. AC16 is a reviewer row. AC22 is a measurement on the operator's machine with its procedure named, not an open decision.

### Lead confirmation of the route, 2026-11-08

Route (b) is confirmed: dicom-object 0.10.0 is vendored with the one-line patch in the `ItemEnd` arm of `collector.rs` and a PATCH-PROVENANCE file and a pins-gate section, as `ritk-codecs` is, with D-18 amended to say so and to name the publication caveat. Route (c), an upgrade, replaces the vendored copy once a published release carries the fix. The marker-scan rule for more Fragments than frames, and the lenient HTJ2K split, stand as the plan states them. Reporting the defect upstream publishes outside the repository, so it is the operator's call and is put to the operator at sprint close.

### Lead rulings after review pass 1, 2026-11-08

- **D1.** A legal even Fragment boundary may divide a codec start or end marker. The marker rule reads bounded logical prefixes and suffixes across adjacent Fragment Values, including a final Item pad, without joining a whole frame just to inspect markers. The exact frame count and false-boundary checks remain. The earlier single-Fragment prefix description is corrected in this change. Permanent cases cover two-byte Fragments and split SOI, SOC and EOI through the checked index and the parsed-file caller, with populated-table controls.
- **D2.** A frame count larger than the number of Fragments is impossible and is refused before any reservation based on that count. The public checked constructor returns a typed error for the review's usize::MAX trigger. Other allocations on the changed path are checked for the same input-driven failure.
- No tolerance, transfer-syntax policy, dependency or gate is changed. AC22 remains the oracle observation on the final integrated corpus.


### Lead ruling, independent metadata through the EOT prefix, 2026-11-09

The current production candidate omitted the EOT multiframe row because the independent reference attribute reader failed, not because a native frame-index refusal was established. The reference rendered the row but reported null attributes. Installed MIT dicom-parser 1.8.21 misreads OV's long length and overruns while traversing the Extended Offset Table. An isolated observation of only this generated fixture confirms that full parsing and stopping at Pixel Data fail, while stopping at the EOT tag preserves its image metadata.

The lead adds `tools/oracle/page/app.mjs` and `tools/oracle/metadata-truth.json` to this fix's write set. The independent snapshot intentionally stops when it encounters (7FE0,0001), using the parser's supported exact `untilTag` option. Every queried tag, including shared and per-frame functional groups, precedes it under PS3.5 2026d section 7.1's ordering. The snapshot does not use the sentinel element or its value. Files without that exact tag retain the existing full parse. Earlier parse errors still return no attributes. No partial exception dataset, Ocelli metadata substitute, dependency patch or generic OV support is introduced.

The permanent metadata truth for this generated row is derived from `SYNTAX_ROWS`, `SYNTAX_COLS`, `syntax_base`, `multiframe_base` and `MULTIFRAME_FRAMES` in the existing generator, with PS3.3 Image Pixel, Multi-frame and Functional Groups citations. It requires three frames, the declared pixel fields and shared rescale/window provenance. Existing per-frame metadata controls remain. The old reference must fail that actual truth consumer. The corrected source requires independent review, fresh reference rendering and a production candidate comparison with no absent view. Before/after reference frame digests must be checked, with any movement investigated. This changes neither pixel arithmetic nor a tolerance, coverage classification, parser refusal policy or baseline.
