# F-234, Bounded sequence depth in parse_part10

**Status**: approved
**Epic ref**: E3.9
**Sprint**: S46
**Estimate**: 1w

Story row, `docs/sprints/BACKLOG.md`: "Bounded sequence depth in parse_part10: the structural preflight refuses sequence nesting deeper than a named constant with a typed ParseError before dicom-rs recurses, with tests at the limit and one past on a 2 MiB thread and a fuzz-shaped regression". Layer Rust, depends on F-225 and F-229, milestone M20.

`allocation.json` notes: "Found by F-225's first review while fuzzing. Answered by the operator on 2026-11-06: fixed in S46 with a typed error."

## Normative source, transcribed

### HLD 23, `docs/hld/20-errors-and-panics.md`

Verbatim. Both are set as one-cell tables here so the author's punctuation stays, and the second is a one-cell table in the HLD as well.

|  |
|----|
| thiserror in the core crates; the boundary maps everything to a stable numeric code and a message. |

|  |
|----|
| **A PANIC POISONS THE INSTANCE** Once a Rust panic aborts inside WebAssembly, that module instance's memory may be inconsistent and it must not be reused. So: panic = "abort" in release; no exported function may let a panic escape as a normal error path; and on panic the worker instance is torn down and rebuilt, with the JavaScript shell reconstructing viewport state from its own copy. |

`Cargo.toml` lines 183 to 188 set `panic = "abort"` under `[profile.release]`, the only profile that sets it.

### DICOM

PS3.5 section 7.5, Nesting of Data Sets, defines a Sequence of Items whose Items are themselves Data Sets that may contain further sequences. This plan found no depth limit in the standard, and the HLD sets none. The limit is therefore this plan's decision.

### The operator's answer, `docs/sprints/CURRENT_SPRINT.md`

"Found by F-225's first review while fuzzing, before the conformance checker runs, so it is older than F-225. With `panic = "abort"` a stack overflow ends the process, so one crafted file can stop the folder viewer's local process. ... **Answered by the operator on 2026-11-06:** fixed in S46. A new story bounds sequence depth in the parser and refuses deeper input with a typed error."

### The finding, `../ocelli-wt-f-225/.claude/reviews/F-225-working-pass-1.md` line 18

"500-level nesting, 20,000 items, 20,001 DS values and a lone-NULL SOP class: no panic, and the output is deterministic. ... At 1,000 levels the debug build overflows the stack inside `parse_part10`, before the checker runs. In release, `parse_part10` succeeds at 3,000 levels and so does the checker. The overflow belongs to the parser, which this story does not change."

## Facts measured for this plan

Read on 2026-11-06 on `sprint/s46` at `1ba258e`.

1. `pub fn parse_part10(bytes: &[u8]) -> Result<ParsedDicom, ParseError>` is at `crates/ocelli-dicom/src/parse.rs:201`. It reads the file meta, adapts the data set, calls `validate_data_set_structure(&data_set, transfer_syntax)?` at line 229, and only then collects through dicom-rs `DicomCollectorOptions::new()...from_reader(...)` and `read_dataset_to_end` at lines 232 to 240.
2. **The preflight does not recurse.** `validate_data_set_structure` (line 349) drives `LazyDataSetReader` in one `loop` and keeps open containers in a heap `Vec`, `let mut containers = Vec::new();`, pushing `DataSetContainer::Sequence` on `SequenceStart`, `Item` on `ItemStart` inside a sequence, `PixelSequence` and `PixelItem` for encapsulated pixel data, and popping on the matching end. So the depth is known there before any recursion.
3. **The recursion is in dicom-rs 0.10.0**, which `Cargo.lock` pins for `dicom-object` and `dicom-parser`. In `dicom-object-0.10.0/src/collector.rs`, `collect_to_object` (line 1075) calls `collect_elements` (1085), which on `SequenceStart` calls `collect_sequence` (1178), which on `ItemStart` calls `collect_to_object` (1289). dicom-rs has no depth option. `LazyDataSetReaderOptions` has `odd_length` and `charset_override` only.
4. `ParseError` (`parse.rs:160` to `187`) is `#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]` and `#[non_exhaustive]`, with eight variants from `TruncatedPart10` to `InvalidDataSet`. Its doc says no variant retains input bytes, values, identifiers or paths.
5. Ocelli also recurses over the parsed tree after `parse_part10` returns: `MetadataSet::from_data_set` and `from_dicom_element` in `metadata.rs` (109, 267), `project_item` and `project_elements` in `frame_open.rs` (998, 1004), and `set_has_overlay_group` in `overlay.rs` (386). F-225 adds a recursive `walk` in `conformance.rs` on its branch. All of them are bounded once the parser bounds the tree.
6. Production callers: `DicomwebSource::consume` in `crates/ocelli-dicom/src/dicomweb.rs` (177, 192), the folder viewer's native process in `examples/folder-viewer/src/preview.rs` (543, 557, 2226, 2358), and `examples/example-server`. `ocelli-wasm` does not depend on `ocelli-dicom`. The folder viewer serves requests on `std::thread::spawn` threads (`serve.rs` 173, 217).
7. `crates/ocelli-dicom/tests/part10.rs` builds synthetic Part 10 bytes with `part10(transfer_syntax, data_set)` (154), `explicit_element_le` (48), `implicit_element_le` (70) and `item_control_tag` (331). `proptest = "1"` is already a dev-dependency of `ocelli-dicom`. There is no fuzz directory and no deep-nesting fixture.
8. `examples/folder-viewer/src/use.html` line 531 lists `ParseError`'s variants "from `TruncatedPart10` and `MissingDicmPrefix` to `UnknownTransferSyntax` and `InvalidDataSet`".

