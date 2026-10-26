# S16 Codex completion implementation and verification handoff

Date: 2026-10-19. Implementer: Codex. This is implementation evidence, not an
independent microscope pass or whole-sprint approval.

## Disposition

The operator assigned the remaining implementation to Codex, approved minimal
stack initialization in F-101, and explicitly deferred independent review.
F-048, F-101 and F-X003 remain in progress. F-X022 and F-X023 remain pending.
S16 stays open. No commit, push, merge, tag or closure is authorized here.
HEAD is `22800f8c1b916818784c63b44dde1760c3512f04` on `sprint/s16`.
The prior scoped producer pass 10 covers its recorded earlier source only.
It does not approve this implementation or the accumulated instruction union.

## Implementation

1. Minimal ticketed stack initialization installs ordered immutable FrameKeys
   and returns a nonzero epoch. Selected-frame commits require that epoch and
   the current index. Scroll invalidates the old source and reports loading.
   A stale neighbour or previous stack cannot settle the current selection.
   Legacy standalone commits refuse while the stack is configured. The public
   metadata adapter validates before allocation. No command ID or state stride
   changed. `writeCommands` views memory only after allocation can grow it.
2. Real release worker benchmarks replace refusal-only interaction runners.
   First frame observes a typed command through a later RAF ring drain.
   Pointer and wheel events batch real commands. Scroll changes the resident
   key. Cine alternates predecoded frames and positively checks canvas pixels
   inside the owning worker before counting changes. Idle schedules no draw.
   The CPU collector sums dedicated Chromium-process CPU deltas over wall
   time, records snapshot uncertainty and refuses invalid counters or changing
   process inventories. Normal CPU acceptance still requires the unavailable
   representative GPU-less incumbent host. Diagnostics create no baseline.
3. Native tier A candidates use the existing RenderGraph and declared Grey
   LUT, with bounded readback and device-extent checks. Candidate execution
   provenance is separate from the source reference environment. The public
   shell receives no pixels. Tier B is not inferred from this run.
4. A paired synthetic SIGMOID reproducer renders the pinned reference and
   real CpuRaster. PS3.3 C.11.2.1.3.1 supplies the independent equation.
   Width 0.5 disagrees on 17 of 20 samples. Candidate bytes follow that width
   within one code. The recorded reference codes fit width 0.5 within one
   code. The installed transfer-function path statically yields width 1.5, so
   the rendered codes and that path disagree and the reference's applied width
   stays unresolved pending an oracle run. Correction dated 2026-10-26: the
   earlier sentence in this item said the pinned reference follows width 1.5,
   and the rendered codes do not support it. No arithmetic or parity policy
   was changed to absorb it. Remaining interior, letterbox and nonsquare-edge
   residuals are unattributed.
5. The staged unsafe guard now inspects the complete index and its bytes.
   Its previous changed-name subset omitted unchanged vendor packages and
   its working-file reads could hide staged unsafe. Allow-list and vendor
   counts are unchanged. Enumeration and blob-read failures refuse evidence.
6. A real native adapter renders the independently derived DPR 1.25 texel
   sequence. Browser GPU canvas and WebGL2 observations remain outstanding.

## Discriminating evidence executed during implementation

| Selection | Observed result |
|-----------|-----------------|
| Native wasm session library | 30 passed after stack mutations were restored |
| Explicit dev wasm boundary suite | 6 passed |
| Explicit shipped release wasm boundary suite | 6 passed |
| Explicit release SIMD-enabled boundary suite | 6 passed |
| Release browser worker instrument suite | 15 passed, zero skipped, including first, pointer, scroll, cine and idle controls |
| TypeScript default suite after export inventory update | 104 passed |
| Unsafe temporary-repository regressions against old source | 4 failed, including a staged unsafe blob hidden by clean working bytes |
| Same unsafe regressions after repair | 4 passed |
| Native tier A hand-derived Grey proof | 1 passed on an opened adapter |
| Native rendered DPR 1.25 proof | 1 passed on an opened adapter |
| Focused oracle check and clippy | Passed |
| Skill adapter regeneration and check | 21 adapters match |

Two controlled stack mutations were executed serially with no other writer.
Each mutation's source SHA-256 and selected assertion are in local logs.
Removing epoch equality admitted an old-stack frame, causing exit 101.
Keeping the prior selected source after Scroll failed the loading/refusal
control, causing exit 101. Source was restored in a finally block. The same
restored native session suite passed all 30 tests. These are semantic mutation
results. An absent old API is not presented as a semantic RED result.

An initial default TypeScript run failed because the exact export inventory
omitted the two newly approved helpers. That inventory was updated and the
suite passed. An initial SIMD release build failed wasm-opt validation because
its ordinary feature list lacked SIMD. The dedicated variant was rebuilt
with wasm-pack `--no-opt`, then explicitly optimized with the ordinary feature
flags plus `--enable-simd`. Its six boundary tests passed. The optimizer was
not disabled for the shipped module or its size gate. Enabling SIMD does not
prove a speedup or an optimized raster inner loop.

