# F-037 review, pass 5

**Reviewed**: the staged working tree, `git write-tree` =
`e8ba4a5ac20e1af8e75a04b2856e40e3e614636e`, against base `3b00890`, with the
pass-4 remediation delta read as `git diff b810ae52 e8ba4a5a`. Independent
reviewer, not the author.
**Result**: 1 defect, 0 smells, 5 nitpicks

**This pass is NOT clean.** One defect. It is a seventh instance of pass 4's
D1, in the same comment block as the paragraph written to warn against it, five
lines below. Six sites were corrected and this one was missed.

All mutations were reverted. The tree hash at the end of this pass is the same
as at the start.

**Pass line**: F-037, pass 5, 1 defect, 0 smells, 5 nitpicks. Trend: 10/3/4,
4/1/5, 2/1/5, 1/0/4, 1/0/5. No finding has ever survived a pass. This one is a
missed instance of the previous pass's finding rather than a new class, which
is the first time that has happened in this loop and is worth noting to the
operator under `.claude/WORKFLOW.md`'s escalation table, although it does not
meet the "same finding survives three passes" trigger.

## Defects

### D1, a seventh site of the adapter gloss, in `bin/ocelli.sh` itself

**Where**: `bin/ocelli.sh:308-311`, inside the `gpu` arm comment.

```
308  # `-- --ignored` runs ONLY the ignored tests, which is the whole set and
309  # nothing else: the default `cargo test --workspace` in the `test` gate
310  # already runs everything that needs no adapter, so this gate is exactly the
311  # complement and neither gate is a subset of the other.
```

**What**: "the `test` gate already runs everything that needs no adapter" is
false, for exactly the reason pass 4's D1 gave.
`a_cpu_override_yields_no_adapter_and_spends_no_startup_cost` needs no adapter
and the `test` gate does not run it, because it is `#[ignore]`d.

**Why it is wrong**: it is the same property gloss over the same growing set,
and it is thirteen lines below the paragraph this remediation added to forbid
it, which now reads:

> It is deliberately not described as "the ones needing an adapter": most do,
> and `a_cpu_override_yields_no_adapter_and_spends_no_startup_cost` does not.
> ... a gloss over a growing set goes stale the first time a test joins it for a
> new reason. **This one already has.**

The two paragraphs are in the same `case` arm and contradict each other.

**The conclusion the sentence draws is still true**, which is why this is a
one-line fix and not a design problem: the two gates ARE exact complements and
neither is a subset of the other, because the partition is the `#[ignore]`
attribute. Only the premise offered for it is wrong. The rest of the block
already has the right vocabulary.

**Evidence**:
```
$ cargo test -p ocelli-render --lib
test result: ok. 75 passed; 0 failed; 3 ignored
```
Three ignored in the lib target, one of which is the tier C test. That it needs
no adapter is measured three ways, not inferred: its own `#[ignore]` string says
"only bites where adapters exist", it passed under MUT Y (`enumerate` stubbed to
`Vec::new()`), and it passed under MUT Z (every `request_device` forced to fail).
```
$ grep -rn "needs no adapter\|without an adapter" bin/ scripts/ .claude/WORKFLOW.md docs/lld crates/ocelli-render
bin/ocelli.sh:310   <- this one
(all other hits are about `caps` being testable without an adapter, which is true)
```

## Smells

None.

## Nitpicks

### N1, "the hardware tier" in the `GATES` row is ambiguous

`bin/ocelli.sh:87`, `gpu|YES|ocelli-render's #[ignore]d tests, the hardware tier
(E6.1, D-04)`. The definitional half is correct and complete. The apposition
reads two ways: "the hardware tier of this project's test pyramid", which is
true, or "tests about the hardware tier", which one of the nine is not, since
`a_cpu_override_yields_no_adapter_and_spends_no_startup_cost` is about tier C.
The GPU column already carries `YES`, so the apposition earns little. Not
blocking, and I am not asking for a fifth wording of this line.

