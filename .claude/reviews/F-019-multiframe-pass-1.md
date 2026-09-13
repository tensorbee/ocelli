# F-019 review, pass 1

**Reviewed**: staged worker tree based on `586e503`
**Result**: 2 defects, 0 smells, 0 nitpicks

## Defects

### D1, Number of Frames accepts invalid IS encodings

**Where**: `crates/ocelli-dicom/src/multiframe.rs:303`

**What**: Length is checked after trimming padding, and the parsed positive
integer is not restricted to the IS value range. Inputs longer than 12 bytes
when padding is included, such as eleven spaces followed by `1` and two more
spaces, are accepted. `2147483648` is also accepted on a 64-bit target.

**Why it is wrong**: DICOM PS3.5 section 6.2 limits an IS Value to 12 bytes
and its represented integer to the signed 32-bit range. Number of Frames must
therefore reject both encodings before exposing a frame count.

**Evidence**: Temporarily adding both inputs to
`number_of_frames_defaults_only_when_absent` made the test fail with `left:
None`, meaning construction returned `Ok`, instead of
`InvalidNumberOfFrames`. The focused test exited 101. The probe was reverted.

### D2, odd-length Fragment Values are accepted as valid encapsulation

**Where**: `crates/ocelli-dicom/src/frame_index.rs:259`

**What**: The public constructors validate only that at least one Fragment is
present. A three-byte Fragment Value is accepted and indexed, even though its
Item Value lacks the required pad byte. Offset calculation then treats the
next Item Tag as starting at an odd position that cannot occur in conforming
encapsulated Pixel Data.

**Why it is wrong**: DICOM PS3.5 section A.4 requires each Fragment Item Value
to have even length. Extended Offset Table Lengths may describe an odd encoded
frame length, but the containing Fragment Value still includes one pad byte.

**Evidence**: A temporary assertion that
`from_basic(1, &[], &[&[1, 2, 3]])` returns an error failed. The focused test
exited 101 because the constructor returned `Ok`. The probe was reverted.

## Smells

None.

## Nitpicks

None.

## Verified clean

The review checked functional-group precedence, duplicate-source retention,
top-level opt-in, one-item group validation, checked frame selection, Basic
and Extended Offset Table origins, odd encoded-frame pad removal, final frame
bounds, F-020 ownership, public exports, LLD claims, native and wasm boundary
claims, and absence of unsafe code. Mutating the Fragment Item header size
from eight to zero made five of seven frame-index fixtures fail and was
reverted. Current DICOM PS3.3 C.7.6.3.1.8 confirms that Extended Offset Table
frames use one and only one Fragment.
