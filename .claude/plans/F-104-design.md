# F-104, TypeScript type generation from Rust definitions

**Status**: approved
**Epic ref**: E16.5
**Sprint**: S17
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/14-the-boundary-in-code.md`, section 17.1, gives the control channel:

```rust
// wire format, little-endian
// [u16 kind][u16 viewport_id][u32 payload_len][payload ...] repeated
// ocelli-wasm/src/lib.rs
#[wasm_bindgen]
impl Session {
    /// Apply a packed command buffer. Returns the number of commands applied.
    #[wasm_bindgen(js_name = applyCommands)]
    pub fn apply_commands(&mut self, ptr: *const u8, len: usize) -> u32 {
        let buf = unsafe { core::slice::from_raw_parts(ptr, len) };
        self.inner.apply(buf)
    }
}
```

`docs/hld/14-the-boundary-in-code.md`, section 17.3, gives the event:

```rust
#[repr(C)]
pub struct Event {
    pub kind: u32,
    pub viewport: u32,
    pub seq: u64,
    pub payload: [u8; 32],
}
// 48-byte stride. JS reads with a DataView at header_size + i * 48.
```

`docs/hld/14-the-boundary-in-code.md`, section 17.4, gives the readback:

```rust
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ViewportState {
    pub camera: [f32; 16],
    pub voi_center: f32,
    pub voi_width: f32,
    pub slice_index: u32,
    pub num_slices: u32,
    pub flags: u32, // bit 0 invert, bit 1 loading, bit 2 error
    pub _pad: [u32; 3],
}
```

`docs/hld/04-boundary-and-data-path.md`, section 5.1, gives the command
granularity:

```text
Small, infrequent, one call per user intent: set camera, set VOI, activate tool, load series. **Never one call per pointer move.** Pointer streams are normalised in TypeScript and batched into a single packed command buffer per animation frame.
```

`docs/hld/12-workspace-and-build.md`, section 15.1, gives the package layout:

```text
> packages/
>
> core/ \# @ocelli/core (TypeScript shell)
>
> react/ \# @ocelli/react
```

`docs/hld/03-architecture-and-crates.md`, section 4, gives the crate that owns
the boundary:

```text
| ocelli-wasm | The only crate that may import wasm-bindgen. Boundary, commands, event ring. | yes | no |
```

`docs/hld/11-decision-log.md` decision D1, quoted in `CLAUDE.md`:

```text
| D1 | Feature parity with a NEW API. Not a drop-in shim. |
```

## What the specification does not cover

The HLD gives the wire layouts as Rust listings and the TS side as prose. It
does not say how the TypeScript declarations are produced, where the numeric
constants live, or what fails when a Rust boundary type changes and the TS side
does not.

Every item below is this plan's decision.

- **The generator.** A Rust test in `crates/ocelli-viewport/tests/typegen.rs`
  writes `packages/core/src/generated.ts` from the boundary tables and checks
  the committed file byte for byte. Regeneration is
  `UPDATE_TYPES=1 cargo test -p ocelli-viewport --test typegen`.
- **Why a Rust test and not `ts-rs`.** `ts-rs` derives a TypeScript shape from
  a Rust struct. The boundary is not a struct shape, it is a byte layout. The
  offsets, the command payload lengths, the enum bytes and the ring stride are
  numbers the shell needs, and a derive does not produce them. A Rust test can
  read `size_of` and `offset_of` on the real `#[repr(C)]` type, so the layout
  exists once and the numbers are derived rather than restated. No new
  dependency is added.
- **The declarative table.** The command ids, payload lengths and enum byte
  maps currently live as literals in `commands.rs` and as a hand-written mirror
  in `commands.ts`. F-104 moves them to a `boundary` module in
  `ocelli-viewport`, and both the Rust decoder and the generator read that
  table. One definition, two consumers.
