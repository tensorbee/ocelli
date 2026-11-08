# F-237 handoff

**Branch**: `work/f-237-xai`
**Base**: `0d73651e26e73e167e8f1d5c6d71453631c790dc`
**Head**: `7d5548dd5cb8eb14221df75747aa4f45cd555117`
**Files touched**: `.agents/skills/release/SKILL.md .claude/commands/release.md .opencode/commands/release.md .github/workflows/ci.yml README.md bin/ocelli.sh ci/guard-probe-budget.json docs/RELEASE.md docs/lld/release-policy.md docs/lld/typescript-packaging.md docs/runbooks/guard-verification.md eslint.config.js examples/folder-viewer/src/use.html examples/viewer-react packages/core/package.json packages/wasm scripts/ci_floor_check.py scripts/guards/catalogue.py scripts/package_check.py scripts/release_wasm.py scripts/tests package-lock.json`
**Review**: `.claude/reviews/F-237-working-pass-5.md`
**Verify tree**: `cd7fdbd76b030c2f6202944958e5659768dc1a07`

Owed after integration: AC4, the `frontend` job green on `ubuntu-latest` once the sprint branch is pushed. Owed at release time: AC3 end to end at the real release HEAD, and AC5 against the registry after publish. The story is integrated but not done until AC4 is observed.
