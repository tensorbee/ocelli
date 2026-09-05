# F-010 review, pass 1

**Reviewed**: the staged working tree on `work/f-010-claude`, 28 files,
+5762/-25, before remediation
**Result**: 8 defects, 12 smells, 12 nitpicks

Three independent reviewers took disjoint sections of
`.claude/commands/microscope.md` so that no finding rests on the author's own
reading. Section 1 and 2 (arithmetic, and would the test fail), section 3 and 4
(prose claims, and things nothing executes), and section 5 and 6 (boundary and
tier, structure) plus plan conformance and the parallel-worker integration
constraints. Every finding below was executed, not read.

## Defects

### D1, the width clamp is unreachable and its comment is false

**Where**: `tools/oracle/src/voi.mjs:32`, test at `tests/params_test.mjs:301`
**What**: `Math.max(1, max - min + 1)`. A constant image yields 1 from the
formula alone, so the clamp fires only when `max < min`, which no caller can
produce. The comment claims the clamp is why a constant image yields 1.
**Why it is wrong**: microscope section 2. A test that passes both ways is a
defect, because it will be counted as coverage forever. Section 4: the guard is
present, authoritative-looking, and never reached.
**Evidence**: removed the clamp entirely and re-ran `node --test
tests/params_test.mjs`: 22 pass, 0 fail. Green both ways.

### D2, the manifest digest's LENGTH is never tested

**Where**: `tools/oracle/src/manifest.mjs:31`, test at `tests/manifest_test.mjs:69`
**What**: the only negative fixture is `"notadigest"`, which fails on the
alphabet. A sha1 (forty valid hex characters) or a truncated sha256 would be
accepted.
**Why it is wrong**: the digest is what ties reference output to one corpus.
**Evidence**: loosened `{64}` to `{1,64}` and re-ran: 10 pass, 0 fail.

### D3, the nested merge in `resolveRenderParams` is asserted by nothing

**Where**: `tools/oracle/src/params.mjs:74-86`
**Evidence**: replaced the whole thirteen-line deep-merge branch with
`target[key] = value` and re-ran both suites: 30 pass, 0 fail. The one
committed rule is a nested apply, so the untested branch is the one that runs.

### D4, the merge puts a contradictory sentence into every colour sidecar

**Where**: `src/params.mjs:82` against `render-params.json:21-25, 45-51`
**What**: the colour rule sets `voi.source` to `none` and, because a nested
apply merges, inherits the base's `voi.why`, which explains why the file's own
window is used. That paragraph is copied verbatim into all four colour
sidecars beside the value it contradicts.
**Why it is wrong**: HLD section 11 makes the sidecar load-bearing output.
**Evidence**: resolved `synthetic/sc_rgb_interleaved.dcm` and read the block
back, and confirmed the same text in the four `out/*.json` colour sidecars.

### D5, "larger than every corpus frame, so no row is downsampled" is false

**Where**: `docs/lld/oracle.md:130`, and the same sentence in
`render-params.json`'s `base.canvas.why`
**Evidence**: scanned every sidecar's `attributes.rows` and `.columns`.
`real/dx_varepop/00000001.dcm` is 879 by 1168 and `real/us_cmb_crc/00000001.dcm`
is 590 by 819, both larger than 512 in both dimensions, so both are fitted down
under `NEAREST`. F-011's tolerance is stated against these frames.

### D6, "`voi.mjs` runs under node in the driver" is false

**Where**: `src/voi.mjs:3`, `docs/lld/oracle.md:150`, `eslint.config.js:68`
**What**: `run.mjs` does not import it. The only importers are `page/app.mjs`
and `tests/params_test.mjs`. The LLD gives the driver import as the REASON the
rendered and recorded windows cannot drift.
**Evidence**: `grep -rn "voi.mjs"` over the tree returns two hits, neither in
`run.mjs`.

### D7, "the last three of those five are peer dependencies" is false

**Where**: `docs/lld/oracle.md:58`
**Evidence**: read the installed manifests. `core` peer-depends on `metadata`
and `utils`. Nothing peer-depends on `@cornerstonejs/dicom-image-loader`, which
is a direct import of `page/app.mjs`.

