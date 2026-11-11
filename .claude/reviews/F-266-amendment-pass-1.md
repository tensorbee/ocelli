# F-266 guard-consumer amendment review, pass 1

Reviewer: Codex, independent of GLM. Reviewed draft amendment patch SHA256 `ee9e1e8893a5fd70088ef40c5d0771eeacc65d3e16711d0a9346f658520abccb` appended to the approved design prefix. No implementation authority follows from this pass.

Result: 3 defects, 0 smells, 0 nitpicks. Revise through GLM before adoption.

D1. The opening says the source write set remains the approved five files while the amendment explicitly adds three paths and leaves an unspecified possible unit-test path. State the exact extension consistently. Include necessary catalogue Probe/spec registration, not helpers and fixtures only, so the dedicated valid-chunk absolute-fetch refusal is actually invoked. Name any additional test path now, or leave it excluded until separately declared. The frozen unapproved catalogue proposal remains unadopted.

D2. AC1 names every emitted file, but the new API inspects only JavaScript assets and index.html, and only one exact package URL plus selected spellings. CSS, maps, other emitted outputs and other absolute reaches under the checkout root remain uncovered by that contract. Specify all-output inspection through the actual caller, with an explicit treatment of binary data, and distinguish the known checkout root and /@fs reach from the one exact package file. At minimum preserve the two independently reproduced non-worker controls, add another emitted-file class and another build-root target, and retain the healthy bundle and the outside-worker countermutant. Do not silently narrow AC1. This review does not require claiming exhaustive detection of arbitrary obfuscated JavaScript.

D3. The normative spelling sentence is ambiguous and conflicts with the approved forms. Write the exact intended mapping: a literal percent is percent-encoded as %25 in the browser URL, a real space as %20, and a literal filename segment %20 becomes %2520. The decoded Rollup path is a separate build-present form. Keep the truthful bounded mixed-spelling description and remove the claim that the literal percent survives unchanged in pathname.

The ratchet approach is otherwise appropriate: discover the actual post-repair refusal sites, preserve unrelated constants, entries, uncovered state and clock values, and regenerate the existing runbook through its renderer. Observed 11 and 10 are expectations to check, not numbers to force. No gate or profile change is approved. All code and acceptance remain pending a corrected amendment and independent approval.
