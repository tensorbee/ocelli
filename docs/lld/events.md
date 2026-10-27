# Events

**F-IDs that contributed:** F-032, F-034, F-101, F-103
**Last updated:** 2026-10-27

The shell's names for what the ring delivers and for what stays in the shell.
Decision D4. The catalogue is `packages/core/src/events.ts`. This file says
what that table means. It is not a second table.

## Delivery

`ring` crosses the boundary. The producer emits it and `decodeNamedEvent`
reads the copied payload. `shell` is a DOM or tool event. It never crosses.
`unavailable` is a cornerstone3D 5.8.2 event Ocelli does not deliver. The
reason says whether the observation is already state, or whether no surface
exists yet.

## Count

The installed `@cornerstonejs/core` and `@cornerstonejs/tools` 5.8.2 enums
hold 50 and 53 events. Appendix B says 50 and 53. An earlier reading of the
F-103 plan said 53 core events. The files do not. The difference from
Appendix B is zero, and the miscount is recorded because a parity note that
hides it would claim a measurement nobody can repeat.

## Ring kinds

The numbers are `RING_EVENT_KIND` in the generated module.

| Kind | Name | Payload |
|------|------|---------|
| 1 | `cache.evicted` | series `u64`, instance `u32`, frame `u32`, bytes `u64` |
| 2 | `cache.displaced` | the same key |
| 3 | `cache.refused` | the same key |
| 4 | `cache.pressure` | tier `u8`, used `u64`, budget `u64`, entries `u32`, evicted `u32`, refused `u32` |
| 5 | `cache.budgetChanged` | tier `u8`, budget `u64`, evicted bytes `u64`, evicted `u32`, dropped `u32` |
| 6 | `frame.presented` | generation `u32`, drawn width `u32`, drawn height `u32`, flags `u32` |

Offsets are little-endian and come from `CacheEvent::encode`,
`TelemetryEvent::encode` and `push_presented`. Pressure and budget leave
three bytes after the tier. A short payload or a tier outside 0, 1 and 2
decodes as null. Kind 0 and any kind above 6 do too.

`cache.displaced`, `cache.refused` and `cache.budgetChanged` have no 5.8.2
name. They are `UNMAPPED_EVENTS`. `viewport.resized` is the shell
ResizeObserver and is unmapped for the same reason. Pairing one of them with
a different cornerstone event would name the wrong observation.

`IMAGE_RENDERED` is `frame.presented`. `IMAGE_CACHE_IMAGE_REMOVED` is
`cache.evicted`. `CACHE_SIZE_EXCEEDED` is `cache.pressure`. Volume-cache
removal is not frame-cache eviction.

## What stays state

Camera, window, display area, colormap, scroll and the selected slice are
viewport state or the F-050 snapshot. They are unavailable rows, not a second
event channel.

## Check

`scripts/event_catalogue_check.py` reads the committed rows. With
`--require-cornerstone` it compares them to the two installed `Events.js`
files and their digests. The `oracle` gate runs that form. CI has no oracle
install, so the floor probe only deletes one committed row and requires the
count refusal.
