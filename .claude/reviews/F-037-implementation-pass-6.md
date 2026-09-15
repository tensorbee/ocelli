# F-037 review, pass 6

**Reviewed**: the staged working tree, `git write-tree` =
`12d50e5c20d52738f220b8875be2e3f9ce80517f`, against base `3b00890`, with the
pass-5 remediation delta read as `git diff e8ba4a5a 12d50e5c`. Independent
reviewer, not the author.
**Result**: 0 defects, 0 smells, 2 nitpicks

## THIS PASS IS CLEAN.

Zero defects and zero smells, which is the bar `.claude/commands/microscope.md`
sets. I have no other open concern about this story.

**Pass line**: F-037, pass 6, 0 defects, 0 smells, 2 nitpicks. Trend across six
passes: 10/3/4, 4/1/5, 2/1/5, 1/0/4, 1/0/5, 0/0/2. No finding survived a pass.

All mutations were reverted. `git write-tree` at the end of this pass is
`12d50e5c20d52738f220b8875be2e3f9ce80517f`, the same as at the start.

## Scope of this pass

You said one sentence plus two reflows and nothing else, and the diff says the
same. `git diff e8ba4a5a 12d50e5c` touches four files, one of which is the
pass-5 review record:

```
.claude/reviews/F-037-implementation-pass-5.md | 229 +++
bin/ocelli.sh                                  |   8 +-
docs/lld/gpu-ownership.md                      |   3 +-
scripts/guards/census.py                       |   4 +-
```

No source file moved. I re-ran the full battery and the full gate set anyway,
because a clean pass is a claim about the tree rather than about the diff.

## Defects

None.

## Smells

None.

## Nitpicks

### N1, "F-037 has landed" is still ahead of the backlog

`docs/lld/tier-resolution.md:460` and `:467` say "F-037 has landed" and "F-037
has since built" while `docs/sprints/BACKLOG.md:163` says `in-progress`. Raised
in passes 1 through 5 and never blocking. `/complete-feature` is the next
command and flipping that row is what makes both sentences true. **Worth one
check after it runs**: if the backlog row does not move to `done`, these two
sentences become a live false claim rather than a premature one, and the sprint
review will find them.

### N2, the CI floor drives `resolve_adapter` in no test

Since the tier C test became `#[ignore]`d in pass 4, no floor-profile test calls
`resolve_adapter`. No detection was lost, because the floor version killed no
mutation, and `caps::tests::tiers_a_and_b_open_a_device_and_tier_c_does_not`
covers the decision with no adapter. Recording it so a later reader does not
rediscover it as a gap. Not worth a change.

### Withdrawn

- **N1 from pass 5, "the hardware tier".** You asked what the second reading is.
  It is "tests ABOUT the hardware tier", under which the row would be slightly
  wrong, because
  `a_cpu_override_yields_no_adapter_and_spends_no_startup_cost` is about tier C.
  The first reading, "the hardware tier OF THIS PROJECT'S TEST PYRAMID", is the
  one the `YES` in the GPU column supports and is plainly the intended one.
  **I withdraw it and want no change.** I raised it as a reading, not a fault,
  and a fifth wording of that line would cost more than the ambiguity.
- **N4 from pass 5, naming a test function in prose.** Your reasoning is right
  and mine was not: that sentence exists to name the one exception, and removing
  the name puts the staleness back into the gloss the whole paragraph is there
  to avoid. A renamed test is a grep away. **Withdrawn.**
- **N3 from pass 5, the ragged continuation in `census.py`.** The reflow moved
  the short line from 188 to 189 rather than removing it, which is what
  rewrapping a hand-wrapped Python string concatenation does. The joined value
  is correct, `guards` and `guards-deep` are green, and it renders identically.
  **Withdrawn. Do not touch it again**, because three passes of whitespace churn
  in a guard file is a worse trade than one short line.

## The one sentence, verified

`bin/ocelli.sh:308-313` now reads:

> `-- --ignored` runs ONLY the ignored tests, which is the whole set and nothing
> else. The default `cargo test --workspace` in the `test` gate runs every test
> NOT marked `#[ignore]`, so the two gates partition the crate's suite on that
> attribute and neither is a subset of the other. The partition is by the
> attribute and not by what a test needs, for the reason the paragraph above
> gives.

Every clause measured rather than read:

| Clause | Measurement |
|--------|-------------|
| "`-- --ignored` runs ONLY the ignored tests" | `cargo test -p ocelli-render -- --ignored` runs 3 + 6 = 9, with 75 + 1 + 1 filtered out |
| "the `test` gate runs every test NOT marked `#[ignore]`" | `gate --list` gives `test  no  cargo test --workspace`. `cargo test -p ocelli-render` runs 77 and reports 9 ignored, 0 filtered out |
| "the two gates partition the crate's suite on that attribute" | `cargo test -p ocelli-render -- --list` counts 86. 77 + 9 = 86, disjoint, nothing outside. An exact partition |
| "neither is a subset of the other" | both parts non-empty, 77 and 9, so neither contains the other |
| "The partition is by the attribute and not by what a test needs" | consistent with `bin/ocelli.sh:295-306` thirteen lines above, which is the paragraph it cites. The contradiction pass 5 found is gone |

**The claim pass 5 rejected is gone from the tree.** `grep -rniE "runs
everything that"` over `bin/`, `scripts/`, `.claude/WORKFLOW.md`, `docs/lld/`
and `crates/ocelli-render/` returns nothing.

