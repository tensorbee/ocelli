"""The viewer build check's URL matches the config's, under a path with a space.

`examples/viewer-react/vite.config.ts` defines its wasm URL as `/@fs` and the
encoded `wasmPackage.pathname`, because a browser never re-encodes the value
it is handed, so the encoding must already be right, and `%20` in a literal
Rollup path is ENOENT only for the pkg-present build, which uses the decoded
specifier instead. The check reads the emitted worker for the encoded string.
The expected strings are written from that sentence and not from the check's
code.

F-266 adds the mode-and-presence split. The one `wasmPackage` URL yields two
specifier forms: the encoded browser URL `'/@fs' + pathname` for dev and for
every build with the pkg absent, and the decoded Rollup specifier
`'/@fs' + decodeURIComponent(pathname)` for a build with the pkg present.
"""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path
from urllib.parse import quote

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import viewer_build_check as v  # noqa: E402


class WasmUrl(unittest.TestCase):
    def test_a_space_is_encoded_in_the_browser_url(self) -> None:
        self.assertEqual(
            v.wasm_url(Path("/a b/c")),
            "/@fs/a%20b/c/crates/ocelli-wasm/pkg/ocelli_wasm.js")

    def test_the_gate_builds_under_a_path_with_a_space(self) -> None:
        self.assertIn(" ", v.TREE_PREFIX)

    def test_the_url_with_a_space_is_what_the_check_finds(self) -> None:
        url = v.wasm_url(Path("/a b/c"))
        worker = f'import("{url}")'
        self.assertEqual(
            v.absent_problems(
                ["assets/core-worker-a1.js"],
                {"assets/core-worker-a1.js": worker.encode("utf-8")},
                url, Path("/a b/c")),
            [])

    def test_a_decoded_url_in_the_worker_is_refused(self) -> None:
        url = v.wasm_url(Path("/a b/c"))
        decoded = url.replace("%20", " ")
        worker = f'import("{decoded}")'
        self.assertNotEqual(
            v.absent_problems(
                ["assets/core-worker-a1.js"],
                {"assets/core-worker-a1.js": worker.encode("utf-8")},
                url, Path("/a b/c")),
            [])


class SpecifierForms(unittest.TestCase):
    """The one live encoded URL calculation and its encoding facts."""

    def test_the_url_encodes_a_space(self) -> None:
        self.assertEqual(
            v.wasm_url(Path("/a b/c")),
            "/@fs/a%20b/c/crates/ocelli-wasm/pkg/ocelli_wasm.js")

    def test_the_url_encodes_a_percent_sign(self) -> None:
        self.assertEqual(
            v.wasm_url(Path("/p%t")),
            "/@fs/p%25t/crates/ocelli-wasm/pkg/ocelli_wasm.js")

    def test_the_url_encodes_unicode(self) -> None:
        self.assertEqual(
            v.wasm_url(Path("/ünïcode")),
            "/@fs/%C3%BCn%C3%AFcode/crates/ocelli-wasm/pkg/"
            "ocelli_wasm.js")


    def test_the_url_preserves_plus_and_encodes_space(self) -> None:
        self.assertEqual(
            v.wasm_url(Path("/plus+ space")),
            "/@fs/plus+%20space/crates/ocelli-wasm/pkg/ocelli_wasm.js",
        )


