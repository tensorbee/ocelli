# F-220, Shipped binary notices

**Status**: approved
**Epic ref**: E38.30
**Sprint**: S46
**Estimate**: 2w

Story row, `docs/sprints/BACKLOG.md`: "Shipped binary notices: the full licence text of every crate compiled into the wasm module and the native binaries, generated from the lockfile and checked by a gate". Layer Build, depends on F-217, milestone M20.

`allocation.json` notes: "F-217's plan question 6 recommended a follow-up. THIRD_PARTY_NOTICES covers the vendored sources only, while MIT and BSD grants in every compiled crate also require their notice on redistribution."

## Normative source, transcribed

The HLD says nothing about notices for compiled dependencies. `grep -n -i 'notice' docs/hld/*.md` finds no obligation outside `DEVIATIONS.md`. The sources below are the project's policy and F-217's decisions.

### `docs/SOURCE-POLICY.md`, section "Third-party obligations", verbatim

"Track them from day one. Bridging to `@cornerstonejs/codec-charls` is fine and so are OpenJPEG, CharLS and OpenJPH, all permissive. Grok is not. Attribution files are cheap to maintain incrementally and expensive to reconstruct."

### F-217's question 6 and its answer, `.claude/plans/F-217-design.md`

Question, verbatim: "**Q6, full attribution for shipped binaries.** Affects AC19. MIT, BSD and Apache-2.0 crates compiled into a shipped wasm or native binary require their notices to accompany it. No artefact that links `ocelli-codec` ships today, and `ocelli-wasm` is `publish = false`. Options: (a) F-217 delivers vendored notices only and a follow-up story generates full texts for every crate in the shipped closure (cargo-about 0.8, MIT OR Apache-2.0), or (b) F-217 adds the generator now, which roughly doubles its estimate."

Answer, verbatim: "Q6. Answered: recommended (a), vendored notices only in this story, and a follow-up story for full attribution texts of the shipped closure (cargo-about 0.8) is filed before this story closes."

F-217 decision 2, the shipped closure, verbatim: "any crate reachable on normal and build edges from `ocelli-wasm` or `ocelli-native`". Its implementation is `_rust_shipped` in `scripts/licence_check.py`: `cargo metadata --format-version 1 --locked` with `--filter-platform wasm32-unknown-unknown` for `ocelli-wasm` and no platform filter for `ocelli-native`, keeping a dependency edge whose `dep_kinds` include kind `null` or `"build"`.

F-217 decision 1, verbatim: "**One allowlist, three readers.** `deny.toml` `[licenses] allow` is the single list."

F-217 decision 8 placed the vendored notices in `THIRD_PARTY_NOTICES`, "the file a distributor reads", and `licence_check.py notices` refuses that file when it names Tensorbee.

### `docs/RELEASE.md`, what ships

"`@ocelli/core` ships the wasm module built from the crates". Preflight item 6, verbatim: "`npm pack` for each published package, with its contents listed. The wasm module must be present in `@ocelli/core` and must be the one this HEAD builds, not a stale artefact from a previous build." And: "`ocelli-native` is `publish = false` for as long as it stays stubbed, because publishing it would claim Phase 2 and Phase 3 entry points that do not exist yet."

### `docs/lld/release-policy.md`, verbatim

"`THIRD_PARTY_NOTICES` carries the upstream OpenJPH notice and the vendored ritk-codecs MIT grant. `NOTICE` names Tensorbee Limited and is published with both npm packages. `python3 scripts/pin_and_size_check.py --require-redistribution` passes."

### The licence conditions being met, cited from the licence texts

MIT: "The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software." BSD-2-Clause condition 2, as F-217 transcribed it: the notice is reproduced "in the documentation and/or other materials provided with the distribution". Apache-2.0 section 4 requires a copy of the licence, and section 4(d) requires the attribution notices of any `NOTICE` file the work carries.

### The Cargo reference on the legacy separator

Cargo's manifest reference for `license` records that multiple licences were once separated with `/`, a usage it calls deprecated, read as `OR`. Cited, not quoted.

## Facts measured for this plan

Measured on 2026-11-05 on this branch, with `cargo metadata --locked`, the local registry cache, and toolchain 1.97.1.

