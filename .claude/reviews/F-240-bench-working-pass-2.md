# F-240 benchmark corrections, independent review pass 2

Model author: local GLM-5.3-Flash-EXL3. Coordinator and repair author:
Codex f225_review. Independent reviewer: Codex root.
Reviewed final tree `30d89857457524df044b2a89a83f630621ba2a07`, including
the complete source against `de9b520fde90dabc85fdc405440362a1fa02388f`
and the four-path repair against `30f5a85a8d986be7a01abd163df35284e26dcf07`.
The immutable original model tree is `0ddc33856e3a15bc96064ac19de4e077ef532d50`.

## Result and attribution

Defects: 0. Smells: 0 in the corrected bounded instrument source.
The original model findings remain one protocol defect, one evidence-overclaim
smell and one numerical-comment nit. The prompt's missing profile-ID update
is a separate coordinator error. The first Codex review missed the protocol
and evidence defects. No matched Claude implementation was run for this task.

Root read the complete fixture helper, runner delta, corrected test controls,
profile provenance and the complete D1/S1 repair. ICC.1 profile ID hashing
zeros flags, rendering intent and the ID before hashing. The corrected
560-byte derivative agrees with the independently generated profile SHA256
`94977cfdf14ee323cfc5709c2382942f21f67073f3525a30c3c5d46a7b101158`.
The helper streams bounded chunks instead of holding a whole RGB16 payload.
Correct reply bytes come from the independent hand vectors. The small fake
wrong-profile and high-byte-only index controls are intentionally wrong
implementations, not a second reference colour pipeline.

The two established untagged workloads now keep their original HTTP sequence.
Only ICC cases read the additional retained census after decode and after the
window steps. The new real-child wire controls reject either extra read on
both original workload kinds, and legacy census metadata stays null. No
budget, threshold, fixture writer or named existing window sequence changed.

The retained census proves charged residency and frame stability. The report
and source no longer claim that it detects every transient or same-sized
rebuild. Source/lifetime inspection and actual timings remain separate
observations. ICC16 requests 512 MiB explicitly and checks the child's actual
budget. Other workloads preserve their previous environment policy.

Root independently executed all 59 tiny controls, direct exit 0, recorded in
`f240-root-repaired-bench-tests.json`. The author retained original failures,
each extra-read mutation returning 1, 23 refusal-site mutations and the
profile-ID mutation. Their author receipts are separate from root's execution.
The permanent guard suite uses disposable checksum-bound profile copies and
never mutates the product or tracked assets.

## Remaining integration and acceptance

The complete patch, caller-specific guard policies and both explicit test lists
must be composed and checked on the current source union. That composition
is a separate review. Actual 8192-pixel fixtures, full reply observations,
512 MiB admission, timings and raw RSS remain unrun. This review does not
complete AC15 or supply a feature profile, browser parity or a final sprint
verification. No corpus or size budget was changed.
