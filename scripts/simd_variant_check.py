#!/usr/bin/env python3
"""The SIMD verification module holds SIMD instructions and the plain one none.

F-233, after its lead rulings on review pass 1: "The check disassembles the
plain and SIMD variant modules and compares their instruction sets. It goes
red when the SIMD module holds no SIMD instruction the plain module lacks, and
when the plain module holds any SIMD instruction."

Measured on F-233's tree through the real native arm. A build whose rustc
wrapper strips `+simd128` from every crate links a module byte-identical to
the plain one, and a build with `+bulk-memory` in place of `+simd128` links a
module with no SIMD instruction. Step 8 refuses both. The rlib comparison this
replaces passed both.

What it does not refuse is a build that keeps the flag on the example and
strips it from every dependency, the pre-F-233 `cargo rustc --` shape. That
build links a module byte-identical to the pre-F-233 one, which holds the same
SIMD instructions as the F-233 build. The release profile is `lto = "fat"`, so
every dependency's rlib carries LLVM bitcode and all code generation runs
once, at the example's link, under the example's flags. What the flag changes
per crate is `cfg(target_feature = "simd128")` source selection, and
`docs/lld/build-targets.md` records that as a measurement taken once, not as
something this check asserts. What pins the build shape instead is
`build_shape_problems`, a structural pin on the runner text that
`scripts/tests/test_simd_variant_check.py` applies to the tracked
`bin/ocelli.sh`: every SIMD variant step passes `+simd128` through
`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS` and none passes a target
feature after `cargo rustc --`. It reads spelling and proves nothing about a
build.

The module is decoded with the standard library only, per the WebAssembly Core
Specification 2.0 section 5. Every function body in the code section is
walked instruction by instruction, so a 0xFD byte inside an immediate or a
custom section is never mistaken for the SIMD prefix. An opcode this decoder
does not know refuses the module rather than being skipped, because skipping
one loses the position of every instruction after it.

Usage:
  python3 scripts/simd_variant_check.py <plain.wasm> <simd.wasm>
"""

from __future__ import annotations

import sys
from collections import Counter
from pathlib import Path

MAGIC = b"\x00asm\x01\x00\x00\x00"
CODE_SECTION = 10
SIMD_PREFIX = 0xFD