1. **The closure is 129 third-party crates.** 76 are reachable from `ocelli-wasm` for `wasm32-unknown-unknown`, 129 from `ocelli-native` with no platform filter, and 83 from `ocelli-native` filtered to `aarch64-apple-darwin`. The union is the unfiltered native set. Workspace members are excluded. 10 of the 129 are proc-macro crates.
2. **Neither vendored crate is in the closure today.** `openjph-core` and `ritk-codecs` are path dependencies of `ocelli-codec`, which neither entry point reaches, which matches F-217's Q6 sentence. The generator must still handle a path source.
3. **113 crates ship at least one licence file**, giving 80 distinct texts and 279,928 bytes when identical texts are kept once. File names seen include `LICENSE-MIT`, `LICENSE-APACHE`, `LICENSE.MIT`, `license-apache-2.0`, `LICENSE`, `LICENSE-ZLIB`, `COPYING`, `UNLICENSE`, `LICENSE.txt`, `LICENSE-BSD`, `LICENSE-MIT.md`, `LICENSE-UNICODE`, `NOTICES.md` and `COPYRIGHT`. Every one decodes as UTF-8. Five contain CRLF line endings. None names Tensorbee, dwv, Horos or Grok.
4. **16 crates ship no licence file**: `block2`, `dispatch2`, `gl_generator`, `jni-sys-macros`, `khronos_api`, `ndk-sys`, `objc2`, `objc2-core-foundation`, `objc2-core-graphics`, `objc2-encode`, `objc2-foundation`, `objc2-io-surface`, `objc2-metal`, `objc2-quartz-core`, `profiling` and `spirv`. Their expressions are `MIT`, `Apache-2.0`, `MIT OR Apache-2.0` and `Zlib OR Apache-2.0 OR MIT`. Three crates in the closure use the legacy `/` separator and all three ship files.
5. **The Rust standard library is compiled into both artefacts and is not in `Cargo.lock`.** Toolchain 1.97.1's `rust-docs` component installs `share/doc/rust/COPYRIGHT-library.html`, 279,302 bytes, SHA-256 `0a65bb747c49c7bb816cbc7188319bd6e4e8d08091c1190b8a3c0971c47968ed`, titled "Copyright notices for The Rust Standard Library". It covers the library's out-of-tree dependencies, and names `dlmalloc` and `hashbrown`. `rust-toolchain.toml` pins `channel = "1.97.1"` with `profile = "minimal"`, which does not install `rust-docs`, so CI cannot read this file at gate time.
6. **No tool beyond cargo-deny is installed.** `cargo about` is not a command here and `cargo-license` is absent. `cargo deny list` (0.19.0) prints licence identifiers and the crates using them, and no text.
7. **No shipped artefact carries compiled Rust today.** `npm pack` of `@ocelli/core` lists `dist` (tsc output), `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, `README.md` and `package.json`, and no `.wasm`, although RELEASE.md item 6 says it must. `ocelli-native` is `publish = false` and nothing builds a native release archive.

## What the specification does not cover

All of it. Every item is this plan's decision, made by the lead under the operator's delegation of 2026-11-05.

1. **A stdlib generator inside `scripts/licence_check.py`, not cargo-about.** It reuses F-217's closure walk, SPDX parser and allowlist, so the notices cover exactly the set the licence gate checks. cargo-about is not installed (fact 6), would be a second pinned tool with its own config schema and template, and would compute a second closure that could drift from `_rust_shipped`. cargo-about's own licence was not inspected in this session, because it is not chosen.
2. **One closure provider.** `_rust_shipped` is refactored to return package records, not names, and both the exception check and the generator call it. The closure stays F-217's: `ocelli-wasm` on wasm32 plus `ocelli-native` on every platform, normal and build edges, workspace members excluded. The unfiltered native set is deliberately over-inclusive. It makes the output identical on the macOS and Linux hosts that run the gate, which a host-filtered set (fact 1, 83 against 129) would not, and listing a notice for a crate a given binary does not link costs bytes, not compliance. Build-dependency and proc-macro crates are not linked into a binary, and they stay in because they are in F-217's closure and one definition is worth more than the bytes.
3. **A tracked generated file, `THIRD_PARTY_LICENSES`, at the repository root.** Tracked, so a lockfile change shows the licence change in the same diff and review sees it. `THIRD_PARTY_NOTICES` stays the hand-checked vendored-source file F-217 built, and the new file is separate because it is generated and its whole content is checked by regeneration, while the old one is checked section by section. The name has no `.md` suffix, so `prose_check.py` does not read licence texts. The header names the generating command, the closure definition and "do not edit".
4. **Which files count as a crate's licence.** In the crate's root directory, every file whose name begins, case-insensitively, with `LICENSE`, `LICENCE`, `COPYING`, `NOTICE`, `UNLICENSE` or `COPYRIGHT`, every file under a `LICENSES/` directory, and the manifest's `license-file` if set. Every such file is included, not a chosen subset, because choosing the file that matches one disjunct needs a text classifier, and including every file the author shipped is never short of the obligation. `NOTICE` files satisfy Apache-2.0 section 4(d).
5. **A crate with no licence file gets the standard text of its elected licence.** The expression is parsed with F-217's `parse_expression`, after the Cargo reference's `/` to `OR` reading when the expression has no operator. The elected licence is the left-most disjunct whose every term is allowlisted, and the text of each term of it is included. Today that is `MIT`, `Apache-2.0` and `Zlib`. The section records the crate's name, version, declared expression, the `authors` from its manifest, and the line "The published crate carries no licence file. The text below is the standard text of the licence it declares." An identifier with no pinned text is refused, naming the crate and the identifier.
6. **Standard texts are pinned and tracked, never fetched at gate time.** `ci/notices/spdx/<identifier>.txt` for `MIT`, `Apache-2.0` and `Zlib`, fetched once at implementation from the SPDX `license-list-data` repository at a release tag, with the tag, the source URL and each file's SHA-256 in `ci/notices/sources.toml`. Before fetching, the implementer applies `docs/SOURCE-POLICY.md`'s three questions to that repository and records its licence in `sources.toml`. If it is not permissive, the texts come from each licence steward's canonical copy instead, recorded the same way. The gate verifies every digest.
7. **The standard library notice is pinned the same way.** `ci/notices/rust-std/COPYRIGHT-library.html` is fact 5's file, byte for byte, with the toolchain version and SHA-256 in `sources.toml`. The gate refuses when the recorded toolchain differs from `rust-toolchain.toml`'s channel, with the instruction to install `rust-docs` and regenerate. The generated file embeds it verbatim in its own section.
8. **Format and determinism.** Crates sorted by name then version. A crate index lists every crate with its version, source (`crates.io` or the path relative to the repository) and declared expression. Texts follow, each distinct text once, keyed by the SHA-256 of its normalised bytes, preceded by the list of `name version file` entries that carry it, in the order of their first appearance in the index. Normalisation is exactly one rule: CRLF and lone CR become LF. No other byte changes. A non-UTF-8 file is refused, naming it. No absolute path, no `CARGO_HOME` and no timestamp appears in the output.
9. **The gate regenerates and compares bytes.** A new floor section `binaries` in `licence_check.py`, added to `FLOOR_SECTIONS`, so `gate licences` runs it with no change to `bin/ocelli.sh`, `scripts/ci_floor_check.py` or the gate list. `python3 scripts/licence_check.py --write binaries` writes the file. The check runs `cargo metadata --locked --offline`, and a missing registry cache is a failure that names `cargo fetch --locked`, never a skip. The CI `guards` job gains a `cargo fetch --locked` step before `gate licences`.
10. **Shipping is enforced where an artefact exists, and written down where none does.** `scripts/package_check.py` refuses a tarball that carries any `*.wasm` without `THIRD_PARTY_LICENSES` and `THIRD_PARTY_NOTICES`. No tarball carries a `.wasm` today (fact 7), so the rule is dormant and watched by a level-1 probe. The story that puts the wasm module into `@ocelli/core` adds both files to `files` and to the package directory, and this rule is what stops it forgetting. A native release archive must carry `LICENSE`, `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, `THIRD_PARTY_NOTICES` and `THIRD_PARTY_LICENSES`. Nothing builds one, so that rule is written into `docs/RELEASE.md` for the milestone that lifts `publish = false`, not enforced by code that has no input.
11. **Shipping it in `@ocelli/core` now was considered and declined.** The tarball carries no Rust code (fact 7), and a 600 KB licence file for code it does not contain would read as a claim that it does.