### N2, `docs/lld/gpu-ownership.md:293` is 136 characters

Every neighbouring prose line in that paragraph is 71 to 79. The long lines
elsewhere in the file are all table rows, where length is unavoidable. This one
is a reflow artefact of the N4 insertion. Markdown renders it identically and
`prose` is green.

### N3, `scripts/guards/census.py:188` left a ragged continuation

`"reason as `test` applies unchanged. It "` is now a short fragment where the
surrounding lines are full. Cosmetic, and the joined string is correct.

### N4, `bin/ocelli.sh:298` names a test function in prose

Naming `a_cpu_override_yields_no_adapter_and_spends_no_startup_cost` makes the
comment stale if that test is renamed. This is a much smaller version of the
risk the same paragraph is about, because naming an exception is inherently
specific and a renamed test is a grep away. Recording it rather than asking for
a change.

### N5, two carried nitpicks, neither blocking

`docs/lld/tier-resolution.md:460` and `:467` still say "F-037 has landed" while
`BACKLOG.md:163` says `in-progress`. This self-resolves at `/complete-feature`,
which is the next command. And the CI floor still drives `resolve_adapter` in no
test at all, since the tier C test became `#[ignore]`d in pass 4. No detection
was lost, because the floor version killed no mutation.

## Answers to the five questions

**1. Are the six new descriptions mutually consistent and true, including
"most of the tests here open one"?**

Five of the six are true and consistent. A seventh site in the same file is not,
which is D1.

| Site | Text | Verdict |
|------|------|---------|
| `bin/ocelli.sh:87` | "ocelli-render's #[ignore]d tests, the hardware tier" | true, apposition ambiguous, N1 |
| `bin/ocelli.sh:295-306` | "THE SET IS EVERY TEST MARKED `#[ignore]` IN `ocelli-render`, and that is the whole definition" plus the named exception | true, and "What the set is today: the device lifecycle, that short-circuit test, and the fill-rate instrument" is exhaustive: 7 + 1 + 1 = 9 |
| `bin/ocelli.sh:335-340` | "**Most of the tests here open one**" | **true**, 8 of 9 |
| `bin/ocelli.sh:429-433` | "It runs `ocelli-render`'s `#[ignore]`d tests" | true |
| `.claude/WORKFLOW.md:117-122` | "That is the whole definition ... at least one is ignored because its MUTATION only dies where adapters exist rather than because it needs one to run" | true and precise |
| `scripts/ci_floor_check.py:462-466` | "which is the whole definition of the set rather than a claim about why each one is ignored" | true |
| `scripts/guards/census.py:187-193` | "filtered to the `#[ignore]`d tests" | true |
| `bin/ocelli.sh:308-311` | "the `test` gate already runs everything that needs no adapter" | **FALSE, D1** |

**2. Are both surviving cases in the refined `#[ignore]` doc real? You asked me
to measure the second.**

Both are real and both are now measured, not reasoned.

The doc says: "`clock` is reached only from `measure`, which only the `Opened`
arm of `measure_candidates` calls. So with the short circuit deleted the
mutation still survives on a machine that enumerates nothing, and also on one
that enumerates an adapter whose device request fails."

- **Case one**, enumerates nothing, measured in pass 4: `enumerate` stubbed to
  `Vec::new()` with the short circuit deleted, test passes.
- **Case two**, enumerates but no device opens, measured this pass. I replaced
  the `request_device` call inside `measure_candidates`'s attempt closure with
  an unconditional `Err`, alongside deleting the short circuit:
  ```
  MUT Z: short circuit deleted AND every request_device fails
  test probe::tests::a_cpu_override_yields_no_adapter_and_spends_no_startup_cost ... ok
  test result: ok. 1 passed; 0 failed; finished in 1.04s
  ```
  Green, so the mutation survives, and the 1.04 seconds is itself the evidence
  that adapters really were enumerated and devices really were attempted. The
  clock was still never called, because `measure` is reached only from
  `AttemptOutcome::Opened` and this run lands in `NoDevice`.

