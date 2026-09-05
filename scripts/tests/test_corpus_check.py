#!/usr/bin/env python3
"""Tests for scripts/corpus_check.py --coverage.

Needs nothing but the standard library, because the coverage mode reads the
manifest and nothing else. That is the point of it: deviation D-04 means CI has
no corpus and no GPU, and coverage is the part of F-009 that CI can still see.

    python3 -m unittest discover -s scripts/tests -v

Every case here builds a manifest in a temporary file and points the module at
it. Nothing touches the real corpus/manifest.tsv.
"""

from __future__ import annotations

import io
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import corpus_check  # noqa: E402
import populate_corpus  # noqa: E402


# The advisory the report prints when the real layer has no chroma anywhere.
# Keyed on the note itself, not on the word "chroma", which also appears in the
# always-printed counts line and would make the negative assertion vacuous.
CHROMA_NOTE = "NOTE: every real class-two case is greyscale"


def row(path: str, modality: str, syntax: str, category: str) -> str:
    return "\t".join([path, modality, syntax, category, "synthetic, F-009",
                      "CC0-1.0", "https://creativecommons.org/publicdomain/zero/1.0/",
                      "0" * 64, ""])


def complete_rows() -> list[str]:
    """A manifest that satisfies every coverage condition, minimally."""
    rows = []
    for index, syntax in enumerate(corpus_check.REGISTRY_TRANSFER_SYNTAXES):
        rows.append(row(f"syntax/case{index}.dcm", "CT", syntax,
                        "synthetic, mono16"))
    rows.append(row("synthetic/colour.dcm", "OT", "1.2.840.10008.1.2.1",
                    "synthetic, colour"))
    rows.append(row("real/ct/1.dcm", "CT", "1.2.840.10008.1.2.1",
                    "real, mono16, burned-in-unchecked"))
    rows.append(row("real/us/1.dcm", "US", "1.2.840.10008.1.2.1",
                    "real, us, burned-in-unchecked"))
    return rows