# Section 5.4.8, sub-opcodes 0 to 255 in order. An empty string is a gap the
# specification leaves unassigned.
_SIMD_NAMES = (
    "v128.load v128.load8x8_s v128.load8x8_u v128.load16x4_s v128.load16x4_u "
    "v128.load32x2_s v128.load32x2_u v128.load8_splat v128.load16_splat "
    "v128.load32_splat v128.load64_splat v128.store v128.const i8x16.shuffle "
    "i8x16.swizzle i8x16.splat i16x8.splat i32x4.splat i64x2.splat "
    "f32x4.splat f64x2.splat i8x16.extract_lane_s i8x16.extract_lane_u "
    "i8x16.replace_lane i16x8.extract_lane_s i16x8.extract_lane_u "
    "i16x8.replace_lane i32x4.extract_lane i32x4.replace_lane "
    "i64x2.extract_lane i64x2.replace_lane f32x4.extract_lane "
    "f32x4.replace_lane f64x2.extract_lane f64x2.replace_lane "
    "i8x16.eq i8x16.ne i8x16.lt_s i8x16.lt_u i8x16.gt_s i8x16.gt_u "
    "i8x16.le_s i8x16.le_u i8x16.ge_s i8x16.ge_u "
    "i16x8.eq i16x8.ne i16x8.lt_s i16x8.lt_u i16x8.gt_s i16x8.gt_u "
    "i16x8.le_s i16x8.le_u i16x8.ge_s i16x8.ge_u "
    "i32x4.eq i32x4.ne i32x4.lt_s i32x4.lt_u i32x4.gt_s i32x4.gt_u "
    "i32x4.le_s i32x4.le_u i32x4.ge_s i32x4.ge_u "
    "f32x4.eq f32x4.ne f32x4.lt f32x4.gt f32x4.le f32x4.ge "
    "f64x2.eq f64x2.ne f64x2.lt f64x2.gt f64x2.le f64x2.ge "
    "v128.not v128.and v128.andnot v128.or v128.xor v128.bitselect "
    "v128.any_true v128.load8_lane v128.load16_lane v128.load32_lane "
    "v128.load64_lane v128.store8_lane v128.store16_lane v128.store32_lane "
    "v128.store64_lane v128.load32_zero v128.load64_zero "
    "f32x4.demote_f64x2_zero f64x2.promote_low_f32x4 "
    "i8x16.abs i8x16.neg i8x16.popcnt i8x16.all_true i8x16.bitmask "
    "i8x16.narrow_i16x8_s i8x16.narrow_i16x8_u f32x4.ceil f32x4.floor "
    "f32x4.trunc f32x4.nearest i8x16.shl i8x16.shr_s i8x16.shr_u i8x16.add "
    "i8x16.add_sat_s i8x16.add_sat_u i8x16.sub i8x16.sub_sat_s "
    "i8x16.sub_sat_u f64x2.ceil f64x2.floor i8x16.min_s i8x16.min_u "
    "i8x16.max_s i8x16.max_u f64x2.trunc i8x16.avgr_u "
    "i16x8.extadd_pairwise_i8x16_s i16x8.extadd_pairwise_i8x16_u "
    "i32x4.extadd_pairwise_i16x8_s i32x4.extadd_pairwise_i16x8_u "
    "i16x8.abs i16x8.neg i16x8.q15mulr_sat_s i16x8.all_true i16x8.bitmask "
    "i16x8.narrow_i32x4_s i16x8.narrow_i32x4_u i16x8.extend_low_i8x16_s "
    "i16x8.extend_high_i8x16_s i16x8.extend_low_i8x16_u "
    "i16x8.extend_high_i8x16_u i16x8.shl i16x8.shr_s i16x8.shr_u i16x8.add "
    "i16x8.add_sat_s i16x8.add_sat_u i16x8.sub i16x8.sub_sat_s "
    "i16x8.sub_sat_u f64x2.nearest i16x8.mul i16x8.min_s i16x8.min_u "
    "i16x8.max_s i16x8.max_u - i16x8.avgr_u i16x8.extmul_low_i8x16_s "
    "i16x8.extmul_high_i8x16_s i16x8.extmul_low_i8x16_u "
    "i16x8.extmul_high_i8x16_u i32x4.abs i32x4.neg - i32x4.all_true "
    "i32x4.bitmask - - i32x4.extend_low_i16x8_s i32x4.extend_high_i16x8_s "
    "i32x4.extend_low_i16x8_u i32x4.extend_high_i16x8_u i32x4.shl "
    "i32x4.shr_s i32x4.shr_u i32x4.add - - i32x4.sub - - - i32x4.mul "
    "i32x4.min_s i32x4.min_u i32x4.max_s i32x4.max_u i32x4.dot_i16x8_s - "
    "i32x4.extmul_low_i16x8_s i32x4.extmul_high_i16x8_s "
    "i32x4.extmul_low_i16x8_u i32x4.extmul_high_i16x8_u i64x2.abs i64x2.neg "
    "- i64x2.all_true i64x2.bitmask - - i64x2.extend_low_i32x4_s "
    "i64x2.extend_high_i32x4_s i64x2.extend_low_i32x4_u "
    "i64x2.extend_high_i32x4_u i64x2.shl i64x2.shr_s i64x2.shr_u i64x2.add "
    "- - i64x2.sub - - - i64x2.mul i64x2.eq i64x2.ne i64x2.lt_s i64x2.gt_s "
    "i64x2.le_s i64x2.ge_s i64x2.extmul_low_i32x4_s "
    "i64x2.extmul_high_i32x4_s i64x2.extmul_low_i32x4_u "
    "i64x2.extmul_high_i32x4_u f32x4.abs f32x4.neg - f32x4.sqrt f32x4.add "
    "f32x4.sub f32x4.mul f32x4.div f32x4.min f32x4.max f32x4.pmin "
    "f32x4.pmax f64x2.abs f64x2.neg - f64x2.sqrt f64x2.add f64x2.sub "
    "f64x2.mul f64x2.div f64x2.min f64x2.max f64x2.pmin f64x2.pmax "
    "i32x4.trunc_sat_f32x4_s i32x4.trunc_sat_f32x4_u f32x4.convert_i32x4_s "
    "f32x4.convert_i32x4_u i32x4.trunc_sat_f64x2_s_zero "
    "i32x4.trunc_sat_f64x2_u_zero f64x2.convert_low_i32x4_s "
    "f64x2.convert_low_i32x4_u"
).split()
SIMD_NAMES = {code: name for code, name in enumerate(_SIMD_NAMES)
              if name != "-"}
