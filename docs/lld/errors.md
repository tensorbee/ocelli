# The error model, the panic path and structured logging

**Area**: `crates/ocelli-core/src/error.rs`, `crates/ocelli-wasm/src/panic.rs`,
`packages/core/src/{errors,panic,fatal}.ts`, `ci/error-codes.json`
**Normative source**: `docs/hld/20-errors-and-panics.md` section 23, with
`docs/hld/14-the-boundary-in-code.md` sections 17.2, 17.3 and 17.4
**F-IDs that contributed:** F-005
**Last updated:** 2026-09-05

Living current-state document. It describes what the code does today.

## The measured fact everything here rests on

Taken on this machine with the pinned toolchain, and it is stronger than the
specification assumes.

```
$ rustc --print cfg | grep panic
panic="unwind"

$ rustc --print cfg --target wasm32-unknown-unknown | grep panic
panic="abort"
```

**`wasm32-unknown-unknown` is `panic = "abort"` in every profile, not only in
release.** That is the target's own default, so section 15.2's
`[profile.release] panic = "abort"` agrees with something already in force.
`catch_unwind` compiles on wasm32 and never catches, so a boundary that caught
a panic and converted it into an error code is not merely forbidden by section
23, it does not work.

**Recovery is therefore not resumption.** The instance traps, the shell reads a
fixed-size record out of linear memory without calling any export, latches the
viewport into an error state, and the worker is discarded and rebuilt. The
sprint's done line, "maps a Rust panic to a JS error the shell can act on
without poisoning the wasm instance", means that the mapping must not itself
require touching a poisoned instance. It does not mean the instance survives.

**The host is `panic = "unwind"`**, so `cargo test` can catch a panic and the
hook's formatting is unit tested natively. A native test proves nothing about
the wasm32 trap, which is why `bin/ocelli.sh gate panic` exists.

## The code registry

`ci/error-codes.json` holds every code, its number, the crate range it falls
in, and either its producer or, where none exists yet, the producer it is
intended for. Three files have to agree and they are edited by different hands:

| File | Holds |
|------|-------|
| `crates/ocelli-core/src/error.rs` | The `ErrorCode` enum a Rust producer names |
| `packages/core/src/errors.ts` | The mirror the shell switches on, and the human text |
| `ci/error-codes.json` | The register that outlives both |

`scripts/error_code_check.py` is the `errors` gate. It refuses a renumbering, a
reused number, a reused name, a code either side cannot name, a code with no
message, a code outside its declared range, a code naming no range, overlapping
ranges, and code `0`.

It does **not** enforce append-only. A removed code is a real event, and
turning it into a gate failure produces a gate somebody disables. What is
refused is reuse of the number, which is the part that is silent.

### The three codes that exist today

A fourth arrives with the story whose code it is, as one appended entry in each
of the three files.

| Code | Number | Producer | Intended producer |
|------|--------|----------|-------------------|
| `Panicked` | 1 | `crates/ocelli-wasm/src/panic.rs` | - |
| `Unavailable` | 700 | none today | `ocelli_compute::ComputeError::Unavailable` |
| `Workgroup` | 701 | none today | `ocelli_compute::ComputeError::Workgroup` |

**Only `Panicked` has a live producer, and the other two name a correspondence
that no code yet expresses.** `crates/ocelli-core/src/error.rs` says so in
terms and this table used to say the opposite: there is no `From<ComputeError>`
anywhere, and `ocelli-compute` does not depend on `ocelli-core` at all, its
dependencies being `ocelli-render`, `wgpu` and `thiserror`. Check both rather
than trusting this paragraph:

```bash
grep -rn "From<ComputeError>" crates/
sed -n '/^\[dependencies\]/,/^$/p' crates/ocelli-compute/Cargo.toml
```

The distinction is worth the column because `scripts/error_code_check.py`
never reads the producer field. It checks the number, the name, the range and
that both sides can name the code, so the producer is unchecked prose in the
file this document calls "the register that outlives both", and unchecked prose
that claims a mechanism is how a register stops being one.

`Unavailable` is deviation D-07's sentence as a number. A feature that cannot
run on the resolved tier reports unavailable and never silently produces a
different answer, and a tier C session says so in exactly the encoding a tier A
session would use, so the shell needs one path and not two.

### The reserved ranges

Declared now because a range is cheap to declare and expensive to retrofit once
two crates have picked overlapping numbers.