class CoverageMode(unittest.TestCase):

    def coverage(self, rows: list[str]) -> tuple[int, str]:
        """Run --coverage over a temporary manifest, returning status and text."""
        with tempfile.TemporaryDirectory(prefix="ocelli-manifest-") as tmp:
            manifest = Path(tmp) / "manifest.tsv"
            manifest.write_text("\n".join([corpus_check.HEADER, *rows]) + "\n",
                                encoding="utf-8")
            original = corpus_check.MANIFEST
            corpus_check.MANIFEST = manifest
            try:
                captured = io.StringIO()
                with redirect_stdout(captured):
                    status = corpus_check.coverage(corpus_check.load())
            finally:
                corpus_check.MANIFEST = original
        return status, captured.getvalue()

    def test_a_complete_manifest_passes(self) -> None:
        status, text = self.coverage(complete_rows())
        self.assertEqual(status, 0, text)

    def test_a_missing_transfer_syntax_fails_and_is_named(self) -> None:
        """Removing one row must name that syntax, not just report a count.
        A coverage report that says "something is missing" is a coverage
        report nobody acts on."""
        rows = complete_rows()
        dropped = corpus_check.REGISTRY_TRANSFER_SYNTAXES[9]
        rows = [r for r in rows if r.split("\t")[2] != dropped
                or r.split("\t")[0].startswith(("real/", "synthetic/"))]
        status, text = self.coverage(rows)
        self.assertEqual(status, 1)
        self.assertIn(dropped, text)

    def test_every_registry_syntax_is_reported_when_the_manifest_is_empty(self) -> None:
        status, text = self.coverage([])
        self.assertEqual(status, 1)
        for syntax in corpus_check.REGISTRY_TRANSFER_SYNTAXES:
            self.assertIn(syntax, text)

    def test_a_synthetic_only_manifest_fails(self) -> None:
        """A corpus that has never seen a real vendor file has never seen
        padding, private blocks or an odd-length value."""
        rows = [r for r in complete_rows() if not r.startswith("real/")]
        rows.append(row("synthetic/us.dcm", "US", "1.2.840.10008.1.2.1",
                        "synthetic, us"))
        status, text = self.coverage(rows)
        self.assertEqual(status, 1)
        self.assertIn("not synthetic", text)

    def test_a_manifest_with_no_colour_or_ultrasound_fails(self) -> None:
        """HLD section 25.1 sets a different tolerance for this class, and an
        untested class has an untested tolerance."""
        rows = [r for r in complete_rows()
                if "colour" not in r.split("\t")[3]
                and " us" not in r.split("\t")[3]]
        status, text = self.coverage(rows)
        self.assertEqual(status, 1)
        # Not "colour or ultrasound": the always-printed counts line contains
        # it, so that assertion would hold on a passing run too.
        self.assertIn("the corpus has no colour or ultrasound", text)

    def test_a_manifest_with_no_monochrome_16_bit_fails(self) -> None:
        rows = [r.replace("mono16", "colour") for r in complete_rows()]
        status, text = self.coverage(rows)
        self.assertEqual(status, 1)
        self.assertIn("the corpus has no monochrome 16-bit", text)

    def test_real_rows_covering_only_one_class_fail(self) -> None:
        """The synthetic layer can satisfy both classes on its own. The real
        layer has to as well, or the second class is only ever exercised
        against bytes this repository generated."""
        rows = [r for r in complete_rows() if not r.startswith("real/us/")]
        rows.append(row("synthetic/us2.dcm", "US", "1.2.840.10008.1.2.1",
                        "synthetic, us"))
        status, text = self.coverage(rows)
        self.assertEqual(status, 1)
        # Not just "real": that word appears in the always-printed summary
        # line "44 of them real", so asserting it would pass on a green run.
        self.assertIn("the real layer has no colour or ultrasound", text)

    def test_a_row_with_no_tolerance_class_token_fails(self) -> None:
        """A row that declares no class cannot be counted for or against
        either one, so it is a hole rather than a case."""
        rows = complete_rows()
        rows.append(row("synthetic/unclassified.dcm", "CT",
                        "1.2.840.10008.1.2.1", "synthetic"))
        status, text = self.coverage(rows)
        self.assertEqual(status, 1)
        self.assertIn("synthetic/unclassified.dcm", text)

    def test_a_row_that_is_neither_real_nor_synthetic_fails(self) -> None:
        rows = complete_rows()
        rows.append(row("synthetic/nolayer.dcm", "CT", "1.2.840.10008.1.2.1",
                        "mono16"))
        status, text = self.coverage(rows)
        self.assertEqual(status, 1)
        self.assertIn("synthetic/nolayer.dcm", text)

    def test_a_row_claiming_both_layers_fails(self) -> None:
        """A case is generated by this repository or it is not. A row claiming
        both would be counted as evidence that the real layer covers a class
        it does not cover."""
        rows = complete_rows()
        rows.append(row("synthetic/bothlayers.dcm", "CT",
                        "1.2.840.10008.1.2.1", "synthetic, real, mono16"))
        status, text = self.coverage(rows)
        self.assertEqual(status, 1)
        self.assertIn("synthetic/bothlayers.dcm", text)


    def test_a_row_with_no_transfer_syntax_is_named(self) -> None:
        """Condition 4 of this story is "at least one case per transfer
        syntax". A row declaring none is neither counted for coverage nor
        reported as unknown, so it is simply invisible, and it is reachable
        through the documented path because `--add` defaults the flag to the
        empty string."""
        rows = complete_rows()
        rows.append(row("real/ghost.dcm", "CT", "", "real, mono16"))
        status, text = self.coverage(rows)
        self.assertEqual(status, 1)
        self.assertIn("real/ghost.dcm: declares no transfer syntax", text)

    def test_a_transfer_syntax_outside_the_registry_is_named(self) -> None:
        """An extra row with a mistyped UID leaves every registry syntax still
        covered, so only this arm speaks. It is the arm that catches a typo on
        the documented `--add` path."""
        rows = complete_rows()
        rows.append(row("synthetic/typo.dcm", "CT", "1.2.840.10008.1.2.4.100",
                        "synthetic, mono16"))
        status, text = self.coverage(rows)
        self.assertEqual(status, 1)
        self.assertIn("not in the registry", text)
        self.assertIn("1.2.840.10008.1.2.4.100", text)

    def test_a_real_layer_with_no_chroma_is_reported_and_does_not_fail(self) -> None:
        """HLD 25.1 names chroma subsampling and YBR conversion as the REASON
        for class two. A greyscale ultrasound satisfies the class as written
        and exercises neither, so the report has to say so. It is not a
        failure: the check stays faithful to 25.1's wording."""
        status, text = self.coverage(complete_rows())
        self.assertEqual(status, 0, text)
        self.assertIn(CHROMA_NOTE, text)

    def test_a_row_claiming_chroma_and_no_chroma_at_once_fails(self) -> None:
        """`chroma-untested` is documentary, like `burned-in-unchecked`. What
        makes it different is that a later change can settle it: add a `colour`
        token to that row and the two now contradict each other, with nothing
        to notice. A documentary token that can be falsified by an edit to the
        same row should be falsified loudly."""
        rows = complete_rows()
        rows.append(row("real/settled.dcm", "US", "1.2.840.10008.1.2.1",
                        "real, colour, chroma-untested"))
        status, text = self.coverage(rows)
        self.assertEqual(status, 1)
        self.assertIn("real/settled.dcm", text)
        self.assertIn("chroma-untested", text)

    def test_a_real_colour_row_settles_the_chroma_note(self) -> None:
        rows = complete_rows()
        rows.append(row("real/colour.dcm", "US", "1.2.840.10008.1.2.1",
                        "real, colour"))
        status, text = self.coverage(rows)
        self.assertEqual(status, 0, text)
        self.assertNotIn(CHROMA_NOTE, text)


