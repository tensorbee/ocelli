# Independent automatic-window check

Run only after the sprint lead authorizes the operator directory. This tool never discovers or groups files. Supply one series per invocation in exact page order:

```json
{"mode":"auto","records":[{"path":"relative/synthetic.dcm","frame":0}],"observed":[{"center":"7","width":"1"}]}
```

Use private JSON containing the actual ordered relative paths and zero-based frames. Observations must be unrounded route/header centre/width strings. Supply first, middle, last and scrolling observations as appropriate. Rounded DOM labels cannot verify binary32 equality.

```sh
.venv/bin/python tools/bench/auto_window_reference.py --mapping PRIVATE_MAPPING --input-root AUTHORIZED_ROOT --result FRESH_PRIVATE_RESULT
```

The result contains only pass/fail, counts and closed stages. Existing result files are refused. Do not print or serialize in-process numerical calculation facts. The CLI discards diagnostics through shutdown. Callable main restores streams after its temporary serial boundary.

For windowed controls use mode `file-voi-control` with an empty observed list. It verifies only the selected sample's file VOI ineligibility. Actual browser labels, first/middle/last ownership and scrolling remain separate manual evidence. Unsupported reader coverage cannot pass. No actual study has been run during preparation.

Targeted verification uses only generated synthetic fixtures, no operator inputs:

```sh
.venv/bin/python -m unittest discover -s tools/bench/tests -p 'test_auto_window_*.py' -v
```

This is an explicit F227 check, not a claim of floor discovery.
