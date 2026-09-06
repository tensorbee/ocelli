# F-005, error model, panic-to-JS mapping, structured logging

**Status**: approved
**Epic ref**: E1.5
**Sprint**: S03
**Estimate**: 2w

## Normative source, transcribed

_Transcriptions below are verbatim except for one normalisation, the same one
the F-008 plan records: a prose semicolon in the source is written as a comma
and an em-dash as a hyphen, because `scripts/prose_check.py` covers
`.claude/plans/` and `docs/hld/` is exempt. No word is changed. Where the exact
bytes matter, the tracked Markdown under `docs/hld/` wins._

### `docs/hld/20-errors-and-panics.md`, section 23, in full

This is the whole section. It is short, and every clause in it is load-bearing
for this story, so it is quoted entire rather than sampled.

> thiserror in the core crates, the boundary maps everything to a stable
> numeric code and a message. The important part is what happens when that is
> not enough.

The callout box, verbatim:

> **A PANIC POISONS THE INSTANCE** Once a Rust panic aborts inside
> WebAssembly, that module instance's memory may be inconsistent and it must
> not be reused. So: panic = "abort" in release, no exported function may let a
> panic escape as a normal error path, and on panic the worker instance is torn
> down and rebuilt, with the JavaScript shell reconstructing viewport state
> from its own copy. That last clause is a design constraint on the shell, not
> an afterthought - the shell must always hold enough state to rebuild a
> viewport from nothing.

The three bullets, verbatim:

> - console_error_panic_hook in development builds only, it costs binary size
>   and leaks symbol names.
>
> - Error codes are stable and versioned. The shell switches on the code, the
>   message is for humans and may change.
>
> - A poisoned instance surfaces to the user as a viewport-level error state,
>   never as a silent blank canvas.

### `docs/hld/14-the-boundary-in-code.md`, section 17.3, the event ring, verbatim

```rust
// ocelli-wasm/src/ring.rs -- single producer (Rust), single consumer (JS)
#[repr(C)]
pub struct RingHeader {
    pub head: u32, // producer writes
    pub tail: u32, // consumer writes
    pub cap: u32, // power of two
    pub dropped: u32, // overflow count; nonzero means JS is not draining
}
#[repr(C)]
pub struct Event {
    pub kind: u32,
    pub viewport: u32,
    pub seq: u64,
    pub payload: [u8; 32],
}
// 48-byte stride. JS reads with a DataView at header_size + i * 48.
```

> *Fixed stride so the JavaScript side is arithmetic, not deserialisation.
> Drain once per animation frame, surface `dropped` in telemetry rather than
> swallowing it.*

### `docs/hld/14-the-boundary-in-code.md`, section 17.4, state readback, verbatim

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

> *Copied into a JS-owned buffer on request. Never returned as a JS object
> graph - that is one allocation and one conversion per field.*

**`flags` bit 2 is `error`, and section 23 says a poisoned instance surfaces as
a viewport-level error state. Those are the same bit.** That is the single most
useful thing section 17.4 contributes to this story.

### `docs/hld/14-the-boundary-in-code.md`, section 17.2, the trap, verbatim

> **NEVER CACHE THE VIEW** A module-level const HEAP = new
> Uint8Array(wasm.memory.buffer) is the classic failure. Any wasm memory growth
> relocates the ArrayBuffer and detaches every outstanding view, the next write
> silently targets a detached buffer or throws far from the cause. Add an
> ESLint rule banning new Uint8Array(wasm.memory.buffer) outside the two
> functions that are allowed to do it.

**"the two functions"**, plural, in a specification whose repository has one
today. That plural is what this plan spends below.

### `docs/hld/21-worker-protocol.md`, section 24, verbatim

```text
main -> decode : { kind:'decode', seriesId, frameIndex, buffer } [transfer]
decode -> render : { kind:'frame', frameId, buffer, meta } [transfer]
main -> render : { kind:'commands', buffer } [transfer]
render -> main : drained event ring [copy]
// Always transfer, never copy:
// worker.postMessage(msg, [msg.buffer]);
```

> Three roles: the main thread, N decode workers each with its own WebAssembly
> instance, and one render worker owning the GPUDevice and every
> OffscreenCanvas. Decode workers never touch the GPU, the render worker never
> decodes.

**There is no error arrow in section 24.** An error travelling upward from a
worker has exactly one specified vehicle, `render -> main : drained event ring
[copy]`, and a decode worker has no upward arrow at all beyond its frame
message. That gap is named in `## What the specification does not cover`.

### `docs/hld/19-render-graph.md`, section 22, the corroborating rule, verbatim

> - **Device loss is a real state, not an error path.** Handle device_lost,
>   rebuild the device and all resources, and restore viewport state from the
>   shell's copy - the same recovery path §23 needs for panics.

Section 22 names section 23's recovery path as one it shares. So the shell-side
rebuild this story constrains is not built for panics alone, and F-037
(`ocelli-render`: device init, capability tiering, device-lost recovery) is its
second consumer.

### `docs/hld/03-architecture-and-crates.md`, section 4, the row that assigns ownership, verbatim

> | ocelli-core | Types, coordinate spaces, geometry primitives, error model.
> No I/O. | yes | yes |

**The error model is `ocelli-core`'s, by the crate table.** Not
`ocelli-wasm`'s. `ocelli-wasm`'s row says "The only crate that may import
wasm-bindgen. Boundary, commands, event ring."

### `docs/hld/12-workspace-and-build.md`, section 15.2, the release profile, verbatim