| Range | Crate |
|-------|-------|
| 1 to 99 | `ocelli-core`, the boundary and the instance lifecycle |
| 100 to 199 | `ocelli-dicom` |
| 200 to 299 | `ocelli-codec` |
| 300 to 399 | `ocelli-pixel` |
| 400 to 499 | `ocelli-geom` |
| 500 to 599 | `ocelli-volume` |
| 600 to 699 | `ocelli-render` |
| 700 to 799 | `ocelli-compute` |
| 800 to 899 | `ocelli-cache` |
| 900 to 999 | `ocelli-seg` |
| 1000 to 1099 | `ocelli-viewport` |

## The thirty-two byte record

One layout for both an error and a log line. The discriminant that separates
them is `Event.kind` in section 17.3's header, so it is not repeated inside the
payload. One layout, two kinds, one encoder and one decoder on each side.

```text
offset  size  field
0       2     code               u16, little-endian, never 0
2       1     severity_or_level  u8, never 0
3       1     arity              how many of a0..a2 are meaningful
4       4     reserved           u32, a producer writes 0
8       8     a0                 u64
16      8     a1                 u64
24      8     a2                 u64
                                 total 32
```

**The `u32` at offset 4 is padding with a name.** Without it the three operands
would sit at payload offsets 4, 12 and 20. Section 17.3's `Event` is
`#[repr(C)]` over `u32, u32, u64, [u8; 32]`, which puts `payload` at struct
offset 16 and gives the stated 48-byte stride, so payload offset 8 is struct
offset 24 and every operand is eight-byte aligned in linear memory. Nothing
reads a `u64` through a pointer today, so this is not correctness. It is that
an unaligned `u64` in a `#[repr(C)]` boundary struct is what a later story adds
a `bytemuck::Pod` derive to and discovers at the worst moment.

**`reserved` is surfaced, not asserted zero.** A consumer that refused a
nonzero value could not read a record written by a newer core, and one that
ignored it would lose the same information twice. Both sides report it, exactly
as section 17.3 says `dropped` is reported.

**A zeroed payload is never a record.** Code `0` is refused, and byte 2 is
refused at `0` as well, because it holds a severity for an error and a level
for a log line and both reserve `0`. One byte, one rule, one decoder.

| Byte 2, error | Byte 2, log |
|---------------|-------------|
| 1 `Recoverable` | 1 `Error` |
| 2 `Fatal` | 2 `Warn` |
| | 3 `Info` |
| | 4 `Debug` |
| | 5 `Trace` |

## Where the message lives, and why it is TypeScript

Section 23: "The shell switches on the code, the message is for humans and may
change." Both halves argue the same way.

The human text is `packages/core/src/errors.ts`, keyed on the code. The Rust
side carries `thiserror`'s `#[error("...")]` strings for `Display` in native
builds, tests and the two native entry points, and does not send them across
the boundary. Three reasons, in order of weight.

1. **The module is measured.** `opt-level = "z"` with `strip = true`, and a
   recorded budget. Format strings and the `core::fmt` machinery to assemble
   them are exactly the weight that budget exists to notice.
2. **"May change" means the message is not a contract.** One that crossed the
   boundary would become one the moment a consumer matched on it.
3. **It makes the code stable by construction.** A shell whose table is keyed
   on the code cannot render a code it does not know without noticing, and
   `describeError()` says so in a sentence naming the number.

**The panic message is the exception, and the only one.** Its whole value is
that it names an unanticipated failure with a file and a line, so it is not
from a table.

## The panic record

A `#[repr(C)]` static in `crates/ocelli-wasm/src/panic.rs`, at a fixed address
in linear memory.

```text
offset  size  field
0       4     magic     u32, 0 until a panic, then 0x3150434F, "OCP1"
4       4     version   u32, 1 today
8       4     code      u32, an ErrorCode number
12      4     msg_len   u32, bytes written, at most 512
16      512   msg       UTF-8, not NUL terminated
                        total 528
```

**One struct and not five statics.** Rust gives no layout guarantee across
separate statics, so five of them could sit anywhere relative to each other and
the single cached pointer the shell holds would address only the first.

**Atomics, and that is what makes this need no `unsafe`.** A `static` written
through `AtomicU8::store` needs only a shared reference, so the hook mutates a
static with no `unsafe`, no `Mutex` that could deadlock inside a panic, and no
`UnsafeCell`. The obvious implementation is a `static mut` and it would have to
be refused: HLD 27.2 R5 allows `unsafe` in `crates/ocelli-wasm/src/ring.rs` and
`crates/ocelli-core/src/cast.rs`, and this is neither. **The allow-list is
unchanged and both permitted files still do not exist.**