class RootSpelling(unittest.TestCase):
    """The root boundary spells a standalone path token, review pass 3.

    Each expected value is the boundary rule applied to the planted
    bytes, not the helper's behavior: a root is a reach when the byte
    before it starts the content or is not a filename continuation byte
    and the byte after it ends the content or is not one. The
    continuation set is `[A-Za-z0-9._-]` and every non-ASCII byte, so a
    glued `/ocelli_wasm` and `/ocelli-x` match nothing and a root
    followed by a quote, an encoded separator or the end of the content
    is caught.
    """

    def test_a_root_at_the_end_of_the_content_is_a_reach(self) -> None:
        # A binary whose last bytes are the root: the root runs to the
        # end of the content.
        self.assertTrue(v._spells_root(b"\x00asm\x01/w", b"/w"))

    def test_a_root_at_the_start_of_the_content_is_a_reach(self) -> None:
        self.assertTrue(
            v._spells_root(b"/w/crates/ocelli-wasm/pkg/ocelli_wasm.js",
                           b"/w"))

    def test_a_root_followed_by_a_separator_is_a_reach(self) -> None:
        self.assertTrue(v._spells_root(b'import("/w/pkg/x.js")', b"/w"))

    def test_a_root_followed_by_a_quote_is_a_reach(self) -> None:
        self.assertTrue(v._spells_root(b'const r="/w"', b"/w"))

    def test_a_root_followed_by_an_apostrophe_is_a_reach(self) -> None:
        self.assertTrue(v._spells_root(b"const r = '/w'", b"/w"))

    def test_a_root_followed_by_a_backtick_is_a_reach(self) -> None:
        self.assertTrue(v._spells_root(b"const r = `/w`", b"/w"))

    def test_a_root_followed_by_an_encoded_separator_is_a_reach(self) -> None:
        self.assertTrue(v._spells_root(
            b'"/w%2Fcrates%2Focelli-wasm%2Fpkg%2Focelli_wasm.js"', b"/w"))

    def test_a_root_followed_by_a_query_mark_is_a_reach(self) -> None:
        self.assertTrue(v._spells_root(b'"https://x.test/r?to=/w?import"',
                                       b"/w"))

    def test_a_root_followed_by_a_fragment_mark_is_a_reach(self) -> None:
        self.assertTrue(v._spells_root(b'"https://x.test/r?to=/w#f"',
                                       b"/w"))

    def test_a_root_followed_by_a_backslash_is_a_reach(self) -> None:
        # A JS escape spelling the next segment: the root is followed by
        # the escape's own backslash.
        self.assertTrue(v._spells_root(b'import("/w\\x2fcrates/x.js")',
                                       b"/w"))

    def test_a_root_followed_by_whitespace_is_a_reach(self) -> None:
        self.assertTrue(v._spells_root(b"/* /w */", b"/w"))

    def test_a_root_glued_into_an_underscored_name_is_not_a_reach(self) -> None:
        self.assertFalse(v._spells_root(b'import("./w_a1.js")', b"/w"))

    def test_a_root_glued_into_a_hyphenated_name_is_not_a_reach(self) -> None:
        self.assertFalse(v._spells_root(b'import("./w-x.js")', b"/w"))

    def test_a_root_glued_after_a_longer_name_is_not_a_reach(self) -> None:
        # The pass 3 N1 case: a checkout at /ocelli must not refuse a
        # dist that merely carries the name in a URL.
        self.assertFalse(v._spells_root(
            b"https://github.com/tensorbee/ocelli/issues", b"/ocelli"))

    def test_a_root_glued_into_a_non_ascii_name_is_not_a_reach(self) -> None:
        self.assertFalse(v._spells_root('="/wé"'.encode("utf-8"), b"/w"))

    def test_a_root_before_a_utf8_continuation_byte_is_not_a_reach(self) -> None:
        # Byte 0x80 is never a lead byte, only a continuation, so a root
        # followed by it is inside a multi-byte character and glued. A
        # continuation test written as `> 0x80` would spell this blob.
        self.assertFalse(v._spells_root(b'="/w\x80"', b"/w"))

    def test_a_root_after_a_non_ascii_name_is_not_a_reach(self) -> None:
        self.assertFalse(
            v._spells_root('const p = "é/w"'.encode("utf-8"), b"/w"))