```toml
[profile.release]
opt-level = "z"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

and the dependency entry this story has to touch:

> thiserror = "2"

### `docs/hld/24-agent-code-standards.md`, section 27.1, the denied lints, verbatim

> \[workspace.lints.rust\]
>
> unsafe_code = "deny" \# allow-listed per file, see 27.2
>
> \[workspace.lints.clippy\]
>
> cast_possible_truncation = "deny"
>
> cast_precision_loss = "deny"
>
> cast_sign_loss = "deny"
>
> float_cmp = "deny"
>
> indexing_slicing = "warn"

### `docs/hld/24-agent-code-standards.md`, section 27.2, the rules this story is judged by, verbatim

> | R2 | Tests derive from the spec or the oracle, never from reading the
> implementation | An agent asked to test a function will assert what it does,
> not what it should do |

> | R3 | Every function doing pixel arithmetic needs a fixture test with
> hand-computed values, citing the DICOM section | This is the defect class
> that reaches patients |

> | R5 | No unsafe outside the allow-list (ocelli-wasm/src/ring.rs,
> ocelli-core/src/cast.rs) | Keeps the audit surface to two files |

### `docs/hld/07-concurrency-and-typescript.md`, section 9, verbatim

> **Start single-threaded per worker.** N decode workers, each holding its own
> WebAssembly instance, one render worker owning the GPUDevice and every
> OffscreenCanvas, the main thread doing DOM, events and tool UI. No
> SharedArrayBuffer, no wasm-bindgen-rayon, no nightly toolchain.

This is decision D5, and it is what makes section 23's recovery affordable. One
instance per worker means tearing one down costs exactly one worker's state,
not a shared arena.

### `docs/sprints/allocation.json`, the F-005 note, verbatim

> A panic must never leave a viewport in an undefined state

### `docs/sprints/CURRENT_SPRINT.md`, what done means, verbatim

> - **F-005** maps a Rust panic to a JS error the shell can act on without
>   poisoning the wasm instance, per HLD section 23.

## The panic strategy today, measured rather than assumed

The story turns on what actually happens on a panic, so this was measured on
this machine with the pinned toolchain before any design sentence below it was
written.

```
$ rustc -V
rustc 1.97.1 (8bab26f4f 2026-07-14)

$ rustc --print cfg | grep panic
panic="unwind"

$ rustc --print cfg --target wasm32-unknown-unknown | grep panic
panic="abort"

$ rustc --print cfg --target wasm32-unknown-unknown -C panic=unwind | grep panic
panic="unwind"
```

Four facts follow, and three of them are stronger than the specification
assumes.

1. **`wasm32-unknown-unknown` is `panic = "abort"` in every profile, not only
   in release.** The target's own default is abort, and
   `[profile.release] panic = "abort"` in the workspace manifest agrees with a
   default that was already in force. Section 23 says `panic = "abort"` in
   release, and the shipping target exceeds that by never offering unwind at
   all unless somebody passes `-C panic=unwind` deliberately. **So there is no
   dev-versus-release split in panic behaviour on the target that ships**, and
   a design that recovers by unwinding cannot exist here even as a debug
   convenience.
2. **`catch_unwind` is not available on wasm32.** Under `panic = "abort"` it
   compiles and never catches. Any design in which the boundary catches a panic
   and converts it into a returned error code is therefore not merely forbidden
   by section 23, it does not work. The workspace already denies
   `unwrap_used`, `expect_used` and `panic` as clippy lints with a comment
   naming section 23 as the reason, which is the mechanism for keeping the
   panic path unreachable rather than recoverable.
3. **The host is `panic = "unwind"`.** `cargo test --workspace` runs the dev
   profile on the host, so `std::panic::catch_unwind` does work in native
   tests. That is what makes the panic hook's own logic testable without a
   browser, and it is the only reason this story has cheap unit coverage at
   all. The corollary is that a native test proves the hook's formatting and
   nothing about the wasm32 trap, which is why the probe in `## Tests` exists.
4. **A panic under abort still runs the hook.** `std::panicking` invokes the
   installed hook before handing control to the panic runtime, and the abort
   runtime is what runs afterwards. **This plan does not assert that as a
   fact about our artefact.** It is the property the wasm probe is built to
   establish, and if the probe shows otherwise, the design in `## Approach`
   fails at its first step and this plan comes back for redesign rather than
   the probe being softened.

**What this means for recovery.** Recovery is not resumption. There is no
in-instance recovery available and none is designed. The instance traps, the
shell reads a record out of linear memory without calling back into the
module, marks the viewport in error, discards the worker and builds a new one.
"Without poisoning the wasm instance" in the sprint's done-line therefore does
not mean "the instance survives a panic". It means **the mapping from a panic
to a JS error must not itself require touching a poisoned instance**, and the
design below is shaped entirely by that reading.

## What the specification does not cover

Section 23 is nine sentences. It fixes the policy and almost none of the
mechanism. Every decision below is this plan's rather than the HLD's.

1. **The wire shape of an error.** Section 23 says "a stable numeric code and a
   message" and does not say how wide the code is, how the code and message
   travel, or whether the message crosses the boundary at all. Section 17.3
   gives a 32-byte payload and section 17.4 gives an `error` flag bit, and
   nothing joins them up.
2. **Where the message lives.** "The message is for humans and may change"
   pushes hard against putting format strings in a module whose budget is
   measured in kilobytes, and section 23 does not draw the conclusion.
3. **How a code is kept stable and versioned.** The words appear, the mechanism
   does not. Nothing in the HLD stops the next story renumbering the space.
4. **How the shell learns of a panic.** The instance has trapped. Section 23
   says the shell tears it down and rebuilds. It does not say how the shell
   finds out **which** failure occurred, and the obvious route, calling an
   exported getter, is calling into an instance section 23 has just said must
   not be reused.
5. **What "structured logging" is.** The story title says it and the HLD has no
   logging section at all. `grep -rn -i "structured log\|logging\|tracing\|log
   level" docs/hld/*.md` returns nothing. The single adjacent sentence is
   17.3's "surface `dropped` in telemetry rather than swallowing it".
6. **How a decode worker reports anything.** Section 24 gives the render worker
   an upward arrow and gives a decode worker none.
7. **What the shell's own copy of viewport state is.** Section 23 makes holding
   it a design constraint on the shell and does not define it. Nothing in the
   repository defines a viewport yet, and F-100 designs the public API.
8. **Whether `console_error_panic_hook` means that crate or that behaviour.**

## Approach

### The shape, in one paragraph

The error model is a value type in `ocelli-core`, because section 4's crate
table says so. It is a `#[repr(u16)]` code plus a small `thiserror` enum with
no heap allocation and no `String`. Recoverable failures return `Err` and, once
the ring exists, ride it as a 32-byte record inside section 17.3's 48-byte
`Event`, which is decision D4 unchanged. Fatal failures panic, and a panic
never returns and never rides the ring, because the ring is in the memory the
panic just made suspect. Instead a panic hook installed at instance start
writes a fixed-size record into a statically allocated region of linear memory
whose address the shell learned at startup, and the shell reads that region
**after** the trap with a plain `DataView` and without calling a single export.
Then it marks the viewport in error and rebuilds the worker.

### A, the error model, `crates/ocelli-core/src/error.rs`

Three items, and nothing else.

