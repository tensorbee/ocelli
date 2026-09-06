#!/usr/bin/env python3
"""Error codes are stable and versioned. HLD section 23, made mechanical.

Section 23 says:

    "Error codes are stable and versioned. The shell switches on the code, the
     message is for humans and may change."

The words are in the specification and the mechanism is not. Nothing in the
HLD stops the next story renumbering the space, and a renumbering is silent:
the crate compiles, the shell compiles, and a shell built yesterday reads
today's code as something plausible. That is the same defect shape as a pixel
that is quietly wrong, in the one place the user is told what went wrong.

`ci/error-codes.json` is the registry, in the same spirit as
`ci/wasm-size-budget.json`: a tracked file that this script compares reality
against. Three files have to agree, and they are edited by different hands:

    crates/ocelli-core/src/error.rs   the enum a Rust producer names
    packages/core/src/errors.ts       the table the shell renders
    ci/error-codes.json               the register that outlives both

## What it asserts

1. Every `ErrorCode` variant in the Rust enum appears in the registry with the
   same number, and every registry entry appears in the enum. A renumbering
   fails here.
2. Every registry entry has an entry in the TypeScript mirror with the same
   number, and a message keyed on that number. A code the shell cannot
   describe reaches a user as a bare number.
3. No number and no name is used twice.
4. Every entry names a declared crate range and its number falls inside it,
   and no two ranges overlap. This is what stops a second crate quietly
   picking an overlapping block.
5. `0` is never a code. `crates/ocelli-core/src/error.rs` refuses it at decode
   time so that a zeroed payload cannot decode as a real record, and the
   registry has to agree or that refusal is a lie.

## What it deliberately does not assert

**That the registry is append-only.** A removed code is a real event, and
turning it into a gate failure produces a gate somebody disables. What is
refused is REUSE of a number, which is the part that is silent.

**That the messages say anything in particular.** Section 23 says the message
is for humans and may change. Asserting its wording would make it a contract,
which is the thing that sentence exists to prevent.

Usage: python3 scripts/error_code_check.py
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REGISTRY_JSON = ROOT / "ci" / "error-codes.json"
REGISTRY_RUST = ROOT / "crates" / "ocelli-core" / "src" / "error.rs"
REGISTRY_TS = ROOT / "packages" / "core" / "src" / "errors.ts"

# `pub enum ErrorCode { ... }` up to its closing brace at column 0. Anchored on
# the brace rather than on a variant pattern, so a variant that stops matching
# is a missing variant rather than a silently smaller enum.
RUST_ENUM = re.compile(r"pub enum ErrorCode \{(.*?)^\}", re.M | re.S)
RUST_VARIANT = re.compile(r"^\s{4}([A-Z][A-Za-z0-9]*) = (\d+),\s*$", re.M)

# `export const ERROR_CODE = { ... } as const;`
TS_ENUM = re.compile(r"export const ERROR_CODE = \{(.*?)\} as const;", re.S)
TS_ENTRY = re.compile(r"^\s{2}([A-Z][A-Za-z0-9]*): (\d+),\s*$", re.M)

# `const MESSAGES: ... = { 1: "...", };`
TS_MESSAGES = re.compile(r"const MESSAGES[^=]*= \{(.*?)^\};", re.M | re.S)
TS_MESSAGE_KEY = re.compile(r"^\s{2}(\d+):", re.M)


def rust_codes(text: str) -> dict[str, int]:
    block = RUST_ENUM.search(text)
    if block is None:
        return {}
    return {m.group(1): int(m.group(2))
            for m in RUST_VARIANT.finditer(block.group(1))}


def typescript_codes(text: str) -> dict[str, int]:
    block = TS_ENUM.search(text)
    if block is None:
        return {}
    return {m.group(1): int(m.group(2))
            for m in TS_ENTRY.finditer(block.group(1))}


def typescript_messages(text: str) -> set[int]:
    block = TS_MESSAGES.search(text)
    if block is None:
        return set()
    return {int(m.group(1)) for m in TS_MESSAGE_KEY.finditer(block.group(1))}


def check(registry: dict, rust_text: str, typescript_text: str) -> list[str]:
    """Every disagreement between the three, as a list of sentences."""
    problems: list[str] = []
    ranges = registry.get("ranges", [])
    entries = registry.get("codes", [])

    # Ranges first, because a code's range assertion depends on them.
    by_crate: dict[str, tuple[int, int]] = {}
    for span in ranges:
        crate = str(span.get("crate", ""))
        first, last = int(span.get("first", 0)), int(span.get("last", 0))
        if crate in by_crate:
            problems.append(
                f"ci/error-codes.json declares the range for {crate} twice. "
                f"One block per crate, or the second one is unenforceable.")
        if first > last:
            problems.append(f"the range for {crate} runs backwards, "
                            f"{first} to {last}.")
        by_crate[crate] = (first, last)

    ordered = sorted(by_crate.items(), key=lambda kv: kv[1][0])
    for (crate_a, (_, last_a)), (crate_b, (first_b, _)) in zip(
            ordered, ordered[1:]):
        if first_b <= last_a:
            problems.append(
                f"the ranges for {crate_a} and {crate_b} overlap at "
                f"{first_b}. Overlapping blocks are how two crates end up "
                f"sharing a number without either one noticing.")

    # The registry's own consistency.
    seen_numbers: dict[int, str] = {}
    seen_names: set[str] = set()
    registry_codes: dict[str, int] = {}
    for entry in entries:
        name = str(entry.get("name", ""))
        number = int(entry.get("number", 0))
        crate = str(entry.get("crate", ""))

        if number == 0:
            problems.append(
                f"{name} is registered as code 0. `0` is never a valid code, "
                f"which is what makes a zeroed payload undecodable in "
                f"crates/ocelli-core/src/error.rs.")
        if number in seen_numbers:
            problems.append(
                f"code {number} is a reuse: {seen_numbers[number]} and {name} "
                f"both claim it. A reused number is how an old shell decodes "
                f"a new code as something plausible.")
        if name in seen_names:
            problems.append(
                f"the name {name} is a reuse. Two entries with one name "
                f"cannot both be resolved by a reader.")
        seen_numbers[number] = name
        seen_names.add(name)
        registry_codes[name] = number

        if crate not in by_crate:
            problems.append(
                f"{name} names the crate {crate}, which declares no range in "
                f"ci/error-codes.json. Every entry declares the range it "
                f"falls in, so a second crate cannot pick an overlapping "
                f"block.")
        else:
            first, last = by_crate[crate]
            if not first <= number <= last:
                problems.append(
                    f"{name} is code {number} and {crate}'s declared range is "
                    f"{first} to {last}. It is outside its own range.")

    # The Rust enum.
    rust = rust_codes(rust_text)
    if not rust:
        problems.append(
            f"no `pub enum ErrorCode` with numbered variants was found in "
            f"{REGISTRY_RUST.name}. Either it moved or it stopped matching "
            f"the shape this script reads, and both are worth knowing.")
    for name, number in sorted(rust.items()):
        if name not in registry_codes:
            problems.append(
                f"ErrorCode::{name} exists in Rust and is not in "
                f"ci/error-codes.json. Add the entry rather than leaving a "
                f"code the register does not know about.")
        elif registry_codes[name] != number:
            problems.append(
                f"ErrorCode::{name} is {number} in Rust and "
                f"{registry_codes[name]} in ci/error-codes.json. Section 23 "
                f"says codes are stable, so this is a renumbering.")
    for name in sorted(set(registry_codes) - set(rust)):
        problems.append(
            f"{name} is registered and has no ErrorCode variant in "
            f"{REGISTRY_RUST.name}.")

    # The TypeScript mirror and its message table.
    typescript = typescript_codes(typescript_text)
    messages = typescript_messages(typescript_text)
    if not typescript:
        problems.append(
            f"no `export const ERROR_CODE` object was found in "
            f"{REGISTRY_TS.name}.")
    for name, number in sorted(registry_codes.items()):
        if name not in typescript:
            problems.append(
                f"{name} is registered and the shell has no entry for it in "
                f"{REGISTRY_TS.name}. A code the shell cannot name reaches a "
                f"user as a number.")
        elif typescript[name] != number:
            problems.append(
                f"{name} is {number} in ci/error-codes.json and "
                f"{typescript[name]} in {REGISTRY_TS.name}.")
        if number not in messages:
            problems.append(
                f"{name} is code {number} and the MESSAGES table in "
                f"{REGISTRY_TS.name} has no line for it. Section 23 puts the "
                f"human text on the shell side, so an absent line is an "
                f"error nobody can read.")
    for name in sorted(set(typescript) - set(registry_codes)):
        problems.append(
            f"{REGISTRY_TS.name} names {name}, which is not in "
            f"ci/error-codes.json.")

    return problems


def load_registry() -> dict:
    return json.loads(REGISTRY_JSON.read_text(encoding="utf-8"))


def main() -> int:
    try:
        registry = load_registry()
        rust_text = REGISTRY_RUST.read_text(encoding="utf-8")
        typescript_text = REGISTRY_TS.read_text(encoding="utf-8")
    except (OSError, json.JSONDecodeError) as exc:
        print("FAIL: the error-code registry could not be read")
        print(f"  {exc}")
        return 1

    problems = check(registry, rust_text, typescript_text)
    if problems:
        print("FAIL: the error-code registry and the code disagree")
        for problem in problems:
            print(f"  {problem}")
        print()
        print("Codes are stable and versioned (HLD section 23). Adding one is")
        print("an appended entry in ci/error-codes.json, a variant in")
        print("crates/ocelli-core/src/error.rs and a line in")
        print("packages/core/src/errors.ts. Changing one is not.")
        return 1

    count = len(registry.get("codes", []))
    spans = len(registry.get("ranges", []))
    print(f"OK: {count} error code(s) agree across Rust, TypeScript and the "
          f"registry, inside {spans} declared crate range(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
