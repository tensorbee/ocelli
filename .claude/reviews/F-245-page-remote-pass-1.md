# F-245 page remote slice review, pass 1

**Reviewed**: unstaged worker files in /private/tmp/ocelli-s47-f245-glm on branch state at review time.
- `examples/folder-viewer/src/page.html` sha256 e4df50d30cee01a8ca2158b6cf6c27655850c088c6fe073b4835336ef4df1137 before and after the pass, diffed against baseline `claude-page-before-remote-a.html` sha256 8de74678523d68b8ee32671229e34de0c57ad1a689bc9ece4ef03904c69107d7.
- `examples/folder-viewer/tests/preview.rs` sha256 c7a741e95e1a6f39ccd185b653b4ee9b0ec4d6a84e54849c4439a30561e63063 before and after, only `the_page_offers_a_hidden_dicomweb_source`, diffed against `claude-preview-before-remote-a.rs`.
- `examples/folder-viewer/tests/browser/dicomweb.mjs` sha256 1ca5b975fa69388e1f5b32767d393b738907c38891608a2a99f0629aca4f9278 before and after, whole file.

**Author / reviewer**: author GLM. Reviewer Claude (root independent reviewer, subagent). The reviewer wrote none of the reviewed material. No worker file was edited. Mutants ran on copies under /private/tmp/claude-501/f245r only.

**Result**: 4 defects, 2 smells, 2 nitpicks.

## Acceptance challenged

- Plan line 119 and line 190: remote state fully separate from the displayed local state, and the fixed product text claims the local image is unchanged. Challenged by measuring the local viewport while the remote panel is open, which the browser suite never observes.
- AC2 live chooser requests: upstream request log and native POST counts in the suite.
- AC3 local preservation: page mutants that relabel `#status`, call `resetChecks()`, or call `dropCache()` on valid staging.
- Refusal attribution: a fixture mutant where instance B is malformed bytes instead of an identity mismatch.
- Races: rapid clicks across studies, series and instances, stale reply after a newer one, what `Aborted` means, and refusal against success ordering.
- Existing segmented controls under the CSS change, with the third source button hidden (the default local-only launch).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `CARGO_BUILD_JOBS=2 cargo test -p ocelli-folder-viewer` | worker tree, hashes above | executed now | exit 0, every binary `ok`, preview 138 passed including `the_page_offers_a_hidden_dicomweb_source` |
| `node examples/folder-viewer/tests/browser/dicomweb.mjs` | existing release binary, Chromium via tools/bench Playwright | executed now | exit 0, `ok`, 15 s |
| `css-probe.mjs` static computed style, baseline against current | the two page files above | executed now | exit 0. Baseline Folder `0px 6px 6px 0px`, current Folder `0px`. Size row M and L baseline `0px 6px 6px 0px`, current `6px` |
| `layout-probe.mjs` static, `#remote` shown with 1, 5, 20, 60 rows | current page, 1280x800 | executed now | exit 0. `#wrap` 848x609 hidden, 848x547 at 1 row, 848x87 at 20, 848x0 at 60. `#remote` is 196 px wide at x 0, scrollWidth about 482 |
| `suite.mjs` copy, `MUT=none` | real viewer, real upstream | executed now | `ok`. `#wrap` 848x529 before opening the panel, 848x323 after valid staging, canvas backing 128x96 both times. Refusal text `The DICOMweb request was refused. Code InvalidValue.` |
| `suite.mjs` copy, `MUT=malformedB` (instance B truncated to 150 bytes, correct UID) | as above | executed now | `ok`, identical refusal text `Code InvalidValue`. Survives |
| `suite.mjs` copy, page mutants through `page.route` with CSP removed and Chromium local network checks disabled, control `MUT=same` | as above | executed now | control `ok`. `relabel` killed by `the fetched instance changed the local image`. `checks` (`resetChecks()`) survives. `cache` (`dropCache()`) survives |
| coordinator `claude-browser-suite-mutants.json`, run logs v1 to v6 | coordinator evidence | prior receipt | relabel mutant killed (by timeout in that variant), native UID mutant survives the browser suite and is killed by Rust `f245_requested_tuple_mismatches_then_valid_retry`. v6 `ok` |

## Defects

### D1, the remote panel takes height from the local image viewport

