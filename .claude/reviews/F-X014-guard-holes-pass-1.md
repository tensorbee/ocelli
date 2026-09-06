# F-X014 review, pass 1

**Reviewed**: staged tree against `72eab18`
**Result**: 2 defects, 0 smells, 0 nitpicks

## Defects

### D1, the handoff parser accepts values outside its declared grammar

**Where**: `scripts/sprint_workflow.py:57-69`, handoff probes in
`scripts/guards/catalogue.py`

**What**: `handoff_field` removes the first and last backtick whenever both
are present, without proving they are the only backticks or that the enclosed
value is non-empty. It also accepts unmatched and embedded backticks as plain
text. Consequently values such as `` `work/f-x014-agent` `forged` ``,
`` `scripts/a.py `` and `abc` followed by a backtick and `def` all count as
present fields. An empty code span also counts as a present non-branch field.
The branch example survives the prefix check because its unwrapped value still
starts with `work/f-x014-`.

**Why it is wrong**: The approved plan and both complete-feature surfaces
define each value as plain text or exactly one Markdown code span. Accepting
multiple, unmatched, embedded or empty code spans weakens the shape check on
the provenance fields and makes the documented grammar false. The current
probes cover one valid backticked branch, one wrong branch and one absent
field, so none observes these malformed forms.

**Evidence**: Direct calls to the production parser returned
`work/f-x014-agent` plus the second forged span for the two-span branch,
returned the unmatched backtick text for `Files touched`, returned an empty
string for an empty `Review` span, and returned the embedded-backtick text for
`Head`. Full disposable handoff fixtures containing the first three malformed
forms each passed `validate-handoff` at exit 0. Require either backtick-free,
non-empty plain text or one non-empty, matching code-span pair. Also reject
duplicate occurrences of a required field so conflicting provenance cannot be
silently resolved by first match. Add refusal probes for the malformed and
duplicate forms while retaining the existing branch-prefix refusal after
unwrapping.

### D2, the LLD index contradicts both edited LLD headers

**Where**: `docs/lld/README.md:32-33`, `docs/lld/benchmarks.md:3`,
`docs/lld/guards.md:3`, `.claude/plans/F-X014-design.md`

**What**: The benchmarks and guards headers both add F-X014, but their index
rows still end at `F-006` and `F-X020` respectively. The plan's exact write set
and `## LLD impact` list omit `docs/lld/README.md`.

**Why it is wrong**: The LLD index explicitly says its F-ID column mirrors each
file's contributor line and requires both to be edited together. This is a
known manually maintained invariant, so a green automated gate does not make
the contradiction acceptable.

**Evidence**: `rg -n "benchmarks\\.md|guards\\.md" docs/lld/README.md` shows
the two stale rows, while each target file's line 3 includes F-X014. Add
F-X014 to both index rows and add `docs/lld/README.md` to the plan's measured
write set and `## LLD impact` list.

## Smells

None.

## Nitpicks

None.

## Verified clean

- `no_std` owns an explicit nine-crate set and compares declared membership in
  both directions before resolving cargo graphs. Removing either comparison
  made its corresponding mutation probe miss the change.
- The benchmark suite reaches all eight `parseArgs` refusals and the pending
  subject refusal through production exports. Removing either a refusal or
  either exact suite registration made the connected tests fail.
- G-02 and G-04 were removed only after their ordinary probes turned green.
  The census reports 633 refusal sites across 61 files, 0 uncovered sites, 0
  open defects and `sweep_complete: true`.
- The six remaining environmental limits name the unavailable graph, install,
  private input, browser or deliberately broken build they require. They do
  not claim F-X014 coverage.
- The repaired deep lint-policy accept probe is coupled to derived target-root
  wording, while its paired refusal probe still proves the moved root is
  scanned.
- The generated runbook and the complete-feature adapter pass their sync
  checks.
- `bin/ocelli.sh gate nostd bench skills guards guards-deep` passed. The probe
  totals were 153 refusals, 24 accepts and 38 controls for `guards`, then 202
  refusals, 38 accepts and 41 controls for `guards-deep`.
- The benchmark gate passed 78 tests with one expected opt-in browser skip.
  The focused benchmark tests passed 18 of 18.
- The 54 catalogue tests, four guard-reader tests and 75 probe-runner tests
  passed in the connected gate run.
- The prose, staged content, deviation, CI-floor and staged diff checks passed.
- No patient data, unsafe code, tolerance change, wasm boundary change, pixel
  transfer, LUT logic or render-loop allocation was introduced.