```
ErrorCode   #[repr(u16)]   stable, versioned, registry-checked
Severity    #[repr(u8)]    0 = Recoverable, 1 = Fatal
Record                     the 32 bytes that fit section 17.3's payload
```

**`ErrorCode` carries only variants with a live producer today.** That is
three. `Panicked`, and the two `ComputeError` variants F-008 already wrote,
`Unavailable` and `Workgroup`. Adding a fourth is the job of the story whose
code it is, and the registry in D below is what makes that an added line rather
than a renumbering. Reserving a space of plausible future codes would be
inventing producers, which is the failure mode `AGENTS.md`'s structural rules
exist to stop.

The reserved ranges, however, are declared now in the registry file, because a
range is cheap to declare and expensive to retrofit once two crates have picked
overlapping numbers. `0` is never a valid code, so a zeroed buffer cannot
decode as a real one.

**`Record` is one layout for both errors and log lines.** The discriminant that
separates them is `Event.kind` in section 17.3's header, so it is not repeated
inside the payload. One layout, two kinds, one encoder and one decoder on each
side of the boundary. `AGENTS.md`'s structural test asks whether a construct
reduces the cases a reader considers, and two byte layouts for two things that
differ only in a header field increases them.

```
offset  size  field
0       2     code            u16, little-endian, never 0
2       1     severity_or_level u8
3       1     arity           u8, how many of a0..a2 are meaningful
4       4     reserved        u32, producer writes 0
8       8     a0              u64
16      8     a1              u64
24      8     a2              u64
                              total 32
```

Two things about this layout are deliberate and worth a reviewer's attention.

- **The `u32` at offset 4 is padding with a name.** Without it the three `u64`
  operands would sit at payload offsets 4, 12 and 20. Section 17.3's `Event` is
  `#[repr(C)]` with fields `u32, u32, u64, [u8; 32]`, which puts `payload` at
  struct offset 16 and gives a total size of 48, matching the stated stride.
  Payload offset 8 is therefore struct offset 24, and every operand is
  eight-byte aligned in linear memory. `DataView.getBigUint64` does not care,
  and the Rust side reading through `from_le_bytes` on a `[u8; 32]` does not
  either, so this is not correctness. It is that an unaligned `u64` in a
  `#[repr(C)]` boundary struct is the kind of thing a later story adds a
  `bytemuck::Pod` derive to and discovers at the worst moment.
- **`reserved` is not asserted zero by the consumer.** The producer writes
  zero. A consumer that refuses a nonzero value cannot read a record written by
  a newer core, and a consumer that ignores it silently loses the same
  information twice. It is surfaced, exactly as 17.3 says `dropped` is
  surfaced: reported, not swallowed, not fatal.

**No allocation anywhere in this module.** No `String`, no `alloc`. Operands
are `u64` and the human text is not here at all, which is item B.

### B, the message, and why it is TypeScript

Section 23: "The shell switches on the code, the message is for humans and may
change." Both halves argue the same way.

The human-readable text lives in `packages/core/src/errors.ts` as a `code ->
string` table. The Rust side carries `thiserror`'s `#[error("...")]` strings
for `Display` in native builds, tests and the desktop and server entry points,
where the text costs nothing that matters. It does not send them across the
boundary.

Three reasons, in order of weight.

1. **The wasm module is measured.** `ci/wasm-size-budget.json` records 14,104
   bytes with a five per cent tolerance, and section 15.2's profile is
   `opt-level = "z"` with `strip = true`. Format strings and the `core::fmt`
   machinery to assemble them are exactly the weight that budget exists to
   notice. Section 23's own first bullet makes the same argument about the
   panic hook.
2. **"May change" means the message is not a contract.** A message that crosses
   the boundary becomes one the moment a consumer matches on it.
3. **It makes the code stable by construction.** A shell that has a table
   keyed on the code cannot render an error it does not know about without
   noticing, which is the third item in D.

**The panic message is the exception, and it is the only one.** A panic's text
is not from a table, because the whole value of it is that it names an
unanticipated failure with a file and a line. It is written into the panic
record in item C.

### C, the panic path, `crates/ocelli-wasm/src/panic.rs`

This is the story's crux and the part the sprint's done-line names.

**The record lives in statics, in linear memory, at a fixed address.**

```
static PANIC_MAGIC:   AtomicU32          // written last, 0 until a panic
static PANIC_VERSION: AtomicU32
static PANIC_CODE:    AtomicU32
static PANIC_MSG_LEN: AtomicU32
static PANIC_MSG:     [AtomicU8; 512]
```

**Atomics, and that is what makes this need no `unsafe`.** A `static` written
through `AtomicU8::store` needs only a shared reference, so the hook mutates a
static without `unsafe`, without a `Mutex` that could deadlock inside a panic,
and without `UnsafeCell`. `crates/ocelli-wasm/src/ring.rs` is not created by
this story, and `unsafe` stays at zero files touched. That is worth stating
plainly, because the obvious implementation of this record is a `static mut`
and it would have to be refused.

The hook, installed by an exported `install_panic_hook()` called once by the
worker before any other call:

1. Formats `PanicHookInfo`'s payload and `location()` into a stack-local
   `[u8; 512]` through a `core::fmt::Write` implementation that **truncates and
   returns `Ok`**. A writer that returns `Err` on overflow makes `write!`
   return an error the hook then has to handle, and the only honest handling
   inside a panic hook is another panic. Truncation is recorded in `msg_len`.
2. Stores the bytes into `PANIC_MSG`, then `PANIC_MSG_LEN`, then `PANIC_CODE`,
   then `PANIC_VERSION`, and **`PANIC_MAGIC` last**. The order is the check:
   the shell treats a record with the wrong magic as absent, so a torn record
   from a panic inside the hook reads as no record rather than as a valid one
   with garbage in it.
3. Under `#[cfg(debug_assertions)]` only, additionally echoes the message to
   `console.error`. See item G.
4. Returns. The abort follows, the module traps, and the JS call that entered
   the module throws.

**Two exports, and they are called before anything can panic.**

```
panic_record_ptr() -> u32
panic_record_len() -> u32
```

The shell calls both immediately after instantiating the module and caches the
two integers. It never calls them again. **This is the whole reason the design
works**: after the trap the shell needs no call into the instance at all, which
is section 23's "must not be reused" honoured literally rather than
approximately. An export that returned the message after the trap would be
reusing a poisoned instance to ask why it was poisoned.

