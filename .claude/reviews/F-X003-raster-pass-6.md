# F-X003 D9 repair review and instruction handoff, pass 6

**Reviewed**: staged tree `289d1e0f7a84ccbe15845274187fc58b27b8a262`
**Previous**: `d8ae7aa9c810e3aa1e619e67a96ca710a9df4502`
**Author / reviewer**: implementing xAI model / Codex
**Result**: 0 defects, 0 smells, 0 nitpicks within the D9 repair scope
**Date**: 2026-10-19

## Disposition and clean coverage

D9 is repaired in the exercised public worker path. Source-length validation
runs before a compatible settled result can return idle. Requested shade is
passed explicitly to drawing and remembered only after success. The pixel
transform owns its source-layout acceptance rule. The session validates the
remembered layout before beginning interaction, rather than advancing the
generation on that layout refusal.

The permanent worker covers incompatible mono-to-RGB and RGB-to-mono requests,
both settled and invalidated. Each rejects with a length error, preserves the
visible frame and event count, and then successfully interacts and settles
without resetting mode from the test. Existing colour, monochrome, failed-blit,
resize, idle and retained ImageData controls remain green.

Replayed the original independent D9 probe on freshly rebuilt wasm. Both
previously failing controls now reject the incompatible RGB request and
successfully draw the next monochrome interaction. After SetInvert, that
interaction is opaque black as expected. The independent 7 by 5 RGB control
still matches reduced sampling, cell replication and full settlement. Exit 0,
zero failed preservation checks. Historical RED remains in pass 5 and its
original log. This pass did not rebuild or rerun the old source.

## Checks executed now

| Command | Result |
|---------|--------|
| `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm` | exit 0 |
| `node packages/core/cpu-canvas.mjs` | exit 0, real headless Chromium with GPU disabled |
| `node /private/tmp/ocelli-pass5-colour.mjs` | exit 0, same browser environment |
| `npx vitest run --config vitest.boundary.config.ts` | exit 0, 5 passed |
| `bin/ocelli.sh test ocelli-wasm --lib` | exit 0, 28 passed |
| `bin/ocelli.sh test ocelli-render --test cpu` | exit 0, 14 passed |
| `bin/ocelli.sh test ocelli-pixel --test color --test palette` | exit 0, 14 and 7 passed |
| `python3 scripts/sync_agent_skills.py --check` | exit 0, 21 adapters match |
| `git diff --check` | exit 0 |

Local logs are `/private/tmp/ocelli-pass6-build.log`, `-worker.log`,
`-colour.log`, `-boundary.log`, `-native.log`, `-raster.log` and `-pixel.log`,
each with the same `ocelli-pass6` prefix. The report and permanent worker
preserve the observations and reproduction route. Temporary logs are supporting
local evidence, not a verification receipt.

No new mutation, release-size, corpus, GPU-tier or representative-host result
is claimed. The bounded pass does not establish completion of all F-X003 ACs.

## Continuous instruction refinement

| Evidence or gap | Instruction response | Current status |
|-----------------|----------------------|----------------|
| D9 accepted an incompatible request via idle and retained a rejected mode | Existing pass-5 design and microscope additions require validation before idle and valid recovery after refusal | Independent instruction pass 3 covers those additions |
| Repeated handbacks identify missing producers and instruments but leave their next step as keeping them open | Design now traces input, production entry, output/event, comparator/collector and result. Implementation handoffs name the smallest next deliverable, owner and check | Preventive execution gap, new text needs independent review |
| Reference-only or synthetic comparison could be confused with applicable-corpus candidate execution | Dicom-tooling now requires actual candidate production, selection/coverage dispositions and independent positive/negative controls | Preventive coverage gap, new text needs independent review |
| Continuous refinement could accumulate duplicate instructions without changing behavior | WORKFLOW connects each finding, regression and canonical instruction in existing reviews, and distinguishes a missed execution step from a missing rule | New text needs independent review |

Changed canonical files in this turn are `.claude/WORKFLOW.md`,
`.claude/commands/design.md`, `.claude/commands/implement-feature.md` and
`.claude/skills/dicom-tooling/SKILL.md`. Regenerated adapters are derived.
All guidance remains unstaged. Earlier instruction passes cover their stated
versions, not these new additions. No new gate or parallel tracker was added.
Record these triggers in the next actual AS_BUILT entry, not a completion
entry fabricated for this review.

## Next implementing-agent assignment

First independently review the new instruction additions above and record the
exact diff reviewed. Do not treat Codex's authorship as independent approval.

Next address F-X003 AC5's missing applicable-corpus candidate producer within
the existing approved lifecycle. Read the current design, progress handoff,
`tools/oracle/src/candidate.rs` and actual harness call chain before editing.
Replace the one-synthetic-frame-only evidence route with production of an
Ocelli settled frame for the applicable selected rows, matched to reference
input frame, presentation parameters, geometry and extent. Preserve the
public pixel boundary and permitted source policy. Resolve any material
unsettled design choice before implementing that choice.

In the existing acceptance record, name the owning modules, producer-to-harness
connection, runnable selection command, supported selection and explicit
unsupported cases. Build a synthetic positive and wrong-candidate control.
Record produced and compared counts so a reference-only run cannot appear
to cover Ocelli. Execute the applicable harness when its resources are
available, with no patient data in logs or reports. Do not invent a new
command before the integration exists. Missing hardware must remain separate
from missing producer code.

Return that staged product tree, exact commands, individual results and updated
acceptance dispositions for independent review. Retain affected boundary and
worker checks when their source changes. Do not treat this scoped clean pass
as AC5 or whole-sprint approval.

After that bounded assignment, the interaction instruments and CPU-time and
wall-time collector remain explicit implementation work. The representative
GPU-less host and incumbent baseline remain externally unavailable. Reuse the
operator's existing answer about the missing host. No invented sample or
threshold is permitted.

F-048, F-101 and F-X003 remain in progress. F-X022 and F-X023 remain pending.
The final reviewed union still needs canonical integrated verification and
independent whole-sprint review. Nothing in this handoff authorizes a commit,
push, merge, tag, sprint close or new sprint.