**Where**: `examples/folder-viewer/src/page.html:108-111` and `:748-752`, the `#remote` section with no grid placement in a body laid out by `grid-template-rows: 44px auto minmax(0, 1fr) 40px 28px` at `:35-41`.
**What**: every explicit cell is occupied, so `#remote` is auto-placed into an implicit sixth row in column 1, 196 px wide under the series panel. The implicit row takes height from the `minmax(0, 1fr)` row that holds `#wrap`. In the real viewer the image area shrinks from 848x529 to 848x323 with one row at each level. Statically, 60 rows collapse `#wrap` to height 0. 64-character UIDs overflow the 196 px column (scrollWidth about 482).
**Why it is wrong**: plan line 119 requires remote state fully separate from the active displayed local state, and the fixed text of line 190 tells the user the local image is unchanged while the displayed local image has just been re-fitted into a smaller viewport, or hidden entirely by a long study list.
**Evidence**: `layout-probe.mjs` and the `MUT=none` suite copy above. The suite's `sameLocal` hashes the 128x96 canvas backing store, which is unaffected by viewport size, so it passes.
**Scope and regression**: present in the worker tree. Smallest regression: record the `#wrap` rect before opening the panel and after each remote stage and require equality, with enough rows to exceed the panel's natural height.

### D2, the CSS selector change alters two existing controls

**Where**: `examples/folder-viewer/src/page.html:101-102` and `:455-456`.
**What**: `:last-child` counts the hidden `#source-dicomweb`, so in the default local-only launch Folder is not the last child and loses its right corner radius (`0px 6px 6px 0px` to `0px`). The inspector size row (`#size-row`, four buttons) also changes: M and L go from `0px 6px 6px 0px` to `6px`, with `border-left: 0` still applied.
**Why it is wrong**: the slice may add a source, but the existing local controls are to be unchanged, and the default configuration has no remote service at all.
**Evidence**: `css-probe.mjs` baseline against current, above.
**Scope and regression**: every launch without the two environment values. Smallest regression: a computed-style assertion on `#source-folder` with the third button hidden, and on `#size-row`. A selector that skips hidden buttons, or a rule keyed to the visible last button, would keep both.

### D3, the browser suite header makes two false claims

**Where**: `examples/folder-viewer/tests/browser/dicomweb.mjs:4-5` and `:11`.
**What**: lines 4-5 say the refused instance's stored SOP class contradicts the study. The fixture at `:262-263` gives both instances the same SOP class `1.2.840.10008.5.1.4.1.1.7`. Instance B differs in SOP Instance UID (`2.25.245099` stored against `2.25.245004` requested). Line 11 says `bin/ocelli.sh gate viewer-browser` runs it after aspect.mjs. `bin/ocelli.sh:69` and the `viewer-browser` case list no `dicomweb.mjs`.
**Why it is wrong**: a factual sentence in a test header is a checkable claim, and both are false today. Registration is owned by a pending shared slice, which makes the present tense wrong rather than the intent.
**Evidence**: reading the fixture and `grep -n 'dicomweb.mjs' bin/ocelli.sh` (no match).
**Scope and regression**: prose only. Delete or correct the sentences.

### D4, the refusal assertion does not attribute the refusal to identity

**Where**: `examples/folder-viewer/tests/browser/dicomweb.mjs:292-295`.
**What**: the suite waits only for the prefix `The DICOMweb request was refused.`. Replacing instance B with truncated, malformed bytes carrying the correct UID gives the same text, `Code InvalidValue`, and the suite prints `ok`. It also never asserts that the upstream received the instance B request.
**Why it is wrong**: the header claims the refusal comes from the native identity check, and the review scope requires the refusal be attributed to identity rather than malformed bytes. A test that passes both ways is not coverage of that claim.
**Evidence**: `MUT=malformedB` survives, above.
**Scope and regression**: test only. The page shows only the code, and both causes map to `InvalidValue`, so the browser cannot discriminate from the panel text alone. A discriminating check needs a distinguishable observable, for example a native or provider refusal reason exposed to the test, or the browser half delegated explicitly to the Rust unit `f245_requested_tuple_mismatches_then_valid_retry` with the header saying so.

## Smells

### S1, two providers can be constructed