## Problem and reproducer

```bash
ls THIRD_PARTY_LICENSES                                  # absent
python3 scripts/licence_check.py binaries                # unknown section
grep -c 'objc2' THIRD_PARTY_NOTICES                      # 0, and 129 crates have no notice anywhere
```

Expected after this story: the file exists, it equals what the lockfile generates, `gate licences` is red when either side changes alone, and a tarball that ever carries the wasm module without it is refused.

## Scope, API and compatibility

In scope: the generator and the `binaries` check, the pinned inputs, the tracked output, the `package_check.py` rule, the CI fetch step, the guard probes, and the docs.

```text
python3 scripts/licence_check.py binaries              # floor, part of the default section list
python3 scripts/licence_check.py --write binaries      # regenerates THIRD_PARTY_LICENSES
```

`--write` refuses when `rust-docs` is absent and the recorded standard library file is stale. It never reaches the network itself.

Non-goals:

- Building the wasm module into `@ocelli/core`, and building native archives. Decision 10.
- npm dependencies. `@ocelli/core` depends on `dcmjs`, which a consumer installs with its own licence file. Nothing is bundled.
- Python tooling. It is never redistributed (F-217 Q4).
- The crates published to crates.io. A crate source archive contains no dependency, so it owes no third-party text. F-217 recorded their missing `LICENSE` files as a separate gap.
- Changing `THIRD_PARTY_NOTICES` or its check.

