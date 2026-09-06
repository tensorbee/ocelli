# F-X001 review, pass 1

**Reviewed**: staged working tree against `0eae742`
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- The contract distinguishes `Available`, `Degraded` and `Unavailable`, and
  permits a fallback only for `Degraded`.
- Required tier and resolved tier remain distinct roles. No future feature
  registry, stable feature number, numeric tier encoding, boundary payload, or
  dependency edge was invented.
- Existing F-004 and F-X016 tier resolution and software-adapter evidence are
  cited as the single implementation. The feature decision does not probe or
  reinterpret adapters.
- Stable code 700 remains the existing unavailable transport. F-101 retains
  ownership of boundary operands, identity and shell conversion when real
  producers and consumers exist.
- The shell contract keeps unavailable features visible, separates diagnostic
  adapter evidence from user-facing wording, and offers an override only when
  the resolver says it is constructible.
- Pixel fallbacks are required to reuse `ocelli-pixel`. No LUT, pixel,
  geometry, render-loop, unsafe, wasm-boundary, or production code was added.
- Every changed LLD has a reciprocal index entry and current contribution
  metadata. The approved conditional code-test rows do not apply because the
  stable operands were deliberately not introduced.
