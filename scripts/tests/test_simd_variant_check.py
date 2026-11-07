#!/usr/bin/env python3
"""Tests for scripts/simd_variant_check.py.

    python3 -B -m unittest discover -s scripts/tests \
      -p 'test_simd_variant_check.py'

Needs nothing but the standard library. Every case hand-assembles a minimal
WebAssembly module and calls `simd_opcodes` or `check` directly, so nothing
here builds anything or reads a real target directory.

**Expected values are from F-233's lead rulings after review pass 1 and from
the WebAssembly binary format, not from the implementation.** The rulings: the
check reads the final linked modules, goes red when the SIMD module holds no
SIMD instruction the plain module lacks, and goes red when the plain module
holds any SIMD instruction. The binary format, WebAssembly Core Specification
2.0 section 5.4: a SIMD instruction is the prefix byte `0xFD` followed by a
LEB128 `u32` sub-opcode, `v128.const` is sub-opcode 12 with a 16-byte
immediate, `v128.load` is 0 and `v128.store` is 11 with a memarg,
`i32x4.add` is 174 with no immediate, `memory.copy` is `0xFC 10` with two
memory indices, and `i32.const` takes a signed LEB128 whose two-byte encoding
of 125 is `0xFD 0x00`. The modules stand in for the two that `bin/ocelli.sh
native` steps 7 and 11 link, `examples/<name>.wasm` under each variant's
release wasm32 target directory, reduced to one type, one function per body
and a code section.
"""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import simd_variant_check  # noqa: E402

END = b"\x0b"
DROP = b"\x1a"
V128_CONST_ZERO = b"\xfd\x0c" + bytes(16)
I32_CONST_0 = b"\x41\x00"
# i32.const 125. Its signed LEB128 is 0xFD 0x00, so a scanner that counts
# 0xFD bytes instead of decoding instructions would see a SIMD prefix here.
I32_CONST_125 = b"\x41\xfd\x00"
# v128.load and v128.store, each with align 4 and offset 0.
V128_LOAD = b"\xfd\x00\x04\x00"
V128_STORE = b"\xfd\x0b\x04\x00"
I32X4_ADD = b"\xfd\xae\x01"
# memory.copy 0 0, the bulk-memory instruction a `+bulk-memory` build emits.
MEMORY_COPY = b"\xfc\x0a\x00\x00"


def uleb(value: int) -> bytes:
    out = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        if value:
            out.append(byte | 0x80)
        else:
            out.append(byte)
            return bytes(out)


def section(ident: int, payload: bytes) -> bytes:
    return bytes([ident]) + uleb(len(payload)) + payload


def module(*bodies: bytes) -> bytes:
    """A module with one `() -> ()` function per body.

    Each body is the instruction bytes after the local declarations, `end`
    included. The memory section exists so the loads and stores are valid.
    The custom section carries two 0xFD bytes and exists so the walker is
    shown to skip a section it does not read. Each function declares one
    `v128` local, which is SIMD syntax but not an instruction and must not
    be counted.
    """
    header = b"\x00asm\x01\x00\x00\x00"
    types = section(1, uleb(1) + b"\x60\x00\x00")
    funcs = section(3, uleb(len(bodies)) + b"".join(uleb(0) for _ in bodies))
    mem = section(5, uleb(1) + b"\x00\x01")
    entries = b""
    for body in bodies:
        entry = uleb(1) + uleb(1) + b"\x7b" + body
        entries += uleb(len(entry)) + entry
    code = section(10, uleb(len(bodies)) + entries)
    name = b"probe"
    custom = section(0, uleb(len(name)) + name + b"\xfd\xfd")
    return header + types + funcs + mem + custom + code


