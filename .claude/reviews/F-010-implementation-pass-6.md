# F-010 review, pass 6

**Reviewed**: the fully staged index on `work/f-010-claude`, 44 files,
+8142/-41, after five rounds of remediation
**Result**: 2 defects, 1 smell, 8 nitpicks

Eighteen mutations run, seventeen red, one green. The code held up under every
one of them and under a re-derivation of all the arithmetic from PS3.3. Both
defects are false factual sentences, and one is the third consecutive round in
which a remediation for a false sentence shipped a new one. That pattern is
worth naming: it is the microscope's own warning about a documentation-heavy
workflow, "a remediation that corrects one sentence and adds three explaining
it ships three new claims for the next pass to falsify".

## Defects

### D1, the round-5 fix introduced a new false sentence, in both files it touched

**Where**: `docs/lld/oracle.md` and `tools/oracle/src/pins.mjs`
**What**: both said the walk "terminates on its first step" because each of the
six packages has exactly one manifest at its own root.
**Why it is wrong**: `installedVersion` starts the walk at
`dirname(require.resolve(name))`, which is the package's MAIN ENTRY directory,
`dist/` or `src/`, one level below the root. Depth 0 reads
`<pkg>/dist/package.json`, which does not exist, and the manifest is found on
the second step. The stated cause is in fact the reason the first step FAILS:
the start directory is not the root. The sentence also quietly contradicts the
module's own design, because if the first step succeeded there would be nothing
for `MAX_WALK` to bound.
**Evidence**: for each of the six, the iteration count before the manifest is
found is 1, never 0.
**Fixed**: both now say the walk starts one level below the root, in `dist/` or
`src/`, and finds the manifest on its second step. The claims in the same
paragraph that WERE true, "six of the sixteen" and the five
`@cornerstonejs/*` markers, were measured and left.

### D2, the claim that Appendix A gates A1 and A2 are answered

**Where**: `docs/lld/oracle.md` and `tools/oracle/unsupported.json`
**What**: both said the two spike gates "have an answer from this", and
`unsupported.json` went further with "both of those are green".
**Why it is wrong**: neither gate asks anything about cornerstone3D.
`docs/hld/A-spike-gates.md` puts A1 as "Does HTJ2K decode correctly in openjp2
under wasm32, bit-exact against OpenJPH?" and A2 as "What is the JPEG-LS
answer, CharLS bridge, self-compiled CharLS, or a young pure-Rust crate?". Both
are questions about Ocelli's own codec build, which does not exist yet.
Observing that the REFERENCE renders those five rows answers neither. "Green"
is this project's word for a resolved gate, and a later reader could have
closed A1 and A2 on it.
**Evidence**: `ls docs/spikes/` holds only `A7-tier-c.md` and `GATES.md`, and
no status record exists for A1 or A2.
**Fixed**: both now say what this story actually gives those gates, which is
the other half of the measurement: reference frames for the five rows, to be
compared against once there is something to compare.

## Smell

### S1, the record of what the reference cannot render had no test

**Where**: `tools/oracle/src/unsupported.mjs`
**What**: `entryFor` is a four-way conjunct, and the LLD makes that strictness a
load-bearing claim. Deleting `entry.boundary === failure.boundary` left
`npm test` at 76 pass, 0 fail. It was the only one of eighteen mutations that
stayed green. The conjuncts are redundant only because the two committed
entries name distinct rows, and the file is explicitly designed to grow.
**Why it would bite**: as it grows, the boundary and transfer-syntax conjuncts
become what stops one entry absorbing another row's unrelated failure. Nothing
would notice them weakening. The fault runner does not cover it either: all six
injections target a row no entry claims, so no injected run ever reaches a
matching entry.
**Fixed**: `tests/unsupported_test.mjs`, twelve tests. The validation was split
out of `readUnsupported` into `validateRecord` so the tests exercise the real
function rather than a copy of its rules. **All four conjuncts now go red
individually**: transfer syntax, boundary, row path and error fragment, each
deleted in turn, each producing at least one failure.

## Nitpicks, and what was done

- `.claude/commands/verify.md` said a skipped oracle gate means the stack is
  not installed. With the exception gone the gate has no skip path at all: an
  absent stack is a refusal from `bin/ocelli.sh`. **Corrected.**
