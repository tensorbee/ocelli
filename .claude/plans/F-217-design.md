# F-217, Licence hygiene

**Status**: approved
**Epic ref**: E38.27
**Sprint**: S46
**Estimate**: 2w

Story row, `docs/sprints/BACKLOG.md`: "Licence hygiene: no copyleft dependency in any tracked manifest, a permissive JPEG reference decoder for the corpus, licence allowlist gates for Rust, npm and Python, and a third-party notices file". Layer All, no declared dependency, milestone M20.

`allocation.json` notes: "A sweep on 2026-11-05 found pylibjpeg-libjpeg 2.4.0 under GPL-3 pinned in pyproject.toml for corpus JPEG decoding, lightningcss 1.33.0 under MPL-2.0 through vite 8 in dev tooling, and vendor/openjph-core with no BSD-2 notice. No transitive licence check exists in any ecosystem."

The operator's instruction, verbatim: "our product is MIT, we shouldn't be using any non permissive license at any cost, find alternatives with permissive licenses".

Input evidence: the read-only sweep of 2026-11-05 (session scratchpad `licence-sweep.md`), the lead's decoder measurement of the same day (pydicom 3.0.2 in a scratch environment outside the repository), and the inspections recorded under "Facts measured for this plan" below.

## Normative source, transcribed

### HLD 27.2, rule R7, `docs/hld/24-agent-code-standards.md`

Quoted in a fence because the author's text carries a semicolon.

```text
| R7 | Source policy: cornerstone3D, VTK, ITK, DCMTK and elastix are in bounds. Horos is not. | Horos is LGPL-3 with a linked AGPL-3 component; agent exposure to it weakens any clean-room position |
```

R2 from the same table governs every probe in this plan: "Tests derive from the spec or the oracle, never from reading the implementation".

### `docs/SOURCE-POLICY.md`, normative

"**This file is the source-provenance policy and it is normative.** `scripts/source_provenance_check.py` enforces it over every tracked text file and every manifest, with no allowlist."

The read and depend table, unchanged from HLD Appendix C.2.1, marks three projects `**NO**` in the Read column (dwv GPL-3.0, Horos LGPL-3 with a linked AGPL-3 component, Grok JPEG 2000 AGPL-3). None of the three may be opened or depended on, and this plan opens none of them.

Section "Third-party obligations", verbatim: "Track them from day one. Bridging to `@cornerstonejs/codec-charls` is fine and so are OpenJPEG, CharLS and OpenJPH, all permissive. Grok is not. Attribution files are cheap to maintain incrementally and expensive to reconstruct."

Section "The rule for anything not yet listed", the three questions, verbatim:

1. Is there a LICENSE, LICENCE, COPYING or NOTICE file at the repository root?
2. Does the hosting platform's metadata report a licence?
3. Does the licence permit the specific use, which for source is usually **derivative works**, not merely use?

"If 1 and 2 are both absent, the answer is no."

The extension row this story closes: `| openjph-core 0.1.0, production use | BSD-2-Clause in registry metadata, no packaged notice | yes | yes, not shippable | 2026-09-13 |`, and the closing action its text names: "Obtain the complete notice, place it at openjph-core-0.1.0/LICENSE, record its hash beside the other vendored licence hashes" (the refusal message in `scripts/pin_and_size_check.py::check_openjph_redistribution`).

### Deviation D-22, `docs/hld/DEVIATIONS.md`

The relevant clause, verbatim: "The package carries no licence text, so the pins gate additionally asserts a recorded `LICENSE` in the vendor tree, scoped to the `--all` and release profiles, and that assertion fails until the notice material is obtained".

### The project licence, `LICENSE` and `README.md`

`LICENSE`: "Ocelli is dual-licensed under either of Apache License, Version 2.0 (LICENSE-APACHE) and MIT license (LICENSE-MIT) at your option." `README.md` section "Licence": "Dual-licensed under **MIT** or **Apache-2.0** at your option". `Cargo.toml` `[workspace.package] license = "MIT OR Apache-2.0"`.

### SPDX 2.3 Annex D, license expressions (the grammar every allowlist reader evaluates)

D.2: "License expression operators (`AND`, `OR` and `WITH`) should be matched in a *case-sensitive* manner." and "License identifiers (including license exception identifiers) used in SPDX Documents or source code files should be matched in a *case-insensitive* manner."

D.4.5, the precedence list and its sentence, verbatim:

```text
+
WITH
AND
OR
```

"where a lower order operator is applied before a higher order operator." The worked example: "`LGPL-2.1-only OR BSD-3-Clause AND MIT` represents a license choice between either LGPL-2.1-only and the expression BSD-3-Clause AND MIT because the AND operator takes precedence over (is applied before) the OR operator."

D.4.4: "A valid `<license-expression>` is where the left operand is a `<simple-expression>` value and the right operand is a `<license-exception-id>`".

### BSD-2-Clause notice conditions, upstream OpenJPH `LICENSE` at tag `0.26.3`

Fetched 2026-11-05 from the OpenJPH repository (a permitted source in the policy table), 27 lines, SHA-256 `5ddf5177863dfc9ab65fa129d587db651241f00e21ed2427b218bea997591f98`. Its holders, verbatim: "Copyright (c) 2019, Aous Naman", "Copyright (c) 2019, Kakadu Software Pty Ltd, Australia", "Copyright (c) 2019, The University of New South Wales, Sydney, Australia". Condition 1: "Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer." Condition 2 requires the same notice "in the documentation and/or other materials provided with the distribution" for binary redistribution. The implementation re-fetches and refuses to proceed if the digest differs.

### The four corpus rows in question, `corpus/manifest.tsv`

| path | syntax | category | sha256 |
|------|--------|----------|--------|
| `syntax/jpeg_baseline_rgb8.dcm` | 1.2.840.10008.1.2.4.50 | synthetic, colour, transfer-syntax | `3aacd331...7e42` |
| `syntax/jpeg_extended_12.dcm` | 1.2.840.10008.1.2.4.51 | synthetic, mono16, transfer-syntax | `800dab39...4879` |
| `syntax/jpeg_lossless_p14.dcm` | 1.2.840.10008.1.2.4.57 | synthetic, mono16, transfer-syntax | `6fe8a0c7...6fb9` |
| `syntax/jpeg_lossless_p14_sv1.dcm` | 1.2.840.10008.1.2.4.70 | synthetic, mono16, transfer-syntax | `6c813ed9...b9e3` |

All four are encoded by DCMTK `dcmcjpeg` (`EXTERNAL_ENCODERS` in `scripts/corpus_synth.py`). The settings, from `scripts/corpus_synth.py`: `.57` `--encode-lossless`, `.70` `--encode-lossless-sv1`, both from the 16-bit reference, `.50` `--encode-baseline --quality 90` from `REFERENCE_RGB8`, `.51` `--encode-extended --bits-force-12 --quality 90` from `REFERENCE_MONO12`.

