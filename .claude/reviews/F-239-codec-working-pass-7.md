# F-239 review, pass 7, codec merge onto F-243

**Reviewed**: GLM's staged tree `2c9462ee3527ce467719832f532c1b33390974d7`, merging `fd11a48243a9c77400ddebb74dd21e87fb4f662f` into `2414d639c274ac649e46f414417c792fe20e5f63`, common ancestor `05d9aca259bd4dd651f32652230a487881cd253c`. The merge is uncommitted in `work/f-239-codec`. The staged repair tree is `fe8030b4e30186dead78ecdbdd8524acf9aaed5b`.
**Author / reviewer**: GLM-5.3-Flash-EXL3 through OpenCode implemented this merge. Codex subagent `glm_codec_merge` independently inspected it and found D1. The reviewer then restored one row from the original source blob under the lead's instruction. The lead must independently approve that repair. Historical Claude reviews are prior evidence, not this participant's work.
**Result**: 1 defect, 0 smells, 1 nitpick on the submitted GLM tree. D1 is repaired, with independent approval of the repair still owed.

## Acceptance challenged

The approved F-243 plan requires the union of both ritk patches, their provenance and source hashes, and both corpus row sets. The review derived that union from the common ancestor and both actual branch tips, rather than accepting the implementer's report.

The independent checker compares every path changed by only one side byte for byte, every complete manifest and decode-digest row by key and value, both patched-source hash maps, the registry function names and literal catalogue probe names. This rejects a merge that keeps the row count while silently changing an existing digest, which is the actual failure D1 found.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| Independent `check_union.py`, preserved under `/private/tmp/ocelli-codex-f239-glm/` | Submitted GLM tree | now | 1, one changed common decode-digest row, D1 |
| Same checker after the controlled repair | repair tree | now | 0. All 269 one-sided payloads identical. 109 manifest rows and 27 digest rows equal the expected union. All 10 ritk and 3 openjph source pins and both provenance hashes match. All 427 literal probe names and 23 registry function names retained without duplicate names |
| `bin/ocelli.sh check ocelli-codec` | repair tree, `CARGO_BUILD_JOBS=2` | now | 0 |
| `bin/ocelli.sh test ocelli-codec --test registry` | same | now | 0, 19 passed |
| `bin/ocelli.sh test ocelli-codec --test jpegls` | same | now | 0, 33 passed |
| `bin/ocelli.sh test ocelli-codec --test jpeg2000` | same | now | 0, 12 passed |
| `bin/ocelli.sh test ocelli-codec --test htj2k` | same | now | 0, 18 passed |
| `cargo test --offline --locked --manifest-path vendor/ritk-codecs-0.6.0/Cargo.toml --lib` | same, target under the task's temporary directory | now | 0, 321 passed |
| `python3 scripts/pin_and_size_check.py` | same | now | 0 |
| `python3 -m unittest scripts.tests.test_pin_and_size_check` | same, includes a planted byte in each of the 10 patched ritk files | now | 0, 18 tests |
| `python3 -m unittest scripts.tests.test_guard_catalogue` | same | now | 0, 57 tests |
| `.venv/bin/python -m unittest scripts.tests.test_corpus_synth` | same | now | 0, 66 tests |
| `python3 scripts/guard_census.py` and `--check-runbook` | same | now | each 0, 1162 refusals in 80 files claimed, 0 watched by nothing |
| `python3 scripts/prose_check.py` | same, before this report was added | now | 0, 979 tracked files |
| `python3 scripts/deviation_check.py` | same | now | 0, 32 deviation rows |

Each direct check was run through `subprocess.run` and its own return code was recorded, without a shell pipeline. Exact arguments, timestamps, durations and complete logs are in `direct-cargo-checks.json` and `direct-python-checks.json` under the task's temporary directory. These checks establish bounded merge coverage, not story completion or sprint readiness.

## Defects

### D1, retyping changed an unchanged corpus digest

**Where**: `corpus/decode-digests.tsv:18`.

