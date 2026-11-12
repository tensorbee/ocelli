# F-205, Validated shared preferences in the single showcase viewer

**Status**: approved
**Epic ref**: E38.15
**Sprint**: S47
**Estimate**: 3w

This is the round 3 revision. It is written against `sprint/s47` at
`ac962c6490d7e4a110a67db20593a0ed29be6a30` (F-245 integrated) in the isolated
design worktree `/private/tmp/ocelli-s47-f205-design`. Every tree fact below
was checked with a command in this session and the command is named where it
matters. The estimate is the lead estimate from the backlog row, not measured
capacity. `tools/oracle/node_modules` does not exist in this worktree, so the
cornerstone3D transcription was read read-only from the main checkout's
installed package at
`/Users/atulsharma/Documents/projects/tensorbee/ocelli/tools/oracle/node_modules/@cornerstonejs/`,
which is MIT licensed and in bounds to read. Nothing under `corpus/data` was
opened, and dwv, Horos and Grok were not opened.

## Lead amendment after design round 2

The round 2 plan was reviewed by the lead with 9 defects, 10 smells and
nitpicks. What changed, per finding:

- **D1, the default reappeared after Reset.** The per-image marker model is
  gone. The stored default is now applied once per viewport per series load,
  at the reset in `loadSeries` (page.html 2520 to 2523), never per image,
  never per cine frame, never again after Reset or after the reader chose
  the file window index 0. AC10 scrolls after Reset and counts preview
  requests during cine.
- **D2, refused documents were "treated as absent".** Refused whole: one
  named sentence, the legacy keys used, the document left byte-identical
  forever, no write at load and no write on change. "Treated as absent" is
  deleted. Every access to the parsed value is guarded. AC4 covers each
  shape.
- **D3, lost updates across tabs.** `savePreference` re-reads the document
  inside the helper, changes one field, writes back, and a `storage`
  listener refreshes in-memory state and visible controls. AC15 is the
  two-page test. Per-key-only storage is weighed in rejected alternatives.
- **D4, sentences hidden with a column.** `prefs-note` moves out of the
  inspector to a new top-level paragraph beside `#sentence`, which the
  column rules at page.html 43 and 44 never hide. AC5 and AC6 assert the
  bounding box with `hideInspector` stored true.
- **D5, the no-patient-data invariant is whole-origin.** AC9 asserts
  `Object.keys(localStorage)` equals exactly the allowed key set and
  `sessionStorage` is empty, after every control, a folder open and a
  DICOMweb open. Unknown fields in a version 1 document are dropped on the
  next write, and AC9 and the schema lines now agree.
- **D6, the cornerstone3D claim was false.** The shipped 5.8.2 ESM was read
  and transcribed with citations (StatsOverlay.js 285 to 311,
  renderingCapabilities.js 3 and 50 to 81). The floor is that cornerstone
  keeps versioned, validated component stores with quiet fallback and
  leaves application preferences to the host. The phrase "cornerstone has
  no persistence" appears nowhere.
- **D7, legacy keys are rewritten on every successful document write.** The
  partial-write sentences are named, the refused-document change rule is
  named, and the migration, state and ownership sections and AC3 agree.
- **D8, F-204's 1:1 zoom mode is not stored.** It is listed under what is
  not stored with the verified reason, and F-204 joined the order section
  with its measured write overlap.
- **D9, the before-reply case was impossible.** Images enter a series only
  after the `POST /file` reply (page.html 2260 to 2264), so no image ever
  has an unknown preset list. AC11's held-reply case is dropped with that
  reason.
- **S1, the F-245 quote was one sentence blended from two.** Line 115 and
  line 264 are quoted separately, each exactly.
- **S2, the operator question is posed fairly.** Three readings of shared
  with their costs, and the BACKLOG and LLD retirement record joined the
  write set.
- **S3**, eslint.config.js joined the write set with the globals block.
- **S4**, the stored default overrides a CT file's own window at series
  load, the corner names the preset, and the apply does not propagate
  through `copyWindow`. AC16 tests it.
- **S5**, the More row keeps Partial with the named gaps and the delivery
  cell addressed.
- **S6**, a real refusal path for a stored token the process does not
  offer, sentence S7 and AC17, and AC13's named failure corrected.
- **S7**, the page.html 1053 to 1054 comment is amended in the write set.
- **S8**, AC7 uses `launchPersistentContext` with a `userDataDir` and
  relaunches the browser as well as the server. AC8's wrong implementation
  is now the plausible process-side store.
- **S9**, missing and mistyped fields in a version 1 document are refused
  per field by name with the default applied, and AC5 names which store
  each case plants into.
- **S10**, the default's behavior on the DICOMweb source and F-202's
  remote path is stated under its own heading.
- **Nitpicks.** The HLD 10 quotation is marked as boundary context and not
  a preference specification. "The previous keys are used" is replaced with
  plain user words. The new default-preset apply is flagged for F-280's
  setter list, note only. `tools/oracle/node_modules` is stated as absent
  rather than empty. `window.mjs` `observeProfileKeys` joined the consumer
  map. The call-site pins at tests/preview.rs 893 and 897 and the
  fallback-return pin at 894 are named explicitly.

## Normative source, transcribed

**No HLD section covers preferences.** `grep -rn -i 'preferen' docs/hld/`
returns nothing, checked with that command this session. The round 2 review
brief states the same. No schema, key or sentence is invented and then
attributed to the HLD.

The one HLD section quoted below is **boundary context, not a preference
specification**, and is marked as such so nobody reads it as normative for
the schema. `docs/hld/07-concurrency-and-typescript.md`, section 10, "What
stays TypeScript", two of its bullets, quoted from the file:

```text
- DOM, pointer, touch and wheel events; canvas lifecycle; ResizeObserver

- Tool interaction state machines; the geometry and hit-testing behind them is Rust, in ocelli-geom
```

A stored mouse profile, play rate, column visibility and default preset are
DOM and interaction state, so they stay in the page. Nothing in this story
crosses the wasm boundary.

`docs/hld/DEVIATIONS.md`, the D-07 closing rule, quoted from the file at
lines 112 to 113:

```text
a feature that cannot run on the resolved tier reports unavailable, and never
silently produces a different answer.
```

This story is not a tier feature, but the refusal policy below applies that
principle to a stored value: a value the page cannot use is refused by name
with a visible sentence and is never quietly replaced by the default as if
it had been accepted.

F-214's approved plan, `.claude/plans/F-214-design.md` line 70, is the
normative source for today's behavior, quoted because this story amends it:

```text
Storage. One key, `ocelli.mouse`, holds a profile id. The page already keeps
`ocelli.no-series` and `ocelli.no-inspector` under the same prefix. Every read
and write of `localStorage` is inside `try` and `catch`, including the two
that exist today. A missing key, an unknown value, or a thrown accessor gives
`ocelli`. A failed write keeps the chosen profile for this page load and shows
the sentence "This browser did not keep the choice. It applies until you
reload." The slider rate is also kept, as `ocelli.fps`, an integer from 1 to
30. Anything else read from that key gives 10.
```

F-214's plan line 103 scopes out "the React example's preferences, which
F-205 owns". F-245's approved plan decided there is no second viewer, in two
separate sentences on two separate lines, quoted each exactly:

- Line 115: "There is no second accepted React viewer and no native
  upstream proxy."
- Line 264: "There is no second showcase page and no code fallback
  renderer."

What that history means for the word shared is open question 1. Under every
option the four existing keys and their behavior are as F-214 decided, and
this story adds validation and the default preset on top.

**cornerstone3D 5.8.2, transcribed from the shipped ESM**, read this session
at the path named in the header, package version confirmed from its
`package.json`:

`dist/esm/RenderingEngine/helpers/stats/StatsOverlay.js`:

```text
285:    readSavedPosition() {
287:            const raw = window.localStorage.getItem(STATS_CONFIG.POSITION_STORAGE_KEY);
        ... JSON.parse, then a validation: left and top must both be finite
        numbers, else return null. A thrown accessor returns null.
304:    savePosition(left, top) {
306:            window.localStorage.setItem(STATS_CONFIG.POSITION_STORAGE_KEY, JSON.stringify({ left, top }));
        in try, with an empty catch.
```

The key's value is in
`dist/esm/RenderingEngine/helpers/stats/constants.js` line 33:
`POSITION_STORAGE_KEY: 'cornerstone-stats-overlay-position'`.

`dist/esm/utilities/renderingCapabilities.js`:

