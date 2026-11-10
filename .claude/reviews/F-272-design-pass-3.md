# F-272 design microscope, pass 3

Reviewer: Codex, independent of the GLM author. Design only, no implementation or runtime acceptance.

Source: draft SHA256 `a785d6b9dca381ecb1f621521120d23a917e0f3b19729e296b96d0b43cbbc054`, from the bounded tool-enabled GLM repair. Root passes 1 and 2 remain unchanged. GLM authored and reviewed its actual file edits and ran its prose repair loop first. The independent controller prose check exited 0 across 1218 tracked files. The missed eight-tool target, 47 actual calls and partial guidance reads remain explicit limitations, not compliant-budget or independent-approval claims.

Result: 0 defects, 0 smells, 2 readability nits. Substantive design is accepted. Apply the two bounded clarifications below and check their exact delta before the approval header and isolated claim.

N1. AC2 currently has six cells under a five-column table. Merge its standalone zero-selected-control cell into the observation or rejected-implementation cell, retaining the stage and owner in column five. No acceptance claim changes.

N2. Approach step 2 describes the child before saying to hash before and after. Make the actual chronology explicit: capture the protected tree before the producer child, run and check the child, then capture and compare the tree afterward. The existing invariant already requires that ordering.

The revised source settles exclusive temporary-directory ownership and collision retry, converts the existing PID-only caller, covers the valid producer success path, preserves read-only consumers and independently reaches the other refusal branches. The actual test executable's list mode must establish nonempty selection containing the required producer tests before the child runs. A stale or zero-test selection refuses. Final-state hashing is deliberately distinct from a transient-write claim. Disposable-source RED and restored GREEN challenge the actual guard without modifying operator outputs.

No new gate, profile, dependency, pixel arithmetic, parent-global environment mutation or external observation is required by this test-isolation design. The declared F-X021 dependency is done. The session explicitly authorizes isolated S47 work after design approval while canonical S46 remains open. Normal claim and start ownership still apply.

After the exact clarity delta is checked, set the reviewed plan to approved and let GLM implement in its isolated branch, run its own microscope and checks, then return the complete source union for independent review. Separate baseline and mutant build outputs and preserve their executable identities. A source edit is not acceptance, and the final source and completion metadata still need normal verification.
