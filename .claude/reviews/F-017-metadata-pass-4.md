# F-017 review, pass 4

**Reviewed**: staged F-017 seam tree containing the pass-3 review, `crates/ocelli-dicom/src/metadata.rs`, and `crates/ocelli-dicom/tests/metadata.rs`
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- The pass-3 test defect is corrected. The test first asserts that the value
  matches `MetadataValue::Sequence(_)`. Its following conditional only reads
  items after that assertion has proved the variant, so a non-sequence value
  can no longer return successfully.
- The non-sequence remediation is mutation-sensitive. In an isolated clone,
  the constructor retained VR `SQ` but returned `MetadataValue::Empty` after
  consuming its input. The targeted test failed with the message `the SQ
  constructor must retain the sequence carrier`.
- Exact VR remains independently tested. Changing the constructor from
  `VR::SQ` to `VR::PN` in an isolated clone made the targeted test fail with
  `left: PN` and `right: SQ`.
- Item ordering remains independently tested. Reversing the constructor input
  in an isolated clone made the targeted test fail because the first item had
  length zero instead of one. The two item shapes remain asymmetric, and the
  assertions check both ordinal positions.
- The implementation fixes the public sequence constructor to
  `MetadataElement::sequence(Vec<MetadataSet>)`. It accepts no caller-supplied
  VR, and the module-private generic constructor is the only path used to pair
  `VR::SQ` with `MetadataValue::Sequence` here. The F-021 DICOM JSON SQ branch
  returns through this fixed constructor before primitive conversion.
- DICOM PS3.5 section 7.5 was rechecked. It requires VR `SQ` for a sequence of
  zero or more nested data-set items and defines those items as an ordered set
  referenced by ordinal position. The fixed VR and preserved vector order
  implement that seam directly.
- The unmodified targeted test passed in the canonical worktree.
  `bin/ocelli.sh check ocelli-dicom`, `bin/ocelli.sh clippy ocelli-dicom`, and
  `bin/ocelli.sh fmt --check` passed.
- The implementation and test seam add no `as` cast, unsafe code, trait,
  generic, dynamic dispatch, `wasm-bindgen`, pixel arithmetic, geometry
  arithmetic, render-loop work, or GPU submission.
- The pass-3 report, implementation seam, and remediated test were read as one
  staged tree. `git diff --cached --check` passed before this report was
  added.
