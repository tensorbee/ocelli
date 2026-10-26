# S16 product repair handoff, pass 1

Reviewed implementation tree `7de677af36b9122e60d2da4700c96b730f422e0a` on
`sprint/s16`, HEAD `22800f8c1b916818784c63b44dde1760c3512f04`.
Codex reviewed implementation attributed to the xAI model by the operator.
The three reports contain 8 defects in total. No product fix was applied by
the reviewer. This is not a clean sprint pass or completion authorization.

## Reports and acceptance disposition

- F-048: `.claude/reviews/F-048-coordinates-pass-1.md`, 1 defect in GPU/CSS
  integration. Shared CPU geometry itself passed the independent control.
- F-101: `.claude/reviews/F-101-boundary-pass-1.md`, 3 live wasm boundary
  defects. Native and fake-sink tests did not establish the target contract.
- F-X003: `.claude/reviews/F-X003-raster-pass-1.md`, 4 defects in invalidation,
  colour scratch ownership and full-session allocation behavior.

CPU canvas integration, the bounded comparator producer and permanent
benchmark instrument bodies are missing implementation. Representative-host
measurements remain unavailable. Keep these dispositions separate. They do
not all reduce to AC5/AC6 waiting for equipment. Do not mark any story done.

## Prompt for the implementing agent

Continue S16 from the pinned implementation above. Read the three reports,
approved plans and current feature scratch notes before editing. Preserve all
staged work and earlier evidence. Do not commit, push, tag, close or open a
different sprint. Keep F-X022/F-X023's existing pending disposition.

Repair the confirmed defects through the existing F-ID lifecycle. Work in
this dependency order, adapting it only for concrete ownership constraints:

1. Fix the allocation contract and target bridge under F-101. A refused alloc
   must never reach a memory write or apply/commit. Cover commands, writeFrame,
   transferred frames and DICOMweb bulk consumers. Preserve the outstanding
   valid owner after a failed second allocation.
2. Resolve allocation lifetime identity under wasm address reuse. Separate the
   write address from authorization if needed. Record the concrete public
   compatibility decision in the design and surface any material unresolved
   choice. A monotonic native counter or retaining every buffer is not a
   repair of the target contract.
3. Supply the real adapter between public metadata objects and numeric wasm
   exports, including refusal results and cleanup. Exercise the public helper
   against the built module, not just a fake matching sink.
4. Give device-to-CSS conversion one owner in the actual GPU camera path.
   Preserve View's CSS and patient/index contracts. The helper test currently
   performs a division that the consumer never performs.
5. Connect successful commands and frame changes to raster invalidation and
   one current generation. Test from settled state, then command, draw and
   Presented observation. Preserve rollback for refused batches and true idle
   for unchanged state. Test delayed older-generation completion.
6. Invalidate settlement after extent/DPR changes, including the same rounded
   device extent with different sampling scale. Failed resize preserves state.
7. Separate source-colour capacity from target pixels. Support valid colour
   downsampling without draw-time allocation or narrowing colour acceptance.
8. Remove deep Presentation clones from the render entry point. Cover actual
   allocation behavior for non-empty modality and VOI sequences and idle
   draws. The raster's self-reported buffer counter is insufficient.

Port the small independent semantic probes into permanent meaningful
regressions. Observe RED on the reviewed source and GREEN after repair with
the same inputs. Keep each result and source identity. The temporary probes
are optional implementation aids, not a dependency of this handoff.

For CPU probes, the independent inputs are:

- Settle 1 by 1 at DPR 1, resize to DPR 2, request the current full 2 by 2
  draw with sentinel output. It must write, then become idle on repetition.
- Fit 2 by 1 RGB `[10,20,30,40,50,60]` into 1 by 1. A valid transform and
  destination must not fail solely because source pixel count exceeds target.
- Bind a Session CPU target, settle generation 0, submit SetInvert via a valid
  bulk slot, then request the next settled draw. Successful state change must
  not return the previous idle result. Include a refused batch control.

For target-boundary probes, build and execute actual wasm. A Node dev build
is sufficient to expose these ownership/signature bugs, while browser worker
and OffscreenCanvas behavior still need browser observations. Use an alloc
occupied-slot refusal, a same-size release/reallocation/delayed old commit,
and the public writeFrame metadata object. Include one current numeric commit
as a control. Tests must reject stale reuse even when allocation addresses
are equal. Do not report a missing API compilation error as semantic RED.

## Finish implementation within approved scope

After the defects, finish the approved CPU browser path and refinement
integration without exporting pixels to the shell. Drawing into an owned
buffer and enqueueing kind 6 does not establish visible canvas presentation.
The event endpoint must reflect the actual presentation boundary.

Implement the bounded stack candidate producer and call the existing
comparator with matched input, geometry and parameters. Keep unavailable
adapter results separate from missing producer code. The first sandboxed native
probe could not open an adapter. Outside the sandbox it opened tier A and
reproduced the synthetic DPR mismatch. No tier A/C or external parity verdict
was produced. Preserve these different observations and do not substitute the
camera probe for the required raster comparison.

Replace the six unconditional throw-only runner bodies with the actual
instrumentation required by their existing subjects. They may refuse when a
real prerequisite is absent. Do not invent an elapsed time, vCPU fraction,
incumbent baseline or colour threshold. Host absence still blocks its own
measurement acceptance, even after the instrument is implemented.

Maintain an acceptance table with evidenced, failed, implementation missing
and external observation unavailable dispositions, plus owner and next step.
Keep acceptance open where the representative host is absent. Do not repeatedly
ask for equipment the operator already said is unavailable.

## Checks and next review

Use focused crate checks and tests while iterating. Run the actual built-target
boundary cases and browser worker/memory-growth/RAF/canvas observations.
Serialize GPU/browser work. Run the appropriate wasm and native proofs and a
real release-size check. Do not move the size budget or any tolerance to pass.

Request fresh independent Codex passes on the repaired feature and integration
material. Record exact source identities, selected tests and limitations.
After all intended source, metadata and instruction changes are reviewed and
staged, invalidate any receipt before the required final sprint profile.
Record its actual result in both verify ledger and sprint run state. A failed
or unavailable gate remains visible. Do not reuse the older union's receipt.
Keep whole-sprint review separate from feature reviews and implementation.

## Instruction refinements from this review

Codex changed canonical design, implement and microscope commands, the
dicom-tooling skill and WORKFLOW. The additions require real export/caller
compatibility and lifetime tests, settled-state invalidation through consumers,
source/target capacity separation, whole-entry-point allocation evidence and
distinct missing-code versus missing-resource acceptance dispositions.
Generated adapters must be regenerated and checked. These changes are
unstaged, outside the reviewed implementation tree, and require another
reviewer. Record this audit trigger in the next actual AS_BUILT entry.