assert len(_SIMD_NAMES) == 256, len(_SIMD_NAMES)
# The relaxed SIMD proposal, sub-opcodes 0x100 to 0x113.
SIMD_NAMES.update(enumerate((
    "i8x16.relaxed_swizzle i32x4.relaxed_trunc_f32x4_s "
    "i32x4.relaxed_trunc_f32x4_u i32x4.relaxed_trunc_f64x2_s_zero "
    "i32x4.relaxed_trunc_f64x2_u_zero f32x4.relaxed_madd "
    "f32x4.relaxed_nmadd f64x2.relaxed_madd f64x2.relaxed_nmadd "
    "i8x16.relaxed_laneselect i16x8.relaxed_laneselect "
    "i32x4.relaxed_laneselect i64x2.relaxed_laneselect f32x4.relaxed_min "
    "f32x4.relaxed_max f64x2.relaxed_min f64x2.relaxed_max "
    "i16x8.relaxed_q15mulr_s i16x8.relaxed_dot_i8x16_i7x16_s "
    "i32x4.relaxed_dot_i8x16_i7x16_add_s").split(), start=0x100))

# SIMD immediates by sub-opcode. Every other assigned sub-opcode has none.
_SIMD_MEMARG = set(range(0, 12)) | {92, 93}
_SIMD_MEMARG_LANE = set(range(84, 92))
_SIMD_LANE = set(range(21, 35))

# Core opcodes with no immediate: control and parametric instructions, then
# the numeric block from i32.eqz to the sign-extension operators, then
# ref.is_null.
_NO_IMMEDIATE = ({0x00, 0x01, 0x05, 0x0B, 0x0F, 0x1A, 0x1B, 0xD1}
                 | set(range(0x45, 0xC5)))
_ONE_INDEX = {0x0C, 0x0D, 0x10, 0x12, 0x20, 0x21, 0x22,
              0x23, 0x24, 0x25, 0x26, 0x3F, 0x40, 0xD2}
_BLOCK = {0x02, 0x03, 0x04}
_MEMARG = set(range(0x28, 0x3F))
# The 0xFC block: saturating truncation, bulk memory and table operators,
# as the number of u32 immediates each sub-opcode takes.
_FC_IMMEDIATES = {**{n: 0 for n in range(8)}, 8: 2, 9: 1, 10: 2, 11: 1,
                  12: 2, 13: 1, 14: 2, 15: 1, 16: 1, 17: 1}
_VALTYPES = {0x7F, 0x7E, 0x7D, 0x7C, 0x7B, 0x70, 0x6F}


class WasmError(ValueError):
    """The bytes are not a module this decoder can walk to the end."""


class _Reader:

    def __init__(self, data: bytes, start: int = 0,
                 end: int | None = None) -> None:
        self.data = data
        self.pos = start
        self.end = len(data) if end is None else end
        if self.end > len(data):
            raise WasmError(f"a length runs to byte {self.end} of a "
                            f"{len(data)}-byte module")

    def byte(self) -> int:
        if self.pos >= self.end:
            raise WasmError(f"unexpected end of input at byte {self.pos}")
        value = self.data[self.pos]
        self.pos += 1
        return value

    def skip(self, count: int) -> None:
        if self.pos + count > self.end:
            raise WasmError(f"an immediate at byte {self.pos} runs past its "
                            f"end at byte {self.end}")
        self.pos += count

    def leb(self, max_bytes: int = 10) -> int:
        """An LEB128 integer, read as unsigned. Signed values are skipped."""
        result = 0
        for shift in range(0, 7 * max_bytes, 7):
            byte = self.byte()
            result |= (byte & 0x7F) << shift
            if not byte & 0x80:
                return result
        raise WasmError(f"an LEB128 longer than {max_bytes} bytes ends at "
                        f"byte {self.pos}")

    def memarg(self) -> None:
        align = self.leb(5)
        if align & 0x40:
            self.leb(5)
        self.leb()