Compatibility: no product API, wire format, pixel path or export changes.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `licence_check.py binaries`, `gate licences` | `_rust_shipped` records, file collection, `parse_expression`, pinned texts, std notice | Floor, CI `guards` job after `cargo fetch --locked` | OK, or each problem |
| `licence_check.py --write binaries` | Same generator | A developer after a lockfile or toolchain change | `THIRD_PARTY_LICENSES` rewritten |
| `licence_check.py rust`, exception check | `_rust_shipped` records | Unchanged policy | Unchanged |
| `package_check.py check_tarball` | New wasm rule | `gate packages` | Refusal when a `.wasm` ships without both files |
| `/release` preflight 6 | `docs/RELEASE.md` | Release profile | The written rule, and the `gate packages` refusal |
| `source_provenance_check.py`, `prose_check.py` | Unchanged | Floor | The new tracked files pass, measured in fact 3 |

Owner of the closure: `_rust_shipped`. Owner of the file rule, the election and the format: the generator. Owner of pinned texts: `ci/notices/sources.toml`.

## State transitions and invariants

Not applicable as a stateful feature. The check is a pure function of `Cargo.lock`, the manifests, the registry cache, the pinned inputs and `rust-toolchain.toml`. Invariant: the tracked file equals the generator's output for the tracked lockfile, so neither can change alone.

## Ownership, commit and recovery

