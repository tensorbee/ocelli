#!/usr/bin/env python3
"""Split docs/Ocelli-HLD.docx into the numbered file set under docs/hld/.

The .docx is the author's original and stays the source. This script converts
it with pandoc and cuts it at its own top-level section boundaries, preserving
document order exactly. Nothing is reordered, reworded or summarised, because a
specification that has been paraphrased is no longer the specification.

`--check` re-derives the split and asserts two things:

  1. Every non-blank line of the converted document appears in exactly one
     output file. A section silently dropped by a mapping edit fails here.
  2. The tracked files are byte-identical to the re-derived ones.

Usage:
  python3 scripts/split_hld.py            # write docs/hld/*.md
  python3 scripts/split_hld.py --check    # assert no drift, no lost content
"""

from __future__ import annotations

import argparse
import json
import re
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HLD = ROOT / "docs" / "hld"

# The authored .docx lives OUTSIDE the repository. It carries Appendix C, the
# competitive and commercial analysis, which is not published with the library.
# See the private folder's README. Default location, overridable:
sys.path.insert(0, str(Path(__file__).resolve().parent))
from source_dir import resolve as _resolve_source  # noqa: E402

SOURCE_DIR, _SOURCE_ORIGIN = _resolve_source()
DOCX = SOURCE_DIR / "Ocelli-HLD.docx"

# Sections cut from the .docx but NOT written into the repository. They are
# written into the private folder instead, so they stay derivable from the
# source without being published.
#
# Appendix C is competitor pricing, satisfaction scores and go-to-market
# analysis. Its one normative part, the C.2.1 source-provenance table, was
# extracted into docs/SOURCE-POLICY.md before this exclusion was added, so the
# read-block on dwv, Horos and Grok survives and the guard still enforces it.
PRIVATE_SECTIONS = {"C-competitive-position.md"}

# Commercial product names are competitive intelligence and are not published
# with the library. Each replacement keeps the ENGINEERING point intact, which
# is the whole reason the sentence is in a design document: a vendor's
# published memory constant is evidence for a design decision, and the
# evidence survives without the name.

# Redaction rules live with the private source documents, NOT here. A map of
# what was redacted still contains what was redacted, so publishing it would
# undo the redaction it performs. The generators cannot run without the private
# source anyway, so the rules travel with it.
def _load_redactions() -> list[tuple[str, str]]:
    path = SOURCE_DIR / "redactions.json"
    if not path.exists():
        return []
    return [(r[0], r[1]) for r in json.loads(path.read_text())["rules"]]

def redact(text: str) -> str:
    """Strip commercial product names from repository-bound output."""
    for old, new in _load_redactions():
        text = text.replace(old, new)
    return text
EXIT_SKIPPED = 3

# Top-level section key -> output filename. Keys are matched against the
# section label at the start of a line in the pandoc output. Order here is
# the document's own order and must stay that way.
SECTIONS: list[tuple[str, str]] = [
    ("Part I", "01-purpose-and-scope.md"),
    ("1", "01-purpose-and-scope.md"),
    ("2", "02-standing-decisions.md"),
    ("3", "03-architecture-and-crates.md"),
    ("4", "03-architecture-and-crates.md"),
    ("5", "04-boundary-and-data-path.md"),
    ("6", "04-boundary-and-data-path.md"),
    ("7", "05-rendering.md"),
    ("8", "06-memory-and-cache.md"),
    ("9", "07-concurrency-and-typescript.md"),
    ("10", "07-concurrency-and-typescript.md"),
    ("11", "08-validation-architecture.md"),
    ("12", "09-migration.md"),
    ("13", "10-extension-points.md"),
    ("14", "11-decision-log.md"),
    ("Part II", "12-workspace-and-build.md"),
    ("15", "12-workspace-and-build.md"),
    ("16", "13-core-types.md"),
    ("17", "14-the-boundary-in-code.md"),
    ("18", "15-lut-chain.md"),
    ("19", "16-volume-representation.md"),
    ("20", "17-cache-and-allocation.md"),
    ("21", "18-codec-registry.md"),
    ("22", "19-render-graph.md"),
    ("23", "20-errors-and-panics.md"),
    ("24", "21-worker-protocol.md"),
    ("25", "22-testing-and-tolerance.md"),
    ("26", "23-performance-rules.md"),
    ("27", "24-agent-code-standards.md"),
    ("28", "25-first-ten-files.md"),
    ("Appendix A", "A-spike-gates.md"),
    ("Appendix B", "B-parity-surface.md"),
    ("Part III", "26-differentiating-capabilities.md"),
    ("30", "26-differentiating-capabilities.md"),
    ("31", "26-differentiating-capabilities.md"),
    ("32", "26-differentiating-capabilities.md"),
    ("33", "26-differentiating-capabilities.md"),
    ("34", "26-differentiating-capabilities.md"),
    ("35", "26-differentiating-capabilities.md"),
    ("36", "26-differentiating-capabilities.md"),
    ("37", "26-differentiating-capabilities.md"),
    ("38", "27-phase1-hooks.md"),
    ("Appendix C", "C-competitive-position.md"),
]

