# S46 colour measurement notes, independent review pass 1

**Reviewed**: root's f240-untagged-comparison-lld-draft.md and the
wasm-named-v6-byte-store/lld-note.md, against their raw artifacts and ledgers.
**Author / reviewer**: root authored the untagged note. glm_codec_merge
authored the Wasm analysis and note, root reviewed it. Codex f244_review
authored neither note, driver, registered instrument nor measured run.
This reviewer prepared and compiled the exact pre-ICC source before root ran
its comparison, a setup role disclosed here rather than hidden.
**Result**: 0 defects, 0 smells, 0 nitpicks in these two bounded notes.
This is documentation/evidence consistency review, not whole-feature or
performance-profile acceptance.

## Untagged comparison

Independently read both actual raw runs and their SHA256 values match the
note and summary: old61c638f9 and assembled3b5fc0eb. Recomputed both medians
from all nine raw window samples. At three decimals every table value
agrees: RGB first4955.386 versus2231.128, window4728.307 versus2751.412.
Palette first19287.031 versus2577.263, window19257.557 versus3128.093.

Each case reports release, decoded first, exactly nine held windows,
30 first and30 warm marker validations, positive actual RSS readings,
zero sampling failures and the actual268435456-byte held budget. All raw
RSS sample counts equal their arrays, and each reported peak equals its
maximum reading times1024. Host objects match exactly and both Node
versions are24.13.0. The old fixture digests are in its raw run as claimed.
The private driver file's actual hash matches the raw external-driver hash.

The original pre-ICC tree12797792 and assembled treec1fe3037 have an exactly
identical colour fixture module. All six shared instrument/helper hashes
in the old run equal both the e4a3b620 instrument tree and assembledc1fe3037
blobs. The old source has no pixel ICC implementation. Its observed cases
have icc false, and its measured case set is exactly RGB8 and palette16.
The explicit old binary hash matches the independently preserved build
receipt, and the assembled binary hash matches its registered run.

The prose does not call these isolated ICC cost, an idle-host observation,
a general clinical result or a pass for monochrome relative-band misses.
It correctly identifies combined-stack changes and limits its improvement
statement to the measured untagged first/window figures. One first request
per workload is reported as a first timing, not a nine-trial median.

## Wasm byte-store note

Read ledger, source identity, handoff, per-section identity and preservation
receipt. Actual shipping file size/hash is2205614 and2065761b, and its
reconstructed predecessor is2205648. Recomputed gzip lengths are872881 and
872915. The raw and gzip deltas are each-34. The note correctly avoids
attributing compressed bytes additively to symbols.

All noncustom-section identity records agree between the current named and
shipping outputs. The only encoded section-size delta is code-34. The two
encoded function-body changes are182 to124 (-58) and7141 to7165 (+24),
which sum exactly-34. The helper payload falls180 to123 (-57), with its
length field falling2 to1. Other group sums are zero. The source session.rs
is byte-identical between3ecb1239 andc1fe3037, so the note correctly calls
its body growth compiled caller variation, not new source or performance.

Ledger fields support5074 functions, unchanged normalized named set,
47 export names/kinds and1275 retained ICC helper bytes. The note explicitly
distinguishes unchanged size from cross-version byte identity after function
reindexing and does not promote absence of named codec symbols into a graph
proof. Decode size/hash remains16455 and07cf6db9 with gzip7663.
The handoff's proposed-note hash matches the exact reviewed note.

## Checks actually run and limits

Only small Python standard-library artifact/JSON checks and Git object reads
ran in this review, direct exit0. No compiler, optimizer, benchmark, browser,
GPU, oracle or native suite was launched. The symbolic build pipeline was
not rerun, and this review does not replace the recorded graph checks or
independent review of its implementation. Historical shipping-profile and
unchanged-budget statements remain bounded by those recorded source/build
identities. No production, note, tolerance, gate or canonical delivery file
was modified. Root may fold these exact notes into the candidate with this
bounded clean result.
