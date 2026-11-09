# F-224 first-image remediation, review pass 1

Author: root Codex. Independent reviewers: f225_review and f244_review,
Codex agents. Local GLM source and acceptance reviews are separate bounded
peer outputs and do not supply approval of an unseen source or runtime.

The initial scheduling proposal had one defect and one smell. Generation,
viewport and series key alone alias an A to B to A selection. One pending
cancel owner now gives each request explicit ownership, and every immediate
probe consumes it before looking up the active series. A requested timer
delay is not a guaranteed browser deadline, and the plan states that limit.
The implemented helper was independently reviewed clean.

The first observer draft had two defects and one smell in its controls: a
late route.fetch reply could be stranded after release, folder and viewport
retirement were missing, and switching reused an old publication epoch.
Those controls were repaired. Later callback-starvation coverage uses interval
polling while animation callbacks are deliberately withheld. All eleven
actual-browser scheduling cases pass. These findings belong to the observer
review, not to a claim of eleven product defects.

The original four-worker policy is explicitly amended to one. Original AC17
and the validator required four, so this is a reviewed design and assertion
policy change. A peak from one through the independent ceiling of four is
now accepted. Zero, fractional, non-finite and five-read peaks are refused.
Every-file, one-byte and one-second checks are unchanged. A new actual
large-sweep coalescing control catches dropping the final active-series
probe. The one-worker source review is zero defects, zero smells and zero
nits at tree `1294d421c041f6bd36fc600dc83cf4cd4baac507`.

Evidence is separated by stage. The original page fails the priority and
retirement controls. Missing cancellation, fallback and queued-probe mutants
fail semantically. The original exact-four policy fails the new serial-policy
positive control, which is an explicit policy change rather than an old
product defect. The upper-bound-removal mutant fails the five-read negative
control. Original source is restored exactly after each mutation. All 334
native viewer tests passed before the one-line concurrency experiment, and
the complete changed-file browser suite passed again on the serial worker.
A final composed feature profile is still required.

The complete five-subject run on the serial worker is within every existing
recorded band and absolute budget. Full numbers, original movement records,
source identities, host qualifications and diagnostic limits are in
`docs/lld/benchmarks.md`. README and Use now name one background read at a
time. No performance baseline, time limit, parser, pixel arithmetic or frame
identity was changed. Independent acceptance arithmetic and final composed
verification remain separate from this source review. Operator observations
are not supplied by these tests.