- The LLD's low-information sentence said "exactly the mono16 transfer-syntax
  set", which is off by one member in each direction. **Rewritten with the
  arithmetic**, and re-verified: 15 rendered `syntax/` mono16 ramps plus
  `synthetic/ct_unsigned_16.dcm`, out of 18 `syntax/` rows of which 16 are
  mono16, one is not rendered and two are colour.
- `page/app.mjs`'s `PS3.10 File Meta` heading covered three reads, of which only
  one is File Meta. **Corrected** to name all three modules. All 25 tag numbers
  in `readAttributes` were checked individually against PS3.6 and all 25 are
  right.
- `run.mjs` said the interpreter resolves "the same way" as
  `corpus_tests.py`. The stop is the same, the last candidate is not.
  **Corrected.**
- `tests/pins_test.mjs`'s header was the un-remediated sibling of pass 5's D2.
  **Corrected** in the same terms.
- The determinism comparison compared only the boolean for a row that failed in
  both passes, so two failures at different boundaries would have read as
  deterministic. **The boundary is now compared too.**
- `unsupported.json` wrote a frame as "20 by 12" where everything else writes
  rows by columns. **Made consistent.**
- `--sprint` and `--all` remain identical arms, and `CLAUDE.md`'s "Current
  state" paragraph is stale. **Both left and handed to the integrator**: the
  first is in another story's region, the second is sprint state that
  `/complete-feature` owns.

## Verified clean

- **Pass 5's D1 and D3 hold**, and its S1 holds: `truncate` now pins a library
  message rather than the boundary name, and its accompanying claim that
  `decoded:` prefixes every decode failure is true of the driver's formatting.
- **Every number in the tracked prose was executed, not sampled**: sixteen
  declared dependencies matching sixteen enforced and sixteen recorded, six of
  sixteen taking the walk, five nested `type: module` markers with their
  contents dumped, 91 rows, 89 sidecars, 87 smaller than the canvas and exactly
  two larger read from the files, 85 file windows and 4 colour with zero
  `modality-default` and zero `full-range`, 16 low-information rows, 9 and 2
  fixture rows matching the checker's own output, and the YBR row's 480 against
  720 bytes.
- **The arithmetic was re-derived from PS3.3**, not read from the code:
  `fullRange`'s edges from C.11.2.1.2 with the asymmetric `<=` and `>` intact,
  `minimumWidth` against C.11.2.1.3.1 and C.11.2.1.3.2, and `canvasScale` from
  VTK's half-height convention and C.7.6.2.1.1, with `reference_mono12`'s
  `parallelScale` of 16 recomputed independently from the file and the
  resulting 0.25 letterbox matching the recorded `blackFraction` exactly. Every
  rounding is `toFixed(6)` applied after comparison, for display only.
- **The specific probes asked for**: `frameStatistics` has no off-by-one and
  reads channels in order, `uniformValue` correctly starts at index 4, the
  base64 round trip is lossless and is additionally enforced end to end by
  `assertFrameIntegrity`, `rowId`'s collision surface is closed for the
  committed manifest by a test the gate runs, `unsupported.json` cannot match
  the wrong entry because every conjunct includes the row path, and
  `check_sidecars.py`'s `_read` handles every VR the corpus carries. One
  looseness found and recorded: `LossyImageCompressionMethod` has VM 1-n and is
  read with `str(value)`, which would disagree with the page's backslash-joined
  string on a multi-valued file. No corpus row has one.
- **Build and resolution claims executed**: all four codec wasm specifiers
  resolve to real files, `dicom-image-loader`'s exports map is exactly as
  `build-page.mjs` describes it, `@cornerstonejs/tools` is imported nowhere and
  is nobody's peer, and `core`'s peers are exactly `metadata` and `utils`.
- **Runbook rows 20 to 22 executed**, row 20 with a control that proves the
  path prefix is the mechanism rather than the image.
- Boundary, tier and structure unchanged and clean. No `wasm-bindgen`, no
  pixels across an Ocelli boundary, no wasm memory view, no render loop, no
  `queue.submit`, no new trait, generic, `Box<dyn>` or forwarding wrapper. The
  eslint node and browser split proved in both directions with probe files.
- After remediation: 88 unit tests pass, `npx eslint .` exits 0,
  `prose_check.py` is clean over 73 files, and `bin/ocelli.sh gate oracle` is
  ALL GREEN with 91 attempted, 89 read back, 2 accounted for, identical digests
  across two passes and all six faults red at their named boundary.