`panic_record_ptr` returns `u32::try_from(PANIC_MAGIC.as_ptr().addr())`,
not an `as` cast. `<*const T>::addr()` is stable and gives the address without
a pointer-to-integer cast, and `try_from` gives the narrowing without
`cast_possible_truncation`. The workspace denies that lint, and section 27.3
makes every cast a human review item, so a design that needs none here is worth
the two extra calls.

**A cached pointer is safe and a cached view is not**, and the distinction is
the one 17.2 is about. WebAssembly memory growth relocates the JavaScript
`ArrayBuffer` and detaches every view over it. It does not move the linear
address of a static. So the cached `u32` stays correct for the life of the
instance, and the `DataView` over it must be built fresh at every read. Item E
is that rule as code.

### D, stable and versioned, made mechanical

`ci/error-codes.json` holds the registry, in the same spirit as
`ci/wasm-size-budget.json`: a tracked file that a script compares reality
against.

`scripts/error_code_check.py`, wired as a new no-GPU gate `errors` in
`bin/ocelli.sh` and into the CI floor, asserts four things.

| Assertion | The defect it catches |
|-----------|----------------------|
| Every `ErrorCode` variant in `crates/ocelli-core/src/error.rs` appears in the registry with the same number | A renumbering, which section 23 forbids and nothing currently prevents |
| Every registry entry has exactly one entry in `packages/core/src/errors.ts` | A code the shell cannot describe, which reaches a user as a number |
| No number is reused and no name is reused | The worst version of the above, where an old shell decodes a new code as something plausible |
| Every entry declares the crate range it falls in | A second crate quietly picking an overlapping block |

**The registry is append-only in practice and the script does not enforce
that**, deliberately. A removed code is a real event, and turning it into a
gate failure produces a gate somebody disables. What the script refuses is
reuse of the number, which is the part that is silent.

This gate is a natural companion to F-X009 in this sprint. See
`## Coordination with the rest of S03`.

### E, the shell side

Three new modules in `packages/core/src`, all exported from `index.ts`.

**`errors.ts`** carries the `ErrorCode` mirror as a `const` object, the
`code -> message` table, `decodeRecord(payload: Uint8Array)` for the 32 bytes of
item A, and `describe(code)`. Pure, no wasm, unit tested with hand-written byte
arrays.

**`panic.ts`** is the second file permitted to build a view over linear
memory, and reads the record of item C:

```
readPanicRecord(memory: WebAssembly.Memory, ptr: number, len: number)
  -> PanicRecord | null
```

Returns `null` when the magic is absent, which is the ordinary case for an
instance that has not panicked and the correct answer for a torn record.

**`fatal.ts`** carries the state, and it is deliberately small:

```
type CoreStatus =
  | { readonly kind: "ok" }
  | { readonly kind: "fatal"; readonly code: number; readonly message: string;
      readonly location: string | null }
```

plus the two functions that move between them. **No class and no interface.**
`AGENTS.md`'s structural rules are written about Rust traits and generics, and
the reasoning transfers exactly: an interface with one implementer increases
the places a reader looks without reducing the cases they consider. A
discriminated union does the opposite.

**What `fatal.ts` does not contain, and why.** It does not terminate a worker,
spawn a replacement or replay viewport state. Those need a worker and a
viewport, and this repository has neither. The rule it encodes is the one that
can be true today and has to be true before either exists: **once a
`CoreStatus` is `fatal` it never returns to `ok`, and no further call is made
into that instance.** The teardown and rebuild are F-101's, and the shell-side
state record that gets replayed is F-100's. See `## Open questions` 5.

### F, the second permitted view, which the HLD already anticipated

`eslint.config.js` bans building any typed array or `DataView` over anything
ending `.memory.buffer`, with `packages/core/src/bulk.ts` as the single
exception. Its own comment says:

> The HLD says "outside the two FUNCTIONS". ESLint scopes overrides by file, so
> the allowance is file-scoped to `packages/core/src/bulk.ts` instead, and that
> file is expected to stay small enough that the difference does not matter.
> Widening the allowance to a second file is a design-plan decision.

**This plan makes that decision.** `packages/core/src/panic.ts` joins the
override list, for three reasons.

1. Section 17.2's own wording is "the two functions that are allowed to do it".
   The specification expected two. The repository has had one only because
   nothing else needed linear memory yet.
2. This is the safest possible instance of the hazard the rule guards. The read
   happens after a trap, when no wasm code can run, so memory growth is not
   merely unlikely, it is impossible between the view's construction and its
   last use.
3. The alternative is worse in a specific way. Putting the panic read inside
   `bulk.ts` would grow the one file whose smallness the comment above relies
   on, and mix a write path with a read path in the file that exists to make
   one ordering unmissable.

`panic.ts` is held to the same discipline as `bulk.ts`: the view is built
inside the function, used immediately and unreachable afterwards, and the
function neither stores it nor returns it. The returned `PanicRecord` carries
copied bytes and a decoded string.

**A third file is not granted.** `packages/core/src/ring.ts` will need one when
F-101 gives it a real ring to drain, and that is F-101's design plan to argue,
not this one's.

### G, `console_error_panic_hook`, and what the words mean

Section 23: "console_error_panic_hook in development builds only, it costs
binary size and leaks symbol names."

This plan reads that as naming a behaviour and a constraint rather than
mandating a dependency, and implements the behaviour with a `wasm_bindgen`
`extern "C"` block declaring `console.error`, under `#[cfg(debug_assertions)]`.
Three consequences.

- **No new dependency.** `console_error_panic_hook` is not in section 15.2's
  list, and `wasm-bindgen` can declare the one import needed with no crate
  added.
- **No feature flag, and therefore no unnamed user.** `debug_assertions` is on
  in the dev profile and off in release, which is what "development builds"
  means, and `AGENTS.md` forbids a feature flag without a named user. The
  shipping artefact is built by `bin/ocelli.sh wasm` in release, so it carries
  neither the import nor the symbol names.
- **The record is written in both profiles.** The console echo is the part that
  is development-only. The fixed record is the mechanism the shell depends on
  and it exists in release, because a production panic that the shell cannot
  describe is exactly the silent blank canvas section 23's last bullet forbids.

This is a reading of section 23 rather than a departure from it, but it is the
kind of reading a reviewer should be given the chance to reject. See
`## Open questions` 3.

### H, structured logging

The HLD has no logging section. This plan therefore states its design and its
limits precisely rather than implying a specification backs it.