- **The state struct.** HLD section 17.4's `ViewportState` does not exist as a
  Rust type today. `write_state` writes literal offsets. F-104 introduces the
  `#[repr(C)]` struct with `bytemuck::Pod`, makes `write_state` copy it, and
  generates the offsets from `offset_of!` and `size_of`. The struct is the
  layout, not a second description of it.
- **What is out of scope.** The error record and the panic record already have
  their own guards in `scripts/error_code_check.py` and the `panic` gate. F-104
  does not subsume them. It covers the command channel, the event ring, the
  state readback and the enums those carry.

## Problem and reproducer

The boundary's TypeScript declarations are hand-written mirrors of Rust
constants. `packages/core/src/commands.ts` declares `COMMAND_ID` as eleven
literals and `payloadOf` as eleven hand-written payload shapes.
`packages/core/src/viewport.ts` declares `VIEWPORT_STATE_BYTES = 96` and reads
offsets 0, 64, 68, 72, 76 and 80. `packages/core/src/ring.ts` declares
`EVENT_STRIDE = 48` and `HEADER_BYTES = 16`. `crates/ocelli-viewport/src/
commands.rs` declares the same numbers in `KIND_*` constants,
`payload_len_of` and `write_state`.

Nothing compares the two. The reproducer is a change to one Rust payload
length, for example `KIND_SET_WINDOW` from 17 to 21 bytes. The Rust decoder
accepts the new length. `commands.ts` still writes 17. Every `SetWindow`
command is refused with `BadLength`, and `npm run test` passes because the
TypeScript unit tests assert the TypeScript constant against itself. That is
the sprint's named defect, two sources of truth for the boundary types.

Source identity: `crates/ocelli-viewport/src/commands.rs`,
`packages/core/src/commands.ts`, `packages/core/src/viewport.ts` and
`packages/core/src/ring.ts`.

## Scope, API and compatibility

In scope:

- A `boundary` module in `ocelli-viewport` with declarative tables for command
  ids, payload lengths, enum byte maps, ring event kinds, state offsets and the
  snapshot header constants F-050 adds.
- A `#[repr(C)]` `ViewportState` matching HLD section 17.4, with `write_state`
  copying it.
- `crates/ocelli-viewport/tests/typegen.rs`, which emits and checks
  `packages/core/src/generated.ts`.
- Refactoring `viewport.ts`, `commands.ts` and `ring.ts` to import the
  generated constants and types instead of declaring them.
- Re-exporting the generated names from `packages/core/src/index.ts`.

Non-goals:

- No new Rust crate and no new npm package.
- No change to the wire format. This story makes the existing format one
  definition, not a new one.
- No generated React types. The React props are shell types, not boundary
  types.
- No generated error or panic records. Their guards stay where they are.
- No runtime code generation. The generated file is committed and checked.

Public generated surface, at `packages/core/src/generated.ts`:

