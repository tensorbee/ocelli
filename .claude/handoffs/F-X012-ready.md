# F-X012 ready, SIGMOID reference divergence measured

**Branch**: work/f-x012-codex
**Base**: f3c7176f801dd691402248d0e7ed357bf0f9be2b, the isolated worker base.
**Head**: 2c80a7a7eb616ec7df29a5496e94b11ec3a42d77, the reviewed feature
commit. The branch tip is one commit above it and adds only this handoff file.
Merge the tip.
**Review**: `.claude/reviews/F-X012-sigmoid-divergence-pass-3.md`, the third
microscope pass, reports 0 defects, 0 smells and 0 nitpicks. Passes 1 and 2
are included with their remediations.
**Verify tree**: d9461dbff32c81288a58c1e69a3a163564c300a6, recorded with
`profile=feature`, `corpus=pass` and `Ocelli-Generated-By: Codex`.
**Files touched**: the three F-X012 review records, `Cargo.toml`,
`bin/ocelli.sh`, `corpus/manifest.tsv`, `docs/lld/comparator.md`,
`docs/lld/corpus.md`, `docs/lld/oracle.md`, `docs/sprints/BACKLOG.md`,
`docs/sprints/CURRENT_SPRINT.md`, `scripts/corpus_synth.py`,
`scripts/tests/test_corpus_synth.py`, `tools/oracle/page/app.mjs`,
`tools/oracle/reference-divergence.json`, `tools/oracle/run.mjs`,
`tools/oracle/src/attribution.rs`, `tools/oracle/src/lib.rs`,
`tools/oracle/src/mutations.rs`, `tools/oracle/src/params.mjs`,
`tools/oracle/src/report.rs`, `tools/oracle/src/sidecar.rs`, and
`tools/oracle/tests/params_test.mjs`.

## What landed

The corpus has one deterministic CT case with `VOILUTFunction = SIGMOID`,
centre 40 and width 0.5. Independent fixture tests compute the PS3.3 values.
The pinned cornerstone3D 5.8.2 helper returned inverted bounds of 39.75 and
39.25, while the rendered display codes remained monotonic and matched the
independent result. The divergence register therefore records the helper
inversion without claiming a presented-pixel defect.

Reference attribution now requires the declared pointwise monochrome
inversion, parameter agreement and geometry agreement. Invalid or empty story
identifiers are refused with the canonical optional suffix grammar. Identity,
unrelated pixel changes, rescale disagreement and geometry drift retain their
ordinary comparator verdicts.

## Review and verification

Pass 1 tightened effect matching and provenance validation. Pass 2 added
geometry precedence and canonical suffixed F-ID coverage. Pass 3 found no
defect, smell or nitpick. Each remediation has a focused mutation that failed
when its protection was independently removed.

`bin/ocelli.sh gate --floor` exited 0 with all 25 gates green. The separate
`bin/ocelli.sh gate corpus` verified all 92 manifest rows with none missing or
mismatched. The metadata audit agreed on modality, transfer syntax and
tolerance class. The ledger record names all 25 floor gates plus `corpus` for
the exact feature tree above.

The browser oracle compared 99 views, reported 71 pass and 28 unmeasured, and
detected all 21 catalogue mutations. No tolerance changed and no patient data
was opened, copied, tracked or placed in a prompt.

## Prepare-only boundary

No sprint totals, AS_BUILT entry, tracker row, done status, integration or push
was performed. The integrator owns the shared completion records.