**Where**: `examples/folder-viewer/src/page.html:1445-1455`.
**What**: `remoteProviderFor` tests `remoteProvider` before `await loadLibrary()` and assigns after it. If the DICOMweb button is clicked, then clicked twice more (close and reopen) before the library import settles, the list is still empty so `remoteOpenStudies` runs twice and two `DicomwebProvider` instances are built. Each has its own generation, so the provider's stale guard no longer orders the two studies replies, and the later-settling one wins the panel text.
**Why it matters**: plan line 115 requires ONE persistent provider, and the race guarantees in the page rest on that single generation counter.
**Evidence**: code-evident, not reproduced (needs a slow `/lib/ocelli-core/index.js`). Memoising the construction promise rather than the instance closes it.

### S2, the DICOMweb button's active state reads as a second selected source

**Where**: `examples/folder-viewer/src/page.html:1553` against `markSource` at `:2100-2103`.
**What**: the button toggles `.active` as a panel-open state inside the single-choice `#source` segment. `markSource` never clears it, so Synthetic study and DICOMweb are both shown selected while local pixels are displayed, and choosing Folder leaves DICOMweb lit. There is no `aria-expanded` or `aria-controls` for what is a disclosure.
**Why it matters**: AC7 requires that the new source UI cannot relabel old local pixels, and a lit DICOMweb segment over a local image is the first step of that. Keyboard parity is fine, it is a native button with a tooltip through the generic `SENTENCE` lookup.

## Nitpicks

- N1, `examples/folder-viewer/tests/preview.rs:815-826` checks string presence only. `probeRemoteSource` present does not prove it is called or that the button unhides. The browser suite covers the behaviour.
- N2, `examples/folder-viewer/src/page.html:1498`: `remoteProviderFor()` sits outside the `try`, so an `InvalidOptions` from the provider constructor (a config the native side should never serve) becomes an unhandled rejection and a blank panel rather than the refusal sentence.

## Verified clean

- Isolation: the page diff writes no `#status`, calls none of `readFolder`, `resetFolderState`, `resetChecks`, `dropCache`, `markSource`, and touches no `sourceMode`, marks, notes, key images or viewports. The only shared call is `loadLibrary`, whose missing-library side effect predates the slice.
- Fixed text at the success path matches plan line 190 in substance, and no story ID appears in product copy or the tooltip.
- No `innerHTML`. Rows and labels go through `textContent` and `replaceChildren`. Row UIDs are additionally validated by `validUid` in the provider before the page sees them (`dicomweb-viewer.ts:236`, `:273`, `:327`), so a hostile upstream string cannot reach a button.
- Error copy is a fixed sentence plus `error.code` from a closed set (Aborted, AllocationRefused, CommitRefused, ContentType, InvalidOptions, InvalidRequest, InvalidValue, Network, HttpStatus). No message, URL or UID is shown.
- Races: the provider advances one global generation per intent across QIDO and WADO, checks freshness after every await and immediately before `resolve`, and rejects stale work as `Aborted` in its catch, which the page drops silently. The page's continuations run in microtasks with no await between the provider's last check and the DOM write, so no click can interleave. A newer refusal cannot be overwritten by an older success or the reverse within one provider. The pre-intent `InvalidRequest` checks cannot be reached from page rows because rows are validated upstream of the page.
- Probe: non-200, a JSON parse failure, arrays, extra or missing keys, and empty strings all leave the button hidden. Exactly two string keys are accepted and frozen.
- AC2: the suite asserts studies, series, instance list and instance A upstream GETs and at least 3 QIDO and 2 instance native POSTs.
- AC3 partial: `#status`, series count and canvas backing hash are compared after refusal and after valid staging, and the relabel mutant is killed by that comparison.

## Limitations and handoff

- Open coverage. AC3 pending stage is never sampled, and marks, SR documents, F225 checks and cache are not observed (`resetChecks()` and `dropCache()` mutants survive). AC6 tier independence has no browser row. AC7 More matrix row and README, Use and More docs are not in this slice. Registration under `viewer-browser` in `bin/ocelli.sh` is owned by another story's shared slice and pending. Rapid-click races are verified by reading, not by a browser test.
- The page mutants ran with the CSP header removed and Chromium local network checks disabled, because a routed document is otherwise refused loopback access. The control mutant passed under the same conditions.
- S1 is not reproduced.
- Next owner: GLM remediates D1 to D4 and S1, S2. A fresh independent pass reviews the changed material.
