# F-240 benchmark working review, pass 1

Original author: local `dgx/GLM-5.3-Flash-EXL3`. Independent original-output
reviewer: Codex. Codex authored the subsequent input correction, guard tests
and registration, which require root's independent review. No author supplies
independent approval of their own material here.

## Identity and disposition

The prepared input is `efe85d40e29f373587c97853b979d3d73ee30e90`, comprising
the reviewed F-239 instrument `de9b520fde90dabc85fdc405440362a1fa02388f`
and the lead's new RGB16-only 512 MiB budget amendment. ICC input profiles
are the project's own frozen tree `81987bd7c13f8e981b62838e18d8afa21b9b7ce4`.
The complete initial GLM output was preserved before any coordinator edit at
`0ddc33856e3a15bc96064ac19de4e077ef532d50`. Its eight changed source/test/
asset paths and the unchanged existing fixture helper are copied under
`/private/tmp/ocelli-s46-codex/f240-colour-bench-glm/initial-output/`.
Initial source and index comparison found no change in any of the 2017
unowned tracked payloads. The sole new unowned review file was the separately
requested More audit, authored by Codex and excluded from GLM attribution.

The initial bounded Codex review found one numeric comment nit but missed
one protocol defect and one evidence overclaim subsequently identified by
root's independent review. The complete original-output findings are now
1 defect, 1 smell and 1 nit, with the coordinator profile-ID omission counted
separately. Neither this record nor a private-unit pass accepts the actual
AC15 measurement, final integration or the coordinator-authored corrections.

## Original model run and checks

One prompt and one model client, no retries or interruption. Start
2026-11-08T15:08:50.425201Z, end 2026-11-08T16:07:22.109541Z, direct process
exit 0, elapsed 3511.674339791 seconds. OpenCode recorded 114 steps,
272932 input tokens, 48119 output tokens, 84987 reasoning tokens and 22689216
cache-read tokens. Tool-call success counts describe transport completion,
not shell exit or test success. The local cost field is zero and is not an
estimate of equivalent hosted-model cost.

Raw prompt, run log, UTC and monotonic timing, stats and sanitized session
`ses_e444b81d3ffejlG1cyD77cOYb8` are preserved in the evidence directory.
The original independent run of
`node --test tools/bench/tests/viewer_colour_frame_test.mjs` exited 0 with
45 tests and no skips. The unchanged project ESLint rules applied to all four
owned JavaScript files, including the new helper, exited 0 with no warnings.
No full-sized file, real viewer, browser, GPU, Cargo or actual timing subject
was run by this task. Every fixture-producing control uses edge 4.

## Reviewed protocol and inputs

The two previous F-239 fixtures retain their exact edge 4 golden bytes, old
sample positions and nine C100..108/W200 steps. The public subject value
remains the untagged RGB8 median. Both new fixtures have one native RGB frame,
interleaved unsigned8 or unsigned16, allocated and stored bits equal. The
retained Single-frame SC IOD has no pixel-format restriction under PS3.3
2026d A.8.1.1, and its module table permits the ICC Profile Module. These are
synthetic instrument inputs, not a claim that the minimal benchmark datasets
have every required IOD attribute.

ICC8 uses the exact gamma 0x0233 profile. ICC16 derives three curv tables
with entries 0,65535 from gamma1. Independent expected marker vectors were
computed before model implementation using the actual quantized colourants,
ICC.1 normalization and IEC sRGB encoding. They are not product readback.
The marker codes cover profile bypass and full 16-bit indexing, including
code 15, whose correct first byte 1 differs from an8-bit-indexed byte 0.
The varied16-bit background spans 8192 distinct codes per channel rather than
only the five neutral markers. The minimum sampled half-tie distance in the
chosen windows is0.0014883, and no tolerance was changed.

Each workload starts its own process. File ingestion is untimed. Its first
id-form preview must say decoded, all nine later id-form replies must say
held, and every response is consumed in full with strict channels, dimensions,
window echo, length and sampled-byte validation. Profile metadata must say
icc_applied true and icc_kind empty, including in wrong-byte controls.
Only ICC16 supplies OCELLI_CACHE_MIB=512. Its requested budget and actual
held budget must agree. Old cases and ICC8 inherit the caller environment.
The one-GiB body cap is above the new 384-MiB RGB16 payload plus metadata.
Actual largest-frame admission remains unobserved and must be investigated
rather than silently raising the budget if 512 does not admit it.

Held reports record samples plus the expected 3072/786432-byte TRC minimum
and stable used/frame counts after first decode and warm steps. This observes
stable charged residency. It does not by itself prove that no transient parse
or TRC evaluation occurred between observations. The library's independent
lifetime tests complement this instrument. RSS retains actual positive
readings, failures and the nominal 20-ms period, from announced URL through
the final measured preview. It does not cover process birth to stop.

## Coordinator input correction and original nit

The prompt directed cloning gamma1 and preserving all non-TRC bytes, but
omitted that its nonzero ICC profile ID must be recomputed. The original
model output followed that instruction. The independent ICC.1:2022 7.2.18
probe exited 1 because the derivative retained gamma1's identifier. This is a
coordinator prompt error, not counted as a model functional defect.