### D8, removing `s01_pre_oracle` left three tracked prose claims false

**Where**: `.claude/WORKFLOW.md:84`, `.claude/commands/verify.md:42`,
`.claude/commands/close-sprint.md:20`
**What**: all three describe the sprint profile's bootstrap skip as live. The
mechanism is gone from `bin/ocelli.sh`, correctly, per the plan's step 8.
**Evidence**: `grep -rn "pre-oracle\|bootstrap exception"`, and the now
byte-identical `--sprint` and `--all` arms in the staged `bin/ocelli.sh`.

### D9, the path-escape refusal in the page server cannot fire

**Where**: `tools/oracle/src/server.mjs:36-41`
**What**: `normalize()` runs first on a pathname that always starts with `/`,
which collapses every leading `..`, so `join(base, ...)` can never leave the
served directory and the 403 branch is unreachable. The comment says the
opposite: "refused rather than normalised into something that happens to
exist".
**Evidence**: thirteen hand-picked traversal payloads plus 454,914 fuzzed
pathnames produced zero escapes.

### D10, false geometry claim in the fixture table

**Where**: `tools/oracle/check_sidecars.py:159`
**What**: "the frame is twice as wide as it is tall".
**Evidence**: pydicom reports Rows 12, Columns 40, a ratio of 3.33.
`corpus_synth.py:429` builds it as `TRAP_COLS * 2`, twice the trap WIDTH.

### D11, "`.gitignore`'s `**/dist/` rule"

**Where**: `tools/oracle/build-page.mjs:19`
**Evidence**: `git check-ignore -v` names `.gitignore:33: dist/`. The behaviour
is right, the rule named does not exist.

### D12, the interpreter is not resolved "the way `corpus_tests.py` does"

**Where**: `docs/lld/oracle.md:367` against `run.mjs`'s
`checkSidecarMetadata` and `scripts/corpus_tests.py:101-134`
**What**: `corpus_tests.py` stops at an explicit `$OCELLI_PYTHON` that cannot
import pydicom, deliberately. The harness fell through to the next candidate.
**Why it is wrong**: an operator who points `$OCELLI_PYTHON` at the wrong
interpreter gets a green cross-read from a different one.
**Evidence**: with `OCELLI_PYTHON=/usr/bin/python3` (no pydicom, exit 3), the
loop fell through to `.venv/bin/python` and reported OK.

## Smells

- **S1**, four declared render parameters that nothing reads. `camera.mode`,
  `voi.fallback`, and `background` and `canvas` per row. Two of them are in
  `APPLY_KEYS`, so a rule may set them and the sidecar would publish them as
  the parameters that produced the frame, having produced nothing. This is the
  exact failure `params.mjs`'s own error message is written against.
- **S2**, `@cornerstonejs/tools` is pinned, pin-checked and never loaded, and
  drags four more declared dependencies that exist only as its peers. Neither
  the package manifest nor the LLD says so.
- **S3**, `dicom-parser` produces the sidecar's load-bearing metadata and is
  outside the pin check.
- **S4**, the four codec packages are `require.resolve`d by name and declared
  nowhere. They resolve only because npm hoists them, and they decide the
  decoded pixels for every compressed row.
- **S5**, production imports test code, and the fault logic is spread over
  three files with two mutation sites.
- **S6**, `bin/ocelli.sh` was edited in five hunks outside the region reserved
  for this worker.
- **S7**, the SENTINEL check is reachable but no fault has ever watched it
  fire, three lines below a paragraph asserting that a guard nobody has watched
  fail is not a guard.
- **S8**, `--report-unsupported` exits 0 having verified nothing.
- **S9**, `--rows` with the default output directory wipes the canonical output
  and repopulates it with a subset labelled `ok`.
- **S10**, `digests` is collected in `report()` and never read.
- **S11**, `sidecar_test.mjs` says it covers the sidecar's shape and never
  touches `attributes`, the field HLD section 11 exists for. Setting it to
  `null` leaves 8 of 8 green.
- **S12**, `check_sidecars.py`'s `_coverage` measures which rows are NAMED, not
  which fields are ASSERTED. Deleting the `photometricInterpretation`
  assertion while keeping the row leaves it green.
