---
description: The completion gate. Runs the gate set for a profile and records the result against the tree hash so the commit-msg hook can write the provenance trailer.
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

There was one bootstrap exception, and it is gone. S01 built the corpus and
contained no port code, while F-010 in S02 built the oracle, so `gate --sprint`
reported the absent oracle as a named skip. F-010 removed the mechanism along
with the skip. **Neither profile has an exception now, and the oracle gate has
no skip path at all**: an absent reference stack is a refusal from
`bin/ocelli.sh`, which is a failure to fix rather than a state to record.

So: `corpus=absent` is permitted while the corpus is being assembled in S01 and
is recorded honestly as `absent`. It is **not** permitted at
`--profile release`.

## Final-state attempt

Stage the final owned changes before checking them. Include completion
metadata, updated plans, LLD and delivery records in the applicable checks.
Do not stage another writer's work. `git add -N` does not count. Set
`OCELLI_AGENT` or pass `--agent` before recording.

Before running this profile, invalidate any prior receipt for this staged
tree with `python3 scripts/verify_ledger.py invalidate`. A failed rerun must
leave no older success applicable. For a sprint attempt, also record a failed
sprint verification on failure using `record-verification --profile sprint
--result fail`, with the actual executed gates and corpus result.

Keep exact commands, own exit statuses, selected/filtered/ignored tests,
configuration, input identity and tool versions. Record only after the required
checks pass. A summary is not an execution. A changed tree requires the normal
profile again. Save independent RED/GREEN results for repairs. GPU absence,
SwiftShader, native adapter evidence and external candidate parity remain
separate qualifications. Never call reference-only rendering viewer parity.

Use distinct build-output directories for isolated source copies and controlled
mutations. A shared output cache can select the other copy's test executable.
If artifact identity is ambiguous, discard that result and rebuild the intended
source before using its outcome as evidence.

## Explicit target checks

For changes to session exports, tickets or public bulk/command adapters, build
and execute the actual wasm boundary suite in addition to the required profile:

```bash
wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm
npx vitest run --config vitest.boundary.config.ts
```

For CPU canvas binding or publication changes, also run:

```bash
node packages/core/cpu-canvas.mjs
```

Rebuild after relevant source changes. The explicit suite and worker script
are not collected by default npm test. These are host workflow requirements,
not additions to `bin/ocelli.sh` gate membership. Keep their command/results
in feature evidence and the named gate list limited to gates actually run.
A required target check that fails or cannot execute remains unmet even if
the profile is green. The dev artifact does not satisfy release-size checks,
GPU parity, corpus coverage or representative-host acceptance.

## Recording, and the trailer

On success:

```bash
python3 scripts/verify_ledger.py record \
  --gates <the gates that actually ran> \
  --corpus <pass|fail|absent> \
  --profile <profile>
```

This keys on `git write-tree`, the hash of the current index.
`.githooks/commit-msg` appends the `Ocelli-Verify` and `Ocelli-Generated-By`
trailers from that entry (HLD 27.2 R6). Pre-commit warns if the receipt is
missing. CI re-reads the trailers.

**Record what actually ran.** Recording a gate that did not run produces a
trailer asserting a check that did not happen, which is worse than no trailer,
because the record then reads as though it ran and nobody looks again.

If you stage a file change after recording, the tree hash changes and the
record no longer applies. That is the intended behaviour, not an inconvenience
to work around.

An unstaged edit does not change the index hash. Check both staged and
unstaged diffs before naming the verified source. An index receipt certifies
the staged identity, while most checks execute working files. Resolve any
relevant mismatch before recording. Do not stage unrelated work to hide it.
Record the tree before and after the checks and disclose any intervening edit.

## Reporting

One line per gate with its result, then a summary. On failure, name the gate,
the exact command and the first real error, not the last line of output.

**Never report a gate as passing because a pipeline exited zero.** Read the
command's own status.

When a guard claims staged-tree coverage, verify both inventory and bytes.
An unchanged dependency remains in the index even when absent from the diff.
A working-tree read cannot certify a staged blob. Challenge both directions:
staged bad bytes hidden by a clean worktree, and unstaged bad bytes outside a
clean index. Repository enumeration or blob-read failure must not become an
empty successful check. Preserve the existing policy limits and dependency
ratchets while repairing identity errors.

For benchmark worker, event or collector changes, run the explicit release
browser controls as well as the profile's floor benchmark suite:

```sh
OCELLI_BENCH_BROWSER=1 node --test tools/bench/tests/run_test.mjs
```

Check new refusal sites against the guard catalogue and state browser-only
coverage limits. When expanding a gate arm, verify that CI invokes the whole
arm in order. Helper modules belong outside a registry-only runner directory.
