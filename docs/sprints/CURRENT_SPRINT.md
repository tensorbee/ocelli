# Current sprint, S46

**Milestone**: M14, out-of-core volume streaming.
**Branch**: `sprint/s46`
**Opened**: 2026-11-05
**Goal**: Brick decomposition and multiscale pyramid generation.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-119 | E30.1 | Brick decomposition and multiscale pyramid generation | Rust | 10w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority.

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S46 / {print $2, $9}'
```

## What this sprint is

S45 is closed at tag `s45`. That close completed M20 and published nothing.
This sprint starts M14. F-119 decomposes a volume into bricks and generates
the multiscale pyramid. F-058 is done, so the volume model already has the
level axis this story fills. Closing this sprint does not publish.

## What is carried in

- **D-22 still blocks publication.** The version stays 0.1.0.
- **One cache.** A brick is a key in `ocelli-cache`. There is no second cache.
- **Level 0 is full resolution.** A series that fits in memory still has the level axis.
- **F-202 to F-205 stay unscheduled.** This sprint does not build them.

## The defect class this sprint is exposed to

A child level that is not a downsample of level 0. Spacing on that level copied from slice thickness instead of the dimension ratio. A missing level that returns level 0. A second brick edge beside `BrickGrid::new`.

## What done means

F-119 generates the coarser levels by downsampling level 0. `append_level` stores caller-supplied bytes and does not downsample, and this story is that downsample. Level 0 stays the full grid. Child spacing is level 0 spacing times the level 0 dimension divided by the child dimension, on each axis. The brick edge stays the one `BrickGrid::new` already chooses. Opacity correction stays F-121.