- **S13**, the multi-frame row's rescale, which lives only in per-frame
  functional groups, is verified by nothing. Both readers agree on "absent" and
  the value that drove the render (-1024) is checked nowhere. This is HLD
  section 11's own sentence, on the row the corpus wrote to trap it.
- **S14**, `width >= 1` applies LINEAR's constraint to LINEAR_EXACT and
  SIGMOID, which PS3.3 C.11.2.1.3 gives `w > 0`.
- **S15**, `fullRange` on a constant image answers width 1, which makes
  `w' = 0`, and both the comment and the test present that as the safe answer.
- **S16**, `full-range` and `modality-default` are reached by no corpus row.
  85 of 89 use the file's window and 4 are colour.

## Nitpicks

`state.capabilities` written and never read. `params.frame` is a stack index
wearing a name that reads as a multi-frame frame number, and the test uses a
value that would be out of range at the render site. `digestOfManifest`
re-implements `digestOf` two lines above it. Three near-identical
spawn-and-collect helpers. `--sprint` and `--all` are now byte-identical arms.
`report()` destructures six fields from `context` and then reads three more off
the object. `decodeURIComponent` can throw inside the request handler.
`unsupported.json`'s `feature` field is presence-checked and never emitted.
`render-params.json`'s `version` is asserted and branched on by nothing. The
`out/` result set is held live as base64 through `report()`, roughly 125 MB.
`check_sidecars.py` re-parses the manifest by column position without checking
the header. `docs/lld/oracle.md` says F-010 touched `src/lib.rs` only to
correct a version, and it also added a paragraph.

## Verified clean

- **The PS3.3 C.11.2.1.2 derivation in `voi.mjs` is correct**, checked
  symbolically against the standard rather than against the comment above it,
  both edges and the `x = max` endpoint, and cross-checked against the pinned
  reference's own `toLowHighRange` for four windows including the
  LINEAR against LINEAR_EXACT pair at centre 40 width 400.
- **VOI arithmetic exists exactly once.** No second copy in `check_sidecars.py`
  or `page/app.mjs`, and `page/dist/app.js` carries `fullRange` with identical
  constants after esbuild renaming.
- Centre and width to range conversion is delegated to cornerstone3D's own
  utility, so no arithmetic of ours enters the reference.
- **All nine sidecar fields the plan lists are present**, and the nine
  hand-written fixture rows were cross-checked against `corpus_synth.py`'s
  constants AND independently against the real files under pydicom 3.0.2, with
  zero mismatches. Both alignment traps, both planar configurations, the
  absent Modality LUT on `cr_monochrome1`, and the JPEG baseline
  RGB-to-YBR_FULL_422 rewrite are all correct by construction.
- **Every claim about cornerstone3D 5.8.2's behaviour was executed**: the
  `createImage.js` string-indexing of `voiLUTFunction`, the inflater reachable
  only from the deprecated provider, the YBR_FULL_422 texture sizing, and the
  console evidence quoted in `unsupported.json`, found verbatim in
  `out/console.log`. v5.8.9 returns 404 from the registry for all three
  packages.
- Measured numbers check out: 480 against 720 bytes for the YBR row, 25.0% and
  70.6% for `reference_mono12`, 89 sidecars and 89 raw and 89 png against 91
  manifest rows minus 2 unsupported, and `run.json`'s manifest digest equals
  `shasum -a 256 corpus/manifest.tsv`.
- **Appendix A gates A1 and A2 have an answer**: all three HTJ2K rows and both
  JPEG-LS rows decoded, presented and read back.
- `assertReferenceEnvironment` checks all four things it claims and is on the
  run path. All six tier and boundary rows of microscope section 5 are n/a for
  a Node and browser harness, and each was answered rather than omitted.
- The new `eslint.config.js` block is load-bearing: zero errors with it, 59
  `no-undef` errors without.
- The new `staged_content_check.py` refusal fires on a real oracle PNG and
  passes a benign path.
- `unsupported.json`'s strictness is genuinely bidirectional and both
  directions are reachable.
- The four forbidden shared-state files were not touched.
- Eleven mutations run in total across the JavaScript and Python paths, of
  which four went green and became D1, D2, D3 and S11 or S12.