class RegistryTable(unittest.TestCase):

    def test_the_registry_list_has_no_duplicates(self) -> None:
        table = corpus_check.REGISTRY_TRANSFER_SYNTAXES
        self.assertEqual(len(table), len(set(table)))

    def test_the_registry_list_is_the_codec_registry_table(self) -> None:
        """PS3.5 Annex A, typed in rather than read back from the module under
        test. Sixteen syntaxes: the two native ones, deflate, the retired
        big-endian one, RLE, four JPEG, two JPEG-LS, two JPEG 2000 and three
        HTJ2K.

        HLD section 21 specifies the registry and names the two open gates but
        lists no syntaxes, so the standard is the only source for this.
        """
        self.assertEqual(list(corpus_check.REGISTRY_TRANSFER_SYNTAXES), [
            "1.2.840.10008.1.2",
            "1.2.840.10008.1.2.1",
            "1.2.840.10008.1.2.1.99",
            "1.2.840.10008.1.2.2",
            "1.2.840.10008.1.2.5",
            "1.2.840.10008.1.2.4.50",
            "1.2.840.10008.1.2.4.51",
            "1.2.840.10008.1.2.4.57",
            "1.2.840.10008.1.2.4.70",
            "1.2.840.10008.1.2.4.80",
            "1.2.840.10008.1.2.4.81",
            "1.2.840.10008.1.2.4.90",
            "1.2.840.10008.1.2.4.91",
            "1.2.840.10008.1.2.4.201",
            "1.2.840.10008.1.2.4.202",
            "1.2.840.10008.1.2.4.203",
        ])


class PopulationSeed(unittest.TestCase):

    def run_seed(self, content: bytes, expected: str) -> tuple[int, bool]:
        with tempfile.TemporaryDirectory(prefix="ocelli-populate-") as tmp:
            root = Path(tmp)
            seed = root / "seed"
            data = root / "data"
            manifest = root / "manifest.tsv"
            source = seed / "real" / "case.dcm"
            source.parent.mkdir(parents=True)
            source.write_bytes(content)
            manifest.write_text(
                f"path\tsha256\nreal/case.dcm\t{expected}\n",
                encoding="utf-8",
            )

            old_data = populate_corpus.DATA
            old_manifest = populate_corpus.MANIFEST
            populate_corpus.DATA = data
            populate_corpus.MANIFEST = manifest
            try:
                captured = io.StringIO()
                with redirect_stdout(captured):
                    status = populate_corpus.copy_seed(seed)
                copied = (data / "real" / "case.dcm").is_file()
            finally:
                populate_corpus.DATA = old_data
                populate_corpus.MANIFEST = old_manifest
        return status, copied

    def test_a_matching_seed_case_is_copied(self) -> None:
        content = b"synthetic test bytes"
        expected = populate_corpus.hashlib.sha256(content).hexdigest()
        self.assertEqual(self.run_seed(content, expected), (0, True))

    def test_a_mismatched_seed_case_is_refused(self) -> None:
        self.assertEqual(self.run_seed(b"different", "0" * 64), (1, False))


if __name__ == "__main__":
    unittest.main()
