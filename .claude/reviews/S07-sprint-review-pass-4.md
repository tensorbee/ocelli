# S07 sprint review, pass 4

**Reviewed**: full repaired sprint history from
`649af7ed05b1c5e42bfa1345f39e15eef07dc735` through
`f0645f9325d713a352ddae4627213e1598f6250b`, plus exact pre-review staged
remediation tree `e92a4e41ba21622a727f68760ff88679e7537c7f`
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass-3 D1 is repaired before map materialization. The custom serde visitor
  checks each object member name before insertion and recursively applies the
  same visitor to array entries and object values. Equal Tag keys at the query
  data-set root and inside an SQ item return the safe structural
  `SourceError::InvalidJson` without retaining either response value.
- The duplicate guard is mutation-sensitive. In a disposable archive of the
  reviewed tree, disabling only the `contains_key` refusal made both committed
  duplicate tests fail. Additional disposable probes showed that repeated
  `vr` and Person Name member names are also refused, that duplicate error
  display and debug text omit a synthetic marker, and that 256 nested arrays
  reach serde_json's recursion refusal as `InvalidJson` without a panic.
- Pass-3 D2 is repaired across every public operation that accepts an
  identifier. `searchSeries`, both `searchInstances` positions, all three
  WADO-RS instance and frame positions, and all three WADO-URI positions call
  the shared predicate before URL construction or fetch. The predicate follows
  PS3.5 2026c section 9.1 for nonempty decimal components, dot separation,
  forbidden leading zeroes except the component `0`, and the 64-character
  maximum. Empty, dot-segment, malformed, leading-zero, and 65-character cases
  are refused. The 64-character boundary is accepted.
- The UID guard is mutation-sensitive. Replacing the predicate with an
  unconditional acceptance in the disposable archive made the committed
  all-method pre-fetch test fail. The unmodified focused TypeScript suite
  passed all eleven DICOMweb tests and made no fetch for any invalid UID.
- Pass-3 D3 is repaired without rewriting append-only delivery history. The
  appended F-021 correction records the twelve-test post-pass-1 suite and the
  two later duplicate tests, which agrees with the current fourteen
  `dicomweb.rs` tests. The appended F-023 correction agrees with the current
  thirteen registry tests. The pass-1 review now names the real sprint base
  ending in `dc735`. Every other full object identifier in the three earlier
  sprint reviews resolves, apart from the deliberately quoted prior typo in
  pass 3's evidence.
- All pass-1 seams remain repaired. The retained High Bit 15 corpus case is
  consistently marked legacy nonconforming and its exact 16 Bits Allocated,
  12 Bits Stored, High Bit 15 descriptor is refused with expected High Bit 11.
  The selected 16-UID codec catalogue equals the manifest set, every selected
  UID reaches F-016 parsing, and F-021 retains JPEG-LS transfer-syntax evidence
  as `KnownUnavailable`. Replacing selected HTJ2K `.203` with parser-known
  MPEG2 `.100` in the disposable archive made the cross-story set proof fail.
- All pass-2 ownership and prose repairs remain accurate. Both F-017 plan
  locations name F-021's scoped `NullSlots` seam. The F-022 status sentence is
  explicitly a design-time snapshot. The TypeScript index and packaging LLD
  acknowledge F-021's live public DICOMweb exports while leaving the later
  viewer and wasm APIs to F-100 and F-101. The repaired F-021 commit names
  existing deviations D-02 and D-18.
- Every one of the eight commits in `origin/sprint/s07..HEAD` passes
  `verify_ledger.py check-commit --require-corpus`. Each trailer's tree matches
  the commit tree and every commit names `Ocelli-Generated-By: codex`.
- The complete Rust suites passed: `ocelli-codec` passed two unit and thirteen
  registry tests. `ocelli-dicom` passed eight unit, fourteen DICOMweb,
  eighteen metadata, sixteen NIfTI, and forty-one Part 10 tests. Its one
  corpus-present test remains deliberately ignored outside the corpus gate.
- F-017 still preserves exact VR, missing versus present-empty state, source
  spelling, signed widths, ordered sequence items, typed null positions,
  carrier identity, provider precedence, and provider identity. The fixed-VR
  sequence test remains mutation-sensitive.
- F-021 keeps fetch, authentication, status, cancellation, and content
  negotiation in TypeScript. Rust owns pure response validation and parsing.
  Bulk response bytes cross the sink once, frame parts borrow ranges from one
  owned response, multipart resource evidence remains exact, and public errors
  retain no request or response data.
- F-022 retains checked header, dimension, offset, payload, and affine
  arithmetic. Its qform mixed terms, sform precedence and column order,
  byte-swapped NIfTI-2 refusal, selected-affine validation, and left-multiplied
  RAS-to-LPS conversion remain covered by non-symmetric fixtures. No tolerance
  changed.
- F-023 retains exact UID lookup, three capability states, checked output
  length, atomic multi-UID registration, collision refusal, caller-owned
  buffers, and propagated decoder errors. It activates no concrete codec and
  keeps the `decode.frame` benchmark honestly unavailable.
- The focused TypeScript bulk, DICOMweb, and public-index suites passed all
  twenty-four tests. Type checking and lint passed. Native crate checks passed
  for `ocelli-dicom` and `ocelli-codec`.
- The verification ledger contains a passing sprint-profile record with corpus
  evidence for the exact reviewed tree and all thirty declared gates. Focused
  prose, skill synchronization, backlog, staged-content, unsafe, bindgen,
  no-std, pin, and guard gates were replayed and passed during this review.
- The full S07 production diff adds no Rust `as` cast, unsafe block,
  out-of-crate `wasm-bindgen`, pixel arithmetic, render-loop work, tolerance
  change, or disabled gate. The `SeriesSource` and `Decoder` traits and decoder
  dynamic dispatch are the two explicit HLD section 13 and section 21
  structural exceptions.
- The manifest's final tab remains the required empty ninth URL field. It is
  the only `git diff --check` warning and is not stray whitespace under the
  manifest parser's exact nine-column contract.