```text
3:  const STORAGE_KEY = 'cornerstone3D.renderingCapabilities';
50: function readCachedFormats(renderer, webgl2) {
52:         const raw = window.localStorage?.getItem(STORAGE_KEY);
57:         if (parsed?.probeVersion !== RENDERING_CAPABILITIES_PROBE_VERSION ||
        ... also renderer, webgl2 and the formats shape. Any mismatch
        returns null, inside try, silently.
70: function writeCachedFormats(renderer, webgl2, formats) {
73:         probeVersion: RENDERING_CAPABILITIES_PROBE_VERSION,
78:         window.localStorage?.setItem(STORAGE_KEY, JSON.stringify(payload));
```

`grep -rn "localStorage"` over `@cornerstonejs/core/dist/esm` returns
exactly five lines, all in these two files, and none in `tools` or
`dicom-image-loader`. **The floor this transcription supports: cornerstone
5.8.2 keeps versioned, validated component stores with quiet fallback, the
probe version checked and a mismatch refusing the whole document silently,
the overlay position validated with a quiet null, and application
preferences left to the host. Ocelli's named refusal is the measured
improvement over those quiet forms.** The More row's cornerstone3D cell,
more.html line 180, stays verbatim.

### The tree as it stands, with commands

- `examples/folder-viewer/src/page.html` lines 1005 to 1007 define
  `MOUSE_KEY = "ocelli.mouse"`, `FPS_KEY = "ocelli.fps"` and
  `FPS_VALUES`, the strings `"1"` to `"30"`. Checked with
  `sed -n '1000,1010p'`.
- Lines 2022 to 2030 define `readPreference(key, allowed, fallback)`, which
  returns `fallback` on a throw or when the stored value is not in
  `allowed`, through `return allowed.includes(stored) ? stored : fallback;`
  at line 2029. Lines 2033 to 2039 define `writePreference(key, value)`,
  which returns false when `setItem` throws. Checked with
  `grep -n "localStorage\.\|return allowed.includes(stored)"`.
- The four keys are read and written at line 1252 (`let activeProfile =
  readPreference(MOUSE_KEY, PROFILE_IDS, "ocelli")`), line 1613 (the two
  panel reads, values `"0"` and `"1"`), line 1629 (the fps read), line
  1752 (the fps change writes `FPS_KEY` and drops the false return), line
  2014 (`togglePanel` writes `"ocelli." + name` and drops the false
  return) and line 2048, where `setProfile` assigns the result of
  `writePreference(MOUSE_KEY, id)` to `mouseUnsaved.hidden`. The profile
  write is the only visible write-failure sentence: `mouse-unsaved` at
  line 916. Checked with `grep -n` over those names.
- Exactly two lines contain `localStorage.` (2025 and 2035), both inside
  the two helpers. `examples/folder-viewer/tests/preview.rs` lines 935 to
  965, `every_storage_access_is_guarded`, requires every `localStorage.`
  line to carry `try {` on itself or the line above and the count to be
  exactly 2. Checked with `sed -n '855,970p'`.
- The F-214 pins stand at tests/preview.rs 893, 894 and 897: line 893
  pins the startup profile call site, the
  `let activeProfile = readPreference(MOUSE_KEY, PROFILE_IDS, "ocelli")`
  line quoted in full at page.html 1252, line 894 pins the quiet fallback
  return, and line 897 pins the fps call site
  (`fpsInput.value = readPreference(FPS_KEY, FPS_VALUES, "10");`). Checked
  with `sed -n '893p;897p'`. Line 894 was confirmed inside the same AC4
  block by reading lines 885 to 905.
- `more_says_the_page_keeps_four_choices` at tests/preview.rs 1012 to 1031
  pins the More sentence and the `partial` index mark. Checked with
  `sed -n '1005,1035p'`.
- `examples/folder-viewer/src/serve.rs` lines 339 to 353, `chosen_port`,
  returns port 0 when `OCELLI_PORT` is unset, so the origin
  `http://127.0.0.1:<ephemeral>/` changes on every launch and per-origin
  `localStorage` is lost. README lines 456 to 457 and page.html line 917
  say this. Checked with `sed -n '337,355p'`. `serve.rs` line 44 compiles
  the page in (`const PAGE: &str = include_str!("page.html");`).
- Presets are CT only. `examples/folder-viewer/src/preview.rs` lines 3440
  to 3445 define `CT_PRESETS` (SoftTissue, Lung, Bone), lines 3449 to 3455
  define their tokens `soft_tissue`, `lung`, `bone`, and `push_presets` at
  lines 3476 to 3493 offers all three on every CT, the offer keyed on
  Modality alone. The refusal sentence "these presets are in Hounsfield
  units and apply to CT only" is at line 339, and the `ct` rule's doc
  comment is at lines 700 to 709. Checked with `sed -n` on each range.
- The page mirrors the module: `PRESET_ROW` at page.html line 1055, the
  disabled rule at lines 3922 to 3923 (`blocked` when a VOI LUT is listed
  or the image is not monochrome), `windowInfo` at lines 3869 to 3878
  returning no presets for colour images, and `presetOffered` at line
  3914. Checked with `sed -n '3900,3935p'` and `sed -n '1040,1060p'`.
- `chooseWindow` at page.html 4021 to 4057 is the window change entry
  point. It bumps `vp.windowChoice`, asks under a trial object, and
  commits `vp.activePreset` only after a successful preview reply and its
  `still()` generation check. It calls `copyWindow(vp)` at line 4054.
  `copyWindow` at lines 6973 to 6990 copies to linked viewports and
  returns early when window linking is off. The corner names an active
  preset through `windowWord` at lines 3953 to 3963, which returns
  `preset.name` from the process's list. Checked by reading those ranges.
