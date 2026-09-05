# F-010 review, pass 7

**Reviewed**: the fully staged index on `work/f-010-claude`, 47 paths,
+8478/-41, after six rounds of remediation
**Result**: 4 defects, 0 smells, 6 nitpicks

Twelve mutations run, eleven red and one green by construction. The pattern the
last three passes named recurred, and this time it was mechanical as well as
textual: the round-6 rewrite of a command file was made without re-running the
adapter sync, so the tree failed a floor gate.

## Defects

### D1, the tree failed `bin/ocelli.sh gate skills`

**Where**: `.agents/skills/verify/SKILL.md` against `.claude/commands/verify.md`
**What**: round 6 rewrote the skip paragraph in `verify.md` and did not re-run
`scripts/sync_agent_skills.py`, so the adapter recorded a source digest for
content that had changed.
**Why it is wrong**: `skills` needs no GPU and is therefore in `--floor`,
`--sprint` and `--all`. **CI was red on this tree.** A stale adapter also
serves Codex an out-of-date instruction while claiming to name the current one,
which is the whole reason the digest is recorded. The `close-sprint` pair was
correct, which shows the sync ran at round 5 and not at round 6.
**Evidence**: `sync_agent_skills.py --check` exited 1 naming `verify/SKILL.md`,
and `bin/ocelli.sh gate skills` reported FAILED.
**Fixed**: re-synced, and both `--check` and the gate are green.

The lesson generalises past this instance. `.claude/commands/` is generated
into `.agents/skills/`, so **any** edit there is two files, and the second one
is easy to forget precisely because nothing about editing the first suggests
it. `AGENTS.md` says to run the sync and then `--check`, and the second half is
what would have caught this.

### D2, the handoff asserted a verification and a review that had not happened

**Where**: `.claude/handoffs/F-010-ready.md`
**What**: four false sentences. It said every gate had been run individually,
when `skills` was red. It said seven review passes existed and the last
reported zero defects and zero smells, when this pass is the seventh and
reports four. It listed `-pass-7.md` among the files that landed. And it named
a head commit that does not exist, because the branch is uncommitted.
**Why it is wrong**: this is the highest-consequence instance of the class the
last three passes flagged. A handoff that pre-declares a clean review is a
claim about work not done, and it is the document `/integrate-feature`
consumes. An integrator reading it would not re-run the gates.
**Fixed**: the handoff was written too early. It is regenerated at the reviewed
head, after this pass and its remediation, with every claim true at the moment
it is written. `/complete-feature` says as much: a handoff naming a head that
later commits have overtaken is a record of work that is not what landed.

### D3, "three of the four conjuncts are redundant today" was measured wrong

**Where**: `tools/oracle/tests/unsupported_test.mjs`, and the same reasoning in
pass 6's own record
**What**: **all four** are individually redundant, not three. The two committed
entries differ in transfer syntax, boundary, row path AND error fragment, so
any one alone separates them. The stated cause named one of four sufficient
causes and the count implied the row-path conjunct is load-bearing today.
**Evidence**: replaying the two observed failures against the committed record
with each conjunct removed in turn gives the same outcome all four times.
**Fixed**: the header now says all four and says how that was measured. The
tests themselves were correct and all four still go red individually.

### D4, `numberOfFrames` was read under a section citation it does not belong to

**Where**: `tools/oracle/page/app.mjs`
**What**: nine of the ten attributes under `// PS3.3 C.7.6.3, Image Pixel.` are
Image Pixel attributes. `numberOfFrames` (0028,0008) is not: it is the
Multi-frame Module, C.7.6.6, and it does not appear in C.7.6.3.1.1's table.
**Why it is wrong**: this is `CLAUDE.md`'s "against the cited specification
section, not against the comment above it", on the one block a later reader
would use to decide whether an attribute is being read from the right module.
Pass 6 checked all 25 tag numbers against PS3.6 and found them right, which is
a different check from the section citations.
**Fixed**: it now sits under its own `// PS3.3 C.7.6.6, Multi-frame Module.`
heading. Every other citation in the function was verified individually and all
are correct. Pass 8 then found that the first attempt at this fix explained
itself with a second wrong citation, and corrected it to name the module and
stop there.