class SimdOpcodes(unittest.TestCase):

    def test_v128_const_is_found_and_named(self) -> None:
        found = simd_variant_check.simd_opcodes(
            module(V128_CONST_ZERO + DROP + END))
        self.assertEqual(dict(found), {12: 1})
        self.assertEqual(simd_variant_check.opcode_name(12), "v128.const")

    def test_an_fd_byte_inside_an_immediate_is_not_an_instruction(
            self) -> None:
        # Red on a byte scan: the module holds 0xFD as an LEB128 byte and in
        # a custom section, and no SIMD instruction.
        data = module(I32_CONST_125 + DROP + END)
        self.assertIn(b"\xfd", data)
        self.assertEqual(dict(simd_variant_check.simd_opcodes(data)), {})

    def test_a_v128_const_immediate_is_sixteen_bytes(self) -> None:
        # Sixteen 0xFD bytes. Consuming fifteen would read the last as a
        # second SIMD prefix.
        found = simd_variant_check.simd_opcodes(
            module(b"\xfd\x0c" + b"\xfd" * 16 + DROP + END))
        self.assertEqual(dict(found), {12: 1})

    def test_a_lane_immediate_is_consumed(self) -> None:
        # Lane index 2 is also the `block` opcode, so a decoder that drops a
        # lane byte opens a block that never closes. v128.load8_lane is
        # sub-opcode 84 with a memarg and a lane byte, and
        # i8x16.extract_lane_s is 21 with a lane byte alone.
        body = (I32_CONST_0 + V128_CONST_ZERO + b"\xfd\x54\x00\x00\x02"
                + b"\xfd\x15\x02" + DROP + END)
        found = simd_variant_check.simd_opcodes(module(body))
        self.assertEqual(dict(found), {12: 1, 84: 1, 21: 1})

    def test_every_body_is_decoded(self) -> None:
        found = simd_variant_check.simd_opcodes(module(
            I32_CONST_0 + V128_LOAD + I32_CONST_0 + V128_STORE + END,
            I32_CONST_0 + I32_CONST_0 + V128_LOAD + I32X4_ADD + V128_STORE
            + END))
        self.assertEqual(dict(found), {0: 2, 11: 2, 174: 1})
        self.assertEqual(simd_variant_check.opcode_name(174), "i32x4.add")

    def test_a_bulk_memory_instruction_is_not_simd(self) -> None:
        body = I32_CONST_0 + I32_CONST_0 + I32_CONST_0 + MEMORY_COPY + END
        self.assertEqual(
            dict(simd_variant_check.simd_opcodes(module(body))), {})

    def test_an_unknown_opcode_is_refused_not_skipped(self) -> None:
        # 0xC5 is unassigned in the core opcode space. Skipping it would
        # lose the position of every instruction after it.
        with self.assertRaises(simd_variant_check.WasmError):
            simd_variant_check.simd_opcodes(module(b"\xc5" + END))

    def test_a_body_without_its_final_end_is_refused(self) -> None:
        with self.assertRaises(simd_variant_check.WasmError):
            simd_variant_check.simd_opcodes(module(V128_CONST_ZERO + DROP))

    def test_a_truncated_module_is_refused(self) -> None:
        data = module(V128_CONST_ZERO + DROP + END)
        with self.assertRaises(simd_variant_check.WasmError):
            simd_variant_check.simd_opcodes(data[:-1])

    def test_every_malformed_body_is_refused(self) -> None:
        # One case per way a body can fail to decode. Each would otherwise
        # shift every later instruction boundary, which is how a SIMD
        # instruction goes uncounted or an immediate byte gets counted.
        cases = {
            "an immediate past the body's end": b"\xfd\x0c" + bytes(3),
            "an LEB128 longer than ten bytes": b"\x41" + b"\x80" * 10
            + b"\x00" + DROP + END,
            "a reserved 0xFD sub-opcode, 154": b"\xfd\x9a\x01" + END,
            "an unassigned 0xFC sub-opcode, 18": b"\xfc\x12" + END,
            "bytes after the final end": I32_CONST_0 + DROP + END + b"\x01",
        }
        for label, body in cases.items():
            with self.subTest(label):
                with self.assertRaises(simd_variant_check.WasmError):
                    simd_variant_check.simd_opcodes(module(body))

    def test_bytes_after_the_last_body_are_refused(self) -> None:
        data = bytearray(module(I32_CONST_0 + DROP + END))
        # The code section is last. Grow its declared size by one and append
        # a byte, so every body decodes and the section still has a tail.
        size_at = data.rindex(b"\x0a") + 1
        data[size_at] += 1
        data.append(0)
        with self.assertRaises(simd_variant_check.WasmError):
            simd_variant_check.simd_opcodes(bytes(data))

    def test_a_file_that_is_not_wasm_is_refused(self) -> None:
        with self.assertRaises(simd_variant_check.WasmError):
            simd_variant_check.simd_opcodes(b"\x7fELF" + bytes(12))


class Check(unittest.TestCase):

    def run_check(self, plain: bytes | None,
                  simd: bytes | None) -> list[str]:
        with tempfile.TemporaryDirectory() as name:
            base = Path(name)
            plain_path, simd_path = base / "plain.wasm", base / "simd.wasm"
            if plain is not None:
                plain_path.write_bytes(plain)
            if simd is not None:
                simd_path.write_bytes(simd)
            return simd_variant_check.check(plain_path, simd_path)

    def test_a_simd_module_with_simd_instructions_passes(self) -> None:
        problems = self.run_check(module(I32_CONST_125 + DROP + END),
                                  module(V128_CONST_ZERO + DROP + END))
        self.assertEqual(problems, [])

    def test_a_simd_module_without_simd_instructions_is_refused(self) -> None:
        # The state a build that lost `+simd128` everywhere produces, and the
        # state the review's `+bulk-memory` build produced.
        body = I32_CONST_0 + I32_CONST_0 + I32_CONST_0 + MEMORY_COPY + END
        problems = self.run_check(module(body), module(body))
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("no SIMD instruction", problems[0])

    def test_a_plain_module_with_simd_instructions_is_refused(self) -> None:
        problems = self.run_check(module(V128_CONST_ZERO + DROP + END),
                                  module(I32_CONST_0 + I32_CONST_0 + V128_LOAD
                                         + I32X4_ADD + V128_STORE + END))
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("plain", problems[0])
        self.assertIn("v128.const", problems[0])

    def test_simd_instructions_the_plain_module_also_holds_are_not_enough(
            self) -> None:
        same = module(V128_CONST_ZERO + DROP + END)
        problems = self.run_check(same, same)
        self.assertEqual(len(problems), 2, problems)
        self.assertTrue(any("plain" in p for p in problems), problems)
        self.assertTrue(any("no SIMD instruction" in p for p in problems),
                        problems)

    def test_a_missing_module_is_refused_and_named(self) -> None:
        problems = self.run_check(module(I32_CONST_0 + DROP + END), None)
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("simd.wasm", problems[0])
        self.assertIn("does not exist", problems[0])

    def test_an_undecodable_module_is_refused(self) -> None:
        problems = self.run_check(module(I32_CONST_0 + DROP + END),
                                  module(b"\xc5" + END))
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("cannot be decoded", problems[0])


if __name__ == "__main__":
    unittest.main()