TITLES = {
    "01-purpose-and-scope.md": "Purpose and scope",
    "02-standing-decisions.md": "Standing decisions",
    "03-architecture-and-crates.md": "System architecture and crate layout",
    "04-boundary-and-data-path.md": "The boundary contract and the data path",
    "05-rendering.md": "Rendering",
    "06-memory-and-cache.md": "Memory and cache",
    "07-concurrency-and-typescript.md": "Concurrency, and what stays TypeScript",
    "08-validation-architecture.md": "Validation architecture",
    "09-migration.md": "Migration",
    "10-extension-points.md": "Designed-in extension points",
    "11-decision-log.md": "Decision log",
    "12-workspace-and-build.md": "Workspace and build",
    "13-core-types.md": "Core types, coordinate spaces and value spaces",
    "14-the-boundary-in-code.md": "The boundary, in code",
    "15-lut-chain.md": "The LUT chain",
    "16-volume-representation.md": "Volume representation",
    "17-cache-and-allocation.md": "Cache and allocation discipline",
    "18-codec-registry.md": "Codec registry",
    "19-render-graph.md": "Render graph",
    "20-errors-and-panics.md": "Error and panic policy",
    "21-worker-protocol.md": "Worker protocol",
    "22-testing-and-tolerance.md": "Testing and the tolerance policy",
    "23-performance-rules.md": "Performance rules",
    "24-agent-code-standards.md": "Standards for agent-generated code",
    "25-first-ten-files.md": "The first ten files",
    "26-differentiating-capabilities.md": "Differentiating capabilities (Part III)",
    "27-phase1-hooks.md": "The hooks Phase 1 must include",
    "A-spike-gates.md": "Appendix A, open questions and spike gates",
    "B-parity-surface.md": "Appendix B, parity surface",
    "C-competitive-position.md": "Appendix C, competitive position",
}

HEADER = """<!-- Generated by scripts/split_hld.py from Ocelli-HLD.docx.
     Do not hand-edit. Change the .docx and re-run the script, or the
     /verify docs gate will fail.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the .docx wins. -->

# {title}

**Source**: `Ocelli-HLD.docx`, {sections}. The authored document is
held outside this repository, see `docs/SOURCE-POLICY.md`.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

"""

ESCAPE_RE = re.compile(r"\\([\\`*_{}\[\]()#+\-.!<>|~])")

# The .docx renders every code listing as a Word style pandoc emits as a
# blockquote, with markdown escaping applied to the code text. All 21 runs in
# the document are code, verified by inspection, and none is prose. Left as
# blockquotes, `\#\[wasm_bindgen\]` and `-\>` reach a reader who is meant to
# copy the signature exactly, which is the one thing this document exists to
# make possible. So each run is fenced and unescaped.
CODE_MARKERS = ("{", "}", "//", "#[", "fn ", "pub ", "struct", "let ",
                "->", "-\\>", "=", "\\#", "impl", "/")


def fence_language(body: str) -> str:
    if "wgsl" in body or "@group(" in body or "var<uniform>" in body:
        return "wgsl"
    if "#!/usr/bin/env bash" in body or "cargo tree" in body:
        return "bash"
    if "[workspace" in body or "opt-level" in body or "= \"z\"" in body:
        return "toml"
    if "const " in body and "session." in body:
        return "typescript"
    if "postMessage" in body or "main ->" in body:
        return "text"
    if any(m in body for m in ("pub fn", "pub struct", "impl", "#[", "use ")):
        return "rust"
    return "text"


