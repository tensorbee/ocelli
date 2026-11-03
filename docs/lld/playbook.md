# Migration playbook

**F-IDs that contributed:** F-116
**Last updated:** 2026-11-03

HLD section 12 keeps the integration seam and replaces one viewport kind at
a time. The order is stack, then mpr, then volume. Shadow mode stays the
existing check. A codemod rewrites a mechanical listener name and leaves a
different concept alone. `rewriteListenerNames` is that codemod.

`nextMigrationKind` returns the next kind only when the done list is a
prefix of that order.

- An empty list returns `stack`.
- `stack` returns `mpr`.
- `stack` then `mpr` returns `volume`.
- All three returns null. That null means the list is finished.
- A skip, a duplicate or an unknown name also returns null. The caller
  tells those two nulls apart by looking at the list. The function does
  not repair it.

Worked calls:

```text
nextMigrationKind([]) is stack
nextMigrationKind(["stack"]) is mpr
nextMigrationKind(["stack", "mpr"]) is volume
nextMigrationKind(["stack", "mpr", "volume"]) is null
nextMigrationKind(["mpr"]) is null
```

This page does not claim that 1.0.0 is published. Tiers A, B and C are
not applicable. The function does not render.