## The two reflows, verified

**N2 from pass 5, `gpu-ownership.md`.** Your claim was "no lines over 100 chars
outside tables". Measured:
```
$ awk '{ if (length($0) > 100) printf "%d(%d) table=%s\n", NR, length($0), (substr($0,1,1)=="|" ? "YES" : "NO") }' docs/lld/gpu-ownership.md
25(102) table=YES   26(133) table=YES   27(118) table=YES   29(105) table=YES
30(108) table=YES   79(108) table=YES   114(167) table=YES  116(109) table=YES
128(178) table=YES  129(404) table=YES  225(113) table=YES  226(106) table=YES
227(116) table=YES  228(102) table=YES
```
Fourteen long lines, every one a table row. The paragraph at 287 to 298 is now
56 to 79 characters throughout. True as stated.

**N3, `census.py`.** The joined `DELEGATED["gpu"]` string is unchanged in value
and correct. See Withdrawn above.

## Regression battery, nine of nine still red

Each mutation applied, run, and reverted. The tree hash was re-checked after the
last revert and matches.

| Mutation | Result |
|----------|--------|
| `recovers_from(Destroyed) = true` | RED, `an_unknown_loss_recovers_and_a_destroy_does_not` and `exactly_one_loss_reason_recovers` on the floor, `a_destroyed_device_is_not_rebuilt_and_the_state_survives_the_refusal` on the gate |
| `opens_a_device(Cpu) = true` | RED, `tiers_a_and_b_open_a_device_and_tier_c_does_not` |
| `record_loss` without its `is_none` guard | RED, `the_first_loss_reported_is_the_one_kept`, floor, no adapter |
| `supports_compute` negated | RED, `compute_availability_on_a_real_context_matches_the_decision` |
| `recover` clears the flag and keeps the dead device | RED, `an_unknown_loss_is_rebuilt_and_the_new_context_is_live` |
| `recover` accepts a live device | RED, `recovering_a_live_device_is_refused_as_not_lost` |
| loss callback not registered | RED, three device tests |
| transposing `edge`/`passes` at `run`'s call site | 2 compile errors, `cargo check` non-zero |
| `detect`'s tier C short circuit deleted | RED, `a_cpu_override_yields_no_adapter_and_spends_no_startup_cost` |

The two declared residues remain green and remain correctly declared at the
lines that carry them: deleting `resolve_adapter`'s `opens_a_device` guard
(`probe.rs:168-182` records it, F-X002 is the story that closes it), and adding
a wildcard arm to `recovers_from` (`caps.rs` and `tier-resolution.md` both say a
runtime test cannot catch it and the compiler can).

## Verified clean

**Gates**, each exit code read from the command itself, not from a pipe:
`fmt`, `clippy`, `test`, `device`, `ci`, `guards`, `guards-deep`, `prose`,
`backlog`, `deviations`, `skills`, `unsafe`, `gpu`, `wasm`. All 0. `gate gpu`
runs 9 tests serialised, all green. `wasm` reports 16,455 against the recorded
16,388, which `CURRENT_SPRINT.md` explains as pre-existing at base `3b00890`.

**Whole-diff rule sweep**, against `3b00890` rather than against the delta:
no em-dash, no en-dash, no prose semicolon, no `as` cast, no `.unwrap()`, no
`.expect(`, no `panic!`, anywhere in the staged diff including the tests.
`cargo check -p ocelli-render --target wasm32-unknown-unknown --lib` exits 0.

**The two phrase families that drove passes 1 to 5**, re-swept in full:

- *Adapter gloss.* `runs everything that`, `need(ing|s)? a real adapter`,
  `needing an adapter`, `tests that need`, `needs no adapter`, `need no
  adapter`. Twelve hits, all correct: two are explicit negations of the gloss
  (`WORKFLOW.md:120`, `bin/ocelli.sh:297`), `probe.rs:248` and `probe.rs:1192`
  say "to bite" rather than "to run", `probe.rs:1256` is the fill-rate
  instrument which genuinely needs one, and the rest are the `caps` family,
  which is true.
- *Two devices.* `never a moment`, `two devices exist`, `one-device invariant`,
  `no second device`. Six hits, all scoped: `probe.rs:9` and `:192` say "on this
  path", `probe.rs:541` quotes the header verbatim inside `measure_candidates`,
  `gpu-ownership.md:206` says "on that path" and points at the lifecycle
  section, `gpu-ownership.md:238` says "On the resolution path",
  `tier-resolution.md:364` says "on the resolution path". No unscoped site. The
  two approved design plans still carry the unqualified sentence and must not be
  edited, which your AS_BUILT note covers.

## Handover

Nothing is blocking. `/verify` and `/complete-feature` are the right next steps.
Three things to carry into the AS_BUILT entry, all of which have been agreed
across this loop rather than being new:

1. The two declared residues above, each with the story that closes it.
2. The `Limits::default()` hole, recorded at `probe.rs:265-277` and in
   `docs/lld/gpu-ownership.md:278-288`, closed by F-042 or F-X002.
3. The note that `.claude/plans/F-037-design.md` and `F-004-design.md` carry the
   unqualified two-devices claim and that approved plans are not edited after
   the fact.

And N1 above: confirm the `BACKLOG.md` row moves to `done`, because two
sentences in `docs/lld/tier-resolution.md` depend on it.
