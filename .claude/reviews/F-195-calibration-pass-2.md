# F-195 review, pass 2

**Reviewed**: uncommitted working tree on sprint/s43 for F-195 after the pass 1 remediation
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 2 smells, 3 nitpicks

## Acceptance challenged

AC1 through AC10 still hold in the current source. The imager fixture uses 0.2 and 0.4 so a swap fails. SUVbsa 90 is exact for these inputs because 5 times 1.8 rounds to 9 before the 10000 scale. The route uses the square 0.2 pair and the unit fixture covers the swap.

## Evidence

The reviewer had no shell. The host had re-run calibration, stats, the route tests and clippy after the pass 1 fixes. The ERMF multiply had been shown red before the guard restructure.

## Defects

none

## Smells

### S1, the fixture comment still cited C.8.7.3

**Where**: `crates/ocelli-geom/tests/calibration.rs` inline comment above the imager assertion
**What**: The header cited C.8.7.5 and the nearest comment still cited C.8.7.3.
**Why it is wrong**: C.8.7.3 is the collimator module. The magnification factor is C.8.7.5.
**Evidence**: Reading the test file against the updated plan.
**Scope and regression**: The citation next to the hand fixture.

### S2, two new contracts were only half tested

**Where**: `crates/ocelli-geom/tests/calibration.rs`
**What**: No test put a NaN factor alone with ultrasound and manual. No test required `length_ratio` to keep a negative sign.
**Why it is wrong**: Moving the incompleteness return above the finiteness check would turn a lone NaN factor into Unavailable and stay green. `Ok(ratio.abs())` would stay green.
**Evidence**: Reading the tests against `calibration.rs`.
**Scope and regression**: Those two contracts only.

## Nitpicks

The stats module comment was wrapped after "this module". `accept` repeats the finiteness check. A negative contour area has no test.

## Verified clean

The units wording in the module comment, the plan, the LLD and the test header names C.8.11.4, C.8.7.5, C.8.5.5.1.17 and C.8.5.5.1.15, and 0003H as centimetres. A NaN imager half with no factor returns Frame. A lone finite factor with manual returns Unavailable. The shared decay and the route delimiters match the plan.

## Limitations and handoff

Change the stale citation. Add the lone NaN factor test and the negative ratio test. Repeat the ERMF multiply mutation after the guard change.