def tidy_code(stripped: list[str]) -> list[str]:
    """Undo two artefacts of the .docx, deterministically.

    Word stores each code line as its own paragraph, so pandoc separates every
    pair with a blank line and the listing arrives double-spaced. A genuine
    blank line in the original is an empty paragraph and arrives as two. So a
    single internal blank collapses away and a run of two or more becomes one.

    Word also carries indentation as paragraph formatting rather than as
    leading spaces, so every line arrives flush-left. Indentation is
    re-derived from bracket depth. It is PRESENTATION and not the author's
    bytes, which is why the generated header says so. It cannot change meaning
    in Rust, TypeScript, WGSL or TOML.
    """
    collapsed: list[str] = []
    blanks = 0
    for line in stripped:
        if not line.strip():
            blanks += 1
            continue
        if blanks >= 2 and collapsed:
            collapsed.append("")
        blanks = 0
        collapsed.append(line.rstrip())

    out: list[str] = []
    depth = 0
    for line in collapsed:
        if not line:
            out.append("")
            continue
        opens = sum(line.count(c) for c in "{[(")
        closes = sum(line.count(c) for c in "}])")
        lead = depth - (1 if line[0] in "}])" else 0)
        out.append("    " * max(lead, 0) + line)
        depth = max(depth + opens - closes, 0)
    return out


def fence_blockquotes(lines: list[str]) -> list[str]:
    """Turn each blockquote run into a fenced, unescaped code block."""
    out: list[str] = []
    run: list[str] = []

    def flush() -> None:
        if not run:
            return
        stripped = [ESCAPE_RE.sub(r"\1", l[1:].lstrip(" ") if len(l) > 1 else "")
                    for l in run]
        body = "\n".join(stripped)
        if sum(1 for m in CODE_MARKERS if m in body) < 2:
            out.extend(run)          # not code, leave the blockquote alone
        else:
            tidied = tidy_code(stripped)
            out.append(f"```{fence_language('\n'.join(tidied))}")
            out.extend(tidied)
            out.append("```")
        run.clear()

    for line in lines:
        if line.startswith(">"):
            run.append(line)
            continue
        flush()
        out.append(line)
    flush()
    return out


SECTION_RE = re.compile(r"^(\d+)\\?\.\s+(.*\S)\s*$")
SUBSECTION_RE = re.compile(r"^(\d+\.\d+)\s+(.*\S)\s*$")
PART_RE = re.compile(r"^(Part [IVX]+|Appendix [A-Z])\s*[—-]\s*(.*\S)\s*$")


ABS_MEDIA = re.compile(r'src="[^"]*/(media/[^"]+)"')


def relativise_media(lines: list[str]) -> list[str]:
    """Rewrite absolute figure paths to repository-relative ones.

    pandoc's --extract-media is given an absolute directory, so it emits
    `src="/Users/.../docs/hld/media/x.png"`. That link resolves on exactly one
    machine, and it publishes that machine's directory layout and username.
    """
    return [ABS_MEDIA.sub(r'src="\1"', line) for line in lines]


def convert() -> list[str]:
    if shutil.which("pandoc") is None:
        sys.exit("pandoc is required to split the HLD")
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "hld.md"
        subprocess.run(
            ["pandoc", "-f", "docx", "-t", "gfm", "--wrap=none",
             f"--extract-media={HLD}", str(DOCX), "-o", str(out)],
            check=True,
        )
        return out.read_text().splitlines()


def cut(lines: list[str]) -> tuple[dict[str, list[str]], list[str], list[int]]:
    """Return (filename -> body lines, front matter, per-line owner index)."""
    mapping = dict(SECTIONS)
    order = []
    for _key, name in SECTIONS:
        if name not in order:
            order.append(name)

    bodies: dict[str, list[str]] = {name: [] for name in order}
    front: list[str] = []
    owner: list[int] = []
    current: str | None = None

    for line in lines:
        label = None
        part = PART_RE.match(line)
        section = SECTION_RE.match(line)
        if part:
            label = part.group(1)
        elif section and section.group(1) in mapping:
            label = section.group(1)

        if label is not None and label in mapping:
            current = mapping[label]

        if current is None:
            front.append(line)
            owner.append(-1)
            continue

        rendered = line
        if part:
            rendered = f"## {part.group(1)}, {part.group(2)}"
        elif section and section.group(1) in mapping:
            rendered = f"## {section.group(1)}. {section.group(2)}"
        else:
            sub = SUBSECTION_RE.match(line)
            if sub:
                rendered = f"### {sub.group(1)} {sub.group(2)}"

        bodies[current].append(rendered)
        owner.append(order.index(current))

    return bodies, front, owner


def source_sections(name: str) -> str:
    keys = [k for k, n in SECTIONS if n == name]
    numeric = [k for k in keys if k.isdigit()]
    other = [k for k in keys if not k.isdigit()]
    bits = []
    if other:
        bits.extend(other)
    if numeric:
        if len(numeric) == 1:
            bits.append(f"section {numeric[0]}")
        else:
            bits.append(f"sections {numeric[0]} to {numeric[-1]}")
    return ", ".join(bits)


