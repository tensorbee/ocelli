# F-240 normalized control, bounded working review pass 1

**Author**: Codex controller for the private transplant, driver adaptation and
LLD fragment. **Independent reviewer**: root Codex, who authored none of those.
**Result**: 0 defects, 0 smells, 0 nits in this bounded source and evidence scope.
The root-authored plan note records diagnostic scope and leaves AC14 unchanged.

Source `c4d6e4b3d17e35c54b4a2240cdb2ff10176cf336` differs from literal historical
12797792 in exactly two files. Its entire cpu.rs equals the already reviewed
c1fe3037 donor. The other delta moves pure RasterColor validation on one
immutable transform outside the band loop. Production dimensions are validated
before HeldFrame construction. No ICC implementation is imported. Guards,
ties-to-even and the separate monochrome address rule remain intact.

Root compared the full private-driver withFixtures body with the historical
entry byte for byte. The new source, binary, output and helper identity checks
are explicit. Focused CPU tests pass two cases with 126 filtered, and the
unique-target release build passes. Root independently rehashed its binary
as `0dc9860fe164d0842dd7a31262ea8ac46df0822a07e58a207d5c71d6ea94c25f`.

The first native run correctly failed because process inspection was denied:
zero RSS samples and no valid result. Its empty output directory and failed
receipt were preserved before the unchanged driver was rerun with access.
The permitted run exits zero in 57.538 seconds. Root recomputed both nine-step
medians and ranges, checked decoded-first and held-step sources, all marker
counts, positive RSS samples with zero failures, historical fixture identities,
load averages and before/after source, binary and driver bindings.

The LLD table accurately records RGB median 2561.928 ms and palette median
3000.127 ms. These are native HTTP observations with external desktop activity,
not a fully idle host or browser comparison. They remove two known shared
optimization differences and do not isolate ICC overhead or satisfy AC14's
within-noise wording by substitution. Literal historical evidence stays intact.

A separate fresh GLM source audit exited zero in 700.421 seconds with six calls
and no product finding. Its immutable report SHA-256 is
`368fad9587802df7df78281b015c51c15b4321b20694ad393e03ab882a9e67b2`.
Its 479 words exceed the requested 450. Its source-only limits are retained.
The phrase without revalidation concerns palette-entry acceptance, while
per-band source and extent checks remain. Native execution belongs to the
controller, not that model review. Raw counters are unnormalized and raw cost zero
is not a free-compute claim.

Complete receipts, patch, independent source assertions, raw measurement and
qualifications are under
`/private/tmp/ocelli-s46-codex/pre-icc-normalized-control/` and
`/private/tmp/ocelli-s46-codex/pre-icc-normalized-audit-glm/`.
No feature completion, overall profile, GPU parity, accepted noise comparison
or operator observation is supplied by this bounded review.