Not applicable. No runtime resource.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | `THIRD_PARTY_LICENSES` exists and `licence_check.py binaries` regenerates it byte-identical | `gate licences` | A file written once and never compared | Implementation |
| AC2 | Closure. The crate index lists exactly the crates of `_rust_shipped` minus workspace members, each once, 129 today. A dev-only crate such as `proptest` is absent | unit test over the real metadata, count in the handoff | A host-filtered native closure, which differs on macOS and Linux. A second closure walk | Implementation |
| AC3 | File rule. A synthetic crate directory holding `LICENSE-MIT`, `license-apache-2.0`, `NOTICE`, `LICENSES/Zlib.txt`, `README.md` and `src/lib.rs` yields the four licence files and not the other two | `test_binaries_collects_licence_files` | Matching `LICENSE*` only, which misses `COPYING`, `NOTICE` and `LICENSES/` | Implementation |
| AC4 | Expression-only crate. Synthetic metadata with `Zlib OR Apache-2.0 OR MIT` and no file yields the pinned `Zlib` text, the authors line and the no-file sentence. `MIT/Apache-2.0` with no file yields `MIT`. `GPL-3.0-only OR MIT` yields `MIT`. `BSD-3-Clause` with no pinned text is refused naming the crate and `BSD-3-Clause` | `test_binaries_elects_*`, probe `licences.binaries-no-text` | Electing the first disjunct without the allowlist. Silently omitting a crate with no file | Implementation, guards |
| AC5 | Pinned inputs. A changed byte in `ci/notices/spdx/MIT.txt` or in `ci/notices/rust-std/COPYRIGHT-library.html` is refused on its SHA-256 | probe `licences.binaries-pin`, level 1 | Trusting a tracked text nobody can trace | guards |
| AC6 | Toolchain. `rust-toolchain.toml` channel changed to `1.98.0` in the sandbox makes `binaries` refuse with the regenerate instruction | probe `licences.binaries-toolchain` | A standard library notice that silently ages | guards |
| AC7 | Drift. One byte of `THIRD_PARTY_LICENSES` changed in the sandbox is refused. A synthetic metadata with one crate version bumped produces a different output than the tracked file | probe `licences.binaries-drift`, deep, needs cargo, plus a level-1 unit test over synthetic metadata | A check of crate names only, which a version bump with a new copyright line passes | guards-deep, implementation |
| AC8 | Determinism. Two runs give identical bytes. The output contains no `/Users/`, no `/home/`, no `.cargo/registry` and no CR byte. Running on the CI Linux host gives the bytes tracked from macOS | unit test, and the CI `guards` job on the pushed head | Absolute paths from `manifest_path`. Unsorted dict order. Host-filtered metadata | Implementation, then external (CI run) |
| AC9 | Separation. The file names no workspace crate and no Tensorbee | unit test | Listing `ocelli-*` crates as third party | Implementation |
| AC10 | Encoding. A synthetic licence file of invalid UTF-8 is refused naming it | unit test | Decoding with replacement characters | Implementation |
| AC11 | Offline. With `CARGO_HOME` pointed at an empty directory, `binaries` fails naming `cargo fetch --locked` and exits non-zero | unit test with the environment overridden | A skip, or a silent network fetch at gate time | Implementation |
| AC12 | Shipping rule. `check_tarball` with names including `dist/ocelli_wasm_bg.wasm` and lacking `THIRD_PARTY_LICENSES` returns a problem. The same list with both files returns none. Today's real tarballs pass | probe `packages.wasm-without-notices`, level 1, with control, and `gate packages` | A rule that requires the file in every tarball, which decision 11 declines, or none at all | guards, implementation |
| AC13 | Standard library. The generated file's Rust standard library section is byte-identical to the pinned `COPYRIGHT-library.html`, whose SHA-256 is `0a65bb74...68ed` for 1.97.1 | unit test | Omitting std because it is not in `Cargo.lock` | Implementation |
| AC14 | Guard registration. The `licences` guard keeps `claims=("*",)` and gains the probes above. `ci/guard-probe-budget.json` re-recorded, runbook regenerated, `--check-runbook` clean. CI `guards` job runs `cargo fetch --locked` before `gate licences` | `gate guards`, `gate guards-deep`, `gate ci` | A refusal site with no probe | guards |
| AC15 | Documentation. `README.md` Licence section, `docs/RELEASE.md` item 6 and the native archive rule, `docs/DEVELOPER_SETUP.md` regeneration with `rustup component add rust-docs`, `docs/lld/release-policy.md` | `gate prose provenance skills` | A notices file nobody is told how to regenerate | Implementation |

Old-source RED and repaired-source GREEN: AC1, AC7 and AC12 are run against the tree before the file exists and before the rule exists, and the red output is kept in the handoff.

## Approach

1. Refactor `_rust_shipped` to return `(id, package)` records and keep `_rust_exception_problems` reading names from them. F-217's tests stay green.
2. Fetch and pin the three standard texts and the standard library notice, recording `sources.toml`.
3. Write the generator as a pure function over metadata records, a directory reader and the pinned inputs, so the unit tests feed it synthetic crates. Then the `binaries` section, which compares, and `--write`, which writes.
4. Generate `THIRD_PARTY_LICENSES`, review its index against fact 1 and fact 4, and commit it.
5. Add the wasm rule to `check_tarball`.
6. CI fetch step, probes, budget, runbook, docs.

Rejected alternatives:

- cargo-about, decision 1.
- `cargo deny list` as the source. It gives identifiers, not texts, and its interpretation of a file can differ from the file's bytes.
- An untracked build artefact generated at release time. A licence change would never be reviewed, and the first time anyone saw the file would be inside a published tarball.
- Fetching standard texts at gate time. The gate would depend on the network and on a moving upstream.
- Choosing one file per crate. Decision 4.

