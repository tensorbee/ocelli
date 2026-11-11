# F-203 showcase schedule review, pass 1

Reviewed tree `803a2536aae40cec92c221ebf924e9c919ca708e` against `4a3c0132b8040f5393b038366b7202404d5a2bd9` in the isolated showcase plan worktree. Author: the scheduling agent, using the lead's earlier More capability audit as input. Reviewer: Codex participant glm_codec_merge, independent of these allocation and draft-plan changes. This report covers scheduling and intake scope, not product implementation or feature completion.

## Result

4 defects, 1 smell. Scheduling is concrete, but the source needs the repairs below before import. Plans remain draft for the normal implementation-design round. No numerical rule or tolerance is approved by this review.

## Defects

### D1. The dependency-warning table was overwritten

BACKLOG's known-defect rows now contain `S76` for F-145 and `S68` for F-152 in the Defect column. Their actual warnings were deleted. F-145 still names nonexistent E4.9 and F-152 still depends on unscheduled Phase 2 work. Moving their sprint does not resolve either dependency. Restore the baseline warnings while retaining the new sprint on each actual story row.

### D2. Regeneration deleted unrelated historical intake

The final `## S16 audit intake, 2026-10-18` section is absent. It contains historical repair dispositions and independent review references outside the replan. The sprint-plan documentation explicitly describes all non-generated prose as hand-curated. Restore the complete baseline tail byte-for-byte. A green generated-table check does not establish preservation of that tail. Root separately observed this loss while I reviewed the allocation.

### D3. F-245's acceptance depends on its own dependent

F-202 depends on F-245. F-245 AC9 requires literal remote frame pixels, but the audited browser worker still lacks the real WADO decode/render that F-202 implements. The plans' common text allows a dependency's source review to precede its consumer, whereas `/start-feature` requires that dependency to be done. A partial reviewed checkpoint cannot satisfy that precondition.

Keep the existing directed graph and S47 allocation. F-245 must finish the canonical shell, source chooser, real QIDO/WADO selection and byte fetch with current identity, preserving the existing native image path. It must explicitly describe the remote viewer as partial until F-202. F-202 then supplies real decoded remote frame pixels through that same viewer. Put literal remote pixel acceptance in F-202, and require dependency completion in all 24 draft plans and the roadmap. The operator's final one-viewer end-to-end outcome remains required. The lead approved this bounded split before remediation.

### D4. The counting explanation still calls scheduled work unscheduled

BACKLOG's updated counting paragraph assigns F-202 to F-205 to M21 but immediately says the one-liner counts it with other unscheduled rows. Remove that stale singular sentence and state that Phase 2 and Phase 3 remain unscheduled.

## Smell

### S1. M20 no longer explains where its later rows appear

The hand-curated cross-reference paragraph was deleted. Its old F-120/F-121/F-122 peers became stale after their shift, so restore an updated explanation. M20's summary itself is correct: 38 stories, 135 estimated engineer-weeks, with 2 in S45, 31 in S46, 4 in S47 and 1 in S48. Its S46 rows appear under M14's first sprint, and its S47/S48 exception rows now appear under M21. Root also identified the removed paragraph during review.

## Checked clean within scope

- 267 original allocation rows become 287 unique rows. F-245 to F-264 are exactly the 20 new IDs. No row is removed. F-202 to F-205 retain their identities and receive their explicit narrower scopes.
- Exactly 38 existing scheduled stories move four sprints. Their estimates and dependencies are unchanged. Their notes additionally require the future one-viewer workflow. F-235, F-237, F-238, F-241 and F-242 retain their prior allocations, and all 225 other unaffected existing rows are unchanged.
- All 24 draft headers match their allocated sprint. Their explicit dependency graph has no cycle, missing ID or later-sprint prerequisite. The hidden D3 acceptance cycle was found separately rather than inferred absent from that graph check.
- Wave totals including the unchanged exceptions are S47 8 stories/32 estimated engineer-weeks, S48 7/30, S49 7/31 and S50 7/30. M21 itself has 24 stories/98 weeks. These are explicitly provisional engineering estimates, not observed capacity or dates.
- Read all 24 scopes, provider maps, tier declarations and all 194 acceptance rows. Each requires the same canonical viewer and a real user workflow, rejects detached demos and label-only controls, and preserves existing S46 behaviour. F-202 worker decode, F-203 MPR, F-204 one-to-one, F-205 preferences and the 20 remaining slices cover the audit's named gaps. F-262 supplies the final inventory gate, including the unchanged video stories.
- GSPS read/apply is explicitly new planned work, with the supported subset deferred to numerical design. Full write remains F-139 in S60. JPEG XL .111 safe reconstruction is explicitly planned rather than called existing support. Browser conformance is distinguished from the existing native report. CPU volume/slab/fusion limits remain explicit. The draft plans do not assert that a library API alone closes viewer acceptance.
- Existing provider paths named by the MPR, slab, fusion, interpolation, overlay, metadata and histogram drafts exist at the frozen source. Detailed command/API and normative numerical design remain open by design, before implementation.

## Executed evidence

`python3 scripts/gen_sprint_plan.py --check` and `python3 scripts/backlog_check.py` both returned 0. The independent allocation checker returned 0 and wrote `allocation-check.json` in the local review evidence directory. It checks row uniqueness, exact shifts, exceptions, headers, dependency order and cycles. These passing checks do not hide D1/D2/D3/D4 or S1.

The full source diff and the hand-curated boundaries were read independently. No browser, GPU, implementation tests, measured performance, publication or completion was claimed. The original tree is immutable. The same reviewer may author the approved repairs next, but those repairs need root's independent review and cannot be called independently clean by this report.
