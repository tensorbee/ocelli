# F-239 shared palette candidate reader, independent review pass 1

Author: Codex f225_review controller. Reviewer: Codex root, who authored
neither the adapter nor its new tests. Reviewed one-file repair at tree
`d19fe5eda71575282cfbd3171ea3f6bf408c858e` against source
`e4a3b620ea853f22a1bf1824008f134c7cdf9ae3`.
Result: 0 defects, 0 smells, 0 nitpicks in this bounded repair.

The complete private palette reader chain is removed. The candidate calls
`ocelli_dicom::palette_color_lut` with eleven projected metadata tags.
Primitive projection preserves the shared provider's input representation.
No Pixel Data is projected or copied by this adapter. Shared reader policy
therefore owns descriptor validation, R1, actual entry depth, signed first
mapping and refusal of any segmented table. Missing palette values and
nonprimitive inputs remain explicit refusals rather than guessed defaults.

The decoder-reported RGB bypass and sidecar-driven non-palette selection
remain the candidate's policy. Malformed unused palette attributes cannot
make an ordinary RGB frame fail. The provider is consulted only after the
existing decode dispatch has returned. This avoids the complete-object
projection's native pixel copy and encapsulated-fragment refusal.

Read the provider and public metadata construction path in addition to the
adapter. The new real Part 10 controls exercise load_colour and the public
colour transform, including R1, true 16-bit entries, below/above-first range
clamping, segmented presence, missing channels, illegal depth and RGB bypass.
A separate controller-owned semantic mutation removes only the segmented
presence projection, fails the actual consumer control, and passes restored.
Provider-only assertions are not presented as the candidate observation.

The first test version at 8ec20 used PixelError Display despite that type
implementing Debug only. Native compile failed before selected tests ran.
The controller's v2 changes only those test diagnostic expressions. Actual
v2 receipts report two selected tests passing, 118 filtered, named crate
check and clippy passing. Census passes without a catalogue amendment because
its Rust-excluding suffix policy is unchanged. Root reviewed these receipts
but did not rerun those same native commands in this review.

The implementation ruling draft preserves the same selection/refusal policy
and explicitly leaves AC27's actual two-palette candidate byte comparison
open. Root finds no new design ambiguity in that amendment. No corpus,
feature-profile, whole-feature or sprint completion is claimed here.
The additional local GLM review remains a separate unfinished observation.