Local logs use `/private/tmp/ocelli-s16-*.log`. Raw corpus images and sidecars
remain under ignored oracle output. No patient information enters this report.

## Numerical result and remaining acceptance

Both native producers selected 85 rows, produced and compared 84, refused one
pixel layout and recorded 19 unsupported rows or reformats. Against the
pinned cornerstone reference each reports zero passes, 67 pixel failures and
17 unmeasured views. That standing external parity result remains red.

Comparing the actual Ocelli tier A and tier C candidates reports 67 pixel
passes, zero pixel failures, 17 unmeasured and zero absent within that produced
set. Across all 84 produced frames, maximum absolute difference is one code,
every pixel is within one LSB and maximum informative signed mean difference
is 0.0037249936353526015. The unchanged global coverage contract reports 14
coverage problems and an overall `coverage-loss` verdict. This is not a green
AC5 gate. No tolerance, sampling, expectation census or baseline was changed.

Current aggregate output is under ignored
`tools/oracle/compare-out/codex-tier-c`, `codex-tier-a` and `codex-tier-a-c`.
The original `pass-6-red` run remains intact. The paired synthetic diagnosis
is under `tools/oracle/compare-out/sigmoid-diagnostic`.

AC6 still needs the representative physical host, incumbent workload,
noise-floor comparison and accepted budget. The operator has no such host.
No local diagnostic is accepted as that measurement. SIMD-enabled execution
is observed, but SIMD optimization and representative performance acceptance
are not established. Browser GPU and tier B coverage are not supplied by the
native proofs. These gaps and deferred independent review block completion.

## Instruction refinements and next reviewer

Existing xAI instruction pass 1 through pass 6 retain their original scope.
New CPU-counter design guidance and staged-blob verification guidance require
independent review with the complete accumulated canonical diff. Workflow
GPU membership now includes the permanent native oracle proof. The unsafe
gate includes its four identity regressions. Adapters are regenerated from
canonical sources. No generated adapter is edited by hand.

The workflow trigger for the next actual AS_BUILT entry is staged-source
misidentification, refusal-only performance instruments and conflated reference
versus tier-consistency evidence. No completion entry is manufactured here.

Read the final index and current handoffs, then independently run:

```text
/microscope F-048 --working
/microscope F-101 --working
/microscope F-X003 --working
/microscope F-X023 --working
```

F-X022's scoped prior review does not approve later shared preparation.
Review all instruction changes as well as product repairs, then the whole
sprint on that same union. Run the final sprint verification after any repair
or metadata change. Do not use this author-written evidence as approval.

Final integrated gate results and exact index identity are recorded separately
in ignored `.claude/scratch/S16-codex-verification.json` and the verify ledger
when applicable. This report records the focused observations above and does
not predeclare a passing integrated receipt. The final handoff must name any
failed attempt as well as a subsequent pass.

## Integrated attempt and corrections

The first full sprint attempt on `dc563f15877cc51c328ba85aaeaf1c90ad7da215`
exited 1. It ran all 31 gates with zero skips. Six failed: test, bench, lint,
ci, guards and guards-deep. That failure is retained in the sprint run state
and local verification history. Two producer assertions still named missing
instruments after implementation. Benchmark helper modules occupied the
registry-only runner directory. The inherited canvas harness had five missing
global qualifiers. CI invoked only the old single unsafe command, and the
new census sites needed declaration and tests.

Corrections update the assertions to say not run by the producer, move the
shared browser instrument outside the runner directory, remove unused native
parser and placeholder endpoint scaffolds, qualify browser globals, invoke
the complete unsafe gate in CI, and register new refusal coverage with its
explicit browser-only limits. No catalogue declaration claims that each
defensive browser failure branch was separately injected.

Artifact reuse now checks transitive workspace and vendor sources, served
wasm glue and compiled core bytes, as well as the module digest. The browser
suite challenges tampered served bytes, absent kind 6 and a constant cine
canvas. Harness identity is recorded separately from product artifact identity.
A fifth unsafe regression explicitly refuses an unreadable staged blob.

Fresh read-only GitHub issue and PR refreshes both failed with HTTP 401.
Current remote intake is unperformed. Earlier empty-list reports remain
historical observations, not a successful refresh by this implementation.

The focused correction run passed bench, lint and CI, then identified a census
connection missing from the worker test. That test now opens the actual
worker source, verifies the served bytes match it and requires each controlled
mutation to apply. The subsequent floor guard probe run passed. Its deep
run exposed an obsolete probe builder that rewrote only the old one-line
unsafe arm. The builder now replaces the complete multiline arm. This keeps
the original comment-only-enforcement mutation meaningful without weakening
the lint policy. These intermediate failures remain in local logs.