```ts
// Generated by crates/ocelli-viewport/tests/typegen.rs. Do not edit.
// Regenerate with UPDATE_TYPES=1 cargo test -p ocelli-viewport --test typegen.

export const VIEWPORT_STATE_BYTES = 96;
export const STATE_CAMERA_OFFSET = 0;
export const STATE_VOI_CENTER_OFFSET = 64;
export const STATE_VOI_WIDTH_OFFSET = 68;
export const STATE_SLICE_INDEX_OFFSET = 72;
export const STATE_NUM_SLICES_OFFSET = 76;
export const STATE_FLAGS_OFFSET = 80;
export const STATE_PADDING_OFFSET = 84;

export const EVENT_HEADER_BYTES = 16;
export const EVENT_STRIDE = 48;
export const EVENT_PAYLOAD_BYTES = 32;

export const COMMAND_ID = { Fit: 1, Pan: 2, Zoom: 3, Rotate: 4,
  FlipHorizontal: 5, FlipVertical: 6, SetWindow: 7, ApplyPreset: 8,
  SetInvert: 9, SetColormap: 10, Scroll: 11 } as const;
export const COMMAND_PAYLOAD_BYTES = { Fit: 24, Pan: 16, Zoom: 8, Rotate: 8,
  FlipHorizontal: 1, FlipVertical: 1, SetWindow: 17, ApplyPreset: 1,
  SetInvert: 1, SetColormap: 1, Scroll: 4 } as const;

export type StackCommand = keyof typeof COMMAND_ID;

export const VOI_FUNCTION_BYTE = { linear: 0, linear_exact: 1,
  sigmoid: 2 } as const;
export const PRESET_BYTE = { soft_tissue: 0, lung: 1, bone: 2 } as const;
export const COLORMAP_BYTE = { grey: 0, stops: 1 } as const;
export const INVERT_BYTE = { identity: 0, inverse: 1 } as const;

export const RING_EVENT_KIND = { evicted: 1, displaced: 2, refused: 3,
  pressure: 4, budget_changed: 5, presented: 6 } as const;

export const FLAG_INVERT = 1;
export const FLAG_LOADING = 2;
export const FLAG_ERROR = 4;
export const FLAG_REFINING = 8;

export const SNAPSHOT_MAGIC = "OCS1";
export const SNAPSHOT_VERSION = 1;
export const SNAPSHOT_ERROR = { ok: 0, non_finite: 1, encode: 2, json: 3,
  bad_magic: 4, unknown_version: 5, bad_enum: 6, bad_lut: 7,
  chain_unavailable: 8 } as const;
```

The snapshot constants are added by F-050, which lands after this story and
regenerates the file. F-050's record is JSON, so there are no header offsets to
generate. The magic, the version and the error codes are the three constants
that keep the shell and the core from disagreeing.

Compatibility: every name `@ocelli/core` exports today stays exported with the
same type and value. `COMMAND_ID`, `EVENT_STRIDE`, `HEADER_BYTES`,
`VIEWPORT_STATE_BYTES`, `FLAG_*` and `STACK_COMMANDS` are re-exported from the
generated module or derived from it. A consumer that imported them sees the
same values.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `packCommands` in `commands.ts` | `COMMAND_ID`, `COMMAND_PAYLOAD_BYTES` | Shell packs one command buffer per frame | A payload whose length equals the generated table |
| `payloadOf` in `commands.ts` | `VOI_FUNCTION_BYTE`, `PRESET_BYTE`, `INVERT_BYTE`, `COLORMAP_BYTE` | Shell maps a public name to a wire byte | The byte the Rust decoder expects |
| `readViewportState` in `viewport.ts` | `STATE_*_OFFSET`, `VIEWPORT_STATE_BYTES` | Shell reads a copied 96-byte block | The same values the Rust writer placed |
| `drainRing` in `ring.ts` | `EVENT_HEADER_BYTES`, `EVENT_STRIDE` | Shell drains a copied ring once per frame | Events at the stride the producer wrote |
| `F-050` snapshot reader | `SNAPSHOT_*` | Shell refuses an unknown version | A header read at the generated offsets |
| `F-103` event catalogue | `RING_EVENT_KIND` | Shell names the events the ring delivers | One catalogue, not two |
| `ocelli-viewport`'s decoder | The `boundary` table | Rust validates and applies | The same ids, lengths and bytes the shell writes |

Unchanged consumers affected by changed providers: `write_state` keeps its 96
bytes and its zero window for a VOI sequence. `apply_commands` keeps its
refusals. The ring keeps its 48-byte stride.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| A committed generated file | `cargo test -p ocelli-viewport --test typegen` | The test compares the emitted text with the committed text | A mismatch fails the test and names the first differing line |
| A stale generated file | `UPDATE_TYPES=1 cargo test ...` | The file is rewritten and the test passes | A write failure fails the test |
| A Rust boundary constant | A change with no regeneration | The `test` gate fails | The build is not green and the change cannot land |
| A TS module | A hand-written duplicate of a generated constant | The generated-file check and the `tsc --build` gate both see one definition | A duplicate is removed by the refactor, and a review item checks no literal remains |

