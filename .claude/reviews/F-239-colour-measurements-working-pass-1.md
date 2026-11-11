# F-209 colour amendment and largest-frame LLD, independent review

Reviewer: Codex `/root/f225_review`. Both drafts were written by root Codex.
Result: 0 defects, 0 smells, 0 notes in this bounded source and record review.
No draft or production source was edited by the reviewer.

The exact review source is tree
`5e6b3b81c398a557f1394d90df2c94b5e92617fa`. Draft and JSON hashes are in
`colour-amendment-lld-review-identity.json`.

## F-209 amendment

F-239 item 14 and AC28 authorize window linking only between colour-kind
peers. `copyWindow` independently checks drawability and matching colour
kind, preserves target invert, and still gates modality presets through
`presetOffered`. Reset affects the source and then calls the same window
copy. Supported RGB remains drawable through the colour route.

`copyZoom` has separate state and uses visible single-image stages, while
skipping hidden stages and tiles. The F-209 operator scenario remains in the
single-image layout described by its acceptance steps. `paintOverlays` sets
`Window not applicable` for an image that fails the viewer's drawability
classification. No universal claim about all possible runtime decode faults
was needed to reconcile this historical unsupported-colour example.

The amendment correctly supersedes that old example, preserves the newer
kind policy and explicitly does not claim an operator observation.

## Largest-frame LLD

All four rounded timing rows, ranges, RSS peaks and sample counts agree with
the after-hoist comparison summary and actual run. Both raw SHA-256 values
match. All four first-preview source headers are `decoded`, all nine following
source headers per case are `held`, and every RSS record has positive samples
and zero failures. The table states native HTTP observations, not browser FPS.

The ICC cache budgets, tone-curve byte counts and retained charges agree with
the actual record before and after windows. Retained cache and process RSS
are correctly distinguished. Stable charges are not claimed to prove absence
of temporary work. Host condition wording does not pretend this was an
externally idle desktop.

The before/after palette medians and the 73.228 percent observed reduction
match the records. The text preserves the unchanged arithmetic and ties rule,
keeps the Cornerstone comparison and monochrome regression observations
separate, and introduces no new threshold or acceptance. The final relative
cost sentence is read in its explicit Cornerstone-comparison context.

## Evidence boundary

No native build, test, profile, benchmark, browser, GPU or operator interaction
was run for this review. The JSON consistency assertions are record checks,
not new measurements. An initial read used the wrong raw-run directory name,
exited 1, and was corrected by file discovery to
`colour-icc-after-hoist-largest/run.json`. It supplies no false green claim.

This clean result approves only these two explanatory drafts against the
provided source and records. It does not complete AC19, AC31, F-239, F-240
or the sprint, and does not substitute for outstanding operator observations.
