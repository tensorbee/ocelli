# S09 whole-sprint review, pass 1

**Reviewed**: the complete sprint diff, `998a384..HEAD`, staged tree
`9fcf3127046f`, which is the tree `gate --sprint` verified at 30 gates green.
**Result**: 1 defect, 1 smell, 0 nitpicks

This pass deliberately does not re-run the per-story reviews. Each of the four
stories reached a clean pass with its own mutations. What this pass looks for is
what none of them could see: the interactions, and the claims that became false
because no single story owned the file they are in.

## Defects

### D1, `CLAUDE.md`'s central claim about this repository is false

**Where**: `CLAUDE.md`, `## Current state`, final paragraph.

**What**: the sentence reads

> **No pixel, LUT or geometry port code has been written**, which is decision D7
> holding: the oracle exists first. S03 did add Rust that HLD Part II specifies,
> the tier resolution of section 7 and the error model of section 23, so the
> unqualified form of that sentence stopped being true and the qualified one is
> the claim.

Every part of it is now wrong, and it has been wrong for two sprints:

- **Pixel code exists.** F-018 wrote the stored-value unpacker, the modality
  stage and the VOI stage in `crates/ocelli-pixel/`.
- **LUT code exists.** F-018 wrote stages 1 and 2 of PS3.3 C.11 and F-029 wrote
  stage 3, with the section 18.3 fixtures behind them.
- **Geometry code exists.** F-018 wrote `ImagePlane` and C.7.6.2.1.1's
  index-to-world transform, and F-020 wrote derived per-frame geometry, stack
  shear and spacing calibration.

**The qualification makes it worse rather than better.** It names S03 as the
moment the unqualified form stopped being true and then asserts the qualified
form still holds. The qualified form does not hold either.

**Why it is wrong**: `/microscope` severity table. `CLAUDE.md` is the file every
agent and contributor reads first, and this paragraph tells them the project has
no pixel arithmetic in it. Someone acting on it would write the LUT chain a
second time, which is the one thing HLD section 18 forbids and the one thing
this sprint's F-029 was careful not to do.

**Evidence**: the four other claims in the same section were checked at the same
time, and three are also stale:

| Claim | Actual | Status |
|-------|--------|--------|
| "No pixel, LUT or geometry port code has been written" | `ocelli-pixel` has all three | **false** |
| "the corpus, 91 rows" | `tail -n +2 corpus/manifest.tsv \| grep -c .` is 92 | **false** |
| "the codec registry will claim, which are listed in `scripts/corpus_check.py` because the crate is still a scaffold" | `ocelli-codec` has five decoder families and 15 of 16 UIDs available | **false**, the crate is not a scaffold |
| "eleven shared crates" | `ls crates \| wc -l` is 13, minus native-only `ocelli-native` and minus `ocelli-wasm`, which `bin/ocelli.sh` step 2 names separately | **correct**, and this row initially read as a discrepancy until the arithmetic was actually done |
| "There is still no Ocelli renderer to compare against" | true | correct |

**This is a whole-sprint finding rather than any story's**, which is why it
survived four clean per-story passes: no story in S08 or S09 touched
`CLAUDE.md`, and each story's review looked at its own diff.

## Smells

### S1, no test registers the five decoder families together

**Where**: `crates/ocelli-codec/tests/`.

**What**: each of `native.rs`, `jpeg.rs`, `jpeg2000.rs`, `jpegls.rs` and
`htj2k.rs` asserts that its own registration is atomic and that its own UIDs
become `Available`. **Nothing registers all five into one `Registry`.** The
sprint added two families to a crate that had three, and the property that
matters at the crate level, that they coexist and partition the known
catalogue without collision, is asserted by nothing.

**Why it is wrong**: `/microscope` section 4. Registration returns
`RegistryError::AlreadyRegistered` on a collision, so a sixth family claiming a
UID an existing one already owns would fail at run time in whatever order the
caller happened to register, and no test would have caught it first. It is a
smell rather than a defect because no collision exists today.

**Evidence**: measured by registering all five in a throwaway example.

```text
1.2.840.10008.1.2.1.99           KnownUnavailable
every other known UID            Available
available 15 of 16
```

The one unavailable syntax is Deflated Explicit VR Little Endian, and that is
**by design**: PS3.5 A.5 deflates the whole data set rather than a frame, so it
belongs to `ocelli-dicom` under D-18 and F-025 deliberately left its UID
unavailable in the codec registry. That fact is in `docs/lld/dicom-ingest.md`
and in no test.

## Nitpicks

None.

## Verified clean

- **`bin/ocelli.sh gate --sprint` is green at 30 gates** on the reviewed tree,
  including `oracle`, whose comparator detected all 29 declared mutations, and
  `corpus`.
- **The shared stored-domain boundary has one implementation and three
  consumers.** `sample_convert::convert_samples` was extracted from
  `jpeg2000.rs` at F-028 and is used unchanged by `jpegls.rs` and `htj2k.rs`.
  `git diff` shows the moved functions are byte-identical to their originals.
  A byte-identical copy in each adapter was the alternative and is the defect
  class this sprint was most exposed to.
- **HLD section 18's arithmetic still exists exactly once.** Two of this
  sprint's three codec adapters call a dependency whose layout type applies a
  modality rescale, and both pin slope to 1 and intercept to 0 at their single
  call site. F-028's fixture asserts decoded sample `n` is exactly `n`, and
  moving the pin fails eight tests. `ocelli-pixel` remains the only place the
  rescale is computed.
- **F-029 did not reimplement stages 1 and 2**, which was the sprint's named
  risk for that story. `apply_window` and `ModalityTransform` are byte
  unchanged across the whole sprint diff and `tests/voi.rs` and
  `tests/modality.rs` are unmodified.
- **F-020 did not reimplement the index-to-world transform**, the sprint's
  named risk for that story. `ImagePlane::index_to_world` is byte unchanged and
  `FrameGeometry::index_to_world` forwards to it.
- **Every UID this sprint added is falsifiably distinct from its neighbours.**
  `.80` against `.81` by the codestream's `NEAR`, `.201` and `.202` against
  `.203` by the wavelet transform, `.202` against the other two by RPCL, and
  every HTJ2K syntax against JPEG 2000 Part 1 by the CAP marker. Each is a
  mutation that was observed red.
- **The deviation register is consistent.** `python3 scripts/deviation_check.py`
  reports 22 deviations with every citation resolving. D-22 is new, D-20 and
  D-21 were widened at design approval, and no plan cites an undeclared row.
- **No `as` cast, no `unsafe` and no `wasm-bindgen` was added by this
  repository across the whole sprint.** The 104 vendored `unsafe` constructs
  are a recorded number that the build now holds, which is stronger than the
  position before the sprint, where a dependency's unsafe was invisible.
- **Every new refusal arrived with its probe.** The guard census reports 848
  refusals in 69 files, all claimed, with the site counts for `pins`, `unsafe`
  and `bench.htj2k` recorded in the same diff that added them, and the two
  stale spike entries retired along with the harnesses they claimed.
- **The three new benchmark bands were each derived from their own measured
  series**, 10, 15 and 5 per cent, and the spread between them is reported as a
  property of the subjects rather than smoothed to one number. No protocol was
  changed to make a band narrower.