### How the conformance tests compare them today, `scripts/tests/test_corpus_synth.py`

- `test_lossless_syntaxes_round_trip_exactly` covers `.57` and `.70`: `np.array_equal(ds.pixel_array, reference.pixel_array)` against the uncompressed reference. Exact.
- `test_lossy_syntaxes_decode_to_the_right_shape_and_stay_close` covers `.50` and `.51`: shape equality, then `error.max() / full_scale <= bound` against the uncompressed reference, with `YBR_EIGHT_BIT_MAX = 0.03` when PhotometricInterpretation starts with `YBR` and `TRANSFORM_MAX = 0.005` otherwise. The comments record the measured basis: "Shipped 0.0157, and 0.0196 on a transposed variant of the same shape, so this bound is 1.5 times the worst measured", and "JPEG extended at quality 90 gives 0.00049".
- `test_the_internally_encoded_case_is_plugin_independent` iterates `RLELosslessEncoder.available_plugins` and requires byte-identical output.

Neither lossy test compares against a previous decode, so there is no "expected decoded value" to keep identical for `.50` and `.51`. The bound against the original is what they assert.

## Facts measured for this plan

These were measured during design, from pydicom 3.0.2's installed source, the PyPI and npm registries, and wheel archives downloaded to the scratchpad and listed without installing.

1. **pydicom's GDCM plugin refuses the 12-bit row.** `pydicom/pixels/decoders/gdcm.py` raises `NotImplementedError("GDCM does not support 'JPEG Extended' for samples with 12-bit precision")` when `bits_stored != 8`, and `pixels/decoders/pillow.py` raises the same for Pillow. The lead measured the refusal. python-gdcm cannot be the single replacement.
2. **GDCM is first in every pydicom plugin list it joins.** `pixels/decoders/base.py` registers `gdcm` ahead of `pylibjpeg` for every JPEG, JPEG-LS and JPEG 2000 decoder, and `pixels/encoders/base.py` registers `gdcm` FIRST for the RLE encoder. Installing python-gdcm would silently move JPEG-LS and JPEG 2000 read-back to GDCM, and silently move `scripts/corpus_synth.py`'s RLE case to GDCM's encoder, whose bytes `test_the_internally_encoded_case_is_plugin_independent` would then have to match. That is a quiet change to a corpus generator, which is the shape this repository distrusts.
3. **Removing pylibjpeg-libjpeg also moves JPEG-LS read-back.** Today no `gdcm` is installed, so `.80`, `.81` and `jpegls_lossless_rgb8.dcm` decode through the `pylibjpeg` plugin, which uses pylibjpeg-libjpeg (Richter's libjpeg, an implementation independent of CharLS). After removal the next plugin is `pyjpegls`, which wraps CharLS, the same library that encoded those cases. ISO/IEC 14495-1 decoding is exact, so the values cannot change, but the read-back stops being independent of the encoder. The sweep did not report this.
4. **imagecodecs 2026.10.10** is `BSD-3-Clause` in its PyPI metadata, `Requires-Python >=3.12`, depends only on `numpy>=2.1`, and ships `cp312-abi3` wheels for macOS arm64 and manylinux x86_64. Its `imagecodecs/licenses/` directory holds 65 files, identical in name on both wheels. A copyleft grep over them finds one hit, `LICENSE-lzf`, a BSD-2 text that adds "the following files carry an additional notice that explicitly allows relicensing under the GPLv2", which is an alternative and not a condition. `LICENSE-libjpeg` and `LICENSE-jpeg` are the IJG release 9 texts, not Richter's GPL libjpeg. `LICENSE-heif`, `LICENSE-jetraw` and `LICENSE-libjxs` are empty, and the lead confirmed `JETRAW` and `JPEGXS` report unavailable in the wheel. `LICENSE-jpg_0xc3` (Chris Rorden, BSD-3) opens "The Software has been developed for research purposes only and is not a clinical tool", a disclaimer rather than a use restriction. **The manylinux wheel bundles `imagecodecs.libs/libgomp-*.so`**, the GCC OpenMP runtime, under GPL-3.0-or-later WITH GCC-exception-3.1, with no licence file of its own. The macOS wheel does not.
5. **numpy 2.5.2's manylinux wheel bundles copyleft runtime libraries** that its metadata does not show. `License-Expression: BSD-3-Clause AND 0BSD AND MIT AND Zlib AND CC0-1.0`, while `numpy.libs/` carries `libgfortran` (its `LICENSE.txt` says "GPL-3.0-or-later WITH GCC-exception-3.1") and `libquadmath` (its `LICENSE.txt` quotes the GNU Lesser General Public License, LGPL-2.1-or-later). CI installs exactly this wheel through `uv sync --locked` on `ubuntu-latest`. The macOS arm64 install in `.venv` has no bundled `.dylibs`. The sweep classified numpy from its metadata and did not see this.
6. **Lead measurement, imagecodecs against the GPL reference.** `ljpeg_decode` on `.57` and `.70` is pixel-identical to the reference decode. `jpeg8_decode` on `.51` differs by at most 1 in 339 of 6144 samples. `jpeg8_decode(outcolorspace="RGB")` on `.50` matches python-gdcm's decode exactly (digest prefix `c91a551360b306d8`) and differs from the GPL libjpeg by at most 3 in 4347 of 18432 samples. Two independent permissive decoders agree and the GPL one is the outlier.
7. **Vite.** `vite@7.3.7` is `MIT`, depends on `fdir`, `rollup`, `esbuild`, `postcss`, `picomatch`, `tinyglobby`, and declares `lightningcss` as a peer with `peerDependenciesMeta.lightningcss.optional = true`, so npm does not install it. `@vitejs/plugin-react@5.2.0` is `MIT` with peer `vite ^4.2.0 || ^5.0.0 || ^6.0.0 || ^7.0.0 || ^8.0.0`. The installed `@vitejs/plugin-react@6.1.1` peers `vite ^8.0.0`. `vitest@4.1.11` declares `vite ^6.0.0 || ^7.0.0 || ^8.0.0` as both dependency and peer. The root lockfile carries twelve `MPL-2.0` entries, `lightningcss` and its eleven optional platform binaries, all `dev: true`.
8. **npm lockfiles carry a licence per package.** All three `package-lock.json` files are lockfileVersion 3. Exactly two entries have no `license` field: the workspace link `examples/viewer-react` (its `package.json` has none) and `tools/oracle` `node_modules/rechoir@0.6.2`, whose `package.json` uses the legacy `licenses` array that npm does not copy into the lock.
9. **uv.lock carries no licence metadata at all.** Its packages are `et-xmlfile`, `numpy`, `openpyxl`, `pydicom`, `pyjpegls`, `pylibjpeg`, `pylibjpeg-libjpeg`, `pylibjpeg-openjpeg`, `pylibjpeg-rle`, `pyyaml`, and the virtual project `ocelli-tooling`.
10. **The guard sandbox copies `git ls-files` only**, so `node_modules` and `.venv` never exist inside it (`scripts/guards/sandbox.py`). A floor probe can therefore read lockfiles and tracked tables, and cannot read installed packages.
11. **Licence files.** `LICENSE-MIT` reads "Copyright (c) 2026 Tensorbee" at the root, and `packages/core` and `packages/react` hold byte-identical regular-file copies. `crates/ocelli-wasm/LICENSE*` are symlinks. `LICENSE-APACHE` is the unmodified Apache 2.0 text whose APPENDIX still shows the template line `Copyright [yyyy] [name of copyright owner]`. There is no `NOTICE` file anywhere. `Cargo.toml` has `authors = ["Tensorbee"]`. `pyproject.toml` has no `license` field. `examples/viewer-react/package.json` has no `license` field.
12. **cargo-deny 0.19.0** is installed locally. The sweep's scratch `deny.toml` (`[graph] all-features = true`, the fourteen-identifier allow list, `confidence-threshold = 0.8`, `[licenses.private] ignore = false`) reported `licenses ok`. `r-efi` passes because its expression `MIT OR Apache-2.0 OR LGPL-2.1-or-later` is satisfied by `MIT`.

## What the specification does not cover

The HLD says nothing about dependency licence gates, a notices file, the corpus read-back decoder or the copyright holder. Every decision below is this plan's.

1. **One allowlist, three readers.** `deny.toml` `[licenses] allow` is the single list. `scripts/licence_check.py` reads it with `tomllib` for npm and Python, so the three ecosystems cannot drift apart. The list is the sweep's fourteen identifiers plus `BlueOak-1.0.0` and `Python-2.0`, both OSI-approved permissive licences used by npm tooling. `unused-allowed-license` is set so the two npm-only identifiers do not warn in cargo-deny (key name verified against 0.19.0's own `cargo deny init` output, not from memory).
2. **Exceptions live in one tracked file, `ci/licences.toml`,** keyed by ecosystem, package and version. Each exception carries `licence`, `reason`, `scope` (one of `dev-only`, `tool`, `build-only`), `decided` (date) and `decided_by`. A stale exception, one naming a package and version the lock does not contain, is refused. An exception for a package in the runtime closure of a published package (`@ocelli/core`, `@ocelli/react`, or any crate reachable on normal and build edges from `ocelli-wasm` or `ocelli-native`) is refused whatever its scope says. `CC-BY-4.0` is not allowlisted. `caniuse-lite` (browser-support data) gets an exception instead, because an attribution licence for data should not admit software in other ecosystems.
3. **Python licences are declared in a tracked table** in `ci/licences.toml`, one row per `uv.lock` package with its SPDX expression and the verbatim `License-Expression` or `License` metadata string. The floor check proves the table complete against `uv.lock` and every row allowed. A separate installed check proves the table true against the installed wheels. This split exists because uv.lock records no licence (fact 9) and the sandbox has no `.venv` (fact 10).
4. **The installed check reads bundled material, not only metadata**, because fact 5 shows metadata hiding LGPL code. It scans every file under each `*.dist-info/licenses/`, every file under a package's own `licenses/` directory, and lists every shared library under `<dist>.libs/` and `<package>/.dylibs/`. A copyleft marker in any licence text, an empty licence file, or a bundled library absent from the recorded per-platform list is refused unless an exception names that exact file. A platform with no recorded list is refused, so a new OS is re-checked rather than assumed.
5. **The decoder replacement is imagecodecs alone**, decoding the encapsulated fragment directly, because imagecodecs is not a pydicom plugin. python-gdcm is not added, even as an optional cross-check, for fact 2.
6. **Every corpus read-back names its decoder.** A helper in the test module maps every syntax in `SYNTAX_CASES` to one declared route, and passes `decoding_plugin=` explicitly to pydicom for the routes that use it. No read-back relies on pydicom's plugin order, which is how fact 2 would otherwise land.
7. **Gate shape.** A floor gate `licences` and a non-floor gate `licences-wheels`, the latter because it needs the locked Python environment, which is exactly why `quirk-mutations` is outside the floor.
8. **The OpenJPH notice is placed twice and the copies are checked equal.** At `vendor/openjph-core-0.1.0/LICENSE`, which is the closing action D-22 and the redistribution refusal already name and which `OPENJPH_ADDITIONS` already permits without touching the pinned 50-file inventory. And inside `THIRD_PARTY_NOTICES` at the repository root, outside the vendor tree, which is the file a distributor reads.
9. **The copyright holder line.** The operator's answer (Open questions, Q1) is applied as `Copyright (c) 2026 Tensorbee Limited` in every `LICENSE-MIT`, a new `NOTICE` reading `Ocelli` then `Copyright 2026 Tensorbee Limited`, and the holder named in `LICENSE`. `LICENSE-APACHE` stays the unmodified licence text, because its APPENDIX is the licence's own instructions and the Apache convention places the holder in `NOTICE`.
10. **Manifest-name matching for the provenance check** is a prefix rule on the final name segment, applied only to dependency names extracted from manifests, with an optional `lib` prefix. Prose matching is unchanged.

