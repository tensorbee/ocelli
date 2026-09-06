# F-013 metadata diff review, pass 1

**Reviewed**: staged diff from `8f84d04` in `/private/tmp/ocelli-f-013`, 14 paths, 1,237 insertions and 210 deletions
**Result**: 7 defects, 0 smells, 0 nitpicks

## Defects

### D1, The expected values do not have one canonical source

**Where**: `tools/oracle/tests/metadata_fixture.rs:70`, `tools/oracle/metadata-truth.json:34`, `tools/oracle/volume-truth.json:17`

**What**: The four C.11 sample triples are copied from `metadata-truth.json`
into a second Rust literal table. The new truth file also copies the volume
normal, projections and gaps already held by `volume-truth.json`. The Rust test
zips the two C.11 tables without checking their lengths.

**Why it is wrong**: The approved plan requires one canonical hand-authored
truth, says both languages consume it, and says no expected value is copied
into two languages. A repeated expected table can drift while each consumer
continues to validate its own copy.

**Evidence**: I removed the fourth C.11 sample from `metadata-truth.json` and
ran `cargo test --manifest-path tools/oracle/Cargo.toml --test
metadata_fixture -- --nocapture`. All four tests passed. The mutation was
reverted. The repeated projection and gap arrays are present in both committed
JSON truth files.

### D2, Declared presence and signed-zero semantics differ between consumers

**Where**: `tools/oracle/src/metadata.rs:170`, `tools/oracle/check_sidecars.py:193`

**What**: Rust maps a missing JSON pointer to `Value::Null`, so a malformed
sidecar that omits a declared member compares equal to a sidecar that records
the DICOM value as absent. Python compares numeric values with ordinary float
equality, so positive and negative zero compare equal there while Rust
deliberately compares their bit patterns.

**Why it is wrong**: The plan requires absent, empty and present values to stay
distinct and requires exact declared-number semantics including signed zero.
Both consumers are described as projections of the same truth contract, so
they cannot give different answers.

**Evidence**: A temporary Rust test compared truth `null` with one explicit
`null` sidecar and one sidecar missing the member. It expected one divergence
and got zero. `float(0.0) == float(-0.0)` is true in the Python comparison at
lines 201 to 202. The temporary test was reverted.

### D3, Opposite-side and partly shared failures receive an arbitrary side

**Where**: `tools/oracle/src/metadata.rs:62`

**What**: `TruthComparison::apply_to` selects the first divergence whose side
is not `Unattributed`. If one truth field is wrong only on the reference and a
second is wrong only on the candidate, the whole view is attributed to the
side of whichever pointer sorts first. A shared error plus a one-sided error
is also assigned to the one side even though `shared_problem` is true.

**Why it is wrong**: Attribution is evidence, not a priority choice. A record
with errors on both producers cannot truthfully name only one producer.

**Evidence**: A temporary unit test used two fields. The reference alone was
wrong on `/attributes/a` and the candidate alone was wrong on
`/attributes/b`. It expected `Unattributed` and received `Reference`. The
temporary test was reverted.

### D4, Metadata truth does not actually run as the rung before pixels

**Where**: `tools/oracle/src/bin/ocelli-compare.rs:247`

**What**: The comparator calculates a truth result, then reads both frames,
runs `compare_view`, and only afterward overwrites the returned record with
`truth.apply_to`. A frame-read or pixel-comparison refusal returns early, so a
known metadata-truth failure never becomes the view's result in that case.

**Why it is wrong**: Approach item 7 and the LLD define metadata truth as an
attribution rung before pixel comparison. Post-processing a pixel result gives
it precedence only on the subset of cases where the lower rung succeeds.

**Evidence**: Lines 260 to 264 order the two frame reads and `compare_view`
before `apply_to`, with `?` on every lower operation.

### D5, Real-row values can still be written into comparator reports

**Where**: `tools/oracle/src/report.rs:352`

**What**: Every `ParameterDivergence` serializes its raw `reference` and
`candidate` values. The existing parameter mutations resolve to
`real__ct_cmb_mml__00000001`, so this path is reachable for a real row.

**Why it is wrong**: Approved approach item 8 requires real-row comparison
without writing values into logs, reports or tracked artifacts. The Python
checker has `<withheld, real corpus row>`, but the Rust report has no matching
redaction.

