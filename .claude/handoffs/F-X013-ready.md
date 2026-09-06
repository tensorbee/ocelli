# F-X013 ready, price the HTJ2K decoder route

**Branch**: work/f-x013-codex
**Base**: 021d892ee8c8dba1670aa0f4b885440d7aea153a
**Head**: b8eca127ac6f4a67110cf30d904b8d9f096655d5, the reviewed and verified feature commit. The branch tip is one handoff-only commit above it because this file cannot name the commit that contains it.
**Review**: `.claude/reviews/F-X013-htj2k-route-pass-1.md` found one defect. `.claude/reviews/F-X013-htj2k-route-pass-2.md` verified the remediation and returned clean with 0 defects, 0 smells and 0 nitpicks.
**Verify tree**: 6f508144a03097412a93b29787cf9e43706f7159, feature profile with all 25 floor gates and `corpus`, recorded with `corpus=pass` and `Ocelli-Generated-By: codex`.

## What landed

The isolated spike prices `openjph-core` 0.1.0 as the pure-Rust HTJ2K route
for E2.6 without adding a production decoder. Native, plain wasm and simd128
wasm decode all three HTJ2K transfer syntaxes through the same exact pin.

The reversible `.201` and `.202` rows reproduce the independent synthetic
ramp and OpenJPH output exactly. The irreversible `.203` row is recorded as a
D14 measured divergence between related OpenJPH implementations. All three
candidate builds agree, while 41 of 6,144 samples differ from OpenJPH by one.

The driver pins the independent reference, all three input codestreams, every
candidate route output, every OpenJPH output and the complete `.203`
divergence. It does not introduce or change a tolerance.

Production adoption remains conditional on obtaining the complete BSD notice,
auditing the dependency's relevant unsafe paths and malformed-input behavior,
replacing or adapting its per-row allocation contract, measuring incremental
product size and retaining a fork plan. HTJ2K stays unavailable until E2.6
satisfies those conditions and raises the section 15.2 deviation.

## Files touched

- `.claude/reviews/F-X013-htj2k-route-pass-1.md`
- `.claude/reviews/F-X013-htj2k-route-pass-2.md`
- `docs/SOURCE-POLICY.md`
- `docs/lld/corpus.md`
- `docs/lld/oracle.md`
- `docs/spikes/A1-htj2k-openjp2.md`
- `docs/spikes/A1-htj2k-route.md`
- `docs/sprints/BACKLOG.md`
- `tools/spikes/x013-htj2k-route/Cargo.lock`
- `tools/spikes/x013-htj2k-route/Cargo.toml`
- `tools/spikes/x013-htj2k-route/run.mjs`
- `tools/spikes/x013-htj2k-route/src/lib.rs`
- `tools/spikes/x013-htj2k-route/src/main.rs`

The backlog change is the feature-start transition from `pending` to
`in-progress`. No sprint total or completion record is changed here.

## Review and evidence

Microscope pass 1 changed the `.203` route to decode the `.201` codestream in
all three builds. The earlier driver still exited 0 despite 5,377 differences
against OpenJPH and a maximum absolute difference of 41.

After remediation, that exact mutation produced six explicit failures and
exit 1. Rebuilding native, plain wasm and simd128 wasm from the restored
`.203` route returned the driver to exit 0. Its enforced divergence is 41
samples, first at index 69 with candidate value 756 and OpenJPH value 757,
maximum absolute difference 1 and histogram `1:41`.

The source-provenance record was completed before candidate source inspection.
The crates.io archive digest matches the registry. Registry and package
metadata declare BSD-2-Clause, while the package omits a licence file and
repository URL. That omission is recorded as a production condition.

## Verification

The first `bin/ocelli.sh gate --floor` invocation ran all floor gates. Twenty
two passed. `panic`, `wasm` and `packages` failed because sandbox restrictions
blocked the pinned wasm and npm caches. Rerunning exactly those three gates
with cache access exited 0. The verified floor set is:

`fmt`, `clippy`, `test`, `bindgen`, `unsafe`, `pins`, `nostd`, `errors`,
`panic`, `bench`, `provenance`, `prose`, `content`, `backlog`, `deviations`,
`skills`, `lint`, `types`, `wasm`, `native`, `device`, `packages`, `ci`,
`guards`, `corpus-tests`.

`bin/ocelli.sh gate corpus` exited 0. It verified 91 manifest rows, including
44 real rows and all 16 transfer syntaxes, with 0 missing files, 0 digest
mismatches and a clean metadata audit. The known coverage note remains: no
real colour case exercises chroma subsampling or YBR conversion.

The committed feature tree carries:

```text
Ocelli-Verify: profile=feature gates=backlog,bench,bindgen,ci,clippy,content,corpus,corpus-tests,deviations,device,errors,fmt,guards,lint,native,nostd,packages,panic,pins,prose,provenance,skills,test,types,unsafe,wasm corpus=pass tree=6f508144a030
Ocelli-Generated-By: codex
```

## Integrator notes

This prepare pass does not update `AS_BUILT.md`, `SPRINT_TRACKER.md`, sprint
totals, sprint state, `CHANGELOG.md` or the backlog row to `done`. Those remain
integrator work. Nothing was pushed or integrated.