The address is taken with `core::ptr::from_ref` and `<*const T>::addr()`, which
gives an address with no pointer-to-integer cast, then narrowed with
`u32::try_from`, which gives the narrowing with no `cast_possible_truncation`.
On a 64-bit host that narrowing legitimately returns `0`, and `0` is the
shell's "no record available". The export exists for wasm32, where `usize` is
32 bits and the conversion is exact.

### The write order is the integrity check

The hook formats into a stack-local buffer, stores the message bytes, then
`msg_len`, then `code`, then `version`, and **the magic last**. A hook that
panics part way through leaves the magic at `0`, and the shell reads that as
**no record** rather than as a valid record with garbage in it.

**That ordering is driven through `record` itself, and until the seventh review
pass it was not.** `fill` and `seal` are split so a test can drive the rule,
and the test that looked like it did called the two from its own body, so what
it asserted was the order it had just written. Swapping the two calls inside
`record` left the crate green. The state that swap produces is the torn record
this design exists to exclude: the magic reads `OCP1`, `code` is still `0`,
`readPanicRecord` returns a record that reads as PRESENT, and `describeError(0)`
reports that this build does not recognise the code instead of section 23's
sentence. Reading the record afterwards cannot tell the two orders apart,
because both leave it byte-identical, so `fill` stamps what the magic held at
the instant it began and `record_writes_the_body_before_the_magic` asserts it
held the absent value. The stamp is `#[cfg(test)]` and the shipped hook makes
exactly the stores it made before.

### The message writer truncates and returns `Ok`

A writer that returned `Err` on overflow would make `write!` return an error
the hook has to handle, and the only honest handling inside a panic hook is
another panic. So it fills what it can, records that it stopped, and reports
success, which is true of everything the caller can act on. A truncation can
land inside a UTF-8 sequence, and the shell decodes leniently, which turns a
split code point into one replacement character rather than into an exception
thrown while it is already handling a dead instance.

### Four exports, and three of them are called exactly once

| Export | Called |
|--------|--------|
| `ocelli_version()` | Any time |
| `install_panic_hook()` | Once, by a worker, before any other call |
| `panic_record_ptr()` | Once, straight after instantiation. Cached |
| `panic_record_len()` | Once, straight after instantiation. Cached |

**This is the whole reason the design works.** After the trap the shell needs
no call into the instance at all, which is section 23's "must not be reused"
honoured literally rather than approximately. An export that returned the
message after the trap would be reusing a poisoned instance to ask why it was
poisoned.

**A cached pointer is safe and a cached view is not**, and that distinction is
what section 17.2 is about. WebAssembly memory growth relocates the JavaScript
`ArrayBuffer` and detaches every view over it. It does not move the linear
address of a static. So the cached integer stays correct for the life of the
instance, and the `DataView` over it is built fresh at every read.

### The second permitted view over linear memory

`packages/core/src/panic.ts` joins `packages/core/src/bulk.ts` in
`eslint.config.js`'s override. Section 17.2's own wording is "outside the two
functions that are allowed to do it", so the specification expected two, and
this repository had one only because nothing else needed linear memory yet.

This is the safest possible instance of the hazard the rule guards: the read
happens after a trap, when no wasm code can run, so a growth between the view's
construction and its last use is impossible. The discipline is kept anyway. The
view is built inside the function, used immediately, and neither stored nor
returned, and the `PanicRecord` handed back carries copied bytes and a decoded
string.

**A third PRODUCTION file is not granted.** `packages/core/src/ring.ts` will
need one when F-101 gives it a real ring to drain, and that is F-101's design
plan to argue.

`ALLOWED_TO_DISABLE` in `eslint.config.js` holds three path patterns and not
two, which is worth stating because the sentence above reads as though it held
two. The first list is the production allowance, `bulk.ts` and `panic.ts`. The
second is `packages/core/src/*.test.ts`, kept as a separate list precisely so
the production allowance does not read as three files when it is two:
`panic.test.ts` builds a view over a `WebAssembly.Memory` it constructed
itself, which nothing can grow, and the rule is syntactic and cannot tell that
memory from the core's.