**What**: the submitted merge changed the file SHA-256 for `syntax/jpeg_lossless_p14_sv1.dcm` from the common source's 64-character value to a 63-character value. The sequence `d9262188e3cce` became `d926218e3cce`. All three source trees have the same original row. The manifest still holds the original value.

**Why it is wrong**: the approved plan requires a union of corpus rows. `scripts/corpus_check.py:526` refuses a recorded file digest that differs from the manifest. `CLAUDE.md` also requires edits in place rather than retyping tool output.

**Evidence**: the first independent union check exited 1 with exactly this row differing. The original submitted TSV is preserved as `pre-self-review-decode-digests.tsv`, SHA-256 `38aeec35ab3830124670a920dd19d1957af87a91b0266351ffa3ce07d45e410d`. GLM's final report called the table value-verified, and its autonomous final output still contained the typo. No corrective instruction was sent to GLM before it finished.

**Scope and regression**: this is a metadata integrity failure introduced by the merge, not a decoder arithmetic change. The existing corpus gate already owns the refusal. The missed execution step was the existing no-retyping rule, so no duplicate workflow rule or implementation-mirroring test was added.

**Repair**: Codex restored only that row mechanically from `git show 2414d639:corpus/decode-digests.tsv`, asserted one matching key, and staged it. The repaired tree differs from the submitted GLM tree in that one row only. The independent union checker then exited 0. The lead owns independent approval of this reviewer-authored repair.

## Smells

None established in the bounded merge review.

## Nitpicks

- N1. `vendor/ritk-codecs-0.6.0/PATCH-PROVENANCE.md:57` says every other published file is byte-identical within the F-239 section, while the document also records F-243's later source patch. The introduction names both patches, so this is a scope ambiguity rather than an established missing patch. A future wording edit can refer to files outside all recorded patches and refresh the provenance pin. It does not block this merge.

## Verified clean

- All implementation files changed by only one branch remain byte-identical to that branch. In particular, both disjoint ritk implementations and the openjph implementation survive without merge edits.
- Every manifest and decode-digest row now equals the common-ancestor union. The row count alone did not establish this and was not accepted as evidence.
- The patched-source hash maps contain exactly both reviewed sets. Current bytes match all 13 source hashes and both provenance hashes. The pin tests reject a planted byte in each recorded ritk file.
- The source scope is the expected 289-path union relative to the common ancestor, with no extra implementation path. No conflict remains. The staged working files matched the index after the controlled repair.
- Focused codec, vendor, generator and catalogue tests pass with actual direct exit codes. These do not re-establish every prior arithmetic or external acceptance claim.

## Measurements and evidence limitations

The GLM run lasted 1777.102 seconds, from 11:25:40 to 11:55:17 UTC on 2026-11-08, process exit 0, zero retries. The model was `dgx/GLM-5.3-Flash-EXL3`, OpenCode v2.1.4. Provider-reported counters were input 221434, output 40702, reasoning 31904 and cache-read 26933824 across 175 assistant messages. Cache accounting is unspecified, and another GLM task overlapped. This is not an isolated throughput measurement or a controlled comparison with Claude.

GLM repeatedly ran checks through `tail` and then printed the shell status, despite the explicit instruction to read the command's own exit. Its report's claim that statuses were read directly is contradicted by the raw log. Those printed statuses were not accepted. The direct rechecks above replace that evidence, while the original report and raw log remain preserved.

## Limitations and handoff

- No browser, GPU, oracle, wasm build, full workspace, floor or sprint profile was run in this task. The existing independent arithmetic reviews remain historical evidence.
- `docs/lld/corpus.md:173` still says the banded rows have not been rendered by the oracle. That sentence is inherited unchanged from F-243's tip and contradicts its AC18 record at `15d4817e`. The lead must correct it during F-243 integration or completion. It is not claimed as accepted prose or waived acceptance here.
- The canonical sprint ledgers were not hand-edited. Their existing history was imported by the three-way merge. No commit or push was made.
- The lead must review the one-row repair and this handoff before committing the merge. Wider feature verification belongs to the final integration tree.