Invariants that span consumers:

- **One definition.** Every boundary number is declared once in Rust and
  appears in TypeScript only in `generated.ts`. The hand-written modules import
  it.
- **The real layout.** `VIEWPORT_STATE_BYTES` and the offsets come from
  `size_of::<ViewportState>()` and `offset_of!`, not from literals.
- **The real decoder.** The command ids and payload lengths come from the same
  table `payload_len_of` reads, so a change moves both sides together.
- **No runtime generation.** The generated file is committed, reviewed and
  checked. A clean clone needs no codegen step to type-check.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| `packages/core/src/generated.ts` | `tests/typegen.rs` | A tracked file, no ticket | The `UPDATE_TYPES=1` run that rewrites it | A refused write leaves the old file and fails the test |
| The committed file | `cargo test` | The tree hash | A passing comparison | A mismatch is a failure, never a skip |
| `ViewportState` | `write_state` | The `StackViewport` | The 96-byte copy | A buffer that is not 96 bytes returns `StateLength` |

No side effect precedes the commit point. The generator writes to a temporary
path and renames it, so an interrupted run cannot leave a half-written tracked
file. An identical repeated run is a no-op when the file already matches.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | `packages/core/src/generated.ts` exists, carries the generator header and the "do not edit" line, and is imported by `viewport.ts`, `commands.ts` and `ring.ts` | `cargo test -p ocelli-viewport --test typegen` and `npx tsc --build` | A hand-written file presented as generated | implementation |
| AC2 | Changing one Rust payload length without regenerating fails the `test` gate and names the differing line. Regenerating makes it pass | Mutate `SetWindow` from 17 to 21, run the test, confirm red, regenerate, confirm green, revert | A comparison that reads the generated file and restates it | implementation |
| AC3 | Every command id and payload length in `COMMAND_ID` and `COMMAND_PAYLOAD_BYTES` equals the Rust table | `crates/ocelli-viewport/tests/typegen.rs` reads the table, and `packages/core/src/commands.test.ts` packs every kind and asserts the payload length | A TS table that agrees with itself and not with Rust | implementation |
| AC4 | Every ring event kind in `RING_EVENT_KIND` equals the Rust `KIND_` constants in `ocelli-cache` and `ocelli-viewport` | The generator reads the constants, and a Rust test asserts the producer emits each kind | A TS kind that names an event the ring never produces | implementation |
| AC5 | Every enum byte map equals the Rust decoder's accepted bytes, and a byte outside the map is refused by both sides | The generator reads the tables, and the boundary test sends each valid byte and one invalid byte | A map that shifts one enum value, which renders a different presentation | implementation |
| AC6 | `VIEWPORT_STATE_BYTES` is `size_of::<ViewportState>()` and every `STATE_*_OFFSET` is `offset_of!` on that struct | The generator reads the struct, and a Rust test asserts the size and offsets against HLD section 17.4 | A literal 96 and literal offsets that drift from the struct | implementation |
| AC7 | The built wasm module accepts one command of every kind packed by `packCommands`, and returns the count of applied commands | `packages/core/src/boundary.wasm.test.ts`, after `wasm-pack build crates/ocelli-wasm --dev --target web` | A TS payload shape that the Rust decoder refuses with `BadLength` or `BadEnum` | integrated, needs the built module |
| AC8 | `readViewportState` on a 96-byte block written by the built module returns the same camera, centre, width, slice, count and flags | The same boundary test, reading `copy_state()` | A shell offset that reads the wrong field | integrated, needs the built module |
| AC9 | `drainRing` on a copied ring written by the built module returns the presented event at the generated stride and the generated kind | The same boundary test, after a `present` call | A stride or header size that reads a neighbour event | integrated, needs the built module |
| AC10 | `npm run lint` and `npm run typecheck` pass, and the `types` gate passes | `bin/ocelli.sh gate types` and `bin/ocelli.sh gate lint` | A generated file that does not compile, or a hand-written module that still declares a duplicate | implementation |
| AC11 | No hand-written module declares a literal that the generated module owns. The check is a review of `viewport.ts`, `commands.ts` and `ring.ts` plus a test that the generated exports are the only source of those names | The refactor and `packages/core/src/index.test.ts` | Two sources of truth, the sprint's named defect | implementation |
| AC12 | Tier rows: code generation is tier-independent and performs no rendering | This section | A generator that needs a device or a browser | implementation |