## Dependencies, write ownership and resources

Declared dependency F-217 is done. Write set:

```text
THIRD_PARTY_LICENSES                                         new, generated
ci/notices/sources.toml                                      new
ci/notices/spdx/MIT.txt, Apache-2.0.txt, Zlib.txt            new, pinned
ci/notices/rust-std/COPYRIGHT-library.html                   new, pinned
scripts/licence_check.py, scripts/tests/test_licence_check.py
scripts/package_check.py                                     check_tarball, one rule
.github/workflows/ci.yml                                     one step in the guards job
scripts/guards/catalogue.py, ci/guard-probe-budget.json, docs/runbooks/guard-verification.md
README.md, docs/RELEASE.md, docs/DEVELOPER_SETUP.md
docs/lld/release-policy.md, docs/lld/guards.md
```

**Order.** `scripts/package_check.py`, `scripts/guards/catalogue.py`, `ci/guard-probe-budget.json` and `docs/runbooks/guard-verification.md` are shared with F-218 (in progress on `work/f-218-xai`) and F-221. **F-220 runs after F-221**, so the sprint order for these files is F-218, F-221, F-220, each based on the previous one's integrated tree. `.github/workflows/ci.yml` and `README.md` are also in F-218's write set, which is one more reason for that order. No file is shared with F-222.

Network once, at implementation, for the pinned texts. `cargo fetch --locked` populates the registry cache as any build does. No GPU, browser or corpus.

## Verification and evidence reuse

```bash
python3 -B -m unittest discover -s scripts/tests -p test_licence_check.py
python3 scripts/licence_check.py binaries
bin/ocelli.sh gate licences packages provenance prose
bin/ocelli.sh gate ci guards guards-deep skills
```

Final profile: `bin/ocelli.sh gate --sprint`. Identities: toolchain 1.97.1, cargo-deny 0.19.0, `Cargo.lock` at the verified tree, the SPDX tag and digests in `sources.toml`. Evidence is invalidated by any `Cargo.lock`, `rust-toolchain.toml` or pinned-text change. AC8's Linux half is observed only by the CI `guards` job on the pushed head, and stays open until that run exists.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a, repository tooling and licence text
- Tier B (WebGL2): n/a, repository tooling and licence text
- Tier C (CPU): n/a, repository tooling and licence text

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | AC2 to AC4, AC7 to AC11, AC13, over synthetic crates and the real metadata | `scripts/tests/test_licence_check.py` |
| guard probe | AC4 to AC7, AC12, each refusal red in the sandbox, with controls | `scripts/guards/catalogue.py` |
| integration | AC1, AC8, AC14 | `gate licences`, `gate guards`, CI `guards` job |

No `fixture` row is owed. No pixel or geometry arithmetic.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no licence row.

## Deviations

None. The HLD has nothing on notices to depart from, and D-22 is unchanged.

## LLD impact

- `docs/lld/release-policy.md`: `THIRD_PARTY_LICENSES`, its closure, its regeneration, the wasm rule in `gate packages`, and the native archive rule.
- `docs/lld/guards.md`: the `binaries` section and its probes, and the packages wasm probe.

## User-facing documentation impact

- `README.md`, section "Licence": one sentence after the `THIRD_PARTY_NOTICES` sentence, naming `THIRD_PARTY_LICENSES` as the full licence texts of the Rust crates and the Rust standard library compiled into the wasm module and the native binaries, and how to regenerate it.
- `examples/folder-viewer/src/use.html` and `examples/folder-viewer/src/more.html`: no change. Neither carries licence or attribution text, and the folder viewer is a local example that is not distributed.

## Open questions

None open.

- Q1, tool. Answered: lead decision under the operator's delegation of 2026-11-05. A stdlib generator in `licence_check.py`, decision 1. F-217's Q6 named cargo-about as the likely tool, and this plan departs from that suggestion for the reasons given.
- Q2, location. Answered: lead decision under the operator's delegation of 2026-11-05. A tracked root file, enforced into any tarball that carries the wasm module, decisions 3, 10 and 11.
- Q3, crates with no file. Answered: lead decision under the operator's delegation of 2026-11-05. The pinned standard text of the elected licence with the declared authors, decisions 5 and 6.
- Q4, the Rust standard library. Answered: lead decision under the operator's delegation of 2026-11-05. The toolchain's own notice, pinned, decision 7.

