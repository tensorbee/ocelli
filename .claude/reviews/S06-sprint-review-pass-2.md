# S06 sprint review, pass 2

**Reviewed**: full sprint diff from
`baaa92146ebd7844cca086c957f1d1015a1eec9f` plus staged remediation at tree
`78cec15ecd8994c224bb56f91f1ad4d0bc102b73`
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, Sequence depth does not validate item-control context

**Where**: `crates/ocelli-dicom/src/parse.rs:311`

**What**: The remediation accepts every `ItemStart` and `ItemEnd` token while
`sequence_depth > 0`, without checking whether the reader is expecting an Item
or is inside one. It also accepts every `SequenceEnd` for a positive depth
without checking whether the sequence has explicit or undefined length.
Consequently, an explicitly sized SQ whose complete eight-byte payload is an
Item Delimitation Item returns `TruncatedDataSet`, and the same SQ containing a
Sequence Delimitation Item returns `Ok`.

**Why it is wrong**: PS3.5 sections 7.5.1 and 7.5.2 require a Sequence value to
contain Items. An Item Delimitation Item ends an undefined-length Item, so it
cannot appear while the sequence parser is expecting an Item. A Sequence
Delimitation Item terminates a Sequence whose length is undefined, not an
explicitly sized Sequence. Both probe inputs are complete malformed nesting
and must return `InvalidDataSet`. The current results contradict the public
error contract and the plan's claim that the structural preflight refuses
malformed nesting.

**Evidence**: A standalone synthetic harness linked against the staged
`ocelli-dicom` library. The SQ was `(0008,1110)` with explicit length 8 and no
Item. Its entire Value was one of these control tokens:

| SQ Value | Observed on all five dispatch paths |
|----------|--------------------------------------|
| `(FFFE,E00D)`, Item Delimitation Item, length 0 | `Some(TruncatedDataSet)` |
| `(FFFE,E0DD)`, Sequence Delimitation Item, length 0 | `None` |

The results were identical for Implicit VR Little Endian, Explicit VR Little
Endian, Explicit VR Big Endian, Deflated Explicit VR Little Endian, and an
encapsulated transfer syntax. An undefined-length SQ containing Item
Delimitation while expecting an Item, followed by its Sequence Delimitation,
does reach `InvalidDataSet` later in collection. That one passing refusal does
not close the explicitly sized cases above.

Replace the scalar depth check with context that distinguishes a Sequence
expecting an Item from an open Item and retains each container's explicit or
undefined length. Reject Item Delimitation outside an undefined-length Item
and reject a literal Sequence Delimitation inside an explicitly sized
Sequence. Add standing public integration tests for both complete malformed
forms across the supported byte-order and adaptation routes.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass-1 D1 is closed at top level. Item, Item Delimitation Item, and Sequence
  Delimitation Item each return `InvalidDataSet` under all five dispatch paths.
- The group FFFE element-header refusal does not reject valid sequence flows.
  Independently encoded empty-item sequences passed under Implicit VR Little
  Endian, Explicit VR Little Endian, Explicit VR Big Endian, Deflated Explicit
  VR Little Endian, and an encapsulated transfer syntax.
- The committed encapsulated Pixel Data fixture still accepts its Basic Offset
  Table, fragment Item, and Sequence Delimitation Item and retains fragments
  without decoding them.
- All earlier completion-marker, partial-header, odd-length, deflate, routing,
  and safe-error findings remain remediated. No retry or alternate syntax was
  introduced.
- The staged remediation changes only the prior sprint review, parser
  preflight, and Part 10 integration tests. Dependencies, D-18, no-std posture,
  public API, patient-safe errors, LLD, and sprint completion records remain
  consistent with the reviewed sprint scope.
- `bin/ocelli.sh test ocelli-dicom` passed 25 executed unit and integration
  tests. The corpus integration test remained ignored in the ordinary suite.
  `bin/ocelli.sh check ocelli-dicom` and
  `bin/ocelli.sh clippy ocelli-dicom` passed.
- `bin/ocelli.sh gate corpus nostd deviations` passed. The corpus gate verified
  92 rows across all 16 declared transfer syntaxes and ran the ignored Rust
  integration test.
- `bin/ocelli.sh native` passed all four stages, including wasm32, native test
  compilation, and identical direct dependency features across targets.
- The S06 workflow records pass 1 against its reviewed tree and records a
  passing 30-gate sprint verification against the current staged tree
  `78cec15ecd8994c224bb56f91f1ad4d0bc102b73`.
- `git diff --cached --check` and `git diff --check` passed.