## Nitpicks, and what was done

- `--report-unsupported` was described as verifying nothing. It checks the
  pins, runs the unit tests and re-hashes every corpus row. **Corrected to
  "verifies no boundary".**
- "the five `@cornerstonejs/*` packages" read as a definite description where
  ten are installed and five carry the marker. **Corrected in three files.**
- `capture()` coerced each Buffer independently, so a multibyte character split
  across a chunk boundary would be mangled in the cross-read's captured output.
  **`setEncoding("utf8")` on both pipes.**
- `structuredClone(spec.base)` was deep, and then `applyBlock` assigned the
  rule's value by reference and the whole defaults table was shared across
  ninety-one resolutions. Nothing mutates them, which is a thing to remember
  rather than a property. **Both are now cloned.**
- `build-page.mjs` said the worker source is "not in the package's exports map,
  so it is reached through the one subpath that is". Ten subpaths are exported.
  **Corrected to the one that locates the root.**
- The reviewer noted an untracked handoff appearing mid-pass. That is D2.

## One thing the pass found while probing, and closed

`setStack` resets `voiRange`, `interpolationType`, `invert` and both flips for
each new image, and it does **not** reset `VOILUTFunction`. The page set that
property only when the row had a window, so a colour row could inherit the
previous monochrome row's function. Harmless while every corpus row resolves
LINEAR, and exactly the shape of a defect the day one does not. **The page now
sets it unconditionally.** Re-verified: every frame digest is byte-identical to
the previous run, so the hardening changed no pixel.

## Verified clean

- **Pass 6's D1, D2 and S1 are genuinely fixed.** The walk was measured from
  the real tree: exactly six of the sixteen fail the direct route, and for each
  the naming manifest is found at depth 1, the second step, never depth 0. The
  A1 and A2 text now claims only reference frames for five rows, and all five
  have `.raw`, `.png` and `.json` on disk. All four `entryFor` conjuncts go red
  individually.
- **`verify.md`'s "no skip path at all" is true**, traced exhaustively:
  `skip()` returns 3 from exactly three arms, `lint`, `types` and `wasm`. The
  `oracle` arm exits 1 when `node_modules` is absent and otherwise returns 0, 1
  or 2.
- **The probes asked for.** `--report-unsupported` was run live: it skips
  `prepareOutput`, renders both passes, prints a well-formed candidate, writes
  no output directory at all and returns 2. Two rows sharing a `sha256` is
  harmless, because rows are keyed by path and each digest is checked against
  its own bytes. A trailing blank line in the manifest is skipped and a CRLF
  file is **refused at the header**, loudly, rather than dropping rows.
  `capture` does not deadlock under heavy stderr. `page/app.mjs`'s `state`
  holds only the element, engine and viewport, and `setStack` resets the
  presentation properties per row.
- **Arithmetic re-derived from PS3.3 rather than read**: `fullRange`'s
  substitution into C.11.2.1.2 giving exactly `min` and `max` with the
  asymmetric operators intact, `minimumWidth` against C.11.2.1.2 and
  C.11.2.1.3, and `canvasScale` from VTK's half-height convention and
  C.7.6.2.1.1, with `reference_mono12`'s 0.25 letterbox matching the recorded
  `blackFraction` to the digit.
- **Every number in the round-6 prose executed, not sampled**, including the
  five-row HTJ2K and JPEG-LS set, the 87 magnified against 2 fitted down, the
  85 file windows and 4 colour with zero fallbacks reached, and the
  sixteen-row low-information set against the eighteen `syntax/` rows.
- Boundary, tier and structure unchanged and clean. No `wasm-bindgen`, no
  pixels across an Ocelli boundary, no wasm memory view, no render loop, no
  `queue.submit`, no new trait, generic, `Box<dyn>` or forwarding wrapper, no
  `as` cast and no `unsafe`.
- After remediation: 88 unit tests pass, `npx eslint .` exits 0,
  `sync_agent_skills.py --check` reports 20 adapters matching,
  `prose_check.py` is clean over 74 files, and `bin/ocelli.sh gate skills prose
  content oracle` is ALL GREEN over four gates with every frame digest
  unchanged.
