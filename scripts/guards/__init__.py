"""The guard harness. F-X009.

Four modules, and the order to read them in:

- `discover.py` finds every refusal site under `scripts/`, `ci/`, `.githooks/`,
  `bin/` and `tools/`, mechanically, so the census counts what is there rather
  than what somebody remembered to list.
- `catalogue.py` declares what each refusal is FOR, citing the specification
  rather than the guard's source, and carries the recipe for the state that
  should trigger it.
- `sandbox.py` builds the disposable repository a probe runs in, and is the
  file to read carefully. Nothing here may write inside the real repository.
- `census.py` proves the catalogue is complete in both directions, and that no
  guard's own configuration has been quietly widened.

The two entry points are `scripts/guard_probe.py` and
`scripts/guard_census.py`, and the `guards` and `guards-deep` gates run them.
"""
