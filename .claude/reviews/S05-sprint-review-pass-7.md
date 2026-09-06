# S05 sprint review, pass 7

**Reviewed**: complete remediated sprint diff `7c5e29c..5156942`
**Reviewer**: independent agent, did not write the sprint implementation or pass 6 remediation
**Result**: 1 defect, 1 smell, 0 nitpicks

## Defects

### D1, Green evidence still accepts impossible derived statistics

**Where**: `scripts/verify_ledger.py:333-408,476-496` and
`tools/oracle/src/frame.rs:250-325`

**What**: The pass 6 remediation closes the four exact reproductions from the
previous review, but its tail-bucket rules remain necessary conditions rather
than a proof that the producer could derive the report. It also removed the
previous full-frame maximum composition check. Starting from the unchanged
genuine 99-record report, three independent mutations are accepted:

```text
one tail pixel with unattainable zero signed sum: ACCEPT
two tail pixels with rank-at-end percentile below maximum: ACCEPT
full maximum below background maximum: ACCEPT
```

The first report has one pixel in `countOverTwo`, maximum 3, percentile 3 and
signed mean 0 in every populated region. Its only possible signed difference
is 3 or -3, never zero. Lines 403-406 check only that the sum magnitude is no
greater than `maximum * countOverTwo`, so they admit unattainable sums whenever
the tail is non-empty.

The second report has two pixels in `countOverTwo`, maximum 255 and percentile
3. The producer selects rank `ceil(0.999 * 2) = 2`. That rank consumes every
pixel, so the percentile must be the maximum, 255. Lines 380-383 make the
percentile exact only when the tail count is one and accept any value from 3 to
the maximum for this equally exact two-pixel case.

The third report partitions two full-frame pixels into an image pixel with
maximum 3 and a background pixel with maximum 255. It reports full maximum 129.
Its bucket totals and weighted signed mean compose exactly, so lines 476-496
accept it even though a union's maximum must be the maximum of its two parts,
255. This check existed before the pass 6 remediation and was deleted rather
than covered by a targeted probe.

**Why it is wrong**: `ChannelReport::of()` derives the maximum, percentile,
buckets and signed sum from one exact 256-bin histogram. Full-frame statistics
are derived from the image and background partition. None of the three
accepted reports can be emitted by that producer. An edited report can
therefore receive a green ledger attestation while contradicting the evidence
model it claims to serialize.

## Smells

### S1, GreenUnmeasuredState is still a test-only parallel authority

**Where**: `tools/oracle/src/report.rs:188-236,1027-1034,1181-1199`,
`tools/oracle/src/attribution.rs:836-975` and
`scripts/guards/catalogue.py:1475-1498,1604-1616`

The typed `Rung` repair is production-owned and closes that half of pass 6 S2.
`GreenUnmeasuredState::ALL` does not. A repository-wide use search finds the
type only at its declaration and inside the contract test. Production
attribution independently creates class, qualifier and rung combinations with
branches and overlays in `attribution.rs` and never consults or validates
against `GreenUnmeasuredState::ALL`.

The seven new catalogue acceptance probes also read a state from the JSON
contract and synthesize a report from that same state. They prove that Python
accepts the declared list, not that Rust production reaches every declared
state or cannot emit an undeclared one. The Rust ratchet then proves only that
the manually declared Rust list equals the manually declared contract list.
This leaves the exact stale-authority route identified in pass 6. Production
construction should use or validate against the type, or producer-facing tests
should drive every state and prove that no additional green unmeasured state
is reachable.

## Nitpicks

None.

## Verified clean

- The genuine 99-record report is accepted unchanged.
- All four pass 6 defect reproductions are now refused. These cover a signed
  sum impossible for exact one-code buckets, negative zero, a single tail
  pixel whose percentile differs from its maximum and a class-one pass with no
  informative pixels.
- The genuine report is accepted from both the repository root and
  `/private/tmp`, so relative path validation is now anchored independently of
  caller location.
- All pass 5 mutation probes remain refused, including canonical input aliases,
  invalid green attribution, impossible maxima, predicate and bias
  contradictions, oversized counts, duplicate keys, non-finite numbers,
  record order, record identity and hash reconstruction.
- The contract loader now rejects unknown container members, duplicate keys
  and invalid semantic declarations. Its typed rung vocabulary is derived from
  the same `Rung` used by production attribution.
- The pass 6 guard catalogue adds direct refusal probes for the previously
  untested semantic and path branches. D1 identifies the deleted maximum rule
  and two remaining tail cases outside those probes.
- No additional defect, smell or nitpick was found in the pass 6 remediation
  or the rest of the sprint diff.
