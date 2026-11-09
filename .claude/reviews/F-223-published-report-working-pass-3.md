# F-223 corrected full comparison, independent working review pass 3

Reviewer: Codex. I authored no F223 runtime or report-generator code. My
colour instruments and AC27 work are outside this approval. Root executed the
actual full comparison. This review uses only frozen metadata, source reads
and pure independent Python arithmetic, without production renderer imports,
hardware jobs, model inference or candidate edits.

Result: **0 defects, 0 smells, 0 nits** in the default full-comparison evidence
and generated-report scope. Pass 1's AC7 ordering defect and memory-report
clarity smell are resolved by actual source and fresh observations. The old
protocol-qualified run remains separate and unchanged.

## Actual order and counts

The direct command runs the ordinary comparison with --publish and exits
zero in 234.050561917 seconds, ending 2026-11-08T22:37:36.059697Z. It does not
use supplemental colour mode or injected services.

The independent checker derives expected starting sides from the approved
ordered metric/workload visits and the lead ruling at plan lines 618 onward.
It reserves unavailable visits, carries the sequence across metrics, and
requires both conditions of a workload to share one start side. Every actual
summary start and alternating trial sequence matches. In particular:

| First-image workload | Cold start | Warm start |
| --- | --- | --- |
| synthetic-ct | cornerstone | cornerstone |
| synthetic-dx | ocelli | ocelli |
| pseudo-ct | cornerstone | cornerstone |
| pseudo-dx | ocelli | ocelli |

Page-ready starts Ocelli. Open starts cornerstone for synthetic CT then Ocelli
for pseudo full. Scroll follows cornerstone then Ocelli. Window workloads
follow cornerstone, Ocelli, cornerstone, Ocelli. Memory starts cornerstone.
CPU reuses the three actual memory sessions per side, without another trial
loop. These observations satisfy the repaired starting-side protocol rather
than merely verifying its pure order helper.

All seven declared metrics and every workload/condition/side cell are present.
Required per-side N is nine for ready/open/first/window, five for scroll and
three for memory and CPU. Unavailable cells have zero observations and a
reason. Each measured summary has exactly one matching raw trial block,
and every side's sample vector equals its ordered trial values. All 20
scroll observations meet the declared key-count and zero-blank predicates,
and their FPS values equal presented frames divided by actual elapsed time.

## Independent arithmetic and memory report

The checker independently reconstructs 44 statistic cells and all 31 Markdown
metric rows. Median, linearly interpolated quartiles, minimum, maximum,
separation decisions, three-significant-figure ratios, duration/CPU rounding
and MiB conversion agree exactly. No new tolerance is used.

All six memory sessions have six recorded collection rounds. Their observed
successive timestamp gaps range from 402.908 to 424.476 ms, above the existing
250 ms minimum. The generator now labels the primary row as the sampled
concurrent footprint lower bound, labels the sum of per-process kernel peaks
as an upper bound, and distinguishes the minimum pause from actual timestamps.
This resolves the earlier clarity smell without changing sampling or schema.

For each session I recomputed maximum concurrent summed footprint and the
sum of each PID's largest kernel peak from saved maps. Lower is no greater
than upper. Product and infrastructure inventories are disjoint, CPU start
and end PID sets match, all counter differences are nonnegative, and their
sum equals the recorded session CPU. The three-session statistical summaries
match. This verifies saved arithmetic and inventory consistency, not the
kernel collector independently.

## Identity, cleanup and publication

Raw comparison.json SHA-256:
69ea6762d001040fb30169de7d75fb7ee6c6c95bf6f8d24a83c74965146891fa.

The raw source binding names HEAD e729d82b7ce3f2d1494d1f058b68658c526bd345,
index tree e53caf0658e8576e38bea26f389cd638539bb761 and worktree matching
index. Native binary SHA-256 is
0b8b34dc38d8da0ed427927fc19c7b2f3d85316543b012d2bc2f15a3d2054c7a.
Comparator SHA-256 is
ce91b6547a5408da6beb7cacf094c73bd101050342be5190b49dedc9a5f866cc.
Comparator-page bundle SHA-256 is
d9cfa522c0a6c934e79c4d34ad2bd40bf5e234a20a951a26b95d0ad8e694c1ca.
Input identity is
d87461fd59a9dc875b6b5ffe96f20ff7640059f8cd69c04c89adc97a7ff5aa58.

The actual recorded browser is Chromium 151.0.7922.34 with Playwright 1.62.1
and both pinned cornerstone packages 5.8.2. The raw closed version table does
not include Node. The saved launcher qualification records the actual command's
explicit PATH prefix selecting the v24.13.0 installation. The receipt helper's
subprocess inherits that environment and the bin wrapper leaves PATH unchanged.
The same-prefix post-run resolution reports v24.13.0, with binary SHA-256
6f2e835e57feb7a29ffcd53a4d4eb60224ec264a1e002c3ba6d99999ea29807d
and a January 12 mtime preceding this run. Node 24.13.0 is therefore attributed
to explicit launcher selection plus qualified post-run resolution. It is not
an in-run self-report or a retrospectively invented preflight.
The workload counts are 1693/1 synthetic CT/DX and 201/200/1 pseudo full/CT/DX.
The pixel precondition passes and matches publication. It supplies a timing
precondition, not a claim of every-frame GPU parity.

The source guard captures the start identity and recomputes source identity
after observations, checking HEAD and tree and requiring worktree/index
agreement. A direct zero exit and measured publication follow that guard.
There is no separate serialized end-identity object, so I do not invent one.
No live binary rehash or full input-content census was run during root's
next timing slot. Identity consistency rests on the retained runtime hashes,
guards and input digest.

The raw pair is comparison.json and comparison.txt. The published tracked
pair has JSON SHA-256
b938ea5aa0343cf161436db2a5192bdfe19648e936b7ef512cdbbe3141c1ea0e
and Markdown SHA-256
d3b5a6f252bf96db32b57ec0b5603d103724cbbc477f5a8517eb8573302e0b1b.
Both frozen files still equal the candidate files byte for byte. Every
published subtree equals its raw counterpart. The closed publication subset
intentionally omits raw source/trial/condition fields while retaining input
identity. Snapshot hashes retain the raw provenance.

Runtime source closes owned actors, static server and temporary input area,
removes the lock, and rejects interrupted or failed cleanup before tracked
publication. Publication also refuses injected services, reused builds and
colour mode. The direct zero receipt and regenerated tracked pair therefore
pass these lifecycle guards. The run's lock is independently absent. I did
not perform an extra whole-machine process census during the next measurement.

## Review execution and limits

The final pure checker exits zero with no arithmetic or protocol discrepancy.
An initial snapshot attempt expected comparison.md and exited one after
copying only the raw JSON. The actual raw text name is comparison.txt. The
corrected capture preserved the first copied JSON after checking identity and
completed the snapshot. That controller filename assumption is not a product
or measurement defect. No incomplete output was audited.

No candidate source, report, baseline, tolerance or gate was edited. This review covers
the default F223 matrix and report. Supplemental colour/ICC observations,
F216 absolute budgets and historical movement bands remain separate. No
feature, sprint-close or operator-acceptance completion is claimed.

## Record identity

This is the preserved private measured-run review, mechanically renumbered from private pass 2 to canonical pass 3 because canonical pass 2 records the distinct source repair. Its original SHA-256 is `a9864e3d5b23e4f8b1c6e61f2785668ae026ccdb0900138f82201a721828ee62`. Review scope, author, measured observations and conclusion are unchanged. This bookkeeping does not supply a new review of later source.