## What the specification does not cover

Every item is a lead decision under the operator's delegation of 2026-11-06.

1. **Depth is the count of open `Sequence` containers** in the preflight's stack, so a top-level SQ element's items sit at depth 1. Items are not counted separately, because an item cannot open without a sequence. Encapsulated pixel data cannot nest and is not counted.
2. **The constant is `pub const MAX_SEQUENCE_DEPTH: usize` in `parse.rs`, re-exported beside `parse_part10`.** Its value follows a rule, and the implementer measures and records both inputs in the handoff. The value is a power of two at least four times the deepest nesting found in any corpus row or tracked test fixture, and `parse_part10` at that depth, followed by `MetadataSet::from_object`, succeeds in a debug test build on a thread given a 2 MiB stack. If no power of two satisfies both, the story returns to the lead before coding.
3. **The refusal is a new variant, `ParseError::SequenceTooDeep`**, message "the DICOM data set nests sequences deeper than this build reads". It carries no depth, value or tag, matching fact 4's patient-safe rule. The enum is already `#[non_exhaustive]`, so callers in other crates already have a wildcard arm.
4. **The check is in the preflight**, at the push of a `Sequence` container, so the refusal happens before dicom-rs's recursion and before any post-parse walk. No check is added to the recursive walkers in fact 5, because they only ever see a tree the parser accepted, and a second limit beside the first would be a second number to keep equal.
5. **The fuzz-shaped regression is a proptest**, not a cargo-fuzz target, because the repository has no fuzz harness and the floor has no nightly toolchain. It generates nesting depth, definite or undefined lengths per level, and explicit or implicit little endian.

## Problem and reproducer

A file whose data set is one SQ, holding one item, holding one SQ, and so on, a thousand deep, overflows the stack of a debug test thread inside dicom-rs's `collect_to_object`, per F-225 pass 1. With `panic = "abort"` in release, a deep enough file ends the folder viewer's local process. Expected after this story: `parse_part10` returns `Err(ParseError::SequenceTooDeep)` for any depth past the constant, on any thread, before recursing.

## Scope, API and compatibility

In scope: `parse.rs` (constant, variant, preflight check), the re-export in `lib.rs`, `tests/part10.rs`, `docs/lld/dicom-ingest.md`, `examples/folder-viewer/src/use.html`. `docs/lld/errors.md` does not mention `ParseError`, measured by grep, and does not change.

Non-goals: limits on item count or element count, which F-225 pass 1 found linear and harmless. A limit inside dicom-rs. The conformance checker's own walk.

Public API: one new `pub const` and one new variant of a `#[non_exhaustive]` enum. Compatible for every external caller.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `parse_part10` | `validate_data_set_structure` | every transfer syntax, after deflate adaptation | `Err(SequenceTooDeep)` past the constant |
| `DicomwebSource::consume` | `parse_part10` | WADO-URI and multipart bodies | the parse error class it already forwards |
| folder viewer `decode_held`, `inspect_json`, `headers_json` | `parse_part10` | native process threads | the file refused with the variant's message, process alive |
| `example-server` | `parse_part10` | native | the same |
| `MetadataSet`, `open_frame`, overlay and conformance walks | the accepted tree | depth at most the constant | unchanged |

## State transitions and invariants

