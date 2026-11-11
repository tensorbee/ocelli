# F-272 design microscope, pass 1

Reviewer: Codex, independent of the GLM author. Design only, no implementation or executed test claim.

Source: base `1d627bd93f0c0e3b44231be9a0f3e76607a8eeb9`, draft SHA256 `fb253f274e93a840887025609a8a8f9657cc179ce7e680d494b668d0917ed107`. Author handoff is `/private/tmp/ocelli-s46-codex/f272-granular-design-recovery/final-design-handoff.json`. The author's zero-remaining counts are not accepted as independent approval.

Result: 3 defects, 1 smell. Approval refused. Status remains draft.

- D1. The map, AC3, AC4 and approach promise a writer hook and transient detection. D3 instead proposes polling. Neither identifies an actual suite entry or test selection. Polling cannot guarantee detection of a write and restore between observations.
- D2. The existing producer test at produce.rs:2286 still owns a PID-only directory and deletes it. Avoiding that directory in a new helper leaves its original collision intact. Audit and isolate all actual producer-writing tests, including this test and the valid-success call at 2329.
- D3. The three absent callers force a missing reference. Their unchanged assertions cannot establish corpus-absent or output-refused behavior. Add independently reachable fixtures for those branches, preserving each actual reason and count.
- S1. Repeated conditional feasibility text presents a hypothetical guard as a settled mechanism. Name its callable entry, selection, ownership and observation limit once.

## Lead design clarification

Use a permanent Rust guard test invoking the current test executable for the producer test selection, explicitly skipping the guard itself. Check the child's own exit. Compare absence and content digests of the actual protected tools/oracle tree before and after, with any dependency-cache exclusions named. Do not mutate the parent's process-global environment. This proves final state, not every transient write. Remove the polling and writer-hook claims rather than inventing their coverage.

Keep each producer-writing invocation's outputs in a uniquely created owned temporary directory, including the existing missing-reference test and the success path. Preserve real reference and corpus reads. Cleanup on normal return and unwind must not remove another invocation's directory.

Record a semantic RED by restoring the old checkout output path in a disposable isolated source checkout with a synthetic sentinel, then restore and observe GREEN. Never mutate the operator's comparison files for a control. Independently reach corpus-absent with a valid reference fixture and output-refused with its required inputs. No new gate or profile is authorized.

GLM must revise the actual plan, run a fresh own microscope over the revised text and return it for independent review before implementation. The existing workflow's concrete-acceptance and evidence-feasibility requirements already cover this finding, so no workflow edit is needed.