## Approach

The smallest implementation in terms of the transcribed source.

1. **`crates/ocelli-viewport/src/boundary.rs`**, a new module. It holds:
   - `COMMAND_TABLE: [CommandSpec; 11]`, where `CommandSpec` has `kind: u16`,
     `name: &'static str` and `payload_bytes: u32`.
   - `VOI_FUNCTION_BYTES`, `PRESET_BYTES`, `COLORMAP_BYTES`, `INVERT_BYTES`,
     each a const array of `(name, byte)`.
   - `STATE_FLAG_*` constants.
   - `ViewportState`, `#[repr(C)]`, `#[derive(Clone, Copy, bytemuck::Pod,
     bytemuck::Zeroable)]`, matching HLD section 17.4.
   - `EVENT_HEADER_BYTES`, `EVENT_STRIDE`, `EVENT_PAYLOAD_BYTES`.
   - The snapshot constants F-050 adds, declared here so the generator emits
     them.

   `payload_len_of` becomes a lookup in `COMMAND_TABLE`, and `write_state`
   copies `ViewportState` through `bytemuck::bytes_of`.

2. **`crates/ocelli-viewport/Cargo.toml`.** Add `bytemuck` from the workspace.
   It is already a workspace dependency in HLD section 15.2 and is used by
   `ocelli-core`'s record type, so this is not a new dependency.

3. **`crates/ocelli-viewport/tests/typegen.rs`.** A test that builds the TS
   text in a `String` from the tables and the struct's `size_of` and
   `offset_of!`, then either writes the file when `UPDATE_TYPES=1` is set or
   compares it with the committed bytes. It writes to a temporary path and
   renames. It refuses to run when the workspace root cannot be found, rather
   than skipping.

4. **`packages/core/src/generated.ts`.** The committed output.

5. **`packages/core/src/viewport.ts`, `commands.ts`, `ring.ts`.** Import the
   generated constants and types. `STACK_COMMANDS` becomes
   `Object.keys(COMMAND_ID)` in declaration order, or a generated tuple. The
   `PackedCommand` union keeps its hand-written payload shape, because that is
   the caller's TypeScript type, not the wire layout. `payloadOf`'s lengths
   come from `COMMAND_PAYLOAD_BYTES`.

6. **`packages/core/src/index.ts`.** Re-export the generated names and keep
   every existing export.

Rejected alternatives:

- **`ts-rs`.** It generates a struct shape and not the byte layout, so it
  cannot produce the offsets, payload lengths, enum bytes or ring stride. It
  also adds a derive dependency to the boundary crate.
- **A Python script that parses Rust source.** The parser would have to model
  const arrays, `offset_of!` and enum discriminants. A Rust test reads them
  from the compiler instead.
- **A new `tools/typegen` crate.** It would sit outside the workspace and need
  its own lockfile and dependency graph, which is more places to look for one
  generated file.
- **Generating at build time from `build.rs`.** A `build.rs` that writes into
  `packages/` makes the TypeScript tree a build artefact and hides the drift
  from review. The committed file is reviewable.

## Dependencies, write ownership and resources

Declared dependencies: F-101, done. Source-level dependencies: none. F-050,
F-102 and F-103 consume the generated module and land after this story.

Files that cannot have concurrent writers:

- `crates/ocelli-viewport/src/{boundary.rs,commands.rs}`
- `crates/ocelli-viewport/Cargo.toml`
- `crates/ocelli-viewport/tests/typegen.rs`
- `packages/core/src/{generated.ts,viewport.ts,commands.ts,ring.ts,index.ts}`
- `docs/lld/{public-api.md,typescript-packaging.md}`