**A log line is a `Record`, the same 32 bytes as an error**, with `Event.kind`
set to `Log` and byte 2 holding a level rather than a severity. Levels are
`1 = Error, 2 = Warn, 3 = Info, 4 = Debug, 5 = Trace`, with `0` reserved so a
zeroed payload is not a valid line.

**There is no free text.** A log line is a registered code plus up to three
`u64` operands, and the shell's table turns it into a sentence. That is what
makes it structured rather than a string, it keeps format strings out of a
measured binary, and it means a log line costs no allocation, which the render
loop requires and section 22's performance rules assume.

**What this story delivers is the format, the registry and the development
sink.** The production sink is the event ring, and the ring does not exist:
`crates/ocelli-wasm/src/ring.rs` is named by `CLAUDE.md`, by the unsafe
allow-list and by `packages/core/src/ring.ts`'s doc comment, and is not a file
in this repository. **F-005 does not create it.** Under `debug_assertions` a
log line goes to `console`, which is a real sink today, and F-101 adds the one
call site that pushes the same 32 bytes into the ring. Fixing the format now
and the transport later is the right order, because the format is what both
sides have to agree on and the transport is one function.

**Two ring rules are stated here so F-101 inherits them rather than inventing
them.**

- **An error event must never be lost to ring overflow.** Section 17.3's
  `dropped` counter is a design admission that events can be dropped, and an
  error that is dropped is a viewport that fails silently. So the error is
  **also** latched: section 17.4's `flags` bit 2 is set and stays set, and a
  `last_error_code` accompanies it in the state readback. The ring event is the
  timely notification and the flag is the durable one. A shell that misses the
  event still sees the flag on its next readback.
- **A log line may be dropped and an error may not.** That is the only
  behavioural difference between the two kinds, and it is why they are two
  kinds rather than one with a level field.

### What this story deliberately does not build

Named, because an omission and a decision read identically later.

- `crates/ocelli-wasm/src/ring.rs`. F-101.
- `Session`, `apply_commands`, `alloc`, `commit_frame`. F-101.
- The `ViewportState` struct of 17.4. F-101. This story fixes the meaning of
  `flags` bit 2 and the latch rule, and writes them into `docs/lld/errors.md`.
- Worker creation, teardown or replacement. F-101, and there are no workers.
- The shell's replayable copy of viewport state. F-100 designs the public API.
- Any mapping of a DICOM, codec or render error. Those codes arrive with the
  stories that produce them.

## Boundary and tier

- wasm-bindgen: **ocelli-wasm only**. `crates/ocelli-core/src/error.rs` is a
  plain `no_std` module with `thiserror` and no target-specific code, and the
  `console.error` import of item G is declared inside `ocelli-wasm` under the
  existing `#[cfg(target_arch = "wasm32")]` gate. Deviation D-12 applies
  unchanged and is not widened: this story adds no dependency that reaches
  `wasm-bindgen` on any target, so `ci/check-bindgen-isolation.sh`'s three
  passes are unaffected, including the source grep, which is the strongest.
- Pixels across the boundary: **no**. Nothing in this story touches a pixel.
- Render-loop allocation: **none**. The panic record is `static`, sized at
  compile time, `512` message bytes plus four `u32` headers. The `Record` type
  is a 32-byte value with `u64` operands and no `String`. The one allocation in
  the story is the `Box` that `std::panic::set_hook` requires, which happens
  once in `install_panic_hook()` at worker start, before a frame has been
  rendered. The formatting buffer inside the hook is a stack local. The shell
  side allocates one string per panic, at most once per instance lifetime.
- unsafe: **none**. Neither permitted file is touched and no new file needs
  one. The static record is written through `AtomicU8` and `AtomicU32`, which
  need only a shared reference, and the address is taken with
  `<*const T>::addr()`. `bin/ocelli.sh gate unsafe` should show the same two
  permitted files after this story as before it.
- Tier A (WebGPU): **full**, and identical to B and C. The error model is not a
  rendering or compute feature and its behaviour does not vary by tier.
- Tier B (WebGL2): **full**, identical.
- Tier C (CPU): **full**, identical. This row is not "n/a", and the difference
  matters. D-07's rule is that "a feature that cannot run on the resolved tier
  reports unavailable, and never silently produces a different answer", and
  **this story owns the sentence in which a feature says so.** `ErrorCode`
  carries `Unavailable` from day one, mapping F-008's `ComputeError::
  Unavailable`, precisely so that a tier-gated feature has a stable numeric
  code to return rather than a per-story invention. A tier C session must be
  able to report a tier A feature unavailable in exactly the encoding a tier A
  session would use to report a device loss, or the shell needs two code paths
  for one situation. So the tier answer for this story is that the answer must
  not vary, and the test table asserts that rather than assuming it.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `unit` | `Record` encodes to a hand-written 32-byte array and decodes back, for an error record and for a log record. The expected bytes are written out by hand from the layout table in `## Approach` item A, not produced by calling the encoder | `crates/ocelli-core/src/error.rs` under `#[cfg(test)]` |
| `unit` | `code` 0 and `severity_or_level` 0 are rejected, so a zeroed payload never decodes as a valid record | `crates/ocelli-core/src/error.rs` |
| `unit` | A nonzero `reserved` decodes and is reported, not refused and not dropped | `crates/ocelli-core/src/error.rs` |
| `unit` | The panic hook's message writer truncates at 512 bytes, returns `Ok`, and records the truncated length. Driven with an input longer than the buffer | `crates/ocelli-wasm/src/panic.rs` under `#[cfg(test)]` |
| `unit` | On the host, a panic inside `catch_unwind` leaves a well-formed record with the right magic, a nonzero length and the panic's file and line in the message. Host only, and the module says so, because wasm32 cannot run it | `crates/ocelli-wasm/src/panic.rs` |
| `unit` | A hook that panics leaves the magic unwritten, so the record reads as absent rather than as valid with garbage. Drives the ordering rule of item C step 2 | `crates/ocelli-wasm/src/panic.rs` |
| `unit` | `decodeRecord` in TypeScript returns the same fields for the same hand-written bytes as the Rust test asserts. **The byte array is copied from the Rust test, and the two sides are the two implementations that have to agree** | `packages/core/src/errors.test.ts` |
| `unit` | `readPanicRecord` returns `null` for an all-zero region, and the decoded record for a hand-built one, over a real `WebAssembly.Memory` | `packages/core/src/panic.test.ts` |
| `unit` | `CoreStatus` never leaves `fatal`, asserted by driving the transition function with every input | `packages/core/src/fatal.test.ts` |
| `browser` | **The one that matters.** A real wasm module built from `crates/ocelli-wasm` is instantiated, `install_panic_hook` is called, the addresses are cached, a deliberate panic is triggered, the call throws, and the record is then read out of `memory.buffer` **with no further export call**. Asserts the hook ran under `panic = "abort"` on wasm32, that memory survives the trap readable, and that the message names the panic's file and line | `scripts/panic_probe.mjs`, new gate `panic` |
| `browser` | The same probe asserts the instance is not called again after the trap, by leaving no export invocation in the post-trap path at all | `scripts/panic_probe.mjs` |
| `unit` | `scripts/error_code_check.py` refuses a renumbered code, a reused number, a Rust variant with no TypeScript entry and a TypeScript entry with no Rust variant. Four negative cases, driven against fixture inputs | `scripts/tests/` |

