# F-017 review, pass 3

**Reviewed**: post-integration seam delta in `crates/ocelli-dicom/src/metadata.rs` and `crates/ocelli-dicom/tests/metadata.rs`
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, The sequence test passes when the constructor returns a non-sequence value

**Where**: `crates/ocelli-dicom/tests/metadata.rs:455`

**What**: The test destructures `MetadataValue::Sequence` with a
`let ... else { return; }`. Returning from the else branch completes a Rust
test successfully. A constructor that retains VR `SQ` but discards every item
into another `MetadataValue` variant therefore leaves the dedicated invariant
test green.

**Why it is wrong**: DICOM PS3.5 section 7.5 requires a value with VR `SQ` to
consist of a sequence of zero or more items, and requires those items to remain
an ordered set. HLD section 27.2 R2 and the microscope mutation rule require
the test to fail when either the carrier or order is wrong. The current else
branch makes the carrier half of that evidence fail open.

**Evidence**: In an isolated clone, the constructor body was changed from
`MetadataValue::Sequence(items)` to `MetadataValue::Empty` while retaining
VR `SQ`. The items were explicitly consumed so the mutation compiled cleanly.
`bin/ocelli.sh test ocelli-dicom
sequence_constructor_fixes_sq_vr_and_preserves_ordered_items -- --exact`
still reported the targeted test as passed.

## Smells

None.

## Nitpicks

None.

## Verified clean

- The implementation seam itself fixes the element VR to `VR::SQ` and moves
  the supplied `Vec<MetadataSet>` directly into `MetadataValue::Sequence`.
  `Vec` retains item order, so there is no re-keying, sorting, flattening, or
  nested-set merge in the constructor.
- DICOM PS3.5 section 7.5 was checked directly. It states that VR `SQ` is used
  for a value consisting of zero or more items, each item contains a data set,
  and the items form an ordered set referenced by ordinal position. The skill
  reference agrees that `SQ` carries nested data sets and may use undefined
  length.
- The generic `MetadataElement::new` remains private. No public constructor
  accepts both an arbitrary VR and a `MetadataValue::Sequence`. The F-021
  DICOM JSON parser handles `VR::SQ` in its own branch, constructs each nested
  `MetadataSet` in input order, and returns through
  `MetadataElement::sequence` before the primitive conversion path.
- Changing the constructor VR from `SQ` to `PN` in an isolated clone made the
  targeted test fail at the exact VR assertion.
- Reversing the constructor's item vector in an isolated clone made the
  targeted test fail because the first item length changed from one to zero.
  The two item shapes are intentionally asymmetric, so the ordering check is
  independent of the implementation.
- The unmodified targeted test passed in the canonical worktree.
  `bin/ocelli.sh check ocelli-dicom`, `bin/ocelli.sh clippy ocelli-dicom`, and
  `bin/ocelli.sh fmt --check` passed.
- The six-line implementation seam and 21-line test seam add no `as` cast,
  unsafe code, trait, generic, dynamic dispatch, `wasm-bindgen`, pixel
  arithmetic, geometry arithmetic, render-loop work, or GPU submission.
- `git diff --check` over the two seam files passed before this report was
  added.