After preserving initial output, Codex added the required MD5 identifier with
flags, rendering intent and ID zeroed for hashing. Independent pre-correction
arithmetic gave profileID `ef3a144416bbf68ae488f9978642e3b1` and SHA-256
`94977cfdf14ee323cfc5709c2382942f21f67073f3525a30c3c5d46a7b101158`.
The separate comparison-fixture author derived the same profile without
reading benchmark expected pixel vectors. Only profile metadata changes.
Colourants, TRCs and sample expectations remain unchanged. A permanent test
binds the literal expected checksum and profile ID. Omitting recomputation
now exits1 in that test and the restored independent probe exits 0.

The model-authored background comment said the maximum was 65520, while its
writer and test correctly give 65528 for 8191*8. Codex corrected this numeric
comment nit. Intermediate model errors in bytes-per-sample naming and a
verification-script JSON path were corrected within its original run and
are visible in the raw log. They are not unrepaired final-output defects.

## Guard registration and permanent controls

The complete root AC31 four-path patch was applied three-way after model
freeze. The older private source required bounded conflicts in all four
paths. Resolution preserves the two exact new guard entries and three tests
from that patch, with existing private source retained. Unchanged newer-root
comparison/pins context was not imported without its corresponding source.
The probe table was regenerated from the reconciled catalogue.

F-240 adds eight runner refusals, so bench.colour-frame moves 16 to 24 sites.
The new bench.icc-frame-fixture entry claims 15 sites. Every new site has a
permanent control. A new test-owned module copy redirects its own asset root
and exposes only two defensive leaves, leaving production source and exports
unchanged. Its copies are tiny, checksum-bound and removed after each case.
No uncovered-site budget, tolerance or gate was weakened or reset.

Fifteen helper guard mutants and eight runner guard mutants each returned
actual exit 1 under their permanent controls. The separate profile-ID omission
mutant returned 1 as well. Exact restored source hashes are in the receipts.
Original applied-metadata wrong-profile-byte and wrong 16-bit-index tests,
wrong actual budget, heap undercount and per-draw heap-growth controls remain.

The coordinator's first guard-test attempt referred to a public helper removed
in the model's final cleanup. Two new tests failed with a TypeError. Codex
replaced those references with the actual public tiny fixture builder and
unknown-workload APIs. This was a coordinator test-author mistake, not a GLM
product defect. A preliminary conflict-resolution script also used the wrong
budget JSON key before any write and was corrected to entry_sites.

Final focused command:
`node --test tools/bench/tests/viewer_colour_frame_test.mjs tools/bench/tests/viewer_icc_frame_guard_test.mjs`
exited 0 with 57 tests and no skips. All five JavaScript files lint clean.
Census, generated runbook, profile-ID probe, prose and owned-delta whitespace
checks exit0. Receipts carry direct subprocess exits, never a piped status.

## Remaining acceptance and independent ownership

Root must independently review the coordinator profile-ID repair, added tests
and guard metadata, merge the complete source delta and wire the new guard
suite into both exact test lists. The isolated inherited benchmark status
check still names F-239 pending, and the new suite is not wired here, because
those live integration edits are root-owned. No gate-readiness claim follows
from a private-unit pass. Root will perform actual 8192 measurements on the
final release candidate and record the four workloads, explicit 512-MiB ICC16
exception, first decode, nine held steps and raw RSS. AC15 remains open until
that observation exists. No code or history commit was created.

## Root independent challenge after the 30f handoff

Root reviewed frozen tree `30f5a85a8d986be7a01abd163df35284e26dcf07`
and found two further issues in model-authored code. The immutable original
report, patch and source identities at that tree are preserved.

**D1, changed legacy protocol.** The new post-first and post-window census
GETs ran for both old untagged workloads as well as the ICC cases. Those two
operations changed the established AC31 HTTP/RSS sequence, violating the
explicit requirement to keep that protocol unchanged. The two new permanent
real-child controls observe complete request order for RGB8 and PALETTE16.
Both fail on the original code at the extra GETs. Codex gated both reads on
`spec.icc` and leaves their metadata null for the old cases. Both controls now
pass. Reintroducing either extra GET separately returns actual exit 1 under
those controls, and the source was restored exactly. This is a GLM protocol
defect missed by the first Codex review, not a prompt error.

**S1, overclaim about work once.** The header and census comments asserted
that per-draw parsing or a TRC rebuild would necessarily appear as retained
heap growth. Transient work or same-sized replacement can leave the census
unchanged. Codex qualified the comments and subject definition as evidence
of retained charge and frame stability only. Source/lifetime checks and
actual timing observations provide the separate work-once evidence. The
minimum-charge comment also no longer infers that a profile was never parsed
merely because its retained charge is missing. This is a GLM evidence
overclaim smell, also missed by the initial bounded review.

The two original requirements were already explicit in the task prompt. No
new gate or workflow exception is added. The coordinator-owned correction
requires root's independent repair review. The follow-up source and receipts
are separate from the preserved 30f snapshot under `repair-handoff.json` and
`protocol-repair-from-30f.patch` in the same evidence directory. The full
combined suite after repair has 59 tiny tests. Actual full-size measurement
and exact-list wiring remain root-owned and open.