**`fixture` does not appear in this table, and that is a decision rather than
an omission.** HLD 27.2 R3 makes a fixture mandatory for pixel arithmetic with
hand-computed values citing a DICOM section. This story contains no pixel
arithmetic and no geometry, so there is no DICOM section for a value to cite
and a fixture row would be citing nothing. What R3's spirit asks for is
supplied by the hand-written byte arrays in the first `unit` row: the expected
bytes are derived from the layout table above, which is derived from section
17.3's struct, and are never produced by running the encoder. That is R2's rule
applied to a layout instead of to a formula.

**The mutation check, per HLD 27.3 and this sprint's standing expectation.**
Every guard added here is observed red before it is claimed, and the mutation
is run in a separate command from the one that adds it. S02 measured what
happens otherwise: F-010's sweep found its own mutation harness broken, so
every earlier "all refusals red" result had a red baseline and proved nothing.
The four mutations are: flip one byte in the expected `Record` array and watch
the encode test go red, change one number in `ci/error-codes.json` and watch
`errors` go red, remove the magic store from the hook and watch the probe go
red, and remove the `debug_assertions` gate from the console import and watch
the size budget move.

## Parity surface covered

**None, and the checklist cannot be consulted for this story.**
`docs/hld/B-parity-surface.md` has no `Covered by` column and no E-IDs in it at
all. It is eight rows of surface counts, viewport types, tool classes, blend
modes, VOI LUT functions, transfer syntaxes, segmentation representations, core
events and adapters, plus source line counts. `grep -n "E1\." docs/hld/
B-parity-surface.md` returns nothing, so the `/design` step 6 lookup keyed on
epic ref E1.5 resolves to no rows.

One row is adjacent without being covered. "Core events | 50 | Plus 53 tool
events, re-shaped rather than copied." This story adds two event kinds, `Error`
and `Log`, to a namespace that eventually has to answer that count. It does not
map any cornerstone3D event, and claiming a parity row for defining an event
kind would be the kind of claim `/parity` exists to stop.

## Deviations

Cited, all existing:

- **D-01**, the toolchain is 1.97.1 rather than section 15.2's 1.85. The
  measurements in `## The panic strategy today` were taken on 1.97.1 and are
  toolchain-specific, in particular `<*const T>::addr()` being available and
  `wasm32-unknown-unknown` defaulting to `panic = "abort"`.
- **D-07**, tier C exists, which is why all three tier rows are answered above
  and why `ErrorCode::Unavailable` is in the first three codes.
- **D-09**, the precedent for the new deviation described in open question 1.
  Same crate table, same `no_std` posture, same shape of fix.
- **D-12**, the wasm32 bindgen rule. Cited to record that this story does not
  widen it.

**One new deviation is needed and this plan does not apply it**, per the
instruction that `docs/hld/DEVIATIONS.md` is not edited here. It is described
precisely as open question 1, ready to be pasted as a row.

## LLD impact

| File | Change |
|------|--------|
| `docs/lld/errors.md` | **New.** The error model, the code registry and its ranges, the `Record` layout, the panic record layout and its read protocol, the latch rule for `flags` bit 2, and the measured panic strategy from `## The panic strategy today` |
| `docs/lld/build-targets.md` | The exported surface grows from one function to four. The `panic-probe` feature and what it does and does not change about the shipped artefact. The new size baseline and why it moved. The measured statement that `wasm32-unknown-unknown` is `panic = "abort"` in every profile, which the current text does not say |
| `docs/lld/typescript-packaging.md` | Three new modules in the `@ocelli/core` tarball. `packages/core/src/panic.ts` as the second file permitted a view over linear memory, with the reasoning from `## Approach` item F |
| `docs/lld/core-types.md` | `ocelli-core` gains a third module, `error`, alongside `space` and `value`, and its first non-`glam` dependency |
| `docs/lld/gpu-ownership.md` | One line. `ComputeError::Unavailable` and `ComputeError::Workgroup` now have stable numeric codes at the boundary |
| `docs/lld/README.md` | The index row for the new `errors.md` |

## Coordination with the rest of S03

Recorded here because four of the six other stories touch something this one
defines, and the sprint runs them in the same tree.

- **F-004, runtime tiering.** Read after that plan was drafted in this tree.
  **It states that it introduces no error type and makes `resolve` infallible**,
  recording a refused operator override in a `TierEvidence` value rather than
  returning an error, and its own open question 7 asks whether F-005's error
  model has to represent a tier refusal. So the two plans agree and the seam is
  clean: F-004 resolves the tier and never fails, and F-005 owns the sentence a
  **feature** uses to say it cannot run on the resolved tier, which is
  `ErrorCode::Unavailable` mapping F-008's existing `ComputeError::Unavailable`.
  This plan's answer to F-004's open question 7 is no. A refused override is an
  outcome of resolution, not a failure, and giving it a code would put a
  successful resolution in the error space.
  **One real collision remains**: both plans re-baseline
  `ci/wasm-size-budget.json`, and F-004's open question about it names the same
  file. Whichever lands second measures a module the first one already grew, so
  the second `--accept-size` has to attribute its delta to its own change rather
  than to the sum. See open question 6.
- **F-006, benchmark harness.** No conflict of substance. One shared file,
  `bin/ocelli.sh`, if F-006 also adds a gate. Both are additive to the `GATES`
  array and a three-way merge handles it.
- **F-011, pixel-diff comparator.** No overlap. The comparator is a host-side
  tool over the oracle's output and touches neither the boundary nor
  `ocelli-core`'s error module. It has its own tolerance policy, which is not
  an error code.