The comment above `RESTRICTED` and `BAN` in `eslint.config.js` carries a
MEASURED list of shapes that escape the ban, taken with a probe file and
`npx eslint` rather than reasoned about, along with a fourth selector and the
one site in `packages/` and `examples/` it would cost. **It is that comment and
not the one above `NO_CACHED_WASM_VIEW_MEMBER`**, which this paragraph named
until the seventh review pass. The comment above the first selector carries the
alias shapes and the third selector and nothing else, so a reader sent there
finds neither the escape list nor the fourth selector, and a cross-reference
that lands on the wrong comment in the right file is harder to notice than one
that lands on the wrong file. **That list is a sample and not the
set**, and the fourth selector does not close all of it: the S03 review's fifth
pass measured three further routes and the sixth confirmed that
`new DataView(wasm.memory["buffer"])` escapes the fourth selector too, because
it keys on a property name and a computed member's property is a `Literal` with
none. F-X017 is the story, its answer is type-aware linting rather than a
config line, and its acceptance test is the computed route.

## The shell side

Three modules, all re-exported from `packages/core/src/index.ts`.

| Module | Holds |
|--------|-------|
| `errors.ts` | The code mirror, the message table, `decodeRecord`, `describeError` |
| `panic.ts` | `readPanicRecord`, and the layout it reads |
| `fatal.ts` | `CoreStatus` and the latch |

```ts
type CoreStatus =
  | { readonly kind: "ok" }
  | { readonly kind: "fatal"; readonly code: number;
      readonly message: string; readonly location: string | null };
```

**The rule this encodes: once a `CoreStatus` is `fatal` it never returns to
`ok`, and no further call is made into that instance.** A new instance gets a
new status, which is what makes the latch safe rather than terminal.
`nextStatus` keeps the FIRST fatal, because the first failure is the one that
explains what happened and a later report is a consequence of it.

**What `fatal.ts` does not contain, and why.** It does not terminate a worker,
spawn a replacement or replay viewport state. Those need a worker and a
viewport, and this repository has neither. Section 23's "the shell must always
hold enough state to rebuild a viewport from nothing" is a constraint recorded
here for F-100 (E16.1), which designs the public API, and F-101 (E16.2), which
builds the boundary and the workers.

`fatalFromPanic(null)` is still fatal. A trapped instance whose record could
not be read is still a dead instance, and reporting `ok` there would be section
23's silent blank canvas.

No class and no interface for the status. A discriminated union reduces the
cases a reader must consider and an interface with one implementer increases
the places they must look, which is `AGENTS.md`'s structural test.

## `console_error_panic_hook`, read as a behaviour

Section 23's first bullet: "console_error_panic_hook in development builds
only, it costs binary size and leaks symbol names."

That names a behaviour and a constraint rather than a dependency.
`console_error_panic_hook` is not in section 15.2's list, and `wasm-bindgen`
declares the one import needed with no crate added, so the hook echoes to
`console.error` under `#[cfg(all(target_arch = "wasm32", debug_assertions))]`.
`debug_assertions` is what "development builds" means and it needs no feature
flag, which `AGENTS.md` would require a named user for.

**The record itself is written in both profiles.** The echo is the part that is
development-only. A production panic the shell could not describe is exactly
the silent blank canvas section 23's last bullet forbids.

## Structured logging

The HLD has no logging section. `grep -rn -i "structured log\|logging\|tracing"
docs/hld/*.md` returns nothing, and the one adjacent sentence is section 17.3's
"surface `dropped` in telemetry rather than swallowing it". So this design is
this repository's and the limits are stated rather than implied.

**A log line is a `Record`**, the same thirty-two bytes as an error, with
`Event.kind` set to `Log` and byte 2 holding a level. **There is no free text.**
A line is a registered code plus up to three `u64` operands, and the shell's
table turns it into a sentence. That is what makes it structured rather than a
string, it keeps format strings out of a measured binary, and a line costs no
allocation, which the render loop requires.

**What exists today is the format and the registry.** The production sink is
the event ring, and `crates/ocelli-wasm/src/ring.rs` does not exist. F-101 adds
the one call site that pushes the same thirty-two bytes into it. Fixing the
format now and the transport later is the right order, because the format is
what both sides have to agree on and the transport is one function.

### Two ring rules, stated here so F-101 inherits them

