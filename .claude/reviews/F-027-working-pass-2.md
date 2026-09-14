# F-027 review, pass 2

**Reviewed**: working tree after pass 1's remediation
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## What pass 1 raised, and what closed it

### D1, the `.201` progression rule

Closed by implementing PS3.5 rather than the design plan. Three `Mode` arms,
`.201` requiring reversibility only, `.202` requiring reversibility and RPCL,
`.203` constraining neither. Re-verified against the standard's structure rather
than against the fixtures: RPCL is a requirement **on** `.202`, not a property
that distinguishes it from the other two, and the corpus's `.203` row being RPCL
as well is what makes that concrete.

**The plan file is corrected too**, because `/complete-feature` step 9 says a
wrong `## LLD impact` is a defect the review should have caught and the same
argument applies to a wrong Approach. A plan left describing a rule the code
does not implement is a plan the next reader will believe.

### S1, the unreached component check

Closed with a spliced three-component SIZ built inside the test, not a new
fixture, since the bytes are a 49-byte marker segment and a fixture file would
hide them from the reader. Re-probed red.

### S2, the vendored unsafe surface

Closed by recording rather than excluding. Re-checked that the direction matters:
`unsafe.vendored-count-moved`, `unsafe.vendored-unrecorded` and
`unsafe.vendored-record-without-a-package` all drive the guard red, and the OK
line prints `openjph-core-0.1.0 104, ritk-codecs-0.6.0 0`. The 104 is the
guard's own scanner's count, which strips comments and string literals, and it
agrees with the independent `grep -c` audit and with F-X013's figure.

## Verified clean

- **Ten integration fixtures and two unit tests pass**, the crate is green
  across its nine targets, and `bin/ocelli.sh clippy ocelli-codec` exits 0.
- **The full floor is green at 26 gates**, including `guards`, `bench`,
  `provenance`, `prose`, `unsafe` and `pins`.
- **`bin/ocelli.sh gate native` is green at 11 steps**, three of which are new
  and run the production HTJ2K adapter over all three syntaxes natively, as
  plain wasm and as `+simd128` wasm.
- **Nine mutations red, re-run after the remediation**, with the source
  restored from a byte copy each time and the suite green afterwards.
- **The redistribution gate works in both directions**, which pass 1 found it
  did not. Unmutated it refuses and names `vendor/openjph-core-0.1.0/LICENSE`.
  With a notice planted it passes. Neither half is assumed.
- **No development profile runs that flag.** Checked by reading
  `bin/ocelli.sh`: `--floor` excludes four gates by name and `--sprint|--all`
  take every `GATES` entry, and the redistribution check is not a gate entry at
  all. It is reachable only through `/release` step 5 and through the probe. That
  matters because `--sprint` and `--all` are the same set in this repository, so
  a gate carrying this refusal would have blocked the sprint rather than the
  release, which is not what the design round decided.
- **Every new refusal arrived with its probe**, which is the rule
  `/implement-feature` section 3 states. Four probes added, two catalogue
  entries added, one stale entry removed, two site counts recorded, one
  retired, and the runbook table regenerated.
- **Nothing was widened to make a gate pass.** No tolerance, no `#[allow]`, no
  allow-list entry, no disabled gate. The one guard that changed,
  `unsafe_allowlist_check.py`, was made **stronger**: before F-027 a
  dependency's unsafe was invisible to it whether vendored or not, and it now
  refuses three states it previously could not see.
- **The one thing this story does not close is open by decision and held by a
  gate**, not by a note: `openjph-core` 0.1.0 carries no BSD notice material,
  the check that refuses publication fails today, and `docs/SOURCE-POLICY.md`,
  `docs/hld/DEVIATIONS.md` D-22 and `/release` step 5 all say so.
