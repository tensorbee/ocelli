# F-239 benchmark refusal controls, working pass 1

Reviewer: root Codex, independent of the guard amendment author.
Source: `00f0d6290b9a54e75035abefe0a2a273d33708c0`, compared with
`589a928b276d3913bf3441c1d4f0ce54653e06f2`.
Result: 0 defects, 0 smells, 0 nitpicks in this bounded guard amendment.

The initial candidate census correctly refused 19 uncatalogued AC31 sites.
The amendment supplies two entries, sixteen runner sites and three fixture
sites, and three permanent control groups. Root read the complete four-path
diff, the existing covered controls and the nine semantic mutation results.
The added cases reject missing and wrong numeric headers, wrong held source,
missed sample counts, HTTP failure, non-object JSON, unknown workload, odd
Value Length and a stalled write. Valid controls establish that each refusal
is reached for its intended reason. Defensive fixture leaf checks execute
from a test-owned export copy, leaving the production API unchanged.

Root independently compared immutable blobs. Production runner and fixture
are byte-identical across the amendment. The budget adds only the two new
site counts. Existing guard counts, constants and uncovered ratchet are
unchanged. The generated runbook matches those declarations. A fresh root
execution of all 32 benchmark controls passed with direct exit 0. The
author's nine distinct guard mutants each fail, then restored source passes.

Evidence: `ac31-guard-root-controls.json`, `ac31-guard-root-identity.json`
and `integration/ac31-mutants/results.json` under
`/private/tmp/ocelli-s46-codex/`. The author also ran the direct census green.
This is guard registration and refusal evidence, not a production timing run
or approval of the upcoming ICC extension or the whole feature.