class PresentEmissionCheck(unittest.TestCase):
    """AC6: the present-state check reads the emitted worker, not the config."""

    @staticmethod
    def _raw(worker: str, extra: dict[str, bytes] | None = None) -> dict[str, bytes]:
        """The one-entry raw-bytes map form, per the approved contract."""
        raw = {"assets/core-worker-a1.js": worker.encode("utf-8")}
        raw.update(extra or {})
        return raw

    _URL = "/@fs/a b/c/crates/ocelli-wasm/pkg/ocelli_wasm.js"
    _BASE = Path("/w")

    def _names(self) -> list[str]:
        return ["assets/core-worker-a1.js", "assets/ocelli_wasm-Bcd23.js",
                "assets/ocelli_wasm_bg-Efg45.wasm"]

    def test_a_worker_importing_a_bundled_glue_chunk_is_accepted(self) -> None:
        problems = v.present_problems(
            ["assets/core-worker-a1.js", "assets/index-a1.js",
             "assets/ocelli_wasm-Bcd23.js", "assets/ocelli_wasm_bg-Efg45.wasm"],
            self._raw('import"./ocelli_wasm-Bcd23.js";import("./ocelli_wasm-Bcd23.js")', {
                "assets/index-a1.js": b"entry",
                "assets/ocelli_wasm-Bcd23.js": b"glue",
                "assets/ocelli_wasm_bg-Efg45.wasm": b"\x00asm\x01",
            }),
            self._URL, self._BASE)
        self.assertEqual(problems, [])

    def test_a_worker_reaching_the_build_machine_path_is_refused(self) -> None:
        url = self._URL
        worker = f'import("{url}")'
        problems = v.present_problems(
            self._names(), self._raw(worker), url, self._BASE)
        self.assertNotEqual(problems, [])
        self.assertTrue(any("absolute" in p or "/@fs" in p for p in problems))

    def test_a_worker_fetching_the_build_machine_path_is_refused(self) -> None:
        # AC7's target shape: the old redundant preflight fetch of the
        # absolute package URL ahead of a bundled import.
        url = self._URL
        worker = f'fetch("{url}");import("./ocelli_wasm-Bcd23.js")'
        problems = v.present_problems(
            self._names(), self._raw(worker), url, self._BASE)
        self.assertNotEqual(problems, [])

    def test_an_absolute_path_anywhere_in_the_worker_is_refused(self) -> None:
        problems = v.present_problems(
            self._names(),
            self._raw('const probe="/@fs/a b/c/crates/ocelli-wasm/pkg/'
                      'ocelli_wasm.js";import("./ocelli_wasm-Bcd23.js")'),
            self._URL, self._BASE)
        self.assertNotEqual(problems, [])

    def test_a_unicode_name_decoded_in_one_segment_is_refused(self) -> None:
        # The URL encodes üt as %C3%BCt. A worker spelling that one segment
        # decoded spells the raw root inside the /@fs URL, where the root
        # is glued to the loader prefix's last byte, so the root boundary
        # does not spell it and the standalone /@fs marker is the refusal
        # that fires. The encoded base appears nowhere in the worker, and
        # the glue import keeps the missing-glue refusal from masking the
        # reach scan.
        url = "/@fs/a%C3%BCt/c/crates/ocelli-wasm/pkg/ocelli_wasm.js"
        worker = ('import("/@fs/aüt/c/crates/ocelli-wasm/pkg/'
                  'ocelli_wasm.js");import("./ocelli_wasm-Bcd23.js")')
        problems = v.present_problems(
            self._names(), self._raw(worker), url, Path("/aüt/c"))
        self.assertTrue(any("/@fs)" in p for p in problems))

    def test_a_percent_name_decoded_in_one_segment_is_refused(self) -> None:
        # The URL encodes p%t as p%25t. A worker spelling that one segment
        # decoded spells the raw root inside the /@fs URL, glued to the
        # loader prefix's last byte, so the standalone /@fs marker is the
        # refusal that fires, and the encoded base p%25t/c appears nowhere
        # in the worker.
        url = "/@fs/p%25t/c/crates/ocelli-wasm/pkg/ocelli_wasm.js"
        worker = ('import("/@fs/p%t/c/crates/ocelli-wasm/pkg/'
                  'ocelli_wasm.js");import("./ocelli_wasm-Bcd23.js")')
        problems = v.present_problems(
            self._names(), self._raw(worker), url, Path("/p%t/c"))
        self.assertTrue(any("/@fs)" in p for p in problems))

    def test_a_space_name_decoded_in_one_segment_is_refused(self) -> None:
        # The URL encodes the space as %20. A worker spelling that one
        # segment decoded spells the raw root inside the /@fs URL, glued
        # to the loader prefix's last byte, so the standalone /@fs marker
        # is the refusal that fires, and the encoded base a%20b/c appears
        # nowhere in the worker.
        url = "/@fs/a%20b/c/crates/ocelli-wasm/pkg/ocelli_wasm.js"
        worker = ('import("/@fs/a b/c/crates/ocelli-wasm/pkg/'
                  'ocelli_wasm.js");import("./ocelli_wasm-Bcd23.js")')
        problems = v.present_problems(
            self._names(), self._raw(worker), url, Path("/a b/c"))
        self.assertTrue(any("/@fs)" in p for p in problems))

    def test_the_fully_encoded_url_without_a_space_is_accepted(self) -> None:
        # The genuine present-state shape: the worker references only its own
        # bundled chunk, and no emitted file carries the build base's root
        # bytes, encoded or raw, nor any /@fs marker.
        problems = v.present_problems(
            self._names(),
            self._raw('import("./ocelli_wasm-Bcd23.js")'),
            "/@fs/a%20b/c/crates/ocelli-wasm/pkg/ocelli_wasm.js",
            Path("/a b/c"))
        self.assertEqual(problems, [])

    def test_a_short_root_glued_into_a_longer_name_is_not_a_reach(self) -> None:
        # A checkout at /ocelli must not refuse a healthy build whose
        # worker and glue spell ./ocelli_wasm names: the root is matched
        # only as a path prefix with a boundary, so a root glued into a
        # longer name is not a root spelling. The base is the same short
        # path, so both roots' raw and encoded patterns are the glued
        # form, and under a bare substring match this healthy emission
        # set is refused.
        saved = v.ROOT
        v.ROOT = Path("/ocelli")
        try:
            problems = v.present_problems(
                ["assets/core-worker-a1.js", "assets/index-a1.js",
                 "assets/ocelli_wasm-Bcd23.js",
                 "assets/ocelli_wasm_bg-Efg45.wasm"],
                self._raw('import"./ocelli_wasm-Bcd23.js";'
                          'import("./ocelli_wasm-Bcd23.js")', {
                              "assets/index-a1.js": b"entry",
                              "assets/ocelli_wasm-Bcd23.js":
                                  b'import "./ocelli_wasm_bg-Efg45.wasm"',
                              "assets/ocelli_wasm_bg-Efg45.wasm":
                                  b"\x00asm\x01",
                          }),
                "/@fs/ocelli/crates/ocelli-wasm/pkg/ocelli_wasm.js",
                Path("/ocelli"))
        finally:
            v.ROOT = saved
        self.assertEqual(problems, [])

    def test_a_reach_after_a_glued_name_is_still_refused(self) -> None:
        # The scan walks every occurrence of the root: the glued
        # ./ocelli_wasm import first in the worker must not stop the real
        # reach later in it from being found. A scan that read only the
        # first occurrence would keep this green.
        saved = v.ROOT
        v.ROOT = Path("/ocelli")
        try:
            problems = v.present_problems(
                self._names(),
                self._raw('import"./ocelli_wasm-Bcd23.js";'
                          'import("/ocelli/pkg/ocelli_wasm.js")'),
                "/@fs/ocelli/crates/ocelli-wasm/pkg/ocelli_wasm.js",
                Path("/ocelli"))
        finally:
            v.ROOT = saved
        self.assertTrue(any("raw root" in p for p in problems))

    def test_a_quote_terminated_root_in_a_binding_is_refused(self) -> None:
        # The pass 3 D1 shape: the root bound to a name and reached by
        # concatenation. The root is followed by a quote, so the old
        # slash-only boundary missed it and the standalone boundary
        # catches it. No /@fs appears, so the marker cannot fire and the
        # root pattern is the refusal that names itself.
        worker = ('const r="/w";'
                  'import(r+"/crates/ocelli-wasm/pkg/ocelli_wasm.js");'
                  'import("./ocelli_wasm-Bcd23.js")')
        problems = v.present_problems(
            self._names(), self._raw(worker),
            "/@fs/w/crates/ocelli-wasm/pkg/ocelli_wasm.js", Path("/w"))
        self.assertTrue(any("raw root" in p for p in problems))

    def test_an_encoded_separator_reach_is_refused(self) -> None:
        # The other pass 3 D1 shape: the root followed by %2F segments,
        # the mixed spelling the module docstring refuses. No /@fs
        # appears, so only the root pattern can refuse.
        worker = ('const u="/w%2Fcrates%2Focelli-wasm%2Fpkg%'
                  '2Focelli_wasm.js";import("./ocelli_wasm-Bcd23.js")')
        problems = v.present_problems(
            self._names(), self._raw(worker),
            "/@fs/w/crates/ocelli-wasm/pkg/ocelli_wasm.js", Path("/w"))
        self.assertTrue(any("raw root" in p for p in problems))

    def test_a_json_field_root_is_refused(self) -> None:
        # The pass 3 D1 JSON shape: the root as a quoted value in a
        # non-worker emitted file. The reach is caught without decoding
        # the file as anything but bytes.
        worker = 'import("./ocelli_wasm-Bcd23.js")'
        problems = v.present_problems(
            self._names() + ["assets/meta.json"],
            self._raw(worker, {"assets/meta.json": b'{"root":"/w"}'}),
            "/@fs/w/crates/ocelli-wasm/pkg/ocelli_wasm.js", Path("/w"))
        self.assertTrue(any("raw root" in p and "meta.json" in p
                            for p in problems))

    def test_a_url_carrying_the_root_name_is_not_a_refusal(self) -> None:
        # The pass 3 N1 case at the consumer: a checkout at /ocelli must
        # not refuse an emitted file that merely carries the name in a
        # URL. Under a right-boundary-only check the URL's /ocelli is
        # followed by a slash and refused, so this goes red there.
        saved = v.ROOT
        v.ROOT = Path("/ocelli")
        try:
            worker = ('const issues = '
                      '"https://github.com/tensorbee/ocelli/issues";'
                      'import("./ocelli_wasm-Bcd23.js")')
            problems = v.present_problems(
                self._names(), self._raw(worker),
                "/@fs/ocelli/crates/ocelli-wasm/pkg/ocelli_wasm.js",
                Path("/ocelli"))
        finally:
            v.ROOT = saved
        self.assertEqual(problems, [])

    def test_a_real_checkout_reach_is_still_refused(self) -> None:
        # The other half of pass 3 N1: the preceding-byte check must not
        # swing so far that a genuine root spelling passes. The reach is
        # quoted on both sides and carries no /@fs, so the root pattern
        # is the refusal that fires.
        saved = v.ROOT
        v.ROOT = Path("/ocelli")
        try:
            worker = ('const lib = '
                      '"/ocelli/crates/ocelli-wasm/pkg/ocelli_wasm.js";'
                      'import("./ocelli_wasm-Bcd23.js")')
            problems = v.present_problems(
                self._names(), self._raw(worker),
                "/@fs/ocelli/crates/ocelli-wasm/pkg/ocelli_wasm.js",
                Path("/ocelli"))
        finally:
            v.ROOT = saved
        self.assertTrue(any("raw root" in p for p in problems))

    def test_missing_chunks_are_still_refused(self) -> None:
        problems = v.present_problems(
            ["assets/core-worker-a1.js"],
            self._raw('import("./x.js")'),
            self._URL, self._BASE)
        self.assertNotEqual(problems, [])

    def test_worker_not_referencing_the_emitted_glue_is_refused(self) -> None:
        problems = v.present_problems(
            self._names(),
            self._raw("const booted = true", {
                "assets/ocelli_wasm-Bcd23.js": b"glue",
                "assets/ocelli_wasm_bg-Efg45.wasm": b"\x00asm\x01",
            }),
            self._URL, self._BASE)
        self.assertNotEqual(problems, [])

    def test_worker_importing_a_wrong_target_is_refused(self) -> None:
        problems = v.present_problems(
            self._names(),
            self._raw('import("./ocelli_wasm-Bcd23.js.extra")', {
                "assets/ocelli_wasm-Bcd23.js": b"glue",
                "assets/ocelli_wasm_bg-Efg45.wasm": b"\x00asm\x01",
            }),
            self._URL, self._BASE)
        self.assertNotEqual(problems, [])

    def test_two_workers_are_still_refused(self) -> None:
        problems = v.present_problems(
            ["assets/core-worker-a1.js", "assets/core-worker-b2.js",
             "assets/ocelli_wasm-Bcd23.js", "assets/ocelli_wasm_bg-Efg45.wasm"],
            self._raw('import("./ocelli_wasm-Bcd23.js")', {
                "assets/core-worker-b2.js": b'import("./ocelli_wasm-Bcd23.js")',
            }),
            self._URL, self._BASE)
        self.assertNotEqual(problems, [])