- **F-X006, codec spike gates.** No overlap. It writes to `docs/spikes/`.
- **F-X007, oracle volume and MPR renders.** No overlap. It writes to
  `tools/oracle/`.
- **F-X009, a standing test for every repository guard.** **The one to
  sequence deliberately.** F-005 adds a new guard, `scripts/
  error_code_check.py`, and F-X009 builds the standing test that every guard
  has one. If F-005 lands after F-X009, the new guard arrives after the sweep
  that was supposed to cover it and is watched by nothing, which is exactly the
  defect F-X009 exists to fix and which S02 measured in F-010. **So either
  F-005 lands before F-X009, or F-X009's sweep is re-run afterwards.** This is
  a scheduling constraint the sprint plan does not currently record. Open
  question 7.

## Open questions

1. **A new deviation row is needed for `thiserror`, and it is measured.**
   Section 4's crate table puts the error model in `ocelli-core`, section 23
   says "thiserror in the core crates", and `ocelli-core` carries
   `#![cfg_attr(not(test), no_std)]`. Section 15.2's workspace entry is
   `thiserror = "2"`, which takes the default features, and `thiserror`'s
   default feature is `std`. Measured on this machine with the pinned
   toolchain, in a throwaway workspace:

   - `thiserror 2.0.20` compiles under `#![no_std]` even with default features,
     so this is not a compilation problem.
   - `cargo tree -e normal,features` reports `thiserror feature "std"`, and
     `scripts/no_std_check.py` searches for exactly `feature "std"`. So
     `gate nostd` **would go red** the moment `ocelli-core` takes the workspace
     entry as written.
   - Setting `default-features = false` on the member entry does not work.
     Cargo prints `` `default-features` is ignored for thiserror, since
     `default-features` was not specified for `workspace.dependencies.
     thiserror`, this could become a hard error in the future `` and the
     feature stays on. The fix has to be at the workspace entry.

   Proposed row, ready to paste, written in D-09's shape because it is the same
   situation:

   The number is the operator's to assign, and is written `D-NN` here rather
   than guessed, because `scripts/deviation_check.py` refuses a plan citing a
   `D-NN` with no row, and a draft plan that fails the deviations gate is a
   plan nobody can run a check over. The highest existing row is D-12.

   > | D-NN | §15.2, `thiserror = "2"` | `thiserror = { version = "2",
   > default-features = false }` | §4 puts the error model in `ocelli-core`,
   > which carries `#![cfg_attr(not(test), no_std)]`. thiserror's default
   > feature is `std`, and `cargo tree -e normal,features` reports
   > `thiserror feature "std"`, which is the string `scripts/no_std_check.py`
   > searches for, so the default entry turns the nostd gate red. Setting
   > `default-features = false` at the member is ignored by Cargo with a
   > warning when the workspace entry does not set it, so the fix has to be at
   > the workspace entry. The pin itself is untouched and only the feature set
   > changes. Verified: thiserror 2.0.20 compiles under `no_std` either way,
   > and emits `core::error::Error`. `ocelli-compute` is not `no_std` and is
   > unaffected. | F-005 |

   **Blocks**: whether `crates/ocelli-core/src/error.rs` can use `thiserror` at
   all, which is section 23's first clause. The fallback is a hand-written
   `core::fmt::Display`, which is worse in a specific way: it puts the message
   strings in the same place with none of the macro's checking, and it departs
   from section 23 in the direction of more code rather than less.

2. **Does the size baseline move, and by how much?** The module is 14,104 bytes
   with a five per cent tolerance, which is 705 bytes. This story adds
   `std::panic::set_hook`, a `Box`ed closure, `core::fmt` for the location
   formatting, 528 bytes of static record, and `ocelli-core` as a dependency of
   `ocelli-wasm` for the first time. **The budget will move and `--accept-size`
   will be used.** `ci/wasm-size-budget.json`'s own note says a re-baseline is
   declared in the design plan, so this paragraph is that declaration, but the
   number is not knowable before the build. **Blocks**: nothing structural, but
   the recorded reason has to name the measured delta, and if the delta is
   large enough to matter for gate A4's 3 to 8 MB estimate that is worth
   saying out loud rather than accepting quietly.

3. **Does `console_error_panic_hook` in section 23 name the crate or the
   behaviour?** Item G reads it as the behaviour and implements it with a
   `wasm_bindgen` extern under `#[cfg(debug_assertions)]`, adding no
   dependency. Reading it as the crate means adding a dev-profile dependency
   that section 15.2 does not list, and a deviation row saying so.
   **Blocks**: `crates/ocelli-wasm/Cargo.toml` and whether a second new
   deviation is needed.

4. **Where does the wasm panic probe run, and is it in the CI floor?** The
   probe needs a built module and an engine. Three options, in the order this
   plan prefers them.

   - **Node, instantiating the built `.wasm` directly.** The four panic exports
     are plain numeric functions needing no bindgen glue, so
     `WebAssembly.instantiate` with stubbed imports reaches them. No new
     dependency, no browser, runs in the CI floor. The risk is the import stubs:
     `ocelli_version()` returns a `String` and pulls bindgen placeholder
     imports into the module, which have to be satisfied with stubs that are
     never called, and if that turns out to be fragile the probe is measuring
     the stubs.
   - **Playwright.** `tools/oracle` already pins `playwright 1.62.1` and drives
     headless Chromium with no GPU needed for this. It is the real engine, and
     it puts a browser download in the CI floor's path.
   - **`wasm-bindgen-test` with `wasm-pack test --headless`.** Rejected. The
     harness installs its own panic hook and treats a panic as a test failure,
     so testing panic behaviour under it fights the tool.

   **Blocks**: the `## Tests` `browser` rows, `bin/ocelli.sh`'s new gate entry
   and whether `.github/workflows/ci.yml` changes.

5. **How much of the shell-side rebuild belongs to F-005?** Section 23's
   sentence "the shell must always hold enough state to rebuild a viewport from
   nothing" is called a design constraint on the shell and not an afterthought.
   This plan honours it by defining `CoreStatus` and the never-returns-to-ok
   rule, and stops there, because there is no viewport, no worker and no public
   API yet, and F-100 designs the last of those. The alternative reading is
   that F-005 should specify the replayable state record now so F-100 inherits
   a requirement rather than discovering one. **Blocks**: the size of
   `packages/core/src/fatal.ts` and whether F-100's plan gains an inherited
   constraint from this one.

