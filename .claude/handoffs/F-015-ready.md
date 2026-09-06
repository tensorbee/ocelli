# F-015 ready, stable render hashes from the comparator

**Branch**: work/f-015-codex
**Base**: 1e186479476431f98b1e711f0584df6aa2d80293
**Head**: 2085fe970dfecc5320642af4b609ed799415c26b, the reviewed and verified feature commit. The branch tip is one handoff-only commit above it because this file cannot name the commit that contains it.
**Review**: `.claude/reviews/F-015-render-hash-pass-1.md`, microscope pass 1, clean with 0 defects, 0 smells and 0 nitpicks.
**Verify tree**: 6ad6ac607f49ddcfde956d274ec13b416c6628b6, feature profile with all 25 floor gates and `corpus`, recorded with `corpus=pass` and `Ocelli-Generated-By: codex`.

## What landed

The comparator now emits a versioned `sha256-rgba8-v1` digest for every
reference and candidate RGBA8 view, plus a canonical aggregate digest for each
side. The byte contract binds view kind, identifier, dimensions, format and
exact tightly packed frame bytes. Run hashes sort their framed view records so
report iteration order cannot move the identity.

The stable JSON report carries the per-view and aggregate values. Concise
stdout prints both aggregate values. Every mutation catalogue case that changes
frame bytes must now move the damaged side's hash as well as produce its
declared comparator result.

Exact identity remains separate from the comparator's tolerance and measured
divergence. An equal digest means equal declared RGBA8 output under this byte
contract. It is not a cross-machine reproducibility claim.

## Files touched

- `.claude/reviews/F-015-render-hash-pass-1.md`
- `docs/lld/comparator.md`
- `docs/sprints/BACKLOG.md`
- `tools/oracle/src/attribution.rs`
- `tools/oracle/src/bin/ocelli-compare.rs`
- `tools/oracle/src/lib.rs`
- `tools/oracle/src/mutations.rs`
- `tools/oracle/src/render_hash.rs`
- `tools/oracle/src/report.rs`
- `tools/oracle/tests/render_hash_fixture.rs`

The approved plan anticipated a tracked
`tools/oracle/compare-out/compare.json`. That path is ignored comparator output
and the repository refuses it as generated output, so it is not part of the
commit.

## Verification

`bin/ocelli.sh gate --floor` exited 0 with all 25 gates green:

`fmt`, `clippy`, `test`, `bindgen`, `unsafe`, `pins`, `nostd`, `errors`,
`panic`, `bench`, `provenance`, `prose`, `content`, `backlog`, `deviations`,
`skills`, `lint`, `types`, `wasm`, `native`, `device`, `packages`, `ci`,
`guards`, `corpus-tests`.

`bin/ocelli.sh gate corpus` exited 0. It verified 91 manifest rows, 44 real,
all 16 transfer syntaxes, 0 missing files, 0 digest mismatches and a clean
metadata audit. The known corpus coverage note remains: no real colour case
exercises chroma subsampling or YBR conversion.

The committed feature tree carries:

```text
Ocelli-Verify: profile=feature gates=backlog,bench,bindgen,ci,clippy,content,corpus,corpus-tests,deviations,device,errors,fmt,guards,lint,native,nostd,packages,panic,pins,prose,provenance,skills,test,types,unsafe,wasm corpus=pass tree=6ad6ac607f49
Ocelli-Generated-By: codex
```

## Evidence

The literal 2 by 1 RGBA8 fixture independently pins the per-view digest
`7ce1f3d20a7aa3652620049f76d3b99123acc1ccf35cd59649bef6a87c1785e0`
and run digest
`de6faff9b893c67a9ef9e00cd174e92fd47ab8a5cbda77393041982d298dc365`.

The full identity run covered 98 declared views. It reported 70 pass,
0 fail, 28 unmeasured and 0 absent with equal aggregate hashes. The mutation
catalogue reported all 21 mutations detected, including the required hash
movement on the damaged side.

## Integrator notes

This prepare pass does not update `AS_BUILT.md`, `SPRINT_TRACKER.md`, sprint
totals, sprint state or the backlog row to `done`. Those remain integrator
work. Nothing was pushed or integrated.