One finding for the operator, outside this story's scope: `docs/RELEASE.md` item 6 says the wasm module must be in `@ocelli/core`, and today's tarball has none (fact 7). `/release` would refuse on that item. No story in S46 builds the wasm into the package.

### Lead amendments after review pass 1

Made by the sprint lead on 2026-11-05 under the operator's delegation of the quality call, after `.claude/reviews/F-220-working-pass-1.md`.

- **A pinned standard text is a template, never a notice.** `ci/notices/spdx/` holds licence terms with no copyright line and no sign-off of any project. The MIT and Zlib templates carry one marked copyright line, and the generator fills it per crate. A pinned file that still carries another project's holder, or an unfilled placeholder in the output, is a refusal.
- **A crate whose package ships no licence file takes its upstream notice.** The generator uses a licence file pinned from the crate's own repository at the tag or commit matching its locked version, stored under `ci/notices/crates/<name>-<version>/` with its source URL and SHA-256 in `ci/notices/sources.toml`. Fetching happens only when a pin is added, never at gate time. Only when the upstream repository carries no licence file does the notice fall back to the template, with the copyright line naming the crate's declared authors, or "the <name> authors" and its declared repository when none are declared. Each fallback is listed in `sources.toml` with that reason. No year is invented.
- **Every copied source has its row.** choosealicense, apache.org, the zlib repository, rust-docs, each upstream crate repository used under the rule above, and the refused SPDX license-list-data each get the row `docs/SOURCE-POLICY.md` requires, under "Extensions to the table". `sources.toml` calls a source only what it is.

### Lead amendments after review pass 2

Made by the sprint lead on 2026-11-05 under the operator's delegation of the quality call, after `.claude/reviews/F-220-working-pass-2.md`.

- **A pin comes from the package's published commit, and the gate checks it.** The commit is read offline from the package's `.cargo_vcs_info.json` in the registry cache. A pin whose URL names any other commit is a refusal, and a level-1 probe watches it. A licence file added upstream after that commit is never used for the locked version.
- **The objc2 family takes `LICENSE.md` and the fallback.** At each published commit the repository holds only `LICENSE.md`, which states the licence expressions and names no holder. The notice reproduces that `LICENSE.md`, then the elected template under the fallback rule, with the crate's declared authors or "the <name> authors" and its repository as the holder. The section sentence says neither the package nor its repository at that commit carries licence text, and the reason is recorded per crate in `sources.toml`.
- **The four MIT-only objc2 crates also name Steven Sheldon.** The repository's root `LICENSE.txt` read `Copyright (c) Steven Sheldon` until commit `cfb19922`. That file is pinned from the commit before it and reproduced for those crates. Naming an earlier holder is the cautious direction.
- **Apache-2.0 stays exactly as published.** Its appendix line is instructions to a licensor, not a notice, and the generator never fills it.
- **Each source is recorded under the licence of what was copied.** choosealicense content, including `_licenses/mit.txt`, is CC BY 3.0. Its row says so and gives the attribution that licence requires.

### Lead amendments after review pass 4

Made by the sprint lead on 2026-11-05 under the operator's delegation of the quality call, after `.claude/reviews/F-220-working-pass-4.md`, and corrected after pass 6.

- **The `binaries` section stays offline.** It reads no network and no pin is fetched at gate time. It checks each file pinned under `ci/notices/` against its recorded SHA-256 and each crate pin's commit against the package's `.cargo_vcs_info.json`.
- **`verify-upstream` is a separate command, part of no gate.** `python3 scripts/licence_check.py verify-upstream` re-fetches every crate pin and every earlier pin from its recorded URL at its recorded commit and refuses a byte difference or a missing file.
- **Outside this story.** The `rust` section's `cargo deny` call from F-217 runs without `--offline`, so on an empty `CARGO_HOME` it downloads the registry.
- **What neither checks.** Nothing checks that an earlier row's commit is the parent of `cfb19922`. An earlier row re-pointed at another commit whose file has the same bytes passes both the gate and `verify-upstream`.
