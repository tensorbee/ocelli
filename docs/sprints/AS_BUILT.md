# As Built, completion log

Append-only record of every completed F-ID. New entries land at the **bottom**.
**Never edit a prior entry.** A correction goes in a follow-up entry that
references the original, because the value of this file is that it records what
was believed at the time.

Written by `/complete-feature` step 2.

## Entry format

```markdown
## F-XXX, {short title}, completed {YYYY-MM-DD}

**What was built.** {1-3 sentences describing the deliverable}

**HLD sections implemented.** {docs/hld/<file>.md sections, with section numbers}
**Deviations.** {D-NN rows in docs/hld/DEVIATIONS.md, or "none"}
**Crates / packages modified.** {paths}
**Tests added.** {paths, count by category from the taxonomy in WORKFLOW.md}
**Fixture provenance.** {for pixel arithmetic: the DICOM section each hand-computed
                         fixture cites. HLD 27.2 R3. Or "no pixel arithmetic".}
**Verification.** {gate set, date, and the Ocelli-Verify trailer's tree}
**Corpus.** {pass with N cases | absent, with the reason | failed-and-justified}
**Tier coverage.** {per tier: A (WebGPU), B (WebGL2 downlevel), C (CPU).
                    full, degraded (how), unavailable, or n/a. All three.}
**LLD updated.** {docs/lld/*.md files updated}
**Deviations from the design plan.** {list with reasons, or "none"}
**Notes for future sessions.** {non-obvious details, limitations, follow-up F-IDs}
```

## Why `Fixture provenance` and `Tier coverage` are their own fields

**Fixture provenance.** HLD section 27.2 R3: every function doing pixel
arithmetic needs a fixture test with hand-computed values, citing the DICOM
section. R2 says why a generic "tests added" line is not enough: an agent asked
to test a function will assert what it does, not what it should do. Naming the
specification section is the difference.

**Tier coverage.** HLD section 7 has two capability tiers and deviation D-07
adds a third, C, for CPU. A feature that works on tier A and silently does
something different on another tier is the failure mode section 31 calls out:
a kernel with no fallback marks its feature unavailable, it never silently
produces a different answer. A story that touched rendering and does not say
which tiers it was exercised on has not answered the question, and "both" is
now an ambiguous answer because there are three.

## Entries

_None yet. The first will be F-001._