## Problem and reproducer

```bash
grep -n 'pylibjpeg-libjpeg' pyproject.toml uv.lock                     # a GPL-3 pin in a tracked manifest
python3 -c "import json;p=json.load(open('package-lock.json'))['packages'];print([k for k,v in p.items() if v.get('license')=='MPL-2.0'])"
ls vendor/openjph-core-0.1.0 | grep -c -i -E '^(licen[cs]e|copying|notice)'   # 0
python3 scripts/pin_and_size_check.py --require-redistribution        # exit 1, names the missing notice
ls deny.toml                                                            # absent
```

Expected after this story: no copyleft licence in any tracked manifest or lockfile except operator-approved exceptions recorded with reason and scope, a gate that turns red when one is added, the OpenJPH notice present, and the redistribution check green. Today every existing gate passes with a GPL-3 pin and an MPL-2.0 tree present, which is the defect.

## Scope, API and compatibility

In scope:

1. `pyproject.toml` and `uv.lock`: remove `pylibjpeg-libjpeg`, add `imagecodecs==2026.10.10`, add `license = "MIT OR Apache-2.0"` to `[project]`.
2. `scripts/tests/test_corpus_synth.py`: the declared decoder routes, the lossless and lossy comparisons through them, a route and version pin test.
3. `deny.toml` and the Rust licence check.
4. The npm lockfile check, the Python table check and the installed-wheel check, with `ci/licences.toml`.
5. The lightningcss resolution (Q2), recommended as `vite ^7.3.7` and `@vitejs/plugin-react ^5.2.0` with a regenerated `package-lock.json`, and a `license` field in `examples/viewer-react/package.json`.
6. `THIRD_PARTY_NOTICES`, `vendor/openjph-core-0.1.0/LICENSE`, the recorded hash, and the rewritten `pins.openjph-notice` probe.
7. `NOTICE`, the holder line in every `LICENSE-MIT`, `NOTICE` in both published npm packages.
8. `scripts/source_provenance_check.py`: `pyproject.toml` and `uv.lock` as manifests, name extraction for six manifest grammars including `name = "..."` lock entries, the prefix rule.
9. Gate, CI, guard catalogue, census and runbook registration.