- **An error event must never be lost to ring overflow.** Section 17.3's
  `dropped` counter is an admission that events can be dropped, and a dropped
  error is a viewport that fails silently. So the error is **also** latched:
  section 17.4's `flags` bit 2 is set and stays set, with a `last_error_code`
  beside it in the state readback. The ring event is the timely notification
  and the flag is the durable one. A shell that misses the event still sees the
  flag on its next readback.
- **A log line may be dropped and an error may not.** That is the only
  behavioural difference between the two kinds, and it is why they are two
  kinds rather than one with a level field.

Section 17.4's `flags` bit 2 is `error`, and section 23 says a poisoned
instance surfaces as a viewport-level error state. **Those are the same bit.**
`ViewportState` itself is F-101's.

## The gates this area owns

| Gate | Runs | Refuses |
|------|------|---------|
| `errors` | `scripts/error_code_check.py`, then its own negative cases | A renumbering, a reused number or name, a code either side cannot name or describe, a code outside or without a declared range, overlapping ranges, code 0 |
| `panic` | A second wasm module carrying the `panic-probe` feature, probed by `scripts/panic_probe.mjs` under node | A hook that does not run under `panic = "abort"`, a trap that does not throw, a memory that is unreadable afterwards, a record without the magic, the version, the code, or the panic's file and line |

Both are in the CI floor. Neither needs a GPU, a corpus or a browser.

### What `bin/ocelli.sh gate panic` builds, and what it does not ship

The probe needs an export that panics on purpose, and the shipped artefact must
not have one. So `panic_probe_trigger()` sits behind the `panic-probe` cargo
feature, and the gate builds a **second** module with it into
`crates/ocelli-wasm/target/panic-probe`, which is gitignored. `bin/ocelli.sh
wasm` builds without the feature, so the module measured against
`ci/wasm-size-budget.json` and published to npm has no way to be asked to
panic. `AGENTS.md` forbids a feature flag without a named user, and the named
user is `scripts/panic_probe.mjs`.

The probe instantiates the module with `WebAssembly.instantiate` rather than
through the wasm-bindgen glue, because the glue installs its own handling
around a trap and the probe would then be measuring the glue. Measured on
wasm-bindgen 0.2.127 with `--target web`, **the release module declares no
imports at all**, so the design plan's stated risk, that stubbed placeholder
imports would defeat the raw instantiate, does not arise. The stub machinery is
kept anyway and throws if a stub is ever called, so the day an import appears
the probe says so instead of quietly measuring something else.

`wasm-bindgen-test` is not used. Its harness installs its own panic hook and
treats a panic as a test failure, so testing panic behaviour under it fights
the tool.

### What the probe measured

Recorded because it is the property the design rests on, and because a claim
about it is only worth what the measurement behind it is worth.

```
note module declares 0 import(s), so no stub can be reached
ok   THE PANIC HOOK RAN under panic = "abort" on wasm32, magic is 0x3150434f
ok   linear memory is still readable after the trap
note recorded message: "ocelli panic probe, F-005 at crates/ocelli-wasm/src/lib.rs:110:5"
```

The file and the line survive `strip = true`, because `core::panic::Location`
is data the compiler emits rather than a symbol name.

## No deliberate panic anywhere is written with `panic!`

The workspace denies `clippy::panic`, `clippy::unwrap_used` and
`clippy::expect_used`, and those denials are section 23's. This story needed
three deliberate panics, two in host tests and one in the probe trigger, and
**none of them switches a lint off and none adds an `#[allow]`.** They arrive
the way a real panic arrives in Rust, through an assertion that does not hold,
with `core::hint::black_box` making the condition runtime data so it is not
`assert!(false)`. That is also the most representative shape available: the
likeliest panic in this system is a decode worker asserting something about
untrusted bytes.

## What this area does not build

Named, because an omission and a decision read identically later.

- `crates/ocelli-wasm/src/ring.rs`, and the `ViewportState` of section 17.4.
  F-101.
- Worker creation, teardown or replacement. F-101, and there are no workers.
- The shell's replayable copy of viewport state. F-100 designs the public API.
- Any mapping of a DICOM, codec or render error. Those codes arrive with the
  stories that produce them.
- A route for a **recoverable** decode error to reach the main thread. Section
  24 gives the render worker `render -> main : drained event ring [copy]` and
  gives a decode worker no upward arrow at all. The panic record works
  identically in any worker, because it reads linear memory rather than using a
  protocol arrow, so nothing here is blocked. F-016 and the codec stories will
  hit the recoverable case, and the honest answer is that section 24 is
  incomplete rather than that this design found a route.
