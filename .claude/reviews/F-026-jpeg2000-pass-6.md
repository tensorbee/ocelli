# F-026 review, pass 6

**Reviewed**: exact fully staged 138-path tree
`abff3cd4918a174741e26d43c1d6a96a78a2fd6e` against
`35b7ca14afa1091d5668d1a4ccff767f29e5633f`, including the complete
provenance-only remediation and all earlier F-026 paths
**Result**: 0 defects, 0 smells, 0 nitpicks

## Verified clean

### Pass-5 remediation

- The remediation delta from pass 5 is exactly three files. It changes only
  the runner's iteration-refusal and clock strings, the accepted baseline's
  clock string, and two permanent tests. The Rust timing source, benchmark
  arithmetic, subject registration, baseline value, 15-value calibration
  series, range, 0.1 tolerance, tolerance provenance, and replacement value
  are byte-identical to the pass-5 tree.
- The parser now says `31 timing samples`. The runner and accepted baseline
  both say that one `std::time::Instant` surrounds each four-decode batch and
  elapsed time is normalized by four. A production run emitted that exact
  clock text with `iterations_run=125`, `iterations_kept=31`,
  `decodes_per_sample=4`, checksum 1,563,934, and the four documented setup and
  allocation exclusions.
- The two new permanent tests cover both corrected claims. Changing the
  baseline's final `four` to `4` made the focused suite exit 1 with 11 passing
  and the accepted-clock assertion failing. After restoration, changing the
  parser's `31 timing samples` to `31 samples` independently made the suite
  exit 1 with 11 passing and the iteration-provenance assertion failing. Both
  exact reverse patches were applied, after which all 12 focused tests passed.
- The structured benchmark remains exactly 31 timed samples of four production
  decodes after one warm-up call. The Rust clock surrounds the four calls and
  divides elapsed milliseconds by four. Descriptor creation, decoder
  construction, fixture access, output allocation, and process startup remain
  outside each measured interval.
- The unchanged calibration arithmetic recomputes to 15 values, median 0.6878
  ms, and range 0.6732 through 0.7426 ms. The lower and upper deviations are
  2.1227 and 7.9674 per cent. The 10 per cent interval remains 0.61902 through
  0.75658 ms, with 7.8773 and 2.0326 percentage points of lower and upper
  headroom. The previous baseline remains recorded as 0.7353.
- Three fresh same-tree `bin/ocelli.sh bench --compare` commands measured
  0.6984, 0.7149, and 0.7134 ms. All three exited 0 and reported `within`.
  The last run's ignored record carried the corrected clock, exact counts,
  observed 0.6597 through 0.7293 ms range, checksum, and exclusions.

### Codec, vendor, and cross-target contracts

- OpenJPEG 2.5.4 regenerated the scalar-derived style-1 fixture and raw truth
  exactly. The codec suite exited 0 with 56 tests, including seven JPEG 2000
  tests. Lossless `.90` and general `.91` remain separate. QCD style and entry
  counts, missing or duplicate QCD, QCC refusal, SIZ, COD, SOT and EOC, one
  legal DICOM NULL pad, descriptor matching, reversible and irreversible
  transforms, finite integral stored-domain conversion, and atomic output are
  all covered.
- The vendor tree remains 78 files and 2,044,376 bytes, the exact 74-file
  package inventory plus four declared additions. The independent inventory
  root, both patched manifests, both licences, provenance bytes, workspace
  exclusion, exact path pin, and no-Rayon native and wasm graphs all remained
  green. Fourteen pin and size tests passed. The unsafe gate checked 159 Rust
  files and found only the two permitted files.
- `bin/ocelli.sh native` completed all eight steps. It executed the production
  decoder natively and from plain and SIMD wasm under Node. The release modules
  were 145,689 and 145,297 bytes. The separate upstream-default versus patched
  dependency evidence remains 222,226 and 222,166 bytes.
- The corpus check verified all 92 files with no missing or mismatched rows.
  The lossless fixture remains exact, the lossy distribution remains fixed,
  and the codec registry, F-004 runner, benchmark lifecycle, and all earlier
  native, RLE, Deflate, JPEG, and multiframe compatibility tests stayed green.

### Gates and independent mutation

- `gate bench` exited 0 with 12 subjects, three baselines, 27 Python tests,
  and 97 Node tests, of which 96 passed and one browser-only test was the
  declared skip. `gate pins`, `gate unsafe`, `gate provenance`, and `gate
  deviations` each exited 0.
- `gate guards` exited 0. It found 828 refusal sites with none unclaimed or
  unwatched. Its 341 refusal probes drove 32 guards red for their declared
  reasons, while 41 accept probes and 50 controls stayed green.
- The two wording mutations above are distinct from the author's fixture-first
  old-provenance failure because they changed already-correct values to new
  near-miss spellings one at a time. Each named permanent assertion went red
  for its own field. Both mutations were removed and `git diff --quiet` exited
  0 before the green gates.
- The complete floor ran with a private npm cache and exited 0 with all 26
  gates green. The focused fixture generator, codec, pin, wasm-runner,
  benchmark, native, guard, corpus, provenance, unsafe, deviation, prose, and
  content checks therefore agree on the reviewed tree.