Not applicable beyond the function. `parse_part10` is pure over its input. The invariant spanning consumers: no tree reaching a recursive walker is deeper than `MAX_SEQUENCE_DEPTH`.

## Ownership, commit and recovery

Not applicable. No resource is staged before the refusal, and the input slice is borrowed.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | At the limit. A data set nested exactly `MAX_SEQUENCE_DEPTH` deep parses to `Ok`, and `MetadataSet::from_object` over it succeeds, on a thread with a 2 MiB stack in the debug test build | `parses_at_the_sequence_depth_limit` | An off-by-one that refuses the limit itself | Implementation |
| AC2 | One past. Depth `MAX_SEQUENCE_DEPTH + 1` returns `Err(ParseError::SequenceTooDeep)` on the same thread | `refuses_one_past_the_sequence_depth_limit` | Counting items as well as sequences, which refuses at half the depth | Implementation |
| AC3 | Refused before recursion. A data set nested 100,000 deep returns `SequenceTooDeep` on a thread with a 256 KiB stack | `refuses_deep_nesting_without_recursing` | A check placed after collection or in a post-parse walk, which overflows first | Implementation |
| AC4 | Every length form. AC1 and AC2 hold for undefined-length sequences and items with delimiters, for definite lengths, and for implicit VR little endian | the same tests, parameterised | A check only on the undefined-length branch | Implementation |
| AC5 | Deflate. AC2 holds for a Deflated Explicit VR Little Endian file, whose data set is inflated before the preflight | `refuses_deep_nesting_in_a_deflated_data_set` | A check run on the encoded bytes | Implementation |
| AC6 | Fuzz-shaped regression. A proptest over depth 0 to `MAX_SEQUENCE_DEPTH + 8`, random definite or undefined lengths per level and the two little endian syntaxes, on a 2 MiB thread, asserts that the result is `Ok` exactly when the depth is at most the limit and `SequenceTooDeep` otherwise, and never panics | `sequence_depth_is_refused_exactly_past_the_limit` | A limit enforced on one encoding only | Implementation |
| AC7 | Old-source RED. On the unchanged parser, AC3 aborts the test process with a stack overflow and AC2 returns `Ok` | recorded in the handoff | Tests that pass before the fix | Implementation, recorded |
| AC8 | The variant is patient-safe. Its `Display` is the fixed message and it is `Copy` | `sequence_too_deep_message_is_fixed` | A variant carrying the depth or the tag | Implementation |
| AC9 | The constant follows decision 2. The handoff records the deepest corpus and fixture nesting measured, the value chosen and the AC1 run | handoff | A constant picked without the two inputs | Implementation, recorded |
| AC10 | Folder viewer. A file one past the limit posted to the local process is refused with the variant's message and the process answers the next request | folder viewer test in `examples/folder-viewer/tests/` | A refusal that ends the process | Implementation |
| AC11 | `cargo test -p ocelli-dicom`, `gate native` and the corpus parse tests pass, so no corpus row is refused | gates | A limit below a real file | Implementation |

## Approach

1. Start after F-225 and F-229 are integrated, because both write `parse.rs`.
2. Measure decision 2's two inputs and choose the constant.
3. Write AC1 to AC6 and AC8 against a nested data set builder added to `tests/part10.rs`, record AC7.
4. Add the constant, the variant and one check at the `Sequence` push in the preflight.
5. AC10, docs.

Rejected alternatives:

- Running the parse on a thread with a larger stack. It moves the depth at which the process aborts and leaves the abort.
- A limit in each recursive walker. Decision 4.
- A pre-scan pass separate from the preflight. The preflight already walks every token once.

## Dependencies, write ownership and resources

Declared: F-225 and F-229, both of which write `crates/ocelli-dicom/src/parse.rs` in flight. **F-234 starts after both are integrated.** F-214 writes `crates/ocelli-dicom/src/lib.rs`, and F-234 adds one re-export line there, so F-234 is based on the tree after F-214 if F-214 has landed, and the three-way merge is one line otherwise.

Write set:

```text
crates/ocelli-dicom/src/parse.rs, crates/ocelli-dicom/src/lib.rs (one re-export)
crates/ocelli-dicom/tests/part10.rs
examples/folder-viewer/tests/<one test>
docs/lld/dicom-ingest.md, examples/folder-viewer/src/use.html
```

No GPU or browser. The corpus parse tests are read for AC11 and decision 2.

## Verification and evidence reuse

```bash
cargo test -p ocelli-dicom
cargo test -p ocelli-folder-viewer
bin/ocelli.sh gate native
```

