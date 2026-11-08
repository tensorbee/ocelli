# F-244 oracle census and evidence, working pass 1

Reviewer: root Codex, independent of the census and observation author.
Scope: the AC22 census changes at source tree
`3aec4ae5e5967354a2023ab2a27d088608b5bae7` and the receipt-only docs at
`3fab32e0b9edd614e5e9cd94ee4d18bf7943f3a4`.
Result: 0 defects, 0 smells, 0 nitpicks in this bounded scope.

All 31 existing comparison census entries retain their exact values. The 17
new entries equal the actual comparator records: 16 weak and one with an
unstated threshold. The two new JPEG XL paths equal the observed decode
refusals and join existing syntax-specific entries. Their error, boundary and
transfer syntax predicates are unchanged. The root probe compares staged JSON
with HEAD and with actual reference and comparator outputs, and exits 0.
Its first scratch run used an incorrect schema key, views instead of
unmeasured. That harness error and the corrected receipt are both retained.

The final unchanged named oracle gate exited 0 in 164.87 seconds from the
reviewed source in an isolated detached worktree with private real output
directories. It reached 119 stack rows, read back 110 and accounted for nine
unsupported rows. Three volumes produced nine reformats and one volume
refusal was declared. Both reference passes matched. Default fault self-tests
and event catalogue checks passed. Identity comparison has 71 passing views,
zero failures, 48 unmeasured and none absent. All 29 comparator mutations
were detected. The earlier missing-census gate refusal is preserved.

The reference uses the pinned ANGLE SwiftShader WebGL2 contract. An identity
comparison compares the reference with itself and does not render an Ocelli
candidate. The heavily clipped frames and the EOT metadata-reader failure
remain explicit limitations. Parser correctness is covered by the earlier
independent parser reviews and native all-frame tests, rather than inferred
from this reference run. No threshold, refusal policy or gate was relaxed.

The final receipt update changes prose only. Feature integration and the
feature profile remain open. No commits were made.

Evidence: `/private/tmp/ocelli-s46-codex/f244-census-root-review-green.json`,
`f244-oracle-gate-final.json`, the corresponding logs, and
`/private/tmp/f244-ac22-final-observation.json`.