def render(bodies: dict[str, list[str]]) -> dict[str, str]:
    out = {}
    for name, body in bodies.items():
        text = "\n".join(body).strip("\n")
        header = HEADER.format(title=TITLES[name], sections=source_sections(name))
        out[name] = header + text + "\n"
    return out


def render_readme(bodies: dict[str, list[str]], front: list[str]) -> str:
    lines = [
        "# Ocelli high-level design",
        "",
        "The authored document is `Ocelli-HLD.docx`, held **outside this",
        "repository**. These files are cut from it by `scripts/split_hld.py`,",
        "in document order, with nothing reordered or reworded.",
        "",
        "**Appendix C is deliberately absent.** It is competitor pricing and",
        "go-to-market analysis, and it is not published with the library. Its",
        "one normative part, the source-provenance table, lives in",
        "`docs/SOURCE-POLICY.md` and the build still enforces it.",
        "",
        "To regenerate or verify, point at the source and re-run:",
        "",
        "```bash",
        "python3 scripts/source_dir.py --set /path/to/source-documents",
        "python3 scripts/split_hld.py",
        "```",
        "",
        "The path is recorded per clone in `.ocelli-source-path`, which is",
        "gitignored, so moving the documents is a one-command fix rather than",
        "an environment variable to remember every session.",
        "",
        "Without it the `docs` gate SKIPS with a stated reason. A check that",
        "cannot run is not a check that passed.",
        "",
        "`python3 scripts/split_hld.py --check` asserts every line of the",
        "converted document lands in exactly one file here, so a mapping edit",
        "cannot silently drop a section.",
        "",
        "## Document header",
        "",
    ]
    lines.extend(line for line in front if line.strip())
    lines.extend([
        "",
        "## Files",
        "",
        "| File | Covers |",
        "|------|--------|",
    ])
    for name in bodies:
        lines.append(f"| [`{name}`]({name}) | {TITLES[name]}, {source_sections(name)} |")
    lines.extend([
        "",
        "## Deviations from this document",
        "",
        "Recorded rather than silently applied. See `docs/hld/DEVIATIONS.md`.",
        "",
    ])
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    if not DOCX.exists():
        print(f"SKIPPED: the authored source is not present at {DOCX}.")
        print(f"Resolved from: {_SOURCE_ORIGIN}. It lives outside the")
        print("repository because it carries Appendix C,")
        print("commercial analysis. Record its location with")
        print("`python3 scripts/source_dir.py --set PATH` to regenerate or")
        print("verify docs/hld. A check that cannot run is NOT a check that")
        print("passed, which is why this exits 3 and not 0.")
        return EXIT_SKIPPED

    lines = relativise_media(fence_blockquotes(convert()))
    bodies, front, owner = cut(lines)

    orphans = [
        line for line, who in zip(lines, owner)
        if who == -1 and line.strip()
    ]
    expected_front = len([line for line in front if line.strip()])
    if len(orphans) != expected_front:
        print("FAIL: content fell outside every mapped section")
        return 1

    files = render(bodies)
    files["README.md"] = render_readme(bodies, front)

    empty = [name for name, body in bodies.items() if not body]
    if empty:
        print("FAIL: mapped to no content: " + ", ".join(empty))
        return 1

    if args.check:
        drift = []
        for name, text in files.items():
            if name in PRIVATE_SECTIONS:
                continue
            path = HLD / name
            if not path.exists() or path.read_text() != redact(text):
                drift.append(name)
        if drift:
            print("FAIL: docs/hld drifted from the .docx: " + ", ".join(drift))
            print("      run: python3 scripts/split_hld.py")
            return 1
        print(f"OK: {len(files) - len(PRIVATE_SECTIONS)} files match the "
              f".docx, {len(PRIVATE_SECTIONS)} held privately, "
              f"{sum(len(b) for b in bodies.values())} lines placed")
        return 0

    private_dir = SOURCE_DIR.parent / "commercial"
    written = 0
    for name, text in files.items():
        if name in PRIVATE_SECTIONS:
            private_dir.mkdir(parents=True, exist_ok=True)
            (private_dir / name).write_text(text)
            continue
        (HLD / name).write_text(redact(text))
        written += 1
    print(f"wrote {written} files to docs/hld/, "
          f"{len(PRIVATE_SECTIONS)} to {private_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