def _block_type(reader: _Reader) -> None:
    first = reader.data[reader.pos] if reader.pos < reader.end else None
    if first == 0x40 or first in _VALTYPES:
        reader.pos += 1
    else:
        reader.leb(5)


def _walk_body(reader: _Reader, found: Counter) -> None:
    for _ in range(reader.leb(5)):
        reader.leb(5)
        reader.byte()
    depth = 1
    while depth:
        at = reader.pos
        op = reader.byte()
        if op in _BLOCK:
            _block_type(reader)
            depth += 1
        elif op == 0x0B:
            depth -= 1
        elif op in _NO_IMMEDIATE:
            pass
        elif op in _ONE_INDEX:
            reader.leb(5)
        elif op in _MEMARG:
            reader.memarg()
        elif op == 0x0E:
            for _ in range(reader.leb(5) + 1):
                reader.leb(5)
        elif op in (0x11, 0x13):
            reader.leb(5)
            reader.leb(5)
        elif op == 0x1C:
            reader.skip(reader.leb(5))
        elif op in (0x41, 0x42):
            reader.leb()
        elif op == 0x43:
            reader.skip(4)
        elif op == 0x44:
            reader.skip(8)
        elif op == 0xD0:
            reader.byte()
        elif op == 0xFC:
            sub = reader.leb(5)
            if sub not in _FC_IMMEDIATES:
                raise WasmError(f"unknown 0xFC sub-opcode {sub} at byte {at}")
            for _ in range(_FC_IMMEDIATES[sub]):
                reader.leb(5)
        elif op == SIMD_PREFIX:
            sub = reader.leb(5)
            if sub not in SIMD_NAMES:
                raise WasmError(f"unknown 0xFD sub-opcode {sub} at byte {at}")
            if sub in _SIMD_MEMARG:
                reader.memarg()
            elif sub in _SIMD_MEMARG_LANE:
                reader.memarg()
                reader.byte()
            elif sub in _SIMD_LANE:
                reader.byte()
            elif sub in (12, 13):
                reader.skip(16)
            found[sub] += 1
        else:
            raise WasmError(f"unknown opcode 0x{op:02X} at byte {at}")
    if reader.pos != reader.end:
        raise WasmError(f"a function body ends at byte {reader.pos} and its "
                        f"declared size at byte {reader.end}")


def simd_opcodes(data: bytes) -> Counter:
    """Every SIMD sub-opcode in the module's function bodies, with counts.

    Raises `WasmError` for anything that is not a module this decoder can
    walk from the first section to the last byte.
    """
    if data[:8] != MAGIC:
        raise WasmError("the file does not start with the wasm magic and "
                        "version 1")
    reader = _Reader(data, 8)
    found: Counter = Counter()
    while reader.pos < reader.end:
        ident = reader.byte()
        size = reader.leb(5)
        start = reader.pos
        if ident == CODE_SECTION:
            code = _Reader(data, start, start + size)
            for _ in range(code.leb(5)):
                body_size = code.leb(5)
                body = _Reader(data, code.pos, code.pos + body_size)
                _walk_body(body, found)
                code.pos = body.end
            if code.pos != code.end:
                raise WasmError("the code section has bytes after its last "
                                "function body")
        _Reader(data, start, start + size)
        reader.pos = start + size
    return found


def opcode_name(sub: int) -> str:
    return SIMD_NAMES.get(sub, f"0xFD {sub}")


def describe(found: Counter) -> str:
    """`name xcount` per SIMD sub-opcode, in sub-opcode order."""
    return ", ".join(f"{opcode_name(sub)} x{found[sub]}"
                     for sub in sorted(found))


