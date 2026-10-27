# S17 review, pass 1

**Reviewed**: `sprint/s17` at `40aa759810909c45a58da767d596cd40c5c3292a`, tree `d343f9ed4331`. The pass read the four story commits against the open record and the living docs those stories leave behind.

**Author / reviewer**: The same session implemented F-102 and wrote this pass. The operator directed that session to review without waiting on DeepSeek. This is not an independent reviewer.

**Result**: 3 defects, 0 smells, 0 nitpicks.

## Acceptance challenged

The sprint's living status. `docs/sprints/BACKLOG.md` marks F-050, F-102, F-103 and F-104 done, and the status table in `CURRENT_SPRINT.md` agrees. The dated open section still spoke as if implementation had not started.

The shell's self-description. `@ocelli/core` now publishes the boundary, the snapshot, the event catalogue and the generated constants. Its readme still called the package a scaffold whose boundary was future work. `docs/lld/typescript-packaging.md` still said to revisit the bundler decision when F-101 landed.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh gate --sprint` | staged tree `d343f9ed4331` | executed now | exit 0, ALL GREEN, 31 gates, corpus 95 verified |
| reading `CURRENT_SPRINT.md`, `packages/core/README.md` and `docs/lld/typescript-packaging.md` | present tense after the four commits | executed now | three sentences describe work that has landed as work that has not |

## Defects

### D1, the open section still says implementation has not started

**Where**: `docs/sprints/CURRENT_SPRINT.md`, the audit intake section
**What**: the text says `/prepare-sprint` is next and that no design plan exists.
**Why it is wrong**: the four stories are done and their plans are in `.claude/plans/`.
**Evidence**: the status table in the same file marks each story done.
**Scope and regression**: a reader of the sprint record would restart preparation.

### D2, the core readme still calls the package a scaffold

**Where**: `packages/core/README.md`
**What**: it says the public API is designed in F-100 and the boundary is built in F-101, as work that has not happened.
**Why it is wrong**: those stories have landed, and S17 added the generated constants, the snapshot and the event names.
**Evidence**: the package exports those modules and the sprint gate's packages check passed.
**Scope and regression**: a consumer reads the published readme.

### D3, the bundler section still waits for F-101

**Where**: `docs/lld/typescript-packaging.md`
**What**: it says to revisit the decision when the wasm path lands in F-101.
**Why it is wrong**: F-101 has landed. The boundary copies bytes out and does not import the wasm asset.
**Evidence**: the same file, a few sections earlier, already describes that copy.
**Scope and regression**: the next bundler decision has no current trigger.

## Smells

None.

## Nitpicks

None.

## Verified clean

The sprint gate on this tree was 31 gates green with the corpus at 95 verified. F-102 submits through `writeCommands`, drains F-103's ring and round-trips F-050's record. F-104's generated constants are what those paths read. No story is carried. The example viewer does not load wasm, and `npm run dev` was not run.

## Limitations and handoff

The remediation of D1, D2 and D3 is the next edit. This pass is not an independent review.