class AbsentShapeKept(unittest.TestCase):
    """AC8: the absent-state refusals other than the fetch shape remain."""

    _URL = "/@fs/a b/c/crates/ocelli-wasm/pkg/ocelli_wasm.js"
    _BASE = Path("/w")

    def _raw(self, worker: str, extra: dict[str, bytes] | None = None) -> dict[str, bytes]:
        raw = {"assets/core-worker-a1.js": worker.encode("utf-8")}
        raw.update(extra or {})
        return raw

    def test_an_unexpected_wasm_asset_is_refused(self) -> None:
        worker = f'import("{self._URL}")'
        problems = v.absent_problems(
            ["assets/core-worker-a1.js", "assets/ocelli_wasm-Bcd23.js"],
            self._raw(worker, {"assets/ocelli_wasm-Bcd23.js": b"glue"}),
            self._URL, self._BASE)
        self.assertTrue(any("ocelli_wasm" in p for p in problems))
        # And one that is an ocelli-named emitted file with no source to
        # come from: the absence refusal names it.
        problems = v.absent_problems(
            ["assets/core-worker-a1.js", "assets/ocelli_wasm-x.js"],
            self._raw(worker, {"assets/ocelli_wasm-x.js": b"x"}),
            self._URL, self._BASE)
        self.assertTrue(any("ocelli_wasm-x.js" in p for p in problems))

    def test_a_wrong_worker_count_is_refused(self) -> None:
        worker = f'import("{self._URL}")'
        problems = v.absent_problems(
            ["assets/core-worker-a1.js", "assets/core-worker-b2.js"],
            self._raw(worker, {"assets/core-worker-b2.js": worker.encode()}),
            self._URL, self._BASE)
        self.assertNotEqual(problems, [])

    def test_the_intentional_external_encoded_import_is_kept(self) -> None:
        # ABSENT keeps its legitimate external encoded /@fs import: no
        # absence refusal bans it, even though the same shape is a PRESENT
        # reach. This is the approved exemption, not an acceptance waiver.
        worker = f'import("{self._URL}")'
        problems = v.absent_problems(
            ["assets/core-worker-a1.js"], self._raw(worker),
            self._URL, self._BASE)
        self.assertEqual(problems, [])