**Evidence**: The independent mutation replay selected the real row for both
image-slope mutations. `ViewRecord::to_json` writes both values without
checking the row class or identifier.

### D6, Functional-group scope is documentation rather than checked truth

**Where**: `tools/oracle/src/metadata.rs:237`, `tools/oracle/metadata-truth.json:161`

**What**: Parsing accepts four scope strings but never relates a scope to its
JSON pointer, the sidecar source, or functional-group precedence. Changing a
resolved multiframe field from `per-frame` to `top-level` leaves every value
comparison unchanged. The browser test supplies a pre-resolved object and
therefore does not test per-frame over shared precedence either.

**Why it is wrong**: The truth format promises to name whether a value came
from top-level, shared, or per-frame metadata. PS3.3 C.7.6.16 requires
per-frame values to win. A label that can lie without making the gate red does
not verify that provenance or precedence.

**Evidence**: I exchanged the scope labels on a top-level resolved intercept
and the multiframe per-frame intercept. Metadata unit tests passed and
`ocelli-compare identity` remained green over all 99 views. The mutation was
reverted.

### D7, Presentation inversion has no positive fixture

**Where**: `tools/oracle/metadata-truth.json:79`, `tools/oracle/page/app.mjs:164`

**What**: The only `PresentationLUTShape` truth is `null`. No generated row
declares `IDENTITY` or `INVERSE`, and no test supplies a present value from the
independent parser.

**Why it is wrong**: The approved metadata surface explicitly includes
presentation inversion. An implementation that always reports `null` for this
tag satisfies the current truth, so the new read is not evidence that a
present declaration is handled.

**Evidence**: Searching the truth, generator and sidecar tests finds the tag
only in the parser addition, the Python field map, and the one null truth
entry. None of the five new mutations targets it.

## Smells

None.

## Nitpicks

None.

## Verified clean

All 14 staged paths and the approved-plan corrections were reviewed. The
independent PS3.3 C.7.6.2.1.1 calculation reproduces both oblique non-square
samples. Column 2 contributes `2 * 0.25 * X`, row 3 contributes `3 * 0.5 * Y`,
and the result is `[10.4, 20.3, 28.5]`. The second sample independently gives
`[11.4, 21.05, 28.0]`. The series normal is the unit cross product
`[-0.6, 0.8, 0]`, and consecutive projected positions give the recorded
uniform gaps and the non-uniform `3.75, 1.25` pair. Neither calculation uses
slice thickness or spacing tags.

The multiframe frame-zero slope, intercept, centre and width match the
generator's per-frame arrays. Shared Pixel Spacing and Slice Thickness match
the shared Pixel Measures item. The top-level rescale values are absent. The
current fixture does not, however, contain the malformed both-present case
needed to prove precedence, which is D6.

The four C.11 values themselves are correct. In particular, D-13's lower
boundary is zero, LINEAR at 40 is `127.81954887218046`, LINEAR_EXACT at 40 is
`127.5`, and the quarter sample is `63.90977443609023` against `63.75`. No LUT
evaluator was added to production code. F-X012's SIGMOID row, reference
divergence register, and tolerance files are unchanged. No tolerance changed.

`./target/release/ocelli-compare mutations` independently replayed all 26
catalogue entries. All five F-013 mutations selected their named synthetic
views, failed at `metadata-truth`, and attributed the candidate. The full
result was `26 mutations, 0 not detected`. Cargo ran all four new metadata
fixture tests. The browser sidecar suite ran 21 tests and passed its new
transport fixture. The new Rust module is exported by `lib.rs`, the comparator
loads it for identity, mutation and census commands, and the `oracle` gate
still owns both reference generation and comparator mutation replay.

The Python real-row mismatch helper still redacts all tested value shapes, and
its self-test remains connected through the oracle runner. D5 is the separate
Rust report path. Non-finite JSON tokens are rejected by `serde_json` before a
truth document or sidecar can be loaded. Ordered arrays compare in order.

The `oracle.md` and `comparator.md` contributor headers exactly match their
rows in `docs/lld/README.md`. The README additions correctly add F-013 and
retain the existing contributors. Every implementation correction in the
approved plan matches the staged ownership boundary. No implementation,
verification state, ledger, commit, integration or push was changed by this
review.