Non-goals, each with its reason:

- Playwright's downloaded browsers and ffmpeg, and system tools such as git and bash. They are not declared in any tracked manifest and are never redistributed. Q4 asks the operator to confirm the boundary.
- `cargo deny check bans`, `sources` and `advisories`. Licence hygiene is the story, and each of those is a separate policy.
- Full per-crate licence texts for every Rust crate compiled into a shipped artefact. Q6 asks whether that belongs here or in a follow-up.
- Replacing pylibjpeg-openjpeg with imagecodecs. It is MIT, and changing the JPEG 2000 read-back would change lossy `.91` decode values for no licence gain.
- LICENSE files inside each crate published to crates.io. Pre-existing, unrelated to third-party licences, recorded here so it is not lost.

Public signatures added:

```text
python3 scripts/licence_check.py [rust|npm|python|notices|holder]...   # default: all five, floor
python3 scripts/licence_check.py installed [--python PATH]            # licences-wheels
bin/ocelli.sh gate licences          # floor
bin/ocelli.sh gate licences-wheels   # outside the floor, corpus-tooling job and --sprint
```

`installed` resolves its interpreter the way `scripts/corpus_tests.py` does (`OCELLI_PYTHON`, then `.venv/bin/python`, then the running interpreter) and asks that interpreter for its `site-packages` and `sys.platform` plus `platform.machine()`. An interpreter that cannot be resolved is a failure, never a skip.

