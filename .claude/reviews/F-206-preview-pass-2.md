# F-206 review, pass 2

**Reviewed**: working tree on sprint/s45 at cac65d0 plus uncommitted F-206 files, after the pass 1 fixes
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 0 smells, 2 nitpicks

## Acceptance challenged

- AC1 to AC4 and AC6 to AC8 still hold. The fixes did not touch the phantom pixels, the preset centre and width, or the LUT call. The pass 1 hand-computed greys still apply to the tests unchanged.
- AC5 is still not observable from a read. Only the author's report says the intercept mutation went red.
- AC6 now covers the DS spellings pass 1 named. `spacing_keeps_row_then_column` adds `00.5`, `-.5`, `5E-1` beside imager `0.6`, and `1\2\3` beside imager `0.6`.
- The design state table row for a bad token now matches the code and the test. Both say the window error.

## Evidence

- Read in full: the design plan, `synthetic.rs`, `preview.rs`, `serve.rs`, `lib.rs`, `tests/preview.rs` and pass 1.
- Read in part: `WindowPreset` and `apply_preset` in `presentation.rs`, and `parse_decimal_string`. A search of `page.html` found no spacing consumer. F-207 has not consumed spacing yet.
- No test, clippy or diff was run. The host's report that tests and clippy pass is unverified here.

## Pass 1 findings

| ID | Status | Basis |
|----|--------|-------|
| D1 | Closed | `clean_decimal` keeps the trimmed text only when `json_number_token` accepts it. Otherwise it writes the parsed value through `Display`. `00.5` becomes `0.5`, `-.5` becomes `-0.5`, and `5E-1` is kept. `spacing_pair` returns `Unusable` when the value is not a pair. `frame_facts` reads the imager pair only when Pixel Spacing is absent. |
| D2 | Closed | A bad preset token uses `window_unusable`. The test asserts `X-Ocelli-Error: window`. A repeated preset header stays a header error through `once`. |
| S1 | Closed | `preview_frame` refuses a preset combined with a centre, a width or a function before parsing. The test covers the function case. |
| S2 | Closed | `preview_frame` builds one metadata set and one multiframe view and passes them to both `lut_chain` and `frame_facts`. Inspect builds its own pair because it is a separate request. |
| N1 | Closed | Study ID (0020,0010) is VR `SH`. |
| N2 | Closed | `element_text` returns `None` for an unknown tag. |
| N3 | Closed | `direction_letter` has no zero branch. `unit_cosine` still guards it. |

## Defects

None.

## Smells

None.

## Nitpicks

### N4

The `FrameFacts` doc still says spacing tokens are the file's decimal strings. After D1 that is not always true. `00.5` is reported as `0.5` and `-.5` as `-0.5`. The doc should say the token is kept when it is already a JSON number, and the parsed value is written otherwise.

### N5

The comment that describes the bad-token request with an empty body sits above the preset-plus-function block, not above that request. It also says "refused from the header", which now reads like the old header error code.

## Verified clean

- `json_number_token` follows the JSON number grammar. It allows an optional minus, then `0` or a non-zero digit run, then an optional fraction, then an optional exponent. It refuses a leading plus, a leading dot, a padded zero and a trailing dot.
- `parse_decimal_string` refuses non-finite values, so the fallback does not render `inf` or `NaN`.
- A spacing token in a response header cannot inject a new header line.
- A Modality LUT Sequence is still refused first. A preset still refuses a VOI LUT Sequence. A numeric override still replaces a VOI LUT Sequence.
- Synthetic VRs match the dictionary for every tag `slice` writes.

## Limitations and handoff

- No shell, so the diff against cac65d0 was not seen and the tests were not re-run.
- N4 and N5 do not block. With no open defect and no open smell, pass 2 is clean under the microscope rule.
- A negative spacing such as `-.5` is reported as Pixel Spacing. F-207 should decide whether a zero or negative spacing is drawn as a scale before it uses the field for length.

COUNTS defects=0 smells=0 nitpicks=2