class EmittedMap(unittest.TestCase):
    """A1: the emitted map is a raw-byte view of ALL recursive dist outputs.

    The controls below are written from literal file contents planted in a
    temporary dist, not from the map's code. Each names the refusal the
    consumer must raise.
    """

    def _tree(self, temp: str, files: dict[str, object]) -> Path:
        base = Path(temp) / "viewer"
        for name, content in files.items():
            target = base / "dist" / name
            target.parent.mkdir(parents=True, exist_ok=True)
            if isinstance(content, bytes):
                target.write_bytes(content)
            else:
                target.write_text(content, encoding="utf-8")
        return base

    def _names(self, raw: dict[str, bytes]) -> list[str]:
        return sorted(raw)

    def test_a_healthy_worker_with_a_bad_main_is_refused(self) -> None:
        # The map carries every recursive output, so a reach inside a
        # non-worker chunk is refused by the consumer. The old view read
        # top-level names plus one worker text and could not see this file.
        with tempfile.TemporaryDirectory() as temp:
            base = self._tree(temp, {
                "assets/core-worker-a1.js": 'import("./ocelli_wasm-B.js")',
                "assets/ocelli_wasm-B.js": "glue",
                "assets/ocelli_wasm_bg-C.wasm": b"\x00asm\x01",
                "chunks/deep/main-D.js": 'fetch("/@fs/w/pkg/ocelli_wasm.js")',
            })
            names, raw = v.emitted(base)
            problems = v.present_problems(
                names, raw,
                "/@fs/w/pkg/ocelli_wasm.js", base)
            self.assertNotEqual(problems, [])

    def test_a_bad_glue_chunk_is_refused(self) -> None:
        # The glue chunk is an emitted file: a reach spelled inside it is a
        # PRESENT ban, not something only the worker is scanned for.
        with tempfile.TemporaryDirectory() as temp:
            base = self._tree(temp, {
                "assets/core-worker-a1.js": 'import("./ocelli_wasm-B.js")',
                "assets/ocelli_wasm-B.js":
                    'export default "/@fs/w/pkg/ocelli_wasm.js"',
                "assets/ocelli_wasm_bg-C.wasm": b"\x00asm\x01",
            })
            names, raw = v.emitted(base)
            problems = v.present_problems(
                names, raw, "/@fs/w/pkg/ocelli_wasm.js", base)
            self.assertNotEqual(problems, [])

    def test_another_output_class_is_refused(self) -> None:
        # A third emitted-file class, a sourcemap, carries the reach: caught
        # like any other output, without decoding it as anything but the
        # bounded worker-role read would.
        with tempfile.TemporaryDirectory() as temp:
            base = self._tree(temp, {
                "assets/core-worker-a1.js": 'import("./ocelli_wasm-B.js")',
                "assets/ocelli_wasm-B.js": "glue",
                "assets/ocelli_wasm_bg-C.wasm": b"\x00asm\x01",
                "assets/index-A.js.map":
                    '{"sources":["/@fs/w/pkg/ocelli_wasm.js"]}',
            })
            names, raw = v.emitted(base)
            problems = v.present_problems(
                names, raw, "/@fs/w/pkg/ocelli_wasm.js", base)
            self.assertNotEqual(problems, [])

    def test_another_root_target_is_refused(self) -> None:
        # A checkout-root path other than the pkg file: the raw-root and
        # encoded-root patterns are the build base itself, so a reach at any
        # root path is caught by the /@fs marker pattern.
        with tempfile.TemporaryDirectory() as temp:
            base = self._tree(temp, {
                "assets/core-worker-a1.js": 'import("./ocelli_wasm-B.js")',
                "assets/ocelli_wasm-B.js": "glue",
                "assets/ocelli_wasm_bg-C.wasm": b"\x00asm\x01",
                "chunks/main-D.js": 'import("/@fs/w/other/path.js")',
            })
            names, raw = v.emitted(base)
            problems = v.present_problems(
                names, raw, "/@fs/w/pkg/ocelli_wasm.js", base)
            self.assertNotEqual(problems, [])

    def test_a_binary_known_root_output_is_refused_not_decoded(self) -> None:
        # A valid binary with no root bytes and no marker is accepted
        # unchanged; the raw bytes pass through the map untouched.
        payload = bytes.fromhex("0061736d01000000000400fffefd")
        with tempfile.TemporaryDirectory() as temp:
            base = self._tree(temp, {
                "assets/core-worker-a1.js": 'import("./ocelli_wasm-B.js")',
                "assets/ocelli_wasm-B.js": "glue",
                "assets/ocelli_wasm_bg-C.wasm": payload,
            })
            names, raw = v.emitted(base)
            problems = v.present_problems(
                names, raw, "/@fs/w/pkg/ocelli_wasm.js", base)
            self.assertEqual(problems, [])
            self.assertEqual(raw["assets/ocelli_wasm_bg-C.wasm"], payload)
        # A second tree whose .wasm embeds the build base's raw root bytes:
        # refused by the raw scan, without the binary ever being decoded.
        with tempfile.TemporaryDirectory() as temp:
            base = self._tree(temp, {})
            root_bytes = base.resolve().as_posix().encode()
            for name, content in {
                "assets/core-worker-a1.js": 'import("./ocelli_wasm-B.js")',
                "assets/ocelli_wasm-B.js": "glue",
                "assets/ocelli_wasm_bg-C.wasm": b"\x00asm\x01" + root_bytes,
            }.items():
                target = base / "dist" / name
                target.parent.mkdir(parents=True, exist_ok=True)
                if isinstance(content, bytes):
                    target.write_bytes(content)
                else:
                    target.write_text(content, encoding="utf-8")
            names, raw = v.emitted(base)
            problems = v.present_problems(
                names, raw, "/@fs/w/pkg/ocelli_wasm.js", base)
            self.assertNotEqual(problems, [])
            self.assertEqual(raw["assets/ocelli_wasm_bg-C.wasm"],
                             b"\x00asm\x01" + root_bytes)

    def test_an_encoded_root_without_fs_is_refused(self) -> None:
        # The once-URL-encoded root bytes in a file with no /@fs marker.
        # The tree lives under a space-named prefix, so the encoded root
        # differs from the raw root and only the encoded pattern matches
        # the planted spelling: under a plain temporary directory the
        # quoted root equals the raw one and the raw pattern matches
        # first, which would prove nothing about this branch.
        with tempfile.TemporaryDirectory(prefix="view er-") as temp:
            base = self._tree(temp, {
                "assets/core-worker-a1.js": 'import("./ocelli_wasm-B.js")',
                "assets/ocelli_wasm-B.js": "glue",
                "assets/ocelli_wasm_bg-C.wasm": b"\x00asm\x01",
            })
            encoded_root = quote(base.resolve().as_posix(), safe="/")
            (base / "dist" / "chunks").mkdir(parents=True, exist_ok=True)
            (base / "dist" / "chunks" / "main-D.js").write_text(
                f'const src = "{encoded_root}/elsewhere.js"', encoding="utf-8")
            names, raw = v.emitted(base)
            problems = v.present_problems(
                names, raw, f"/@fs{encoded_root}/pkg/ocelli_wasm.js", base)
            self.assertTrue(any("encoded root" in p and "main-D.js" in p
                                for p in problems))

    def test_a_resolved_checkout_path_is_refused(self) -> None:
        # make_tree links node_modules and packages into the real checkout,
        # so a build that resolves those links spells the checkout path,
        # not the temporary base. ROOT is pointed at a space-named tree, as
        # the encoded test beside this one does, so the raw and encoded
        # forms differ, and the planted spelling is the raw form. The
        # refusal must name the raw checkout root: on a checkout whose
        # path has nothing to percent-encode the raw and encoded bytes are
        # identical, so an encoded-only match would keep this green.
        with tempfile.TemporaryDirectory() as temp:
            base = self._tree(temp, {
                "assets/core-worker-a1.js": 'import("./ocelli_wasm-B.js")',
                "assets/ocelli_wasm-B.js": "glue",
                "assets/ocelli_wasm_bg-C.wasm": b"\x00asm\x01",
            })
            fake_root = Path(temp) / "check out"
            fake_root.mkdir()
            saved = v.ROOT
            v.ROOT = fake_root
            try:
                (base / "dist" / "chunks").mkdir(parents=True, exist_ok=True)
                (base / "dist" / "chunks" / "resolved-D.js").write_text(
                    f'const lib = "{fake_root.as_posix()}/packages/core/dist/'
                    f'index.js"', encoding="utf-8")
                names, raw = v.emitted(base)
                problems = v.present_problems(
                    names, raw, "/@fs/unused/pkg/ocelli_wasm.js", base)
            finally:
                v.ROOT = saved
            self.assertTrue(any("raw checkout root" in p
                                and "resolved-D.js" in p for p in problems))

    def test_an_encoded_checkout_root_is_refused(self) -> None:
        # The checkout root's encoded spelling, for a checkout whose own
        # path percent-encodes. ROOT is a module constant naming the real
        # checkout, so the test points it at a space-named tree and puts it
        # back: the planted spelling is the encoded form, the raw form with
        # the space appears nowhere, and no /@fs is present.
        with tempfile.TemporaryDirectory(prefix="view er-") as temp:
            base = self._tree(temp, {
                "assets/core-worker-a1.js": 'import("./ocelli_wasm-B.js")',
                "assets/ocelli_wasm-B.js": "glue",
                "assets/ocelli_wasm_bg-C.wasm": b"\x00asm\x01",
            })
            fake_root = Path(temp) / "check out"
            fake_root.mkdir()
            saved = v.ROOT
            v.ROOT = fake_root
            try:
                (base / "dist" / "chunks").mkdir(parents=True, exist_ok=True)
                encoded = quote(fake_root.as_posix(), safe="/")
                (base / "dist" / "chunks" / "link-E.js").write_text(
                    f'const lib = "{encoded}/packages/core/dist/index.js"',
                    encoding="utf-8")
                names, raw = v.emitted(base)
                problems = v.present_problems(
                    names, raw, "/@fs/unused/pkg/ocelli_wasm.js", base)
            finally:
                v.ROOT = saved
            self.assertTrue(any("encoded checkout root" in p
                                and "link-E.js" in p for p in problems))

    def test_another_root_target_is_a_separate_map(self) -> None:
        # The map is rooted at the explicit base, so a sibling tree's dist
        # never leaks into it.
        with tempfile.TemporaryDirectory() as temp:
            first = self._tree(temp + "/one", {
                "assets/core-worker-a1.js": "one",
            })
            second = self._tree(temp + "/two", {
                "assets/core-worker-b2.js": "two",
            })
            self.assertEqual(sorted(v.emitted(first)[1]),
                             ["assets/core-worker-a1.js"])
            self.assertEqual(sorted(v.emitted(second)[1]),
                             ["assets/core-worker-b2.js"])


    def test_an_encoded_temp_base_tree_maps_the_same(self) -> None:
        # A tree whose absolute path percent-encodes (the known TEMP base
        # with a space, which URL.pathname would spell %20): the map is
        # dist-relative, so the keys are identical to a plain-rooted tree's.
        with tempfile.TemporaryDirectory(prefix="view er-") as temp:
            base = self._tree(temp, {
                "assets/core-worker-a1.js": 'import("./ocelli_wasm-B.js")',
                "assets/ocelli_wasm-B.js": "glue",
                "assets/ocelli_wasm_bg-C.wasm": b"\x00asm\x01",
            })
            names, raw = v.emitted(base)
            self.assertEqual(
                names,
                ["assets/core-worker-a1.js", "assets/ocelli_wasm-B.js",
                 "assets/ocelli_wasm_bg-C.wasm"])
            # No reach: the planted worker text has no root bytes and the
            # base itself has a space in its temp prefix, so the encoded
            # root pattern would catch a misspelled reach.
            problems = v.present_problems(
                names, raw, f"/@fs{quote(base.resolve().as_posix())}"
                "/pkg/ocelli_wasm.js", base)
            self.assertEqual(problems, [])

    def test_a_nonworker_encoded_root_with_plus_and_space_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            base = self._tree(str(Path(temp) / "plus+ space"), {
                "assets/core-worker-a1.js": 'import("./ocelli_wasm-B.js")',
                "assets/ocelli_wasm-B.js": "glue",
                "assets/ocelli_wasm_bg-C.wasm": bytes.fromhex("0061736d01000000"),
            })
            meta = base / "dist" / "meta.json"
            meta.write_text(
                str(base.resolve()).replace(" ", "%20"),
                encoding="utf-8",
            )
            names, raw = v.emitted(base)
            problems = v.present_problems(
                names, raw, "/@fs/unused/pkg/ocelli_wasm.js", base)
            self.assertTrue(
                any("meta.json" in problem and "encoded root" in problem
                    for problem in problems),
            )

    def test_outside_worker_inspection_drop_is_a_countermutant_failure(self) -> None:
        # The countermutant: a version of the check that inspects only the
        # worker chunk must FAIL these fixtures. This is the D1 class.
        with tempfile.TemporaryDirectory() as temp:
            base = self._tree(temp, {
                "assets/core-worker-a1.js": 'import("./ocelli_wasm-B.js")',
                "assets/ocelli_wasm-B.js": "glue",
                "assets/ocelli_wasm_bg-C.wasm": b"\x00asm\x01",
                "chunks/deep/main-D.js": 'fetch("/@fs/w/pkg/ocelli_wasm.js")',
            })
            names, raw = v.emitted(base)
            worker_key = [n for n in names
                          if n.rsplit("/", 1)[-1].startswith("core-worker-")][0]
            worker_only = {worker_key: raw[worker_key]}
            mutant_problems = v._absolute_reach(
                worker_only, base)
            self.assertIsNone(mutant_problems)
            real_problems = v._absolute_reach(
                raw, base)
            self.assertIsNotNone(real_problems)


if __name__ == "__main__":
    unittest.main()