- Window state lives on the viewport. `resetView` sets the untouched state
  at lines 3849 to 3853 (the lead ruling's "about 3850 to 3854"),
  `showIndex` keeps window state across slices at
  lines 2640 to 2662 (reset only when colour-ness changes, and the preset
  is dropped at 2662 only when not offered), and the series load reset in
  `loadSeries` clears the state at lines 2520 to 2523. Checked with
  `sed -n '2514,2524p'`, `sed -n '2630,2670p'`, `sed -n '3846,3856p'`.
- Images enter a series only after the `POST /file` reply: the open loop
  at page.html 2241 to 2267 takes each file's `info` from the reply and
  pushes `{ file, info }` at line 2264, the JSON parsed at line 2260.
  `countedFilePost` is at line 1959. Checked with `sed -n '2230,2270p'`
  and `sed -n '1950,1970p'`.
- The comment at page.html lines 1053 to 1054 says "The page keeps no
  preset and no window number." Checked with `sed -n '1053,1054p'`.
- `examples/viewer-react/src/measurement-panel.tsx` line 23 renders
  "Preferences unavailable", pinned by its test at
  `measurement-panel.test.tsx` line 15. Checked with
  `grep -rn "Preferences unavailable" examples/viewer-react/`.
- `bin/ocelli.sh` line 70 is the `viewer-browser` description and lines
  214 to 231 are the case arm chaining twelve checks with `&&`. The
  checks live in `examples/folder-viewer/tests/browser/`. Checked with
  `sed -n '60,75p'` and `sed -n '210,235p'`.
- `tools/bench/src/viewer_work.mjs` `startViewer(binary, env = {})` at
  line 370 sets `OCELLI_PORT: "0"` before spreading `env`, so a caller
  may pass a fixed port, and refuses the reserved ports 8742 and 8765 at
  line 389. `stopByPid` is exported from
  `examples/folder-viewer/tests/browser/aspect.mjs` at line 118. Checked
  with `sed -n '365,400p'` and `grep -n stopByPid`.
- Every existing browser check launches Chromium with
  `chromium.launch({ headless: true })` and no persistent profile.
  `grep -n "launchPersistentContext\|userDataDir"` over the suites returns
  nothing. Playwright is pinned at exactly 1.62.1 in
  `tools/bench/package.json` line 22. `chromium.launchPersistentContext`
  is Playwright's documented persistent-context launch. It was not
  executed in this worktree because `tools/bench/node_modules` is absent
  here, and that limit is stated in Open question 3.
- Folder opens in the suites use
  `page.locator("#folder").setInputFiles(folder)`, as auto.mjs does at
  line 336, and request holds use `page.route("**/preview", ...)` in the
  pattern of changed.mjs line 747. Checked with `grep -n`.
- `examples/folder-viewer/src/mark-documents.mjs` contains no
  `localStorage`, `sessionStorage` or `indexedDB` reference, checked with
  `grep -n`, so the page's two helper lines are the whole of its web
  storage use and the marks are kept in memory until saved as files.

## What the specification does not cover

Everything below is this plan's decision, because no HLD section names a
preference store, a schema, a refusal sentence or a default preset.

1. **The schema, its version field and its closed sets.** Section Scope.
2. **Migration from the four legacy keys**, the both-present rule, the
   refused-document rule, the missing and mistyped field rule and the
   downgrade rule. Section Scope.
3. **Named refusal against quiet fallback.** F-214 decided a silent
   fallback. This story reverses that for invalid stored values, with one
   visible sentence per condition, and amends F-214's pinned tests. A
   thrown accessor on read stays quiet, exactly as F-214 decided, because
   a browser that refuses storage entirely gives the page nothing to
   name. The difference is stated so nobody reads it as an oversight.
4. **Per-key write-failure sentences and the two partial-write
   sentences.** Section Scope.
5. **The default preset's application point, its non-application cases and
   its refusal when not offered.** Section Scope.
6. **The closed-set no-patient-data invariant over the whole origin.**
   CLAUDE.md's hard rule refuses patient data in git, and the corpus sits
   behind an ignored directory for the same reason. This story extends the
   principle to the one place the viewer writes into the browser: nothing
   that names a study, a series, a folder, a URL or any free text is ever
   stored, and the stored key set is closed and tested.
7. **What shared means.** Open question 1, for the operator.

## Problem and reproducer

`cargo run -p ocelli-folder-viewer`, choose a mouse profile other than
Ocelli, drag the FPS slider, hide a column. Quit and relaunch without
`OCELLI_PORT`. The choices are gone, because the origin changed with the
ephemeral port. That is documented behavior. Now start twice on the same
`OCELLI_PORT` and hand-edit one stored value to garbage, for example
`ocelli.mouse` to `radiant`. The page silently uses Ocelli and says
nothing, which is F-214's decided quiet fallback at page.html line 2029. A
reader cannot tell a refused value from an accepted one. There is also no
way to start on a chosen window preset, the gap More records as "General
tool, window and layout preferences remain".

Reproducer for the quiet fallback: with the viewer on a fixed port, run in
the page `localStorage.setItem("ocelli.mouse", "radiant")`, reload, and
observe that no sentence appears anywhere while the Ocelli profile is
used. After this story the same reload shows the profile's refusal
sentence in `prefs-note`. Reproducer for the missing preset preference:
open the synthetic CT and press 1. There is no control that makes the
viewer start there.

## Scope, API and compatibility

### The schema

One versioned document under one new key, `ocelli.prefs`, holding JSON:

```json
{
  "version": 1,
  "mouseProfile": "ocelli",
  "cineFps": 10,
  "hideSeries": false,
  "hideInspector": false,
  "defaultPreset": "none"
}
```

Closed value sets, enforced on every read and every write:

| Field | Closed set |
|-------|------------|
| `version` | exactly the number `1` |
| `mouseProfile` | `ocelli`, `scroll-left`, `window-left`, the page's `PROFILE_IDS` |
| `cineFps` | an integer from 1 to 30, the page's `FPS_VALUES` range |
| `hideSeries`, `hideInspector` | the booleans `true` and `false` |
| `defaultPreset` | `none`, `soft_tissue`, `lung`, `bone` |

The refusal rules, in the order they are applied:

1. **A document that is not a usable version 1 object is refused whole.**
   Unparsable text, JSON `null`, an array, a number, or an object whose
   `version` is not exactly the number 1 (2 or higher, or `"1"` as a
   string, or missing) are all the same case. One named sentence, S6 in
   the table below. The legacy keys are used. **The document is left
   byte-identical forever**: no load-time write over it, and a change in
   this state writes the changed field's legacy key only and never the
   document. Every access to the parsed value is guarded so `null` or a
   primitive cannot throw outside the helpers.
2. **A version 1 document with a missing field or a field of the wrong
   JSON type is refused per field by name.** A missing `cineFps`, or
   `"cineFps": "10"`, or `"hideSeries": "1"`, is that field's sentence
   with the default applied for that field, and the document's other
   fields apply normally. Unknown fields inside a version 1 document are
   ignored on read and dropped on the next write, not refused.
3. **A load that finds no document and no legacy key writes nothing.** A
   load that finds no document and any legacy key writes the document once
   (the migration below) and then rewrites all four legacy keys from it.

No field may hold a string outside its closed set, and there is no
free-text field of any kind. The document is written when the reader
changes a choice, and at the migration load.

### Migration from the four legacy keys

The four keys `ocelli.mouse`, `ocelli.fps`, `ocelli.no-series` and
`ocelli.no-inspector` stay in the write path, not only the read path.

- On load with no document and any legacy key present, each legacy value is
  validated against its closed set, the document is written with the
  validated values, a value outside its set shows that key's refusal
  sentence and stores the default, and then **all four legacy keys are
  rewritten from the document's values**, so an invalid legacy value is
  repaired in place and a downgrade reads the current choices.
- On load with no document and no legacy key, defaults apply and nothing
  is written until the first change.
- On load with a version 1 document, the document wins. **Every successful
  document write rewrites all four legacy keys from the document's
  values**, so an older binary always reads the current choices,
  including a default applied at migration for an invalid legacy value.
  When both stores are present and disagree, the document wins and the
  next write restores agreement.
- On load with a refused document, rule 1 above applies: the S6 sentence,
  the legacy keys used, the document byte-identical forever.
- Downgrade: an older binary reads only the four legacy keys, which the
  sync writes above keep current. Stated cost of the both-present rule:
  because `localStorage` gives no order between the two stores, a choice
  made in an older binary between two runs of the new binary is lost when
  the document wins. There is no honest way to detect that case, and the
  plan does not pretend there is.

### Refusal and write-failure sentences

The sentences live in a new paragraph `<p id="prefs-note"
class="note-text" hidden></p>` immediately after the `#sentence` paragraph
at page.html line 870, explicitly grid-placed at column 2, row 6, with the
body grid's `grid-template-rows` at page.html line 38 gaining a trailing
`auto`. That row is outside both columns: the
`body.no-series` and `body.no-inspector` rules at lines 43 and 44 collapse
the columns and never hide it, and the row collapses to nothing while
hidden, so no existing layout moves. It carries one line per outstanding
condition, so several can show at once. The profile's existing
`mouse-unsaved` paragraph keeps its sentence and its position at line 916.
The scope note at line 917 is reworded to name all the kept choices, not
only the profile and the play rate, and keeps the `OCELLI_PORT` sentence.

| Condition | Sentence |
|-----------|----------|
| stored `mouseProfile` invalid | The saved mouse profile is not one this viewer knows, so the Ocelli profile is used. |
| stored `cineFps` invalid | The saved play rate is not a whole number from 1 to 30, so 10 is used. |
| stored `hideSeries` invalid | The saved series column choice is not readable, so the series list is shown. |
| stored `hideInspector` invalid | The saved tools column choice is not readable, so the tools column is shown. |
| stored `defaultPreset` invalid | The saved default preset is not one this viewer knows, so no default preset is applied. |
| the document is refused whole, in any of its shapes | The saved preferences could not be used, so the viewer's usual choices apply and the saved document is left unchanged. |
| a stored default preset is not offered on an image whose rows are choosable | The saved default preset is not offered on this image, so it is not applied. |
| write of `cineFps` failed | This browser did not keep the play rate. It applies until you reload. |
| write of `hideSeries` failed | This browser did not keep the series column choice. It applies until you reload. |
| write of `hideInspector` failed | This browser did not keep the tools column choice. It applies until you reload. |
| write of `defaultPreset` failed, or a default preset change while the document is refused, which has no legacy key to keep it | This browser did not keep the default preset. It applies until you reload. |
| write of `mouseProfile` failed | unchanged, `mouse-unsaved`, "This browser did not keep the choice. It applies until you reload." |
| a document write succeeded and at least one legacy rewrite failed | The choice was kept in this browser, but the copy that older versions of this viewer read was not updated. |

The reverse partial, a document write that failed while the legacy write
succeeded, shows the changed field's own write-failure sentence from the
table. That sentence is truthful there: the document wins on the next
load, so the choice ends at reload. No sentence echoes a stored value. A
refused value is arbitrary text of unknown origin, and the page does not
propagate it into the DOM.

### Cross-tab changes

`savePreference` never writes from a page-held copy of the document. On
each change it re-reads the document inside the helper, changes only that
one field, and writes it back, so a tab that opened earlier cannot drop
another tab's field. A `storage` event listener re-runs the same resolve
the load uses and refreshes the in-memory state and the visible controls,
the select, the slider and its label, the body classes and the sentences,
without a reload. The listener ignores its own tab's writes, which the
`storage` event already does.

### The default preset

A labelled select `default-preset` is added to the WINDOW section of the
inspector, after `window-note` at page.html line 875, with the options
None, Soft tissue, Lung and Bone. Its three tokens are the page's copy of
the library's preset tokens and are pinned against `preset_token` in
`examples/folder-viewer/src/preview.rs` lines 3449 to 3455, in the pattern
of `the_page_and_the_module_read_the_same_document_classes`.

**Application point, once per viewport per series load.** When
`loadSeries` resets a viewport's window state at lines 2520 to 2523, and
the stored default is a preset token, and the series' current image is
drawable, and the preset rows are choosable on it by the same rule the
rows use at lines 3922 to 3923, no VOI LUT listed and the image
monochrome, and `presetOffered(image, token)` at line 3914 is true, the
page calls `chooseWindow(vp, { preset: token }, { propagate: false })`
once for that series load. It is never called per image, never per cine
frame, and never again afterwards: not after Reset, not after a drag, and
not after the reader chose the file window index 0. `showIndex` keeping
window state across slices at lines 2640 to 2662 carries the applied
preset through the stack, and drops it only where the process does not
offer it, which is the existing rule at line 2662. A reader choice in
flight wins: `chooseWindow`'s own `still()` generation checks refuse the
apply if the reader has acted, and nothing retries.

**No propagation.** `chooseWindow` gains an optional third argument,
`options = {}` with `{ propagate = true }`, and skips the `copyWindow(vp)`
call at line 4054 when `propagate` is false. The stored-default apply
passes false. Reset and reader choices keep today's propagation, which the
link tip at line 1044 documents. A stored default loading into one
viewport never overwrites a linked viewport's reader-set window.

**Non-application, all silent with the window list as the visible
reason.** On an MR or a colour image the local process offers no presets
(`push_presets` at preview.rs 3476 offers them on Modality `CT` only, and
`windowInfo` at line 3871 returns none for colour), so the stored preset
is not applied and no fault is shown. On a CT with a VOI LUT Sequence the
rows are disabled by the rule at lines 3922 to 3923 and the process
refuses the choice, so it is not applied there either. Applying
Hounsfield numbers to an MR would be a quietly wrong window, so
non-application is the designed outcome, not a gap.

**Not offered, named.** A stored token the process does not offer is
skipped by the `presetOffered` guard before `chooseWindow` runs, so it can
never surface as a window fault. When the rows are choosable and the
token is still not offered, which can only be a page and process
disagreement such as a future token or a typo, the named sentence S7
shows in `prefs-note` and nothing is applied. A test plants this state by
holding a `**/file` reply with `page.route` and stripping the presets
array from the JSON info, in the pattern of changed.mjs.

### The default preset and the file's own window

At series load a stored default **overrides the CT file's own stored
window**: the apply goes through `chooseWindow`, which commits the preset
over the untouched file state the reset at lines 2520 to 2523 has just
written, and the corner's window line then names the preset through
`windowWord` at lines 3953 to 3963 instead of the file window's label.
Reset returns the file window, and the default never re-applies after it.

### The DICOMweb source and the remote display path

Today's DICOMweb source displays no image: `remoteFetchInstance` at
page.html 1587 to 1596 fetches and validates and says remote display is
not available, so no series load happens and no default is applied there.
The rule as designed keys on the series load into a viewport, not on the
source, and the schema is source-neutral, so F-202's planned remote
display, which its plan AC1 routes through the same canonical viewer's
owned worker surface, gets the same behavior for any image that enters
the page's series structures with the same info shape. F-202's plan here
does not yet name the route its remote frames take into the series
structures, so that hook point is re-checked when its design lands, and
that is recorded in Open question 2. A DICOMweb open writes nothing
either way, which AC9 tests.

### What is not stored

Layout grids, window and level values, per-key tool bindings, opened
folder paths, study and series UIDs, URLs and any free text are not
stored. Per-key rebinding stays out of scope as F-214 declared. Hanging
layouts are F-257. **F-204's 1:1 zoom mode is not stored**: zoom is
per-view state on the viewport, like pan, and cornerstone 5.8.2 stores no
zoom either, which the transcription above establishes, its five
`localStorage` lines being the whole of that release's persistence. The
layout grid is session state like the active tool. Adding a field later
means a new version of the document, decided then.

### Compatibility

The four legacy keys keep their exact names and value encodings
(`"0"`/`"1"` for the columns, the profile id, the rate as its decimal
string), so an older binary keeps working beside the new store. The scan
JSON, the grey reply and every crate are untouched. Under the recommended
answer to open question 1, `packages/` is untouched and the React demo
keeps rendering "Preferences unavailable", pinned by its existing test,
with the retirement recorded by name in BACKLOG and in the LLD.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context and policy | Observable output |
|------------------------|-------------------|---------------------------|-------------------|
| Startup, before line 1252 | `loadPreferences()` in page.html, reading the document and the legacy keys through the two raw helpers | Document first, closed sets, refusal rules 1 to 3 | `activeProfile`, `fpsInput.value`, the body classes, the `prefs-note` lines |
| Profile select and `setProfile` | `savePreference` through `writePreference` | Closed set, re-read then write, legacy sync | The select, the MOUSE key list, `mouse-unsaved` on failure |
| `fpsInput` change at line 1752 | `savePreference` through `writePreference` | Integer 1 to 30, re-read then write | The slider value and label, `prefs-note` on refusal or failure |
| `togglePanel` at lines 2011 to 2016 | `savePreference` through `writePreference` | Booleans, re-read then write | Body classes, `prefs-note` on refusal or failure |
| `default-preset` select | `savePreference` through `writePreference` | Token closed set, re-read then write | The select, `prefs-note` on refusal or failure |
| `storage` event | the resolve `loadPreferences` uses | Another same-origin tab's write | In-memory state and visible controls refreshed, no reload |
| Series load reset, `loadSeries` 2520 to 2523 | the stored-default apply through `chooseWindow(vp, { preset }, { propagate: false })` | Once per viewport per series load, rows choosable, offered, drawable | The applied preset window, the corner naming it, or nothing and a named sentence |
| `window.mjs` `observeProfileKeys`, line 788, which reads `ocelli.mouse` directly at line 795 | unchanged consumer | The legacy key keeps its name and encoding and is rewritten on every document write | The check keeps passing without change |
| `every_storage_access_is_guarded` | the two helpers | Still exactly two `localStorage.` lines | The test stays green |
| `examples/viewer-react` | unchanged, under open question 1's recommended answer | Out of scope, retirement recorded | "Preferences unavailable" stays |

Every consumer reads and writes only through the schema's functions. No
consumer touches `localStorage` directly, which the count test enforces
mechanically, and no consumer applies its own fallback. The refusal policy
is the shared provider's, not each caller's.

## State transitions and invariants

| Entry point or starting state | Input or event | Resulting state and output | Refusal and preserved state |
|-------------------------------|----------------|----------------------------|-----------------------------|
| No document, no legacy key | Load | Defaults applied, nothing written | A thrown accessor gives the same defaults and writes nothing |
| No document, legacy keys present | Load | Choices applied, document written, all four legacy keys rewritten from it, refusal lines for refused legacy values | A thrown accessor gives defaults and writes nothing, as F-214 decided |
| Version 1 document | Load | Document's choices applied, per-field sentences and defaults for missing or mistyped fields | Unknown fields ignored, dropped on the next write |
| Refused document, any shape | Load | The S6 sentence, legacy keys used | The document is never rewritten, at load or on a change |
| Any state, change of one choice | Select, slider, toggle | Document re-read inside the helper, one field changed, written back, all four legacy keys rewritten from the document, control state applied | A refused document writes only the changed field's legacy key, and a default-preset change shows the not-kept sentence |
| Document write succeeded, a legacy write failed | Control | The choice is kept, the partial sentence shows | The next load re-reads the document and repairs the legacy key |
| Document write failed, the legacy write succeeded | Control | That field's write-failure sentence | The document wins on the next load, so the choice ends at reload |
| Another tab writes | `storage` event | Resolve re-run, state and controls refreshed | The writing tab is not disturbed |
| Series load, stored preset valid and offered | `loadSeries` reset | `chooseWindow` applies it once, no link copy | A reader choice in flight wins via the still checks, nothing re-applies |
| Series load, MR or colour or VOI LUT rows | `loadSeries` reset | Nothing applied, no fault | The window list is the visible reason |
| Series load, rows choosable, token not offered | `loadSeries` reset | The S7 sentence, nothing applied | No window request carries the token |

Invariant: every stored byte is one of the schema's closed values. No URL,
study or series UID, folder name, path or free text is ever written, the
stored key set is closed, and no refused value is ever applied silently.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate behavior |
|---------------------------|-----------------------|-------------------|-------------------------|-------------------------------|
| `ocelli.prefs` and the four legacy keys | `loadPreferences` migration and `savePreference` | This browser origin, the loopback port | `setItem` returned true | A throw is caught and that condition's sentence shows, the in-page value stays. Writing the same value again changes nothing visible. A refused document is never written |
| `prefs-note` lines | `loadPreferences`, each save and the `storage` listener | The page load | The paragraph is unhidden with the line | A refused key never blocks the page, every control works |
| The stored-default apply | the `loadSeries` reset hook | Once per viewport per series load | `chooseWindow` resolved true | A raced or refused apply marks nothing and never retries. An identical series load applies again, which is a new load |
| Window state | existing `chooseWindow` | `vp.windowChoice` generation | Successful preview reply | Unchanged by this story except the `propagate` option, F-280 will consolidate the owner |

The commit point of a preference is the browser's `setItem`. There is no
pending state to recover. A document write that throws leaves the previous
document untouched and still writes the changed field's legacy key, which
the field's own sentence covers. A document write that succeeds and a
legacy rewrite that throws leaves the disagreement the partial sentence
names, and the next load's document win plus sync rewrite repairs it. A
default-preset change while the document is refused is kept nowhere, and
its sentence says so.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test or observation | Wrong implementation rejected | Evidence stage |
|----|-------------------------------|---------------------|-------------------------------|----------------|
| AC1 | The stored document is the version 1 schema with exactly the five fields and closed sets, unknown fields are ignored on read and dropped on the next write | New pins in `tests/preview.rs` on the schema constants and closed sets, and a browser case asserting the exact stored JSON after a full interaction pass, including a planted extra field that the next write removes | A validator that accepts any string for `mouseProfile`, or a write that keeps unknown fields, fails the exact-JSON assertion | implementation and integrated browser |
| AC2 | Legacy keys only, all valid: the four choices are applied, the document is written with them on load, and all four legacy keys are rewritten from the document | Browser case planting the four legacy keys, reloading, asserting the select, slider, classes, the stored document and the rewritten keys | A document-only store that ignores the legacy keys loses the reader's choices, and a migration that leaves an invalid legacy value in place fails the rewrite assertion | integrated browser |
| AC3 | Legacy and document both present: the document wins, and the next change rewrites the legacy key to agree | Browser case planting disagreeing values, asserting the document's values are shown, changing one choice, asserting the legacy key matches | A store that prefers the stale legacy value, or never syncs, fails the agreement assertion | integrated browser |
| AC4 | A refused document in each shape, unparsable text, `null`, an array, a number, and version 2 and `"1"` as a string: the S6 sentence shows, the legacy keys are used, the document is byte-identical after a change | Browser case planting each shape in `ocelli.prefs` in turn, asserting the sentence, the legacy values applied and `localStorage.getItem("ocelli.prefs")` unchanged after changing the profile | A store that overwrites the refused document, silently destroying the other version's data, or that treats it as absent and writes a fresh one over it | integrated browser |
| AC5 | An invalid stored value per field shows that field's sentence and applies the default, never quietly. Each case names its store: `mouseProfile` `"radiant"`, `cineFps` `"10"` and `hideSeries` `"1"` planted in the document's JSON, and `ocelli.fps` `"45"`, `ocelli.no-series` `"2"` planted as legacy keys with no document | Browser case asserting each sentence in `prefs-note` and the applied default, the bounding box of `prefs-note` non-zero and inside the viewport with `hideInspector` planted true, and for the legacy-planted cases the rewritten legacy key holding the default after the load | F-214's `return allowed.includes(stored) ? stored : fallback;` silent path at page.html 2029, which is the current behavior, and the pin at tests/preview.rs 894 replaced by the validator's refusal | implementation and integrated browser |
| AC6 | A failed write per key shows that key's sentence and the in-page choice still applies, the profile keeps `mouse-unsaved` | Browser case injecting a throwing `setItem` before page scripts, changing each control, asserting each sentence and the applied control state, with `prefs-note`'s bounding box non-zero and inside the viewport while the tools column is hidden | The current silent drops at page.html 1752 and 2014, which discard the false return | integrated browser |
| AC7 | Choices survive a real server restart and a real browser relaunch: the check starts the viewer on a fixed port found by binding port 0 and closing the probe, skipping the reserved 8742 and 8765, sets the profile, rate, columns and default preset, stops the process by PID with `stopByPid`, closes the persistent context, restarts the server on the same port, relaunches `chromium.launchPersistentContext` on the same `userDataDir`, and opens a fresh page, where all choices are restored before any folder is opened | New `tests/browser/preferences.mjs`, registered in the `viewer-browser` arm, using `startViewer(binary, { OCELLI_PORT: String(port) })`, `stopByPid` and `launchPersistentContext` with a per-run `mkdtemp` directory | Evidence from `page.reload()` in a living tab or a fresh context, which cannot see the origin loss, the ephemeral-port launch, `sessionStorage`, or a browser restart | integrated browser |
| AC8 | A different `OCELLI_PORT` is a different origin: the same check starts a second server on another fixed port in the same persistent context and the defaults apply there | Same check, second start, second origin | A process-side store, the rejected Rust-side alternative, which would show the first server's choices in the second origin's page | integrated browser |
| AC9 | The no-patient-data invariant over the whole origin: after exercising every control, a local folder open through `#folder` `setInputFiles` and a DICOMweb open against the synthetic upstream, which is `dicomweb.mjs`'s imported from the shared module when F-204's extraction has landed and otherwise a minimal copy in this suite, `Object.keys(localStorage)` equals exactly the five allowed keys and `sessionStorage` is empty, and a planted version 1 document carrying a folder path field and a UID has both dropped at the next write | The AC1 browser case plus the whole-origin sweep, the planted document asserted dropped from the written JSON | A `writePreference("ocelli.lastFolder", name)` implementation, which the key-set equality fails, or any sentence echoing a stored path or UID | integrated browser |
| AC10 | On a CT where the library offers presets, the stored preset is applied once at series load, Reset returns the file's window state, the first usable window or the automatic one when the file stores none, and it stays across scrolling, and play after Reset sends no preset-headered preview request, while play with the default still applied sends the preset header on each frame's single fetch | Browser case on the synthetic CT with a fetch-counting init script: open with a stored preset and assert the corner names the preset, press Reset and scroll, assert the file's window state stays, play in both phases and count `/preview` requests | An apply hooked per image or per repaint that re-applies after Reset, which would put the preset header on the frames fetched after Reset, or a second path setting `vp.activePreset` outside `chooseWindow`'s generation checks | integrated browser |
| AC11 | On an MR and a colour image the stored preset is not applied and no fault shows, and on a CT with a VOI LUT Sequence the rows are disabled and it is not applied either. The before-reply case of the round 2 draft is dropped: images enter a series only after the `POST /file` reply at page.html 2260 to 2264, so no image ever has an unknown preset list | Browser case using the existing MR and colour fixtures and a VOI LUT fixture, asserting no preset row active, no window fault and disabled rows | Applying Hounsfield numbers to an MR, the quietly wrong window, or applying through disabled VOI LUT rows | integrated browser |
| AC12 | Existing F-214 behavior is unchanged: the three profiles and their bindings, the fps range 1 to 30 starting at 10, the panel toggles and body classes, and exactly two `localStorage.` lines inside the two helpers. The call-site pins at `tests/preview.rs` 893, page.html 1252, and 897, page.html 1629, are amended to the new shape, and the fallback-return pin at 894 is replaced by the validator's refusal | `the_page_offers_mouse_profiles`, `every_storage_access_is_guarded` and the play tests, amended only where they pinned the quiet fallback or the four-choice sentence | A third storage access outside the helpers, which turns the count test red, or a changed binding table | implementation |
| AC13 | The page's default-preset tokens are exactly the library's, `soft_tissue`, `lung` and `bone`, and a token the process does not offer is never sent | A test comparing the page's option tokens with `preset_token` in `preview.rs` lines 3449 to 3455, in the pattern of `the_page_and_the_module_read_the_same_document_classes` | A token typo that reaches the process and surfaces as a window fault, which the offered guard and the agreement pin both reject, the typo failing the agreement test at implementation time | implementation |
| AC14 | More and the README describe the validated preferences truthfully: the More row keeps Partial with the named gaps, per-key tool bindings and hanging layouts, the delivery cell keeps naming S47 F-205, the cornerstone3D cell at more.html line 180 stays verbatim, the `#preferences` section and index mark and the README sentences updated | `more_says_the_page_keeps_four_choices` rewritten to the new sentence, mark and row, and the README pins | A row claiming Implemented while more.html line 541 still names tool bindings and hanging layouts, or a cornerstone3D cell edited away from "Configurable APIs. Persistence belongs to the host" | implementation |
| AC15 | Another tab's change is neither lost nor stale: two pages in one context, A changes the profile, B, opened earlier, changes the play rate, a fresh page sees both, and A's rate control updates without a reload through the `storage` listener | Browser case opening two pages on one origin, acting in order, asserting the fresh page's document and A's slider label | A helper that writes the whole document from the page's stale in-memory copy, which drops A's profile and fails the fresh page's assertion | integrated browser |
| AC16 | A stored default overrides a CT file's own stored window at series load, the corner names the preset, and the apply does not propagate: with window linking on and a second viewport holding a reader-set window, loading a stored-default series into one viewport changes no other viewport | Browser case with `links.window` enabled, a reader-set window in the second viewport, a series load in the first, asserting the second viewport's window unchanged and the first corner naming the preset | An apply that goes through the default propagation and overwrites the linked viewport's reader-set window | integrated browser |
| AC17 | A stored token the process does not offer on an image whose rows are choosable is skipped by the offered guard before any request and named by S7, with no window fault and no preset-headered request | Browser case holding a `**/file` reply with `page.route` and stripping the presets array, then loading the series with a stored default | A silent skip indistinguishable from no stored default, or an apply that faults the window notice | integrated browser |

Rows AC1, AC5, AC6, AC7, AC8, AC9, AC10, AC11, AC15, AC16 and AC17 run in
one new browser check. A row that needs a host the suite does not have
stays open with its owner named and is never passed.

## Approach

1. In `page.html`, add the schema constants beside `MOUSE_KEY` and
   `FPS_KEY` at lines 1005 to 1007, the document key, the closed-set
   literals and the sentence strings. Add `loadPreferences`,
   `savePreference` and the small validators beside `readPreference` and
   `writePreference` at lines 2022 to 2039. The two existing helpers
   become the raw accessors, `readPreference(key)` returning the stored
   string or null and `writePreference(key, value)` returning false on a
   throw, and they remain the page's only two `localStorage.` lines, so
   `every_storage_access_is_guarded` holds with no count change. The
   closed-set checks, the re-read on change and the sentences live in the
   schema layer above them, never in a third accessor.
2. Rewire the six call sites: line 1252 reads from the loaded
   preferences, line 1613 the columns, line 1629 the rate, line 1752 and
   line 2014 save through the schema, and `setProfile` at lines 2044 to
   2051 saves through the schema and keeps `mouse-unsaved`.
3. Add the `storage` listener that re-runs the resolve and refreshes the
   state and controls.
4. Amend the comment at lines 1053 to 1054, which today says the page
   keeps no preset and no window number, to say the page stores choices
   only through the preference schema and the numbers it shows come from
   the local process.
5. Add the `prefs-note` paragraph immediately after the `#sentence`
   paragraph at line 870, grid-placed at column 2 row 6 with the trailing
   `auto` row at line 38, and add the labelled `default-preset` select in
   the WINDOW section after `window-note` at line 875.
6. Add the `propagate` option to `chooseWindow` at line 4021, defaulting
   true, skipping the `copyWindow` call at line 4054 when false.
7. Add the stored-default apply at the `loadSeries` reset, lines 2520 to
   2523, guarded by drawable, the rows-choosable rule, `presetOffered`
   and the S7 sentence.
8. Amend the F-214 pins in `tests/preview.rs` at 893, 894 and 897 and the
   `more_says_the_page_keeps_four_choices` pins, add the schema, sentence
   and token-agreement pins, and add `tests/browser/preferences.mjs`,
   registered in `bin/ocelli.sh` line 70 and the case arm at lines 214 to
   231.
9. Update `more.html`, `README.md` and the BACKLOG and LLD retirement
   records per the documentation section.

Rejected alternatives. A Rust-side store in the local process would
persist across origins and machines and would break the documented
per-browser origin story, and AC8 is the control that rejects it. A second
validation copy in Rust beside the page's would be two places to look.
**Per-key versioned keys instead of one document** would make D3's
cross-tab change trivial, one key per field never drops another tab's
field, but they would give the migration no single version to refuse
whole, D2's whole-document refusal would become five per-key versions to
keep consistent, and the downgrade channel would need the same legacy
mirror either way, so the document with an in-helper re-read is kept. A
library-module store is open question 1's option (a), weighed there.

## Dependencies, write ownership and resources

Declared dependencies: F-245, integrated at `ac962c64` on this tree, and
F-214, done. No other dependency, no crate manifest change, no new npm
dependency.

Write set:

- `examples/folder-viewer/src/page.html`
- `examples/folder-viewer/tests/preview.rs`
- `examples/folder-viewer/tests/browser/preferences.mjs`, new
- `bin/ocelli.sh`, the `viewer-browser` description at line 70 and the
  case arm at lines 214 to 231
- `eslint.config.js`, the browser-suite globals block whose `files`
  selector is at line 533 and which closes at line 592, gaining only the
  page globals the new check reads, such as
  `prefs-note` and `chooseWindow`, in the existing conditional pattern
- `examples/folder-viewer/src/more.html`
- `examples/folder-viewer/src/use.html`, the section 7 search of the names
  this story adds, expected to conclude no sentence change
- `README.md`
- `docs/sprints/BACKLOG.md`, one retirement record in the M21 preamble
  after the reuse sentence at line 409, naming that F-205's preferences
  are page-local to the folder viewer and that F-214's transfer of the
  React panel's preferences to F-205 is retired, the panel keeping its
  named unavailable text. It lands with the completion commit
- `docs/lld/public-api.md`, the Example measurement section at lines 580
  to 592

Not written: `serve.rs`, `csp.rs`, every crate, `packages/`. The page's
inline script is hashed at runtime by `csp.rs`
(`configured_main_policy` at lines 33 to 54 calls `collect_inline_bodies`
and `push_hash` over the script bodies), so editing its body needs no
literal update. `serve.rs` line 44 compiles the page in, so no serving
change is needed.

Order against the other stories that write these files:

- **F-267**, approved, pending, writes `page.html` (the grey facts parse
  and the `paintOverlays` acquisition lines), `tests/preview.rs` (new
  corner tests), `corners.rs`, `preview.rs`, `serve.rs`, `lib.rs`,
  `bin/ocelli.sh` and `README.md`. The overlaps are `page.html` (disjoint
  regions), `tests/preview.rs` (F-267 appends corner tests, F-205 amends
  the F-214 pins and appends its own) and the `viewer-browser` arm and
  its description string, where both add one check and one module name.
  **Recommendation: F-267 lands first**, because it is approved and its
  line numbers are measured, and F-205 rebases onto it. The reverse order
  works the same way, with F-267 rebasing. No concurrent writer.
- **F-204**, pending, draft round 3 in its own worktree, writes
  `page.html` confined to the zoom, layout, pan, key, tip and listener
  blocks, `more.html`, `README.md`, `bin/ocelli.sh`'s additive arm and
  description lines, `eslint.config.js`'s conditional browser-suite
  globals block, a new `tests/browser/one-to-one.mjs`, and
  `docs/lld/build-targets.md`, read at its plan lines 1164 to 1174 this
  session. The lead's ruling also lists `use.html` and the key bindings
  in the overlap. The key bindings are page.html's KEYS panel at lines
  919 to 932 and the tip strings, which F-204 extends with its 1:1 key
  and F-205 touches only through the section 7 name search. On
  `use.html` the current F-204 text names no write there, its round 3
  notes recording that giving `use.html` to F-204 was F-202's plan's
  error, so the measured overlap today is the seven paths above and this
  plan claims only those, re-checked against F-204's final text at
  implementation. Same one-writer discipline: whichever lands second
  rebases. F-204's new suite joins the same `viewer-browser` arm and the
  same eslint block, so their additive entries merge mechanically.
- **F-241's viewer phase**, approved, pending, writes every folder viewer
  file F-239 touched plus a page worker, so `page.html`, `README.md`,
  `more.html`, `use.html` and a browser check. It lands after this story
  or rebases onto it, same discipline. Its cine presentation work does
  not touch the schema, and its inspector sentences do not touch
  `prefs-note`.
- **F-202**, pending, draft plan, will write `page.html` and the browser
  suites. Same discipline, later. Its remote display path is Open
  question 2.
- **F-245**, integrated. Its tree is this plan's baseline. Its owed AC8
  is its own bench evidence on the F-216 host and does not touch these
  files.
- **F-258** depends on F-205, so this story completes in S47 or F-258
  carries with it.

The browser check starts and stops the viewer binary by PID inside the
check, leaving no server running, per the no long-lived servers rule. The
fixed port is taken by binding port 0 on 127.0.0.1, reading the assigned
port and closing the probe, skipping 8742 and 8765, so the check never
collides with the reserved ports and never assumes a free numbered port.
The persistent context writes only into its own per-run `mkdtemp`
directory.

For F-280's designer, a note this plan does not act on: the stored-default
apply is a window setter, and F-280's row names "a preset" among the
setters its one owner must hold. The apply enters through `chooseWindow`
and carries the `propagate` option, so the single owner must keep both.
BACKLOG is not edited for this.

## Verification and evidence reuse

Focused checks while iterating: `bin/ocelli.sh check ocelli-folder-viewer`,
`bin/ocelli.sh test ocelli-folder-viewer`,
`bin/ocelli.sh clippy ocelli-folder-viewer`. The crate name is the
manifest's. The diff touches `examples/`, so also
`npm run lint && npm run typecheck`, and `eslint.config.js` is in the
diff, so lint is the check that proves the new globals. The diff touches
tracked prose and sprint planning data, so `bin/ocelli.sh gate prose` and
`bin/ocelli.sh gate backlog deviations`. The browser rows run under
`bin/ocelli.sh gate viewer-browser`, which needs the built viewer binary,
the Playwright browser install and `tools/bench/node_modules`, and is
outside the floor. The final required profile is the feature profile at
the integration tree, and the sprint profile runs once at close.

Mutations, recorded red: change the `mouseProfile` closed set to accept
one extra id and watch AC5's browser case go red. Remove the legacy sync
write and watch AC2, AC3 and AC7 go red. Remove the in-helper re-read and
watch AC15's fresh-page assertion go red. Apply the stored preset on an
MR and watch AC11 go red. Hook the apply into `showIndex` instead of the
`loadSeries` reset and watch AC10's Reset and scroll steps go red. Drop
the `propagate` false and watch AC16 go red.

Evidence invalidates on any later change to the schema, the closed sets,
the sentence strings, the migration and refusal rules, the cross-tab
helper, the application hook or the `prefs-note` element. The sentences
are typed into the tests from this plan, not read from the page, per HLD
27.2 R2.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: none. The page asks the local process for a
  preview exactly as today, and the process's own CT rule decides whether
  a preset is offered
- Render-loop allocation: none. Preference reads happen once at startup
  and on the `storage` event, writes happen on user input events, and the
  stored-default apply happens once per series load, never per frame
- unsafe: none
- Tier A (WebGPU): n/a. Browser storage and page state, no rendering or
  compute change
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

All three rows are not applicable. The stored choices affect which request
the page sends and which sentences it shows, and the grey bytes they
annotate are produced by the existing tier-independent local process, the
same argument F-267's plan recorded for its metadata work.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| contract | The schema constants, the closed sets, the sentence strings, the token agreement with `preset_token`, the amended F-214 pins at 893, 894 and 897, the two-line storage count | `examples/folder-viewer/tests/preview.rs` |
| browser | Migration, both-present, every refused shape, per-field refusals planted per store, write failures under an injected throwing `setItem`, the exact stored document, the whole-origin key set with a folder open and a DICOMweb open, persistence across a real server restart and browser relaunch, the second-port origin scope, the cross-tab two-page case, the default preset on a CT, an MR, a colour image, a VOI LUT file and a stripped reply | `examples/folder-viewer/tests/browser/preferences.mjs`, registered in the `viewer-browser` arm |
| documentation | The More section, row, delivery cell and index mark, and the README sentences | `more_says_the_page_keeps_four_choices`, rewritten |

No fixture is required. There is no pixel arithmetic, no geometry and no
DICOM parsing in this story. The one number involved, the fps range 1 to
30, is F-214's decided range and is pinned as a string set, not computed.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` names no preference row, checked with
`grep -n -i 'preferen' docs/hld/B-parity-surface.md`, which returns
nothing. On the comparison: cornerstone 5.8.2 keeps versioned, validated
component stores with quiet fallback and leaves application preferences to
the host, per the transcription above, and the More row's cornerstone3D
cell says the host half of that, "Configurable APIs. Persistence belongs
to the host", which this story keeps verbatim. Ocelli's named refusal is
the improvement the row's built cell records.

## Deviations

None. No new `D-NN` row is needed. The tier answer is n/a under D-07's own
rule, and no tolerance, budget or gate is touched.

## LLD impact

`docs/lld/public-api.md`, the "Example measurement" section at lines 580
to 592, whose sentence "Metadata browser and preferences read
unavailable. Those controls are F-205." spans lines 589 to 590 and changes
when this story completes: the preferences half names the folder viewer
page's versioned schema and records the React panel's retirement by name
under open question 1's recommended answer. No other file in `docs/lld/`
carries a preferences row, the other grep matches being unrelated uses of
the word in `benchmarks.md` line 215 and the `tier-resolution.md` table,
checked this session. `/complete-feature` step 9 makes the edit.

## User-facing documentation impact

- `README.md`, "Start the local series viewer". Step 5's sentence "The FPS
  control runs from 1 to 30, starts at 10 and is kept in this browser."
  at lines 394 to 395 extends to the validated choices and the refusal
  sentences. Step 7's sentence at lines 456 to 457 about the kept profile
  and `OCELLI_PORT` extends to the default preset and the side columns.
  Any other sentence naming what the browser keeps is found by the
  section 7 search and fixed.
- `examples/folder-viewer/src/more.html`. The comparison row at line 180:
  the cornerstone3D cell stays verbatim, the built cell names the five
  validated choices, and the viewer-state cell keeps Partial and names
  the gaps, per-key tool bindings and hanging layouts, because line 541
  of the same page defines preferences to include them and neither is
  delivered here. The delivery cell keeps S47 F-205. The `#preferences`
  section at lines 538 to 551: the status span at line 540 keeps Partial
  and names the gaps, the four choices sentence at line 542 becomes the
  five validated choices with the refusal behavior named, the module
  sentence states where the schema lives per open question 1's answer,
  and the tiers line at line 550. The index entry at line 237 keeps its
  `partial` mark.
- `examples/folder-viewer/src/use.html`. The section 7 search for the
  names this story adds. `grep -c -i 'preferen'` over the file returns 0
  and no `OCELLI_PORT` sentence exists there, checked this session, so
  the expected outcome is no change, and any stale sentence found is
  fixed in the same diff.

## Open questions

1. **What does shared mean, and where does the schema live?** The story
   title predates the one-viewer decision, and F-214's plan line 103 gave
   F-205 the React example's preferences before F-245 decided, at its
   lines 115 and 264 quoted above, that there is no second viewer. Owner:
   operator. Affects the write set and the startup order, not the AC
   behavior, except that AC9's DICOMweb open exercises the store under
   reading (c). Three readings of the word, each with its cost:

   - **(a) A module in `@ocelli/core`, used by the folder viewer and
     `examples/viewer-react`.** This is the option F-214's line 103
     points at. Costs: a new `packages/core/src/preferences.ts` with the
     same closed sets and a test, an export, and `packages/` joins the
     write set. The page's startup preference reads at lines 1252, 1613
     and 1629 are synchronous today and would become dependent on an
     asynchronous module import, so the startup must await the import
     before first paint or defer applying the choices until it lands,
     which is a visible ordering change a reviewer should see. When the
     library build is missing, the page can show a named "preferences
     unavailable" sentence, the page's existing pattern at lines 1061 to
     1074, which keeps the store single-copy rather than adding a
     page-local fallback beside the module. The React panel gains real
     controls, and its demo remains reachable only through
     `dev:boundary-demo`, a non-canonical page per F-245 line 264.
   - **(b) A page-local schema in the folder viewer.** The
     recommendation. The constants, validators and sentences live in
     `page.html`. Costs: the schema lives in one page, so a future second
     consumer re-implements or imports it then, and the React panel keeps
     "Preferences unavailable" with its pin, which the BACKLOG and LLD
     retirement records name so F-214's line 103 ownership does not
     dangle. Startup stays synchronous.
   - **(c) Shared across the local and DICOMweb sources of the one
     viewer.** Combinable with (b), and in fact already true of the
     design: the schema is source-neutral, it keys on choices, not on the
     source that opened them, and the application rule keys on the series
     load. Today the DICOMweb source displays nothing, so there is
     nothing further to share until F-202 lands, and the cost of
     deferring is zero. If F-202's remote path needs source-scoped
     choices, that is a new field and a version decision then.

   Deferral cost: the write set differs by `packages/core` files in
   option (a), and the startup restructure is the kind of change a
   reviewer should see coming, so this question is answered before
   implementation starts, not during it. Every AC's behavior is identical
   under options (a) and (b), only the module's location and the startup
   ordering differ.

2. **Where do F-202's remote frames enter the series structures?** Its
   plan here does not name the route, and the stored-default apply hooks
   the `loadSeries` reset. When F-202's design lands, its designer
   re-checks that hook against the remote entry route. Owner: F-202's
   designer. Affects no AC here. Deferral cost: if the remote route
   bypasses `loadSeries`, the default would not apply to remote series
   until a one-line hook is added there.

3. **Is `chromium.launchPersistentContext` verified against the pinned
   Playwright?** It is Playwright's documented persistent-context launch,
   and the pin is 1.62.1 at `tools/bench/package.json` line 22. It was
   not executed in this worktree, because `tools/bench/node_modules` is
   absent here and the equipped host runs the suite. Owner: the
   implementer, on the first run of AC7. Deferral cost: none, the first
   AC7 run is the verification.

## Operator design round, 2026-11-11 and 2026-11-12

The round 1 draft was a generic template and was rewritten. The round 2
rewrite was reviewed with 9 defects, 10 smells and nitpicks, and this
file is the round 3 revision, amended per the lead rulings recorded at the
top. It stays draft until the open questions are answered and the
coordinator's review passes.

## Operator decision and lead amendment after design round 3, 2026-11-12

This section is authoritative over every earlier sentence it touches.

**Open question 1 is decided by the operator**: page-local and
source-neutral. One versioned, validated schema lives in the folder viewer
page and applies to its local-folder and DICOMweb sources alike. No
`@ocelli/core` preference API is added. The React example's
`Preferences unavailable` panel is recorded as retired by name in
`docs/sprints/BACKLOG.md` and `docs/lld/public-api.md`, and the same LLD edit
names F-249 as the owner of the metadata browser half of that sentence.
`cargo run` and `npm run dev` share preferences only on the same
`OCELLI_PORT`, and the README says so. Open question 3 is closed:
`launchPersistentContext` exists in the pinned Playwright 1.62.1.

**The default preset applies after the viewport moves.** The apply runs
after `showIndex` sets the new series' index in `loadSeries`, not at the
reset before it, so `chooseWindow` captures the new series' image. AC10
gains a row loading a CT series into a viewport previously scrolled to a
non-zero index of another series, which a pre-move apply fails.

**The not-offered sentence fires only on a real mismatch.** Sentence S7
shows only when the image's preset list is non-empty and lacks the stored
token. An MR or any image with an empty list stays silent. AC17 replaces one
token in the held `**/file` reply instead of removing the array, and AC11
asserts S7 is absent on the MR and colour cases.

**A write serializes only validated values.** Every document write carries
each known field's validated value, and a field refused at load is written
as the value in effect, never carried forward. The four legacy keys are
written from those validated values. An AC9 case plants a path string in a
known field, changes a different field, and asserts that the document and
all four legacy keys hold closed-set values only.

**AC15 isolates the in-helper re-read.** Page B installs an init script that
captures and drops `storage` events before the page's listener runs, so only
the re-read inside the helper can keep page A's change. A separate case
keeps the listener and asserts page A's change reaches page B's controls.
The listener's refresh is a non-writing apply: it updates in-memory state
and the visible controls, keeps the current tool, writes nothing, and a
running cine keeps playing at its rate until the next play.

**Coverage.** AC3 asserts all four legacy keys equal the document after one
change. A key-selective throwing `setItem` case covers both partial-write
sentences, and AC4 gains a default-preset change in the refused state with
its not-kept sentence. AC4 gains the missing-`version` shape and AC5 the
missing-field case. AC16 loads into the outlined viewport, because
`copyWindow` returns early for any other source. The origin sweep also
asserts `indexedDB.databases()`, `caches.keys()` and `document.cookie` are
empty. `mouse-unsaved` stays in the tools column because its only control
lives there, and S7 clears at the next series load.

**Base and order.** The base is `sprint/s47` at `4e0e08d6`, which holds
F-267 and the approved F-204 plan. F-267 is in the base. F-204 lands before
F-205, and F-205 rebases onto it: both write the README paragraph at about
455 to 460, more.html, page.html in different blocks, the `viewer-browser`
arm (219 to 237, thirteen suites today), its description line and the
eslint block, and each new suite inserts before `launcher.mjs`. F-202 comes
after both. F-204 does not write `use.html`. The implementer re-reads every
cited line on the new base, where F-267 moved page.html lines by plus 10
from about 3298 and plus 23 from about 3678, and treats a moved line as the
same site. AC9's DICOMweb case adds a one-line export of
`startSyntheticDicomweb` from `dicomweb.mjs`, which joins the write set.
The comparison with cornerstone3D says its StatsOverlay store has no
version field and that the named refusal is a stated improvement, not a
measured one.

**Further to the above, after the confirmation review.**

*The cine rate follows the listener.* A slider drag changes a running
cine's rate live today, because `stepInterval` reads `fpsInput.value` on
every step. The listener's refresh does the same, so a cine in the other
tab plays at the new rate from its next step. The earlier sentence that a
running cine keeps its rate until the next play is withdrawn.

*A series is never drawn at two windows.* The page's F-227 rule
(page.html about 2736, tested in `auto.mjs` about 641) forbids a series
showing the file window and then the preset. When a stored default will be
tried on a series load, the viewport's first paint of that series waits
until the `chooseWindow` trial settles: on commit the first paint is the
preset, and on refusal or failure the first paint is the file window. Only
the viewport being loaded waits. AC10 asserts that the first window painted
on a stored-window CT with a stored default is the preset, which an apply
that paints the file window first fails.

*The write rule keys on the re-read document.* Inside the helper, a field
whose value in the re-read document is valid is kept as read, and a field
whose re-read value is invalid is written as this tab's value in effect.
So a valid value another tab wrote is never overwritten by a field this
tab refused at load.

*The listener's obligations have tests.* AC15's listener case spies on page
B's `localStorage.setItem` and asserts no call during the refresh, asserts
the active tool is unchanged, and asserts a running cine's next step uses
the new rate. A new row, AC18, covers the key-selective throwing `setItem`
partial-write sentences, and the summary of browser rows lists it. S7 is
held per viewport and clears at the next series load into the viewport that
showed it, which AC17 asserts, and a load into another cell leaves it.

*Superseded text.* The base named at the top and the "F-267 first"
recommendation are superseded by the base and order above. AC9 uses the
one-line export, not F-204's extraction or a copy. Open question 3 is
closed. The named refusal is a stated improvement, not a measured one,
wherever the plan says otherwise.

**The first-paint hold, specified.** This replaces the hold sentence above.
1. `loadSeries` evaluates, once, whether the stored default will be tried
   for the image it is about to show, and that single evaluation is the
   apply's eligibility. When it is true, `loadSeries` sets the viewport's
   hold before it calls `showIndex` (about page.html 2532), recording the
   load's `loadToken`.
2. While the hold is set, every paint site for that viewport waits: the
   tile-mode paint (about 2696), the cache-hit `applyGrey` (about 2700 to
   2702), the tile paint after the F-227 wait (about 2741) and `pumpMain`'s
   `applyGrey` after its fetch (about 2747). Requests may still be sent.
3. Only the trial of the same `loadToken` releases the hold, and it
   releases on every outcome: commit, refusal, fetch failure, an early
   `return false`, and supersession by a newer load. A newer load sets its
   own hold under its own token, and an older trial's release never opens
   it. Release repaints the viewport from its current window state, which
   is the preset on commit, the reader's window if the reader chose one
   during the trial, and otherwise the window the page would have shown,
   the file's stored window or F-227's automatic series window.
4. AC10 holds the preset `/preview` reply with `page.route`, logs the
   corner window label during the hold, and asserts nothing was painted for
   that series until release and that the first label is the preset. It
   runs a cache-hit case, a tile-mode case and a superseding-load case,
   where a second series load during the held trial must paint only the
   second series' outcome.

**Smaller fixes.** The write rule keeps the field being changed at the
user's new value, and applies "kept as read if valid, else the value in
effect" to every other field. AC15 spies on `Storage.prototype.setItem`
filtered to `this === localStorage`, with a positive control where page B
changes its own control and a call is registered. The cine case uses a
series without Frame Time and asserts the rate from the step after the
delay already running. S7 is per-viewport state, and the single
`prefs-note` shows the active viewport's S7. Activating another cell shows
that cell's state, returning shows the first again, and a load into a cell
clears only that cell's S7. AC18 gets its table row: the key-selective
throwing `setItem` makes one key's write fail, the page shows that key's
partial-write sentence and no other, and an implementation that reports
every write as failed, or none, fails it, at the integrated browser stage.
The browser-row summary lists AC2, AC3, AC4 and AC18.