6. **Which of F-004 and F-005 re-baselines the wasm size budget, and how is the
   second one's delta attributed?** Both plans say they will move
   `ci/wasm-size-budget.json`, and there is one file and one recorded number.
   The second story to land measures a module the first already grew, so a
   naive `--accept-size` records a delta that is the sum of two changes and
   attributes it to one. The fix is cheap and has to be decided rather than
   discovered: the second story records the baseline the first left, measures
   its own delta against that, and says both numbers. **Blocks**: nothing
   structural, and it is exactly the kind of quiet mis-attribution that makes a
   budget stop meaning anything, which is the same defect class as widening a
   tolerance. The related question, whether F-004 needs an error code at all,
   is answered no by F-004's own plan and by this one, and is recorded in
   `## Coordination with the rest of S03` rather than left open.

7. **Does F-X009's guard sweep run before or after F-005's new gate?** A guard
   added after the sweep that was meant to cover it is watched by nothing,
   which is the exact defect F-X009 exists to fix and which S02 measured.
   **Blocks**: sprint ordering, or an explicit re-run of F-X009's sweep after
   F-005 lands.

8. **Is a decode worker allowed to report an error at all?** Section 24 gives
   the render worker `render -> main : drained event ring [copy]` and gives a
   decode worker no upward arrow. A decode worker that panics is the most
   likely panic in the whole system, since it is the one parsing untrusted
   bytes. This plan's panic record works identically in any worker, because it
   reads linear memory rather than using a protocol arrow, so nothing here is
   blocked. What is not covered is a **recoverable** decode error, which today
   has no specified route to the main thread. **Blocks**: nothing in F-005.
   Recorded because F-016 and the codec stories will hit it, and because the
   honest answer is that section 24 is incomplete rather than that this plan
   found a route.

## Write set

Every file the implementation creates or modifies, and nothing else.

**Created**

- `crates/ocelli-core/src/error.rs`
- `crates/ocelli-wasm/src/panic.rs`
- `packages/core/src/errors.ts`
- `packages/core/src/errors.test.ts`
- `packages/core/src/panic.ts`
- `packages/core/src/panic.test.ts`
- `packages/core/src/fatal.ts`
- `packages/core/src/fatal.test.ts`
- `ci/error-codes.json`
- `scripts/error_code_check.py`
- `scripts/panic_probe.mjs`
- `scripts/tests/test_error_code_check.py`
- `docs/lld/errors.md`

**Modified**

- `Cargo.toml`, the workspace `thiserror` entry, subject to open question 1
- `crates/ocelli-core/Cargo.toml`, add `thiserror`
- `crates/ocelli-core/src/lib.rs`, declare and re-export `error`
- `crates/ocelli-wasm/Cargo.toml`, add `ocelli-core`, and the `panic-probe`
  feature
- `crates/ocelli-wasm/src/lib.rs`, declare `panic`, export `install_panic_hook`,
  `panic_record_ptr`, `panic_record_len`
- `packages/core/src/index.ts`, re-export the three new modules
- `eslint.config.js`, add `packages/core/src/panic.ts` to the override
- `bin/ocelli.sh`, add the `errors` and `panic` gates
- `.github/workflows/ci.yml`, invoke them, subject to open question 4
- `ci/wasm-size-budget.json`, re-baselined with `--accept-size`
- `docs/lld/build-targets.md`, `docs/lld/typescript-packaging.md`,
  `docs/lld/core-types.md`, `docs/lld/gpu-ownership.md`, `docs/lld/README.md`
- `docs/hld/DEVIATIONS.md`, the `thiserror` row of open question 1, **applied
  by the operator and not by
  this plan**

**Notably not touched**

- `crates/ocelli-wasm/src/ring.rs`, which does not exist and is not created here
- `crates/ocelli-core/src/cast.rs`, which does not exist and is not created here
- `packages/core/src/bulk.ts` and `packages/core/src/ring.ts`
- Any file under `docs/sprints/`, and `CHANGELOG.md`

---

## Decisions taken in the design round

Answers to `## Open questions`, taken in the S03 consolidated round.

**1. `thiserror` deviation. Applied as D-15.**
`thiserror = { version = "2", default-features = false }` at the workspace
entry. The measurement in this plan stands: the default entry turns `gate
nostd` red, and `default-features = false` at the member is ignored by Cargo
when the workspace entry does not set it, so the fix has to be at the workspace
entry. Section 23's "thiserror in the core crates" is honoured rather than
replaced by a hand-written `Display`.

**2. The wasm size baseline moves, and F-005 is the only story that moves it.**
F-004 was answered "do not wire into `ocelli-wasm`", so there is no second
delta and no attribution problem. Use `--accept-size` once, and **the recorded
reason names the measured delta and its cause**, which is `ocelli-core` becoming
a dependency of `ocelli-wasm` for the first time plus the panic hook and its
static record. If the delta is large enough to bear on gate A4's 3 to 8 MB
estimate, say so in the completion entry rather than accepting it quietly.

**3. `console_error_panic_hook` names the behaviour, not the crate.** Implement
it as this plan proposes, a `wasm_bindgen` `console.error` extern under
`#[cfg(debug_assertions)]`, adding no dependency and needing no second
deviation. Section 15.2 does not list the crate, and reading the phrase as the
crate would require adding one to satisfy a name.

**4. The wasm panic probe runs under node raw-instantiate, in the CI floor.**
It adds no dependency and keeps a browser out of the floor. If the bindgen
placeholder imports defeat it, that is a finding to report rather than a reason
to move the probe to Playwright without saying so.

**5. The shell-side rebuild stops where this plan stops.** Define `CoreStatus`
and the never-returns-to-`ok` rule, and build no viewport reconstruction,
because no viewport, worker or public API exists to reconstruct. Section 23's
constraint on the shell is recorded for E16.1 and E16.2 to inherit.

**6. Superseded by decision 1 above.** F-004 does not touch
`ci/wasm-size-budget.json`.

**7. F-X009 lands last in this sprint**, so its sweep is taken against the
integrated tree and picks up `scripts/error_code_check.py`. This story lists the
guards it adds in its handoff so F-X009's census does not have to rediscover
them.

**8. A decode worker reporting a recoverable error is out of scope**, recorded
here for F-016 and the codec stories. Section 24 gives it no upward arrow and
this story does not invent one.

**9. `packages/core/src/panic.ts` gets the second ESLint linear-memory
allowance.** `eslint.config.js` says widening it is a design-plan decision, and
this is that decision. HLD 17.2's own wording is "the two functions", so the
specification already expected two. A third is not granted here.
