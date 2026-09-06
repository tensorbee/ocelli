# F-X012, SIGMOID divergence review, pass 1

**Reviewer**: independent agent, did not write the implementation
**Diff reviewed**: the complete staged diff in `/private/tmp/ocelli-f-x012`,
base `f3c7176f801dd691402248d0e7ed357bf0f9be2b`. Twenty tracked files are
modified, with 403 insertions and 71 deletions. The ignored
`.claude/scratch/F-X012-progress.md` evidence was read but is not part of the
diff.
**Result**: 2 defects, 0 smells, 1 nitpick

## Defects

### D1, a matching row attributes any candidate pixel defect to the reference

**Where**: `tools/oracle/src/attribution.rs:742-778` and the new fixture at
`:1480-1581`

**What**: the register match is computed only from the reference sidecar's
function and width. Once any pixel predicate fails, the match unconditionally
sets `Side::Reference` and rung `register`. It never checks whether the pixel
effect has the reversed-ramp shape declared by the entry. The test proves the
intended full-ramp case and identity case, but no unrelated mismatch case.

This means a candidate frame that is standard-correct except for one damaged
pixel on this SIGMOID row is reported as a reference divergence. The
`divergent-while-unmeasured` qualifier still makes the run red, so this does not
silently make the gate green. It does make the attribution false, exactly where
D14 requires a measured divergence with a side attached.

**Why it is wrong**: F-X012's evidence narrowed the known defect to an inverted
helper range while the presented reference pixels are monotonic and
standard-correct. The register entry itself says a future pixel effect would be
a reversed ramp. A sidecar condition proves only that the helper condition is
reachable. It cannot prove that an arbitrary candidate pixel mismatch was
caused by that helper. `docs/lld/comparator.md` says the mismatch must be
"explained by" the matching entry, which is stronger than the implementation's
mere co-occurrence test.

**Evidence**: I mutated only the new Rust fixture so the reference and candidate
were both the ascending `0..=255` ramp, then changed candidate pixel zero from
0 to 3. The test still passed all of its assertions, including
`Side::Reference`, `rung == "register"` and the reference-divergence qualifier.
The source was restored byte-for-byte afterward.

```text
$ bin/ocelli.sh test ocelli-oracle an_inverted_sigmoid_reference_is_attributed_only_when_pixels_differ
running 1 test
test attribution::tests::an_inverted_sigmoid_reference_is_attributed_only_when_pixels_differ ... ok
test result: ok. 1 passed; 0 failed
```

The same causal-scope problem exists one rung earlier at lines 752-762. Any
parameter mismatch on a matching SIGMOID sidecar, including an unrelated
rescale-slope change, is labelled as explained by this helper entry.

### D2, `resolvedBy` validation accepts an empty or non-story provenance value

**Where**: `tools/oracle/src/attribution.rs:357-378`

**What**: the new parser requires `resolvedBy` to exist and be a JSON string on
a reachable entry, but accepts `"resolvedBy": ""` and arbitrary strings such
as `"not-a-story"`. The new test checks only the valid literal `F-X012`.

**Why it is wrong**: the approved plan explicitly requires both provenance
events to be validated, and the new error says the story is part of the
entry's provenance. An empty string records no story, so presence plus JSON
type is not validation of that claim. At minimum an empty value must be
refused. If this register's contract is an F-ID, its format should be checked
here too, once, rather than left to review convention.

**Evidence**: `resolved_by.is_none()` is the only reachable-entry check. A
present empty string becomes `Some(String::new())` at lines 357-366 and passes
line 367. No test supplies an empty or malformed identifier.

## Nitpicks

### N1, the recorded centre byte is described as rounded when it is truncated

**Where**: `docs/lld/oracle.md:334-336` and
`tools/oracle/reference-divergence.json:15`

The independent PS3.3 calculation gives exactly 127.5 at the centre. The
observed browser byte is 127. Calling 127 the PS3.3 value "rounded to RGBA8" is
false under ordinary nearest-integer rounding, which gives 128. The browser
measurement itself is useful and the one-code result remains inside the
unchanged tolerance. The prose should call these observed quantised or
converted bytes, or state the conversion rule that produces 127.

```text
PS3.3 values: [4.586483540333347, 30.396745115639977, 127.5,
               224.60325488436, 250.41351645966665]
Python round: [5, 30, 128, 225, 250]
recorded browser bytes: [5, 30, 127, 225, 250]
```

## Independent checks

- The generated DICOM is 12 by 20, declares SIGMOID, centre 40, width 0.5,
  slope 0.25 and intercept 0. Its first row is exactly stored values 151 through
  170. Its SHA-256 is
  `194a72529a7481ae9fb183aa7e8293d7556ac20a9586b2fb9add3c9fe48836c9`,
  equal to the manifest row.
- The five fixture values independently reproduce PS3.3 C.11.2.1.3.1 exactly.
- `bin/ocelli.sh corpus` reports 92 verified, 0 missing, 0 mismatched and 92 in
  the manifest.
- `bin/ocelli.sh compare census` reports 71 gating class-one views, 70 measured
  and 1 declined. Of the 70 measured, 51 can fail the bias bound and 19 cannot.
  The smallest blind window remains 678.
- `bin/ocelli.sh compare` reports 99 views, 71 pass, 0 fail, 28 unmeasured and
  0 absent. All 21 catalogue mutations are detected.
- The staged diff changes no HLD or tolerance source.
- The reachable register no longer absorbs identity. Identical frames produce
  `Pass`, `Side::None`, rung `pixels`, no register entry and no
  reference-divergence qualifier.

## Commands run

```text
git status --short
git diff --cached --stat
git diff --cached --name-only
git diff --cached -- <all 20 staged paths, reviewed in groups>
git diff --cached --check
rg -n "sigmoid|SIGMOID|ct_sigmoid|resolvedBy|register_entries|register_entry" ...
rg -n "tolerance|MAX_|BIAS|0.1|99.9" ...
UV_CACHE_DIR=/private/tmp/uv-cache uv run python -c '<independent DICOM and PS3.3 probe>'
UV_CACHE_DIR=/private/tmp/uv-cache uv run python -m unittest scripts.tests.test_corpus_synth.SigmoidFixture
node --test tools/oracle/tests/params_test.mjs
bin/ocelli.sh test ocelli-oracle attribution
bin/ocelli.sh test ocelli-oracle
UV_CACHE_DIR=/private/tmp/uv-cache uv run python -m unittest discover -s scripts/tests
npm --prefix tools/oracle test
bin/ocelli.sh corpus
bin/ocelli.sh compare census
bin/ocelli.sh compare
```

The full restored suites passed with 69 Rust tests, 232 Python script tests and
219 oracle Node tests. The focused parameter suite passed 43 tests.

## Story mutation checks

Each mutation was applied to the working tree only, tested, then restored. A
final unstaged diff check was empty.

```text
generated VOILUTFunction SIGMOID -> LINEAR
  SigmoidFixture: 3 run, 1 failed

generated WindowWidth 0.5 -> 1
  SigmoidFixture: 3 run, 2 failed

register pixel rung disabled
  focused Rust test: 1 run, 1 failed
```

No implementation, tolerance, sprint, verification, completion, integration,
commit or push action was performed.
