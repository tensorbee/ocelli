---
description: The completion gate. Runs the gate set for a profile and records the result against the tree hash so the pre-commit hook can write the provenance trailer.
---

# /verify [--fast] [--full] [--profile feature|sprint|release]

The gate. Nothing is completed, pushed or released without it.

## Profiles

| Profile | Gates | Used by |
|---------|-------|---------|
| `feature` (default) | `--floor` plus `corpus` | `/complete-feature` |
| `sprint` | `--sprint` | `/run-sprint` consolidated verification |
| `release` | `--all`, corpus required to PASS | `/release` |

`--fast` runs `fmt clippy` and the changed crate's tests only. It is for the
inner loop and **is not acceptable for completion**.
`--full` is `--profile sprint`.

## What runs

```bash
bin/ocelli.sh gate --list     # the authoritative list, with what each covers
```

Run that command for the count and the coverage. `bin/ocelli.sh` is the
definition. This file does not duplicate the list, and it does not state how
many there are either, because two lists drift and then nobody knows which is
the gate, and a number is just the shortest possible second list.

## The GPU tier, and why it is not optional here

CI runs no GPU and no corpus (`docs/hld/DEVIATIONS.md` D-04). That makes
`/verify` the only place the oracle runs before a release.

HLD decision D7 is that the oracle exists before port code because it is the
mechanism that makes generated Rust safe to merge at volume. A `/verify` that
skips it is not a weaker gate, it is a different gate that does not check the
thing this project is most likely to get wrong.

There is one bootstrap exception. S01 builds the corpus and contains no port
code, while F-010 in S02 builds the oracle. `gate --sprint` reports the absent
oracle as a named skip only when the active sprint is S01 and F-010 is still
pending in S02. `gate --all` has no exception. Once F-010 moves from pending,
the sprint profile is strict too.

So: `corpus=absent` is permitted while the corpus is being assembled in S01 and
is recorded honestly as `absent`. It is **not** permitted at
`--profile release`.

## Recording, and the trailer

On success:

```bash
python3 scripts/verify_ledger.py record \
  --gates <the gates that actually ran> \
  --corpus <pass|fail|absent> \
  --profile <profile>
```

This keys on `git write-tree`, the hash of the current index. `.githooks/
pre-commit` looks that entry up and appends the `Ocelli-Verify` and
`Ocelli-Generated-By` trailers (HLD 27.2 R6). CI re-reads them.

**Record what actually ran.** Recording a gate that did not run produces a
trailer asserting a check that did not happen, which is worse than no trailer,
because the record then reads as though it ran and nobody looks again.

If you change a file after recording, the tree hash changes and the record no
longer applies. That is the intended behaviour, not an inconvenience to work
around.

## Reporting

One line per gate with its result, then a summary. On failure, name the gate,
the exact command and the first real error, not the last line of output.

**Never report a gate as passing because a pipeline exited zero.** Read the
command's own status.
