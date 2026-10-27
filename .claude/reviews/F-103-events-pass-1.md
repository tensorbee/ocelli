# F-103 review, pass 1

**Reviewed**: uncommitted F-103 work on `sprint/s17` at parent `c73269404056e6e9c0616701c84d2c6dd5f24a97`. The pass covered `packages/core/src/events.ts`, the payload decoders, the 5.8.2 mapping, `scripts/event_catalogue_check.py`, the oracle arm and `docs/lld/events.md`.

**Author / reviewer**: The same session wrote the implementation and this pass. The operator directed that session to review and repair without waiting on DeepSeek. This is not an independent reviewer. The discriminating check below was executed rather than inferred.

**Result**: reported 0 defects, 0 smells, 0 nitpicks. The floor that followed this pass falsified that result. `scripts/event_catalogue_check.py` prints `FAIL:` and no catalogue entry claimed the site. Pass 2 records the repair.

## Acceptance challenged

AC5. `readCacheKey` reads series at offset 0. Replacing that with offset 1 made `decodes_fixture_payloads` fail. The decoded series was `216172782113783808n` against the hand value `7n`. Restoring offset 0 made the same test pass.

AC1 and AC6. The installed 5.8.2 `Events.js` files hold 50 core keys and 53 tool keys. `python3 scripts/event_catalogue_check.py --require-cornerstone` printed `OK: catalogue matches 5.8.2 (50 core, 53 tool)` and exited 0. The approved draft said 53 core and 106 rows. The implementation follows the files. Appendix B's 50 plus 53 matches the files, so the difference is zero.

AC3. `cache.displaced`, `cache.refused` and `cache.budgetChanged` have no 5.8.2 name. They are `UNMAPPED_EVENTS` with delivery `ring`. They are not paired with a different cornerstone event. The six ring names still match `RING_EVENT_KIND`.

AC8. Production sources under `packages/core/src` and `packages/react/src` do not call `addEventListener`, and only `events.ts` declares `OCELLI_EVENTS`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npx vitest run packages/core/src/events.test.ts -t decodes_fixture_payloads` with series read at offset 1 | `events.ts` `readCacheKey` | executed now | 1 failed, series `216172782113783808n` against `7n` |
| Same command after restoring offset 0, full `events.test.ts` | restored `events.ts` | executed now | 8 passed, exit 0 |
| `python3 scripts/event_catalogue_check.py --require-cornerstone` | pinned 5.8.2 `Events.js` | executed now | exit 0, 50 core and 53 tool |
| `python3 scripts/prose_check.py` | plans, reviews, LLD and sprint prose | executed now | exit 0 |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The decoders, the mapping and the external check agree with the pinned enums and with the encoder comments. Feature verification is the next action.