The mechanism sentence is also exactly right: `detect`, `resolve_adapter` and
`measure_candidates` never call `clock` themselves, so `measure` is the only
route, and only the `Opened` arm calls it.

**3. Is N4's new sentence accurate about what else is in the set?**

Yes. `docs/lld/gpu-ownership.md:289-293`: "**They are not that gate's whole
set**, which is every `#[ignore]`d test in the crate and also holds the tier C
short-circuit test and the fill-rate instrument." The gate runs nine. Seven are
device lifecycle, six in `tests/device.rs` and one in `gpu.rs`, which is what
the sentence before it names. The remaining two are exactly the two it names.
Nothing else is in the set.

**4. Did any previously-red mutation regress, including the ninth?**

No. Nine of nine still red, each reverted.

| Mutation | Result |
|----------|--------|
| `recovers_from(Destroyed) = true` | RED, 2 floor tests plus `a_destroyed_device_is_not_rebuilt_and_the_state_survives_the_refusal` |
| `opens_a_device(Cpu) = true` | RED |
| `record_loss` without its `is_none` guard | RED, floor, no adapter |
| `supports_compute` negated | RED on the `gpu` gate |
| `recover` clears the flag and keeps the dead device | RED |
| `recover` accepts a live device | RED |
| loss callback not registered | RED, three device tests |
| transposing `edge`/`passes` | 2 compile errors |
| **`detect`'s tier C short circuit deleted** | **RED**, the ninth, still biting |

The two declared residues are still green and still correctly declared in the
places that matter: deleting `resolve_adapter`'s `opens_a_device` guard, and a
wildcard arm in `recovers_from`.

**5. Is this pass clean?** **No.** One defect, zero smells. It is a single
sentence at `bin/ocelli.sh:308-311`. Everything else in this remediation is
correct, and the structural change you made instead of patching six sentences is
the right one.

## Verified clean

**The property gloss is gone everywhere else.** Swept
`need(ing|s)? a real adapter`, `needing an adapter`, `tests that need`,
`needs no adapter`, `need no adapter`, `without an adapter`,
`no adapter is needed` across `bin/`, `scripts/`, `.claude/WORKFLOW.md`,
`docs/lld/` and `crates/ocelli-render/`. The only survivors are correct: the two
occurrences that are explicit negations of the gloss (`WORKFLOW.md:120`,
`bin/ocelli.sh:297`), `probe.rs:248` and `probe.rs:1192` which say "to bite"
rather than "to run", `probe.rs:1256` which is the fill-rate instrument and does
need one, and the `caps` family which is about `caps` being testable with no
adapter and is true.

**The two-devices claim.** Re-swept with the ten phrasings from pass 4. Six
scoped survivors, all correct, no unscoped site. Unchanged from pass 4.

**Gates.** `fmt`, `clippy`, `test`, `device`, `ci`, `guards`, `guards-deep`,
`prose`, `backlog`, `deviations`, `skills`, `unsafe`, `wasm` and `gpu` all exit
0, read from the command itself. `gate gpu` runs 9 tests serialised, all green,
in 2.5 seconds. `gate --list` shows `gpu YES` beside `oracle`. `wasm` reports
16,455 against the recorded 16,388, which `CURRENT_SPRINT.md` explains as
pre-existing at base `3b00890`.

**Rules.** `cargo check -p ocelli-render --target wasm32-unknown-unknown --lib`
exits 0. No em-dash, no prose semicolon in the pass-4 to pass-5 delta. No `as`
cast, no `.unwrap()`, no `.expect(`, no `panic!` anywhere in the whole staged
diff against `3b00890`, re-swept rather than assumed.

**On what happens next.** If D1 is fixed and nothing else moves, pass 6 needs to
re-verify that one sentence, re-run the nine-mutation battery and the gates, and
nothing more. I have no other open concern about this story.
