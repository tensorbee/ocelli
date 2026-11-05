# F-217 ready for integration

**Branch**: `work/f-217-xai`
**Base**: `965f6fbdf1c7eb9c3ea26828c5c056e19c8e496f`
**Head**: `e3fc29a26165f079140667e192e6e6fd4acf78b8`
**Files touched**: `.agents/skills/dicom-tooling/SKILL.md .agents/skills/release/SKILL.md .claude/commands/release.md .claude/plans/F-217-design.md .claude/reviews/F-217-working-pass-1.md .claude/reviews/F-217-working-pass-2.md .claude/reviews/F-217-working-pass-3.md .claude/reviews/F-217-working-pass-4.md .claude/reviews/F-217-working-pass-5.md .claude/skills/dicom-tooling/SKILL.md .github/workflows/ci.yml .opencode/commands/release.md Cargo.toml LICENSE LICENSE-MIT NOTICE README.md THIRD_PARTY_NOTICES bin/ocelli.sh ci/guard-probe-budget.json ci/licences.toml corpus/README.md corpus/decode-digests.tsv deny.toml docs/DEVELOPER_SETUP.md docs/SOURCE-POLICY.md docs/hld/DEVIATIONS.md docs/lld/codecs.md docs/lld/corpus.md docs/lld/guards.md docs/lld/release-policy.md docs/lld/typescript-packaging.md docs/runbooks/guard-verification.md examples/viewer-react/package.json examples/viewer-react/vite.config.ts package-lock.json package.json packages/core/LICENSE-MIT packages/core/NOTICE packages/core/package.json packages/react/LICENSE-MIT packages/react/NOTICE packages/react/package.json pyproject.toml scripts/ci_floor_check.py scripts/corpus_check.py scripts/guards/catalogue.py scripts/licence_check.py scripts/package_check.py scripts/pin_and_size_check.py scripts/source_provenance_check.py scripts/tests/test_corpus_synth.py scripts/tests/test_licence_check.py scripts/tests/test_source_provenance_check.py uv.lock vendor/openjph-core-0.1.0/LICENSE `
**Review**: `.claude/reviews/F-217-working-pass-5.md, 0 defects, 0 smells, 0 new nitpicks`
**Verify tree**: `2bb50b322760cc98a38104040bcc1cd5b0489618`

The branch merged sprint/s46 up to e47b9ba during review. AC17's Linux half
is external evidence, the corpus-tooling CI job on the pushed head, and is
confirmed at sprint close. The lead-decided licence file waivers are listed
for the operator in docs/sprints/CURRENT_SPRINT.md.