Final profile: `bin/ocelli.sh gate --sprint`. Evidence is invalidated by a change to the preflight, the constant, the dicom-rs pin, or the debug stack size the tests set.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a, parsing precedes every tier
- Tier B (WebGL2): n/a, parsing precedes every tier
- Tier C (CPU): n/a, parsing precedes every tier

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | AC1 to AC5, AC8 over hand-built nested data sets | `crates/ocelli-dicom/tests/part10.rs` |
| property | AC6 | `crates/ocelli-dicom/tests/part10.rs` |
| integration | AC10 through the folder viewer's local process | `examples/folder-viewer/tests/` |
| recorded | AC7, AC9 | handoff |

No `fixture` row is owed. No pixel or geometry arithmetic.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no parser robustness row.

## Deviations

None.

## LLD impact

`docs/lld/dicom-ingest.md`, the `parse_part10` section: the preflight's depth check, the constant and the rule that chose it.

## User-facing documentation impact

`examples/folder-viewer/src/use.html` line 531, the `ParseError` sentence, names `SequenceTooDeep` and `MAX_SEQUENCE_DEPTH`. `README.md` and `more.html` do not list parse errors and do not change.

## Open questions

None open.

- Q1, where the limit is enforced. Answered: lead decision under the operator's delegation of 2026-11-06. In the non-recursive preflight, before dicom-rs recurses, decision 4 and facts 2 and 3.
- Q2, the value. Answered: lead decision under the operator's delegation of 2026-11-06. A rule the implementer measures against, decision 2.
- Q3, the refusal. Answered: lead decision under the operator's delegation of 2026-11-06. `ParseError::SequenceTooDeep`, decision 3.
- Q4, the fuzz shape. Answered: lead decision under the operator's delegation of 2026-11-06. A proptest, decision 5.

### Lead amendments after review pass 1

Made by the sprint lead on 2026-11-06 under the operator's delegation of the quality call, after `.claude/reviews/F-234-working-pass-1.md`.

- **The depth rule counts every DICOM writer the repository has and the templates it targets.** Decision 2's inputs are the deepest nesting in the corpus, in every fixture or output any repository writer produces, Rust or TypeScript (the TID 1500 that `writeTid1500` writes included), and in the PS3.16 template shapes those writers target, each wrapped once more by de-identification's Original Attributes Sequence. The implementer measures each input and states the result, and the constant follows the plan's rule from the largest.
- **AC10 is the folder viewer's own refusal.** A deep file posted to the folder viewer answers 400 `not-part10`, as `part10_error` maps every `ParseError`, and the next request is served. The test uses a file deep enough to abort the process without the check.

### Lead ruling after review pass 2

Made by the sprint lead on 2026-11-06 under the operator's delegation of the quality call, after `.claude/reviews/F-234-working-pass-2.md`. The template shapes count every sequence a conformant file may carry at each level, Type 3 included, because a valid file that uses an optional sequence must not be refused. The deepest input is therefore the PS3.16 TID 1501 to TID 300 or 301 to TID 320 chain with its Type 1 Referenced SOP Sequence and an optional nested reference, wrapped once more by de-identification, as the review measured it, and the constant follows the plan's rule from that input.

### Lead ruling after review pass 3

Made by the sprint lead on 2026-11-07 after `.claude/reviews/F-234-working-pass-3.md`. The figure of 9 in the pass 2 ruling was the lead's error. Pass 3 derived from PS3.16 and PS3.3 a deeper route, TID 1500 to TID 1501 to TID 300 to TID 301 row 11 to TID 315 row 2, whose NUM item opens Measured Value, Measurement Units Code and the Type 3 Equivalent Code Sequence (0008,0121), so 8 open Sequences and 10 inside the de-identification wrapper.

- **The fixture builds that route.** It is the deepest shape known to the lead, and the prose says known rather than deepest, because these templates are Extensible and the deepest is not unique.
- **The margin is checked against the fixture, not a literal.** A test measures the fixture's maximum open-Sequence depth by parsing it and asserts it equals `DEEPEST_SHAPE_DEPTH`, and the compile-time margin assert stays. `MAX_SEQUENCE_DEPTH` stays 64, which the margin rule still admits.
- **Tag order.** Every item the fixture writes is in ascending tag order per PS3.5 7.1.
- **Prose.** The parse.rs doc comment, `DEEPEST_SHAPE_DEPTH`'s doc, the fixture comment and `docs/lld/dicom-ingest.md` state the route and the counts measured in this round, and the re-measured date covers only what was re-measured.
