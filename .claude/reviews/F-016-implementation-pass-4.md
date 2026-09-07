# F-016 review, pass 4

**Reviewed**: current staged implementation and design, including the pass-3
remediations and prior review records
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass-3 D1 is closed. Completion no longer searches a finite private-tag
  namespace or scans untrusted value bytes, so legal binary values cannot
  exhaust marker selection.
- Pass-3 D2 is closed. The fixed marker is the defined File Meta Information
  Group Length element `(0002,0000)` with VR `UL`, a four-byte value, and the
  correct implicit or explicit encoding and byte order for every route.
- The structural preflight runs before marker insertion. It refuses top-level
  group 0002 input, odd value lengths, malformed delimiters, incomplete values,
  and unfinished sequence nesting. This makes the fixed marker unforgeable by
  input and keeps partial top-level headers observable.
- The marker is required and removed before the parsed object crosses the
  public boundary. The collector then rejects any remaining top-level group
  0002 element.
- The design now accurately records one data-set adaptation followed by a
  strict structural preflight and one collection under the resolved transfer
  syntax. It no longer claims the removed `read_dataset_with_ts` path.
- Transfer-syntax routing remains fixed by File Meta Information with no retry
  or guessed fallback. Deflated Explicit VR Little Endian is adapted once, and
  encapsulated pixel fragments remain undecoded.
- D-18, the workspace dependencies, the crate manifest, and the no-std guard
  agree that `ocelli-dicom` is a `std` crate using four direct dicom-rs
  components with defaults disabled and only data-set deflate enabled.
- `bin/ocelli.sh test ocelli-dicom` passed 22 executed unit and fixture tests.
  The corpus integration test remained ignored in the ordinary suite.
- `bin/ocelli.sh check ocelli-dicom` and `bin/ocelli.sh clippy ocelli-dicom`
  passed.
- `bin/ocelli.sh gate corpus` passed over 92 verified rows and all 16 declared
  transfer syntaxes, including the ignored Rust integration test.
- `git diff --cached --check` passed.
- Public errors and corpus failures expose only fixed failure classes, row
  numbers, and transfer-syntax identifiers. They do not retain or report input
  bytes, paths, DICOM values, or instance identifiers.
