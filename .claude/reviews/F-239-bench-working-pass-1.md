# F-239 colour benchmark instrument, working pass 1

Reviewer: root Codex, independent of the local GLM author and the Codex repair author.
Scope: corrected instrument tree `de9b520fde90dabc85fdc405440362a1fa02388f`, four paths from the reviewed CSP baseline.
Result: 0 defects, 0 smells, 0 nitpicks in the corrected instrument scope.

The original GLM output and its three measurement defects remain preserved under
`/private/tmp/ocelli-s46-codex/f239-colour-bench-glm/initial-output/`.
It accepted zero RSS, omitted actual RSS readings and left a partial fixture
when generation failed. Codex repaired all three. Root's initial brief also
specified the wrong body-form hold sequence and supplied the earlier rounding
contract. These are coordinator input corrections, not model defects.

Root read the complete corrected runner and fixture helper, the focused tests,
the correction diff and six semantic mutation controls. The helper is outside
the registry-only runners directory. The corrected path posts `/file` once,
retains its metadata, requires `decoded` on the first id preview and `held` on
nine subsequent id previews. Each consumes the entire reply and validates
channels, dimensions, window, length and deterministic sampled pixel bytes.
The fixtures stream bounded writes and derive values from the cited DICOM
LINEAR and palette rules. Each workload uses its own release viewer process.
The binary SHA-256 and reported release profile are recorded. Positive integer
RSS readings, failures and peak are retained. Cleanup includes partial creation.

A fresh root execution of the named Node suite passed 29 controls with direct
exit 0. The six preserved semantic mutants reject zero RSS, omitted readings,
partial cleanup, a non-held warm source, a missing first held id and upward
rounding at a tie. Root inspected the mutation driver and its exact restored
source identities. The selected real window steps do not contain half ties,
verified separately with exact fractions.

Evidence is the preserved model run, corrections, six red mutation receipts and
`/private/tmp/ocelli-s46-codex/colour-bench-root-suite.json` with its full log.
The model took 4023.673 seconds, no retries. This is wall time for this task,
not a controlled comparison or a throughput or cost claim.

Root subsequently wired the exact new suite into both required test lists and
corrected two measurement descriptions: RSS runs from the announced ready URL
through the last measured preview, and fixture payloads total 320 MiB plus
palette tables. Those root-authored edits require their own bounded review.
No largest-frame production measurement or performance acceptance is claimed.
