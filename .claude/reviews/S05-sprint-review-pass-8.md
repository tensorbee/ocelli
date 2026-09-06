# S05 sprint review, pass 8

**Reviewed**: complete remediated sprint diff `7c5e29c..4006590`
**Reviewer**: independent agent, did not write the sprint implementation or pass 7 remediation
**Result**: 2 defects, 1 smell, 0 nitpicks

## Defects

### D1, Approximate signed means are accepted as exact producer values

**Where**: `scripts/verify_ledger.py:499-507` and
`tools/oracle/src/frame.rs:101-116,298-302`

**What**: Starting from the unchanged genuine report, I replaced each
populated region of a class-two record with one tail pixel whose maximum and
percentile are 3, but whose signed mean is `2.9999995`. The report was accepted:

```text
one tail pixel with non-producer approximate signed mean: ACCEPT
```

The verifier multiplies the mean by the pixel count and uses
`math.isclose(..., abs_tol=1e-6)` to accept a nearby integer. It then rounds the
result and proves attainability for that different integer. With one pixel of
absolute difference 3, the producer can emit only signed mean 3 or -3. It
cannot emit `2.9999995`.

**Why it is wrong**: The producer converts an exact `i32` signed sum to `f64`
and divides it by an exact `u32` pixel count. A verifier may need to recover the
candidate integer despite multiplication rounding, but it can then require
that `signedMeanDiff == candidate_sum / pixels`. The current absolute tolerance
instead creates values that no producer calculation can serialize. Edited
evidence can therefore receive a green ledger attestation.

### D2, Percentile and signed-sum proofs do not describe the same tail histogram

**Where**: `scripts/verify_ledger.py:481-507` and
`tools/oracle/src/frame.rs:218-249,298-325`

**What**: The new signed-sum feasibility algorithm is exact for the four
published buckets and maximum considered alone. The percentile rule is also
exact for the rank cases it can infer. The verifier does not prove that both
values can come from one distribution inside `countOverTwo`.

A class-two report with 2,000 tail pixels, maximum 255, 99.9th percentile 3 and
signed mean 255 is accepted:

```text
tail percentile and signed sum cannot share one histogram: ACCEPT
```

The percentile requires at least 1,998 of those pixels to have absolute
difference at most 3. Since every pixel is in `countOverTwo`, those pixels must
have absolute difference 3. The signed mean of 255 requires every signed
difference to be positive 255. Both marginal checks pass, but the conditions
cannot hold in one producer histogram.

**Why it is wrong**: `ChannelReport::of()` derives the buckets, maximum,
percentile and signed sum from the same 256-bin histogram and signed sample
stream. Validating the fields independently is not enough. The attainable-sum
calculation must incorporate the order-statistic constraints imposed by the
reported percentile, or an equivalent joint proof must reject this report.

## Smells

### S1, Standing probe generation mirrors two contract values

**Where**: `scripts/guards/catalogue.py:1506-1522,1657-1668` and
`tools/oracle/report-contract.json:157-205,219-220`

The catalogue helper `_comparison_run_hash()` hardcodes
`sha256-rgba8-run-v1` even though the fixture and verifier obtain the run hash
algorithm from the closed report contract. The green-unmeasured probe factory
also hardcodes `range(7)` even though the states are already an array in that
contract.

These copies do not break today's probes. They make future contract changes
require an unrelated edit in the probe implementation, and the state count can
silently omit direct acceptance coverage for a newly declared state. The
catalogue should read both values from the same contract used to construct its
green fixture.

## Nitpicks

None.

## Verified clean

- The pass 7 reproductions now refuse. A single tail pixel with signed sum zero
  fails feasibility, a two-pixel rank-at-end percentile must equal the maximum,
  and the full-frame maximum must equal the maximum of image and background.
- I compared `_signed_sum_is_attainable()` with exact exhaustive enumeration
  for 825 tractable bucket and maximum combinations. I then used dynamic
  enumeration for 5,537 wider cases with up to six one-code pixels, six
  two-code pixels, seven tail pixels and maxima through 255. Both runs found
  zero false accepts and zero false refusals.
- The percentile last-rank condition matched an independent rank calculation
  for every pixel count from 1 through 10,000, with zero mismatches.
- `GreenUnmeasuredState::permits()` is now called by the production
  `RunReport::gate_verdict()`. Every undeclared unmeasured class, qualifier and
  rung combination makes the run red. The test iterates every declared state
  and checks an undeclared control, so pass 7 S1 is closed.
- The scheduler-order repair compares the two command outputs as a set while
  still requiring exactly `M1` and `M2`. It removes only the nondeterministic
  ordering assumption and retains the execution and parser assertions.
- The contract loader, path anchoring, maximum composition, typed rung and all
  earlier report-shape repairs remain intact. No regression was found in the
  rest of the sprint diff.
