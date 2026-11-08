# Benchmark measurement update, independent review pass 1

**Reviewed**: benchmarks-latest-measurements-proposed.md against the candidate
benchmarks.md and the three named raw largest-frame records. Root authored
the proposed update. Codex f244_review authored none of its prose, instrument
or measured records. **Result**: 0 defects, 0 smells, 0 nitpicks in the
bounded new delta. No new benchmark or whole-feature acceptance is claimed.

The diff changes only the largest-frame identity/table and explanatory
qualification, then appends the pre-ICC section independently reviewed in
S46-colour-measurement-notes-pass-1.md. Earlier monochrome observations and
open relative-band misses remain unchanged.

Independently recomputed all four latest table rows from actual C1/Node24
raw arrays: first, nine-sample median, extrema, RSS maximum and sample count.
Every printed value matches at its shown precision. Each case has decoded
first and nine held sources, positive actual RSS readings and zero sampling
failures. Actual budgets are256MiB except explicit ICC16 at512MiB. Both ICC
charges remain stable across the nine windows and match the printed
201330640/403440616 bytes. Their TRC table charges match3072/786432 bytes.

The actual latest binary hash is b32c96cf and the source is c1fe3037, with
Node24.13.0. The older raw files match both preserved hashes and Node26.6.0.
The proposed prose explicitly restricts the73.228 percent hoist comparison
to that older matching Node26 pair. It does not treat the newer Node24
figures as isolated byte-store speed or ICC cost. No new threshold, baseline
or idle-machine claim appears. The generated largest-frame subject remains
one-sided native HTTP, with the equipped comparison and monochrome
regression rows separately open.

Only JSON/text/hash and small arithmetic checks ran. The first checker
attempt had a Python syntax error before any assertion or file write,
exit1. Corrected checker exits0. No product or documentation failure was
observed. No build, test suite, browser, GPU, benchmark or actual source
modification ran. Root may fold this exact proposed delta with the previously
reviewed note, subject to normal staged guards. This review does not approve
a performance profile or waive any comparator failure.
