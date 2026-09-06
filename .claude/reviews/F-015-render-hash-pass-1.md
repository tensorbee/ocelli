# F-015 review, pass 1

**Reviewed**: working tree against `1e18647`
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Recomputed the literal view and run digests independently with Python
  `hashlib` from the documented domains and little-endian framing. Both match
  the Rust fixtures.
- Checked that the per-view domain binds kind, identifier, width, height,
  format, byte length, and exact RGBA8 bytes. The aggregate binds its view
  count and sorted framed entries.
- Checked that hashing reuses the validated `Frame` and never reads PNG bytes,
  padding, report ordering, elapsed time, environment identity, or
  presentation parameters.
- Checked all changed Rust for new casts, unsafe blocks, and panic-prone
  `unwrap` or `expect` calls. None were added.
- Mutated the format token from `RGBA8` to `RGBA9`. The pinned independent
  digest test failed at exit 101 with the changed digest, then passed after the
  mutation was reverted.
- Ran all five render-hash fixtures after the revert. Five passed and none
  failed.
- Confirmed the untracked `compare-out` example named by the plan remains
  ignored and is not staged. The implementation correctly treats that plan
  path as verification output rather than tracked source.
- Read the report and mutation integration. Identity reports equal side hashes,
  frame mutations require the damaged side hash to move, and equal hashes are
  not treated as a tolerance or cross-machine reproducibility claim.