def inventory(path: Path) -> tuple[Counter | None, str | None]:
    if not path.is_file():
        return None, (f"{path} does not exist. The native arm's build step "
                      f"for this variant did not produce it.")
    try:
        return simd_opcodes(path.read_bytes()), None
    except WasmError as error:
        return None, f"{path} cannot be decoded as a wasm module: {error}."


def check(plain: Path, simd: Path) -> list[str]:
    """One refusal per way the pair fails the lead rulings."""
    problems: list[str] = []
    plain_ops, error = inventory(plain)
    if error:
        problems.append(error)
    simd_ops, error = inventory(simd)
    if error:
        problems.append(error)
    if plain_ops is None or simd_ops is None:
        return problems
    if plain_ops:
        problems.append(
            f"the plain module {plain} holds SIMD instructions, "
            f"{describe(plain_ops)}. The plain variant is built without "
            f"`+simd128` and must not contain any.")
    added = Counter({sub: n for sub, n in simd_ops.items()
                     if sub not in plain_ops})
    if not added:
        problems.append(
            f"the SIMD module {simd} holds no SIMD instruction the plain "
            f"module lacks. The build did not enable `+simd128` at code "
            f"generation, which under fat LTO is the example's link.")
    return problems


SIMD_RUSTFLAGS = ('CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS='
                  '"-C target-feature=+simd128"')
SIMD_TARGET_DIR = "--target-dir target/codec-wasm-simd"


def _native_arm(text: str) -> list[str] | None:
    """The native arm's commands, one per logical line, continuations joined.

    The arm runs from `  native)` to the next four-space `;;`, which is how
    bin/ocelli.sh writes every arm of its dispatch `case`.
    """
    lines = text.splitlines()
    try:
        start = lines.index("  native)")
    except ValueError:
        return None
    commands: list[str] = []
    pending = ""
    for line in lines[start + 1:]:
        if line == "    ;;":
            return commands
        stripped = line.strip()
        if not pending and (not stripped or stripped.startswith("#")):
            continue
        if stripped.endswith("\\"):
            pending += stripped[:-1].strip() + " "
            continue
        commands.append(pending + stripped)
        pending = ""
    return None


def build_shape_problems(text: str) -> list[str]:
    """A structural pin on the runner text, F-233 decision 1.

    F-233's lead ruling after the pass 1 repair: every SIMD variant step
    passes the feature through `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS`
    and none passes it after `cargo rustc --`. The module check cannot
    separate those two shapes, because under fat LTO both link a module with
    the same SIMD instructions, so this reads the arm's text instead. It pins
    the spelling and proves nothing about a build.
    """
    commands = _native_arm(text)
    if commands is None:
        return ["bin/ocelli.sh has no `  native)` arm ending at `    ;;`, so "
                "the SIMD variant steps cannot be read."]
    problems: list[str] = []
    steps = [c for c in commands if SIMD_TARGET_DIR in c]
    if not steps:
        problems.append(
            f"the native arm has no SIMD variant step, no command naming "
            f"`{SIMD_TARGET_DIR}`.")
    for step in steps:
        if not step.startswith(SIMD_RUSTFLAGS + " cargo build "):
            problems.append(
                f"a SIMD variant step does not start with `{SIMD_RUSTFLAGS} "
                f"cargo build`, which is the only spelling that reaches every "
                f"crate's cfg: {step}")
    for command in commands:
        head, sep, tail = command.partition(" -- ")
        if sep and "cargo rustc" in head and "target-feature" in tail:
            problems.append(
                f"a step passes a target feature after `cargo rustc --`, "
                f"which reaches the final crate only: {command}")
    return problems


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: simd_variant_check.py <plain.wasm> <simd.wasm>",
              file=sys.stderr)
        return 2
    plain, simd = Path(argv[0]), Path(argv[1])
    problems = check(plain, simd)
    if problems:
        print("FAIL: simd variant modules")
        for problem in problems:
            print(f"  {problem}")
        return 1
    found = simd_opcodes(simd.read_bytes())
    print(f"OK: {simd.name} SIMD variant holds {sum(found.values())} SIMD "
          f"instructions and the plain variant none: {describe(found)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
