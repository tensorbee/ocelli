# F-269 design microscope, pass 2 and approval

Reviewer: Codex, independent of the GLM author. Design only.

GLM corrected draft SHA256: `407d476b301af5321660a420fe4e3fbd2e368b1fabde4de27eb093cf22d6b25e`. The author repaired and reviewed the actual file before this independent pass. Its eighteen actual calls, twelve-call target and inaccurate self-reported count remain separately qualified. Source and table punctuation checks passed.

Result: 0 defects, 0 smells, 0 blocking nits. Design approved. Root changes only the approval header. Approved plan SHA256: `2d7e5d73f506db8d761b979af673056fa5ac96e612673b8c9182594acb8d9d24`.

D1 is resolved by separate high- and low-MAXVAL formulas, the complete T1, T2 and T3 expressions, explicit floors and the exact lower-bound CLAMP rule. They agree with Figures C.3 to C.5 of the preserved official T.87 source. The independent MAXVAL 127, NEAR 0 inspection distinguishes 2, 3, 10 from the incorrect high-branch generalization 2, 3, 4. This is not an executed codec test.

D2 is resolved by the conditional D-20 description path in the write set and AC4b. Its count changes only with the actual patched-file inventory and in the same comment and pin commit. The published inventory remains immutable and no deviation policy changes.

D3 is resolved by the actual direct vendored encoder call in the Ocelli regression test and the vendor's own encoder tests. Independent tracked-source search and the named test confirm that consumer. No public Ocelli JPEG-LS encoding API is invented. The decoder's historical damaged-stream sweep remains distinct from encoder source inspection and no fresh universal proof is claimed.

F-243 is done. Isolated S47 work after this approval is explicitly allowed while canonical S46 stays open. The worker owns vendor JPEG-LS comment text, the existing provenance and patched-hash records and the declared conditional documentation. No decoder or encoder logic, corpus, tolerance, gate, profile or original package inventory may change. Findings about behavior are separate work, never smuggled into this audit.

GLM must audit every factual comment, record the checked inventory and clause, preserve false versus doubtful ratings and caller-specific cap premises, then run its own microscope and actual checks before independent implementation review. A runtime decode pass cannot demonstrate comment correctness. Baseline and final decode observations remain pending, with native resources coordinated separately. This approval supplies no execution, implementation completion, S46 acceptance, integration, push or closure.