No GPU, no browser, no corpus. The boundary test needs a dev wasm build, and it
is serial by nature.

## Verification and evidence reuse

Focused checks:

```bash
bin/ocelli.sh check ocelli-viewport
bin/ocelli.sh test ocelli-viewport
UPDATE_TYPES=1 cargo test -p ocelli-viewport --test typegen
bin/ocelli.sh clippy
bin/ocelli.sh fmt
npm run lint
npm run typecheck
npx vitest run packages/core/src/commands.test.ts packages/core/src/viewport.test.ts packages/core/src/ring.test.ts
wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm
npx vitest run --config vitest.boundary.config.ts
```

Final profile: `bin/ocelli.sh gate --sprint` over the staged union, with the
corpus result recorded. The boundary suite is an explicit required suite for
the changed boundary path, per `.claude/WORKFLOW.md`.

Evidence identities: the generator test names the Rust table and the committed
file. The boundary test names the built module path and the export names. The
mutation in AC2 is the evidence that the check would fail if the code were
wrong, which HLD section 27.3 asks a human to confirm.

What invalidates evidence: any edit to a Rust boundary constant, the
`ViewportState` struct, the generated file or the hand-written modules after
the verification. The generated file is part of the checked tree.

## Boundary and tier

- wasm-bindgen: not touched. The generator is a native test and the generated
  file is TypeScript.
- Pixels across the boundary: no. The generated constants describe state and
  command bytes.
- Render-loop allocation: none. Generation happens at test time.
- unsafe: none.
- Tier A (WebGPU): n/a, code generation.
- Tier B (WebGL2): n/a.
- Tier C (CPU): n/a.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `unit` | The generator's table matches the decoder's table, and the state struct's size and offsets match HLD section 17.4 | `crates/ocelli-viewport/tests/typegen.rs` and `src/boundary.rs` under `#[cfg(test)]` |
| `unit` | `packCommands` writes the generated payload length for every kind, and `readViewportState` reads the generated offsets | `packages/core/src/commands.test.ts` and `viewport.test.ts` |
| `property` | For every command kind, the packed bytes decode to the same command the Rust table names, over a generated family of values | `crates/ocelli-viewport/tests/typegen.rs` and `packages/core/src/commands.test.ts` |
| `browser` | The built wasm module accepts every packed kind, writes the 96-byte state at the generated offsets and drains the ring at the generated stride | `packages/core/src/boundary.wasm.test.ts` |

`fixture` is not applicable. This story moves constants, and its arithmetic is
byte offsets rather than pixel or geometry values. The boundary test is the
independent observation that the two languages agree.

## Parity surface covered

None. Appendix B counts capabilities, and type generation is a build property
of the boundary rather than a cornerstone3D surface. The row it enables is
F-103's event catalogue and F-050's state record.

## Deviations

None.

## LLD impact

- `docs/lld/public-api.md`: the generated module and the constants it owns.
- `docs/lld/typescript-packaging.md`: `generated.ts` in the tarball and the
  drift check.

## Open questions

**Q1, answered 2026-10-27. The generation mechanism.** The operator chose the
Rust test generator over `ts-rs` and a separate generator crate. The plan
applies that answer.

**Q2, decided locally. The generated file's home.** `packages/core/src/
generated.ts`, committed and imported by the hand-written modules. A separate
`@ocelli/boundary-types` package would add a workspace member, a tarball, a
version and an install for one file that `@ocelli/core` already owns. This is a
routine structural choice and does not need an operator decision.

**Q3, decided locally. The drift gate.** The drift check is a `#[test]` in the
existing `test` gate. `cargo test --workspace` already runs it, the floor
already includes `test`, and a new gate would need a CI step and a
`scripts/ci_floor_check.py` reconciliation for no additional coverage.