`rust` runs `cargo deny --locked --format json check licenses` (flags verified against 0.19.0's `--help`), first refusing any `cargo deny --version` other than the pinned `0.19.0`. A missing cargo-deny is a failure with the install command, not a skip.

Compatibility: no product API, wire format, pixel path or wasm export changes. The corpus files and `corpus/manifest.tsv` digests do not change, because only readers change.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `test_lossless_syntaxes_round_trip_exactly`, `.57` and `.70` | `imagecodecs.ljpeg_decode` on `generate_frames(...)` fragment 0 | exact equality, unchanged | pass or the case name |
| same test, `.80`, `jpegls_lossless_rgb8.dcm` | pydicom `decoding_plugin="pyjpegls"` (was pylibjpeg with libjpeg) | exact equality, unchanged | pass |
| same test, `.90`, `.201`, `.202`, RLE | pydicom `decoding_plugin="pylibjpeg"`, unchanged in effect | exact equality | pass |
| `test_lossy_syntaxes_decode_to_the_right_shape_and_stay_close`, `.50` | `imagecodecs.jpeg8_decode(frag, outcolorspace="RGB")`, colour converted once inside libjpeg-turbo | `YBR_EIGHT_BIT_MAX` unchanged | worst-pixel ratio |
| same test, `.51` | `imagecodecs.jpeg8_decode(frag)` | `TRANSFORM_MAX` unchanged | worst-pixel ratio |
| `test_jpeg_ls_near_lossless_holds_its_declared_near_bound` | `decoding_plugin="pyjpegls"` | `<= JPEG_LS_NEAR` unchanged | pass |
| `SigmoidFixture`, planar pair, native rows | pydicom native decode, no plugin | unchanged | pass |
| `test_the_internally_encoded_case_is_plugin_independent` | `RLELosslessEncoder.available_plugins` | unchanged, and no new plugin joins because python-gdcm is not added | pass |
| `scripts/corpus_synth.py` encoders | pydicom, pyjpegls, pylibjpeg-openjpeg, DCMTK, OpenJPH | unchanged, no encoder depends on pylibjpeg-libjpeg | identical digests |
| `bin/ocelli.sh gate corpus` (local corpus) | new `corpus_check.py --decode-digests` through the resolved interpreter | after file digests match the manifest | per-row decode digest |
| `bin/ocelli.sh gate licences` | `licence_check.py`, cargo-deny 0.19.0, `deny.toml`, `ci/licences.toml` | floor, CI `guards` job | OK line or each problem |
| `bin/ocelli.sh gate licences-wheels` | `licence_check.py installed` | locked env, CI `corpus-tooling` job on Linux, local macOS | OK line or each problem |
| `pin_and_size_check.py --require-redistribution`, `/release` step 5 | vendor `LICENSE` plus recorded hash | release profile | green after this story |
| `bin/ocelli.sh gate packages`, `scripts/package_check.py` | `REQUIRED` gains `NOTICE` | tarball contents | refusal if absent |
| `bin/ocelli.sh gate provenance` | widened manifest set and name extraction | floor, pre-commit `--staged` | unchanged OK on today's tree |
| `bin/ocelli.sh gate ci`, `scripts/ci_floor_check.py` | `NOT_IN_FLOOR` gains `licences-wheels` | set equality with the `--floor` case line | pass |
| `bin/ocelli.sh gate guards`, census | catalogue entries, recorded constants, budget | floor | pass |
| `examples/viewer-react` `vite build`, `npm run test`, lint, typecheck | vite 7.3.7, plugin-react 5.2.0 (Q2) | frontend CI job | pass |

Owner of selection: the route table in the test module owns which decoder reads which syntax. Owner of colour conversion for `.50`: libjpeg-turbo inside `jpeg8_decode`, and the helper applies no second conversion. Owner of the allowlist: `deny.toml`. Owner of exceptions: `ci/licences.toml`, written only with an operator decision recorded in the row.

## State transitions and invariants

Not applicable as a stateful feature. Every check is a pure function of tracked files plus, for `installed`, the installed environment. The one invariant that spans consumers: the allowlist is read from one file by all three readers, and the census records it so a widening appears in a diff.

## Ownership, commit and recovery

Not applicable. No runtime resource, frame or device is acquired. The only staged state is the lockfiles, and regenerating them is an ordinary edit whose result the gates check.

## Numbered acceptance inventory

| ID | Trigger and expected behaviour | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | `pyproject.toml` and `uv.lock` contain no `pylibjpeg-libjpeg` and pin `imagecodecs==2026.10.10`. `uv lock --check` exits 0 | `licence_check.py python`, `ToolchainPins`, grep | leaving it as a transitive dependency of `pylibjpeg` | implementation |
| AC2 | Every `SYNTAX_CASES` syntax has exactly one declared read-back route, and no route names `gdcm` or `pylibjpeg-libjpeg` | `test_every_syntax_has_one_declared_decoder` | relying on pydicom's default plugin order, which puts GDCM first (fact 2) | implementation |
| AC3 | The installed imagecodecs version and its libjpeg-turbo version equal a recorded `DECODED_WITH` table | `test_reader_versions_match_decoded_with` | a silent imagecodecs bump moving lossy digests unattributed | implementation |
| AC4 | `.57` and `.70` decode exactly equal to the uncompressed reference through `ljpeg_decode` | `test_lossless_syntaxes_round_trip_exactly` | a tolerance quietly introduced for lossless rows. Mutation: add 1 to one decoded sample, the test goes red | implementation, mutation retained |
| AC5 | `.50` and `.51` stay within the unchanged `YBR_EIGHT_BIT_MAX` and `TRANSFORM_MAX`. The bound comments are updated to the re-measured worst pixel under imagecodecs. If either measured value exceeds its bound, implementation stops and asks, and no constant is raised | `test_lossy_syntaxes_decode_to_the_right_shape_and_stay_close` | a widened bound to make a new decoder pass. A double YBR conversion. Mutation: apply `convert_color_space` a second time to `.50`, the test goes red | implementation, mutation retained |
| AC6 | Regression, decode digests. On the local corpus, after manifest digests match, the SHA-256 of each decoded array (dtype, shape and bytes) is recorded for every `SYNTAX_CASES` row. Measured once under the current GPL environment and once under the new one: every row except `.50` and `.51` is byte-identical, including the three JPEG-LS rows. `.50` and `.51` are re-baselined, and their maximum absolute difference against the old decode is recorded (lead: 3 and 1). The new digests are written to `corpus/decode-digests.tsv` and checked by `corpus_check.py --decode-digests` in `gate corpus` | before and after tables in the handoff, then `gate corpus` | a decoder change that moves a lossless or JPEG-LS row. A digest table checked against files whose own digest is not first verified | implementation (local, corpus present), then permanent local gate |
| AC7 | `deny.toml` carries the allowlist. `cargo deny --locked check licenses` passes with all features over the full graph including dev-dependencies | `gate licences` | an allowlist containing a copyleft identifier, `exclude-dev = true` | implementation |
| AC8 | Rust refusal. In the sandbox, `vendor/openjph-core-0.1.0/Cargo.toml` `license = "BSD-2-Clause"` becomes `GPL-3.0-only`, and `licence_check.py rust` exits 1 naming the crate | probe `licences.rust-copyleft`, deep, needs cargo | a reader that checks only registry crates or only workspace members | guards-deep |
| AC9 | Rust disjunction accepted. The same crate as `MIT OR GPL-3.0-only` passes | probe `licences.rust-disjunction`, accept polarity | treating any copyleft token as fatal, which would also refuse `r-efi` | guards-deep |
| AC10 | cargo-deny pin. A version other than 0.19.0, or none, fails the gate with the install command | unit test with a stub on `PATH` | a skip, or an unpinned tool whose config schema drifts | implementation |
| AC11 | npm refusal. A `package-lock.json` entry added with `"license": "GPL-3.0-only"` is refused, in each of the three lockfiles | probe `licences.npm-copyleft`, floor, plus unit tests | reading only the root lockfile | guards |
| AC12 | npm missing licence. An entry with no `license` field and no declaration in `ci/licences.toml` is refused. `rechoir@0.6.2` is declared `MIT` with the evidence that its `package.json` uses the legacy `licenses` array | probe `licences.npm-undeclared`, unit test | treating an absent field as permissive | guards |
| AC13 | Exception rules. An exception missing `reason`, `scope`, `decided` or `decided_by` is refused. A stale exception is refused. An exception on a package in the published runtime closure is refused even with scope `dev-only` | probes `licences.exception-stale`, `licences.exception-shipped`, unit tests | an exceptions file that accumulates entries nobody re-reads | guards |
| AC14 | SPDX evaluation from Annex D. `MIT OR Apache-2.0 AND GPL-3.0-only` accepted (AND binds first). `(MIT OR Apache-2.0) AND GPL-3.0-only` refused. `Apache-2.0 WITH LLVM-exception` accepted only as the allowlisted compound. `GPL-2.0-only WITH Classpath-exception-2.0` refused. `mit` accepted, `MIT or Apache-2.0` refused as unparseable. `LicenseRef-x`, `UNLICENSED`, `SEE LICENSE IN x` and the empty string refused | `test_spdx_*` in `test_licence_check.py` | left-to-right evaluation, which accepts the second and refuses the first. Case-insensitive operators | implementation |
| AC15 | Python table complete and allowed. Every `uv.lock` package except the virtual project has a row whose expression is allowed or excepted. A `uv.lock` package with no row, or a row changed to `GPL-3.0-only`, is refused | probes `licences.python-unrecorded`, `licences.python-copyleft` | trusting uv.lock, which records no licence | guards |
| AC16 | Installed wheels. Metadata strings match the table. A copyleft marker in any bundled licence text, an empty licence file, or an unrecorded bundled shared library is refused unless an exception names that file. An unrecorded platform is refused | level-1 probes over a synthetic `site-packages` built in a temporary directory (`licences.wheel-marker`, `licences.wheel-library`, `licences.wheel-platform`), then real runs | checking `License-Expression` only, which passes numpy on Linux (fact 5) | guards, then `licences-wheels` |
| AC17 | Real installed runs pass with only operator-approved exceptions: macOS arm64 locally and manylinux x86_64 in the `corpus-tooling` job. The Linux run must list numpy's `libgfortran` and `libquadmath` and imagecodecs' `libgomp` as named `tool` exceptions, and imagecodecs' `LICENSE-lzf` as `BSD-2-Clause` with its GPLv2 relicensing notice recorded as an alternative not taken, or fail | `gate licences-wheels` on both hosts | a gate green on macOS that has never seen the Linux wheels | integrated (local) and external (CI run on the pushed head) |
| AC18 | Vite. `package-lock.json` has zero `lightningcss` entries. `npm run test`, `npm run lint`, `npm run typecheck`, `npm run build --workspaces --if-present` and `vite build` in `examples/viewer-react` pass | `gate lint types packages`, frontend CI | upgrading around it with an `overrides` stub | implementation, conditional on Q2 |
| AC19 | Notices. `THIRD_PARTY_NOTICES` has an OpenJPH section whose body is byte-identical to the upstream file (SHA-256 `5ddf5177...1f98`) and to `vendor/openjph-core-0.1.0/LICENSE`, a note naming the port (openjph-core 0.1.0, published by Michael Knopke, BSD-2-Clause in registry metadata, no notice in the archive), and a ritk-codecs section byte-identical to the vendored `LICENSE-MIT` with a pointer to `LICENSE-APACHE` | `licence_check.py notices`, probe `licences.notice-drift` | a hand-retyped notice that differs in one character | guards |
| AC20 | Separation. No third-party section of `THIRD_PARTY_NOTICES` names Tensorbee, and the project's own copyright appears only in `LICENSE`, `LICENSE-MIT` copies and `NOTICE` | `licence_check.py notices` | mixing the project's grant into the third-party file | guards |
| AC21 | Redistribution. `pin_and_size_check.py --require-redistribution` exits 0 and the vendor `LICENSE` hash is recorded as `OPENJPH_LICENCE_SHA256`. Probe `pins.openjph-notice` is rewritten: control unmodified accepts, mutation deletes the vendor `LICENSE` and must refuse with "carries no BSD notice", a second mutation changes one byte and must refuse on the hash | `gate guards`, `/release` step 5 | leaving the old two-way probe, which now fails because its control accepts | guards |
| AC22 | Holder and SPDX. Every `LICENSE-MIT` reads `Copyright (c) 2026 Tensorbee Limited`. `NOTICE` exists at the root, in `packages/core` and in `packages/react` naming `Tensorbee Limited`. `LICENSE` names the holder. `LICENSE-APACHE` is byte-identical to its current blob `d6456956...`. Every tracked `Cargo.toml` resolves to `MIT OR Apache-2.0` (workspace value, every member `license.workspace = true`), every tracked `package.json` declares `MIT OR Apache-2.0`, and `pyproject.toml` does. The SPDX expression is not changed anywhere | `licence_check.py holder`, probes `licences.holder-drift`, `licences.manifest-spdx` | a copy left at "Tensorbee", a manifest narrowed to `MIT` | guards |
| AC23 | Tarballs carry `NOTICE`. `@ocelli/core` and `@ocelli/react` `files` include it and `package_check.py` `REQUIRED` requires it | `gate packages` | a NOTICE in the repository that never reaches the published package | implementation |
| AC24 | Provenance manifests. `pyproject.toml` and `uv.lock` are manifests. Dependency names are extracted from Cargo.toml keys, `name = "..."` entries in `Cargo.lock` and `uv.lock`, PEP 508 strings in `[project] dependencies`, `package.json` dependency keys and `package-lock.json` `node_modules/...` keys. A name whose final segment begins with a read-blocked project's name, optionally after `lib`, is refused | probes `provenance.python-dependency`, `provenance.lock-entry`, `provenance.prefixed-name`, unit tests | the whole-word match that misses `grokj2k` (not permitted), and the current hole where a `Cargo.lock` entry is never matched | guards |
| AC25 | Provenance false-positive controls. `python3 scripts/source_provenance_check.py` stays OK over today's tree. A tracked review file containing the agent trailer `Ocelli-Generated-By: grok` (the agent, not the blocked codec) produces the same result before and after the change, because prose matching is untouched. The name `grokj2k` in a `.md` file is not a manifest hit | unit tests, accept probe `provenance.agent-trailer` | widening the prose rule and flagging ninety-three review files | guards |
| AC26 | Gates wired. `licences` is in `GATES`, in the floor, and run by `bin/ocelli.sh gate licences` in the CI `guards` job after cargo-deny 0.19.0 is installed with `cargo install --locked`. `licences-wheels` is in `GATES`, excluded in the `--floor` case line and in `NOT_IN_FLOOR`, and run in the `corpus-tooling` job after `uv sync --locked`. The CI `provenance` step becomes `bin/ocelli.sh gate provenance`, because its arm gains a unit-test command | `gate ci` | a floor gate CI never runs, which `ci_floor_check.py` refuses | guards |
| AC27 | Guard registration. A `licences` guard entry with `claims=("*",)` covers every refusal site in `scripts/licence_check.py`. New recorded constants: the `deny.toml` allow list, the provenance `MANIFESTS`, the copyleft marker list and the exception field set. `guard_census.py --record` re-recorded, runbook table regenerated with `--render-runbook`, `--check-runbook` clean | `gate guards`, `gate guards-deep` | a guard added without a probe, a widened list nobody sees | guards |
| AC28 | Every gate proved to refuse in a scratch copy. For each of Rust, npm and Python: a copyleft dependency added in the sandbox turns `gate licences` red with the intended message, and the unmodified sandbox is green. Recorded as old-source RED and repaired-source GREEN in the handoff | AC8, AC11, AC15 probes plus one manual run each | a probe whose red is an unrelated failure | guards, recorded |
| AC29 | Documentation. `corpus/README.md` prerequisites row, `.claude/skills/dicom-tooling/SKILL.md` and its generated adapters, `docs/DEVELOPER_SETUP.md` (cargo-deny 0.19.0), `docs/SOURCE-POLICY.md` rows for imagecodecs and for openjph-core now shippable, `README.md` Licence section | `gate skills prose provenance` | a README still telling a developer to install the GPL plugin | implementation |

## Approach

Order, smallest first, each step green before the next.

1. **Baseline evidence (AC6).** With the current `.venv`, record the decode digest of every `SYNTAX_CASES` row on the local corpus, through the exact pydicom route each one takes today. Keep the arrays for `.50` and `.51`.
2. **SPDX evaluator and readers.** `scripts/licence_check.py` with a recursive-descent parser over the Annex D grammar (`OR` lowest, then `AND`, then `WITH`, parentheses), operators case-sensitive, identifiers case-insensitive against the allowlist. Satisfiable means some disjunct has every conjunct allowed. A `WITH` compound is allowed only as a whole allowlisted compound. Unit tests first, from AC14.
3. **npm** reads `packages` from each tracked `package-lock.json`, skips the root `""` entry, resolves `link: true` entries to their workspace `package.json`, and computes the published runtime closure by walking non-dev `dependencies` from `packages/core` and `packages/react`.
4. **Python** reads `uv.lock` with `tomllib` and the table in `ci/licences.toml`. `installed` lists dist-info directories through the resolved interpreter and applies fact 4's scan.
5. **Rust** runs cargo-deny and translates its JSON diagnostics into problems, after the version check. The shipped-closure walk for AC13 uses `cargo metadata --format-version 1 --filter-platform wasm32-unknown-unknown` for `ocelli-wasm` and the host platform for `ocelli-native`, on normal and build edges.
6. **Python decoder swap.** Edit `pyproject.toml`, `uv lock`, `uv sync --locked`. Add the route helper, `DECODED_WITH`, and the two new tests. Re-measure the lossy bounds and update only the comments. Run step 1 again and write `corpus/decode-digests.tsv`.
7. **Notices and holder.** Fetch the OpenJPH file, check its digest, place it in the vendor tree and in `THIRD_PARTY_NOTICES`, record the hash, rewrite the probe. Edit the four `LICENSE-MIT` regular files (the ocelli-wasm one is a symlink and follows), add the three `NOTICE` files, update `files` and `REQUIRED`.
8. **Vite (Q2).** `npm install --save-dev vite@^7.3.7 @vitejs/plugin-react@^5.2.0`, confirm no `lightningcss` in the lock, run the frontend gates.
9. **Provenance widening.** Replace the two manifest regexes with per-grammar name extraction, add the two manifests, add the prefix rule, add the unit test file to the `provenance` arm.
10. **Registration.** Gate rows and arms, `NOT_IN_FLOOR`, CI steps, catalogue entries and probes, census record, runbook render, `ci/guard-probe-budget.json` timings re-recorded.

Rejected alternatives:

- **python-gdcm**, alone (refuses `.51`, fact 1) or as a cross-check (fact 2 side effects on plugin order and the RLE encoder).
- **DCMTK `dcmdjpeg`** as reader. Permissive, but it is the encoder's own toolkit, so a symmetric defect would round-trip unseen.
- **Pillow.** Refuses 12-bit (fact 1).
- **Ocelli's own decoders** as the Python reader. They are the system under test.
- **A pydicom plugin wrapping imagecodecs.** It would register globally and take part in plugin order, the exact hazard of fact 2. A test-local helper does not.
- **A licence scanner package** (`license-checker` style or `pip-licenses`). A new dependency for what a stdlib reader over lockfiles does, and those tools read installed `package.json` fields only, which would miss the bundled-library case.
- **Retyping the OpenJPH notice.** CLAUDE.md forbids re-typing file content from tool output. It is fetched and digest-checked.

## Dependencies, write ownership and resources

No declared dependency. Write set:

```text
deny.toml                                   new
ci/licences.toml                            new
THIRD_PARTY_NOTICES                         new
NOTICE, packages/core/NOTICE, packages/react/NOTICE   new
corpus/decode-digests.tsv                   new, synthetic-case decode digests only
scripts/licence_check.py                    new
scripts/tests/test_licence_check.py         new
scripts/tests/test_source_provenance_check.py   new
vendor/openjph-core-0.1.0/LICENSE           new, a permitted addition
LICENSE, LICENSE-MIT, packages/core/LICENSE-MIT, packages/react/LICENSE-MIT
pyproject.toml, uv.lock
package.json, package-lock.json, examples/viewer-react/package.json
packages/core/package.json, packages/react/package.json
scripts/tests/test_corpus_synth.py, scripts/corpus_check.py
scripts/source_provenance_check.py, scripts/pin_and_size_check.py, scripts/package_check.py
scripts/ci_floor_check.py, bin/ocelli.sh, .github/workflows/ci.yml
scripts/guards/catalogue.py, ci/guard-probe-budget.json, docs/runbooks/guard-verification.md
corpus/README.md, .claude/skills/dicom-tooling/SKILL.md, .agents/skills/dicom-tooling/SKILL.md (generated)
docs/DEVELOPER_SETUP.md, docs/SOURCE-POLICY.md, docs/hld/DEVIATIONS.md (D-22 row, on approval), README.md
docs/lld/guards.md, docs/lld/corpus.md, docs/lld/release-policy.md, docs/lld/typescript-packaging.md
```

Exclusive files, no concurrent writer while this story is open: `bin/ocelli.sh`, `.github/workflows/ci.yml`, `scripts/ci_floor_check.py`, `scripts/guards/catalogue.py`, `ci/guard-probe-budget.json`, `docs/runbooks/guard-verification.md`, `package-lock.json`, `uv.lock`. Any S46 story touching the gate runner, the catalogue or a lockfile serialises after this one. F-216's release build may touch `package.json` and must rebase on the Vite change.

Network is needed once, for `uv lock`, `npm install`, the OpenJPH fetch and `cargo install cargo-deny`. No GPU, no browser, no server.

**Sprint build order, decided in the S46 design round on 2026-11-05.** This story is first: it is tooling and writes no folder-viewer file. Then F-216, F-210, F-211, F-208, F-209, F-212, F-213, F-214 and F-215. F-119 is independent and may run in parallel. Landing first also settles the Vite change before F-216 or any later story touches `package.json`.

## Verification and evidence reuse

Focused, in order:

```bash
python3 -B -m unittest discover -s scripts/tests -p test_licence_check.py
python3 -B -m unittest discover -s scripts/tests -p test_source_provenance_check.py
python3 scripts/licence_check.py
bin/ocelli.sh gate licences licences-wheels provenance pins packages lint types
bin/ocelli.sh gate corpus-tests corpus
bin/ocelli.sh gate ci guards guards-deep skills prose deviations
python3 scripts/pin_and_size_check.py --require-redistribution
```

Final profile: `bin/ocelli.sh gate --sprint`, recorded by `/verify` against the staged tree.

Identities: imagecodecs 2026.10.10 with its reported libjpeg-turbo version, pydicom 3.0.2, cargo-deny 0.19.0, vite 7.3.7, `@vitejs/plugin-react` 5.2.0, upstream OpenJPH `LICENSE` SHA-256 `5ddf5177...1f98`, corpus manifest digests unchanged.

What invalidates evidence: any lockfile regeneration after the AC6 measurement, an imagecodecs or libjpeg-turbo version change, or a change to `deny.toml` or `ci/licences.toml` after the guard budget is recorded.

External observation: AC17's Linux half is observed only by the `corpus-tooling` job on the pushed head. Until that run exists, AC17 is open, not passed.

Baseline policy: `corpus/decode-digests.tsv` is new. Its `.50` and `.51` rows are a re-baseline accepted under AC5's bound, and its other rows must equal the pre-change measurement.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a, repository tooling, no rendering or compute
- Tier B (WebGL2): n/a, repository tooling, no rendering or compute
- Tier C (CPU): n/a, the corpus read-back is Python test tooling and not an Ocelli code path

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| conformance | lossless rows exact, lossy rows within unchanged bounds, through declared routes | `scripts/tests/test_corpus_synth.py` |
| unit | every syntax has one route, reader versions pinned | `scripts/tests/test_corpus_synth.py` |
| regression | decode digests per row on the verified local corpus | `scripts/corpus_check.py --decode-digests`, `gate corpus` |
| unit | SPDX Annex D precedence, case rules, WITH, refusals | `scripts/tests/test_licence_check.py` |
| unit | npm, Python table, installed scan, exceptions, holder, notices, each refusal and its control | `scripts/tests/test_licence_check.py` |
| unit | manifest name extraction for six grammars, prefix rule, prose rule unchanged, agent trailer | `scripts/tests/test_source_provenance_check.py` |
| guard probe | every refusal driven red in the sandbox, accept probes for disjunction and the agent trailer | `scripts/guards/catalogue.py`, `gate guards`, `gate guards-deep` |
| mutation | AC4 and AC5 retained red results | handoff |

No `fixture` row is owed. No Ocelli function doing pixel arithmetic is added or changed. The lossy bounds are sanity checks on corpus cases, and the decoded values come from a third-party reader.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no licence or tooling row, and no rendered output changes.

## Deviations

No new deviation. D-22's clause "that assertion fails until the notice material is obtained" becomes historical, and its row gains one sentence recording that F-217 placed the notice and that `--require-redistribution` now passes. That sentence records an outcome, so it is written at `/complete-feature`, after the notice is placed and the check is observed green, and not at approval. `/design` step 3 governs new rows, and this is an amendment to an existing one.

## LLD impact

- `docs/lld/guards.md`: the `licences` guard, its probes, the new recorded constants, the rewritten `pins.openjph-notice`.
- `docs/lld/corpus.md`: the read-back route table, `DECODED_WITH`, the decode-digest check, the independence note from Q5.
- `docs/lld/release-policy.md`: `THIRD_PARTY_NOTICES`, `NOTICE`, the redistribution check now green.
- `docs/lld/typescript-packaging.md`: `NOTICE` in both tarballs, the Vite line.

## User-facing documentation impact

- `README.md` Licence section: name Tensorbee Limited as copyright holder, point to `NOTICE` and `THIRD_PARTY_NOTICES`, keep the dual-licence wording unchanged.
- `examples/folder-viewer/src/use.html` and `examples/folder-viewer/src/more.html`: no change. Neither page carries licence or attribution text, and the folder viewer is a local example that is not distributed.

## Open questions

**Q1, answered.** MIT only, or keep `MIT OR Apache-2.0`. The operator's answer, relayed by the lead during this design round on 2026-11-05: keep the dual licence `MIT OR Apache-2.0` everywhere, because Apache-2.0 is wanted so the work can be copyrighted and commercialised in future. The copyright holder is Tensorbee Limited. Every LICENSE file and any NOTICE file names it (AC22, AC23), the third-party notices file stays separate from the project's own copyright (AC20), and the SPDX expression itself is not changed. Recorded as decided, not open.

**Q2, lightningcss.** Affects AC18. Options: (a) pin `vite ^7.3.7` with `@vitejs/plugin-react ^5.2.0`, where lightningcss is an optional peer npm does not install (fact 7), or (b) keep Vite 8 and record `lightningcss` and its platform binaries as a `build-only` exception, since MPL-2.0 is file-level and CSS it transforms is not a covered work. Recommendation (a), because the operator's rule is "at any cost" and (a) costs only the previous major (esbuild and Rollup instead of Rolldown) for a private example viewer and vitest. Consequence of (a): a later Vite 8 upgrade is refused by the gate until Vite drops the hard dependency or the operator approves (b). Consequence of deferral: the gate cannot be green without (b)'s exception. Decision owner: operator.

**Q3, copyleft runtime libraries inside permissive Python wheels.** Affects AC16 and AC17. On Linux, numpy 2.5.2 bundles `libgfortran` (GPL-3.0-or-later WITH GCC-exception-3.1) and `libquadmath` (LGPL-2.1-or-later), and imagecodecs bundles `libgomp` (GCC runtime exception). imagecodecs' `LICENSE-lzf` offers GPLv2 as an alternative to BSD-2. None is in a tracked manifest by name, all are dynamically loaded, unmodified, used by tooling only and never redistributed. There is no practical permissive numpy: pydicom's pixel API requires it, and a source build without OpenBLAS would be a new build system for CI. Options: (a) record each as a `tool` exception with that reason, or (b) treat the rule as absolute and drop Python pixel tooling from CI, which removes the corpus conformance job. Recommendation (a). Consequence of deferral: `licences-wheels` cannot pass on Linux, so AC17 stays open. Decision owner: operator.

**Q4, the scope boundary.** Affects the non-goals. Playwright downloads Chromium and ffmpeg (LGPL-2.1), and Chromium itself is built with LGPL components, so a browser-hosted oracle cannot be copyleft-free whatever drives it. Git, bash and the CI images are GPL tools. Recommendation: the rule covers dependencies declared in tracked manifests and lockfiles, plus libraries those dependencies bundle, and excludes tools that are downloaded or installed on a developer or CI host and never redistributed. Record the boundary in `docs/SOURCE-POLICY.md`. Consequence of deferral: the story's claim "no copyleft dependency" has an unstated edge. Decision owner: operator.

**Q5, read-back independence.** Affects AC2 and AC6. After the change, JPEG-LS rows are read back by CharLS (pyjpegls), the library that encoded them, where today Richter's libjpeg reads them independently. The two DCT rows are read by libjpeg-turbo, which shares IJG ancestry with DCMTK's encoder. The lossless JPEG rows are read by liblj92, which is independent. Every permissive JPEG-LS decoder available to Python is CharLS. The Rust decoders and the cornerstone3D oracle remain independent checks of the same files. Options: (a) accept and record in `docs/lld/corpus.md`, or (b) add Ocelli's own JPEG-LS decoder as a second reader, which is the system under test. Recommendation (a). Decision owner: operator, because it weakens existing evidence.

**Q6, full attribution for shipped binaries.** Affects AC19. MIT, BSD and Apache-2.0 crates compiled into a shipped wasm or native binary require their notices to accompany it. No artefact that links `ocelli-codec` ships today, and `ocelli-wasm` is `publish = false`. Options: (a) F-217 delivers vendored notices only and a follow-up story generates full texts for every crate in the shipped closure (cargo-about 0.8, MIT OR Apache-2.0), or (b) F-217 adds the generator now, which roughly doubles its estimate. Recommendation (a), with the follow-up filed before this story closes. Decision owner: operator.

**Q7, exception approval.** Affects AC12 and AC13. The exceptions this plan expects to write, each needing an operator decision recorded in its row: `caniuse-lite` (CC-BY-4.0 data, dev-only), `rechoir@0.6.2` declared MIT (legacy `licenses` array, tool), the Q3 libraries if (a), and lightningcss only if Q2 is (b). Decision owner: operator, in the same round as Q2 and Q3.

### Answers

Given by the operator on 2026-11-05 in the S46 consolidated design round and relayed by the sprint lead.

- Q1. Answered earlier in the same round: keep `MIT OR Apache-2.0`, copyright holder Tensorbee Limited.
- Q2. Answered by the operator: (a), move to `vite ^7.3.7` and `@vitejs/plugin-react ^5.2.0`. No `lightningcss` exception is written.
- Q3. Answered by the operator: (a), record the Linux bundled runtime libraries as named `tool` exceptions: numpy's `libquadmath` (LGPL-2.1-or-later), numpy's `libgfortran` and imagecodecs' `libgomp` (both GPL-3.0-or-later WITH GCC-exception-3.1, the GCC runtime library exception). imagecodecs' `LICENSE-lzf` is not one of the three. It is recorded as `BSD-2-Clause`, the licence it is used under, with its GPLv2 relicensing notice noted as an alternative not taken, which is fact 4's reading. AC17 says so.
- Q4. Answered: recommended, the rule covers dependencies declared in tracked manifests and lockfiles plus the libraries they bundle, and excludes tools downloaded or installed on a developer or CI host and never redistributed. The boundary is recorded in `docs/SOURCE-POLICY.md`.
- Q5. Answered by the operator: (a), accept the JPEG-LS read-back independence loss and record it in `docs/lld/corpus.md`.
- Q6. Answered: recommended (a), vendored notices only in this story, and a follow-up story for full attribution texts of the shipped closure (cargo-about 0.8) is filed before this story closes.
- Q7. Answered: recommended, the expected exceptions are approved in this round and each row records that decision: `caniuse-lite` (CC-BY-4.0 data, dev-only), `rechoir@0.6.2` (legacy `licenses` array, tool), and the three Q3 libraries. `lightningcss` is not excepted, because Q2 is (a).

No question is left open.
