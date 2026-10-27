# F-103 review, pass 2

**Reviewed**: uncommitted F-103 work on `sprint/s17` at parent `c73269404056e6e9c0616701c84d2c6dd5f24a97`, including the guard claim added after pass 1. The pass covered `packages/core/src/events.ts`, the payload decoders, `scripts/event_catalogue_check.py`, `scripts/guards/catalogue.py` and `docs/lld/events.md`.

**Author / reviewer**: The same session wrote the implementation and this pass. The operator directed that session to review and repair without waiting on DeepSeek. This is not an independent reviewer.

**Result**: 0 defects, 0 smells, 0 nitpicks.

## Acceptance challenged

The census claim. Pass 1 left `scripts/event_catalogue_check.py` printing `FAIL:` with no catalogue owner. `event-catalogue` now claims that site. `event-catalogue.short-map` deletes the `ERROR_EVENT` row and the check must name 102 rows. The unmutated control must exit 0. A green census over an unclaimed site is the failure this pass rejects.

AC5 and AC6 were re-run on the unchanged decoder and the unchanged mapping. The offset-1 mutation stays the pass 1 receipt. This pass did not repeat it, because the decoder bytes did not move.

AC8. The listener scan now also skips a `.test.tsx` file. Production sources under `packages/core/src` and `packages/react/src` still do not call `addEventListener`, and only `events.ts` declares `OCELLI_EVENTS`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `python3 scripts/guard_probe.py --only event-catalogue.short-map` | catalogue with the `ERROR_EVENT` row removed | executed now | exit 0, `red event-catalogue.short-map`, control green, 8.5s |
| `python3 scripts/guard_census.py` | the guard catalogue after the new entry | executed now | exit 0, 893 refusal(s), all claimed, 0 watched by nothing |
| `npx vitest run packages/core/src/events.test.ts` | `events.ts` and `events.test.ts` | executed now | 8 passed, exit 0 |
| `python3 scripts/event_catalogue_check.py --require-cornerstone` | pinned 5.8.2 `Events.js` | executed now | exit 0, 50 core and 53 tool |
| `python3 scripts/event_catalogue_check.py` | committed catalogue only | executed now | exit 0, 50 core and 53 tool |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The six decoders still read the encoder offsets. Cache key is series at 0, instance at 8, frame at 12 and bytes at 16. Pressure and budget read the tier at 0 and refuse a tier outside 0, 1 and 2. Presented reads four `u32` values from offset 0. Unknown kinds 0 and 7 return null. The three ring kinds with no 5.8.2 name stay on `UNMAPPED_EVENTS`. The installed-enum comparison stays on the oracle arm. The floor probe does not open `node_modules`.

## Limitations and handoff

The full `gate oracle` browser path was not run in this pass. AC6's python check did run. Feature verification is the next action. F-102 is not started.
