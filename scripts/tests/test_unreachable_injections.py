"""The fourteen consumer injections from the F-218 review.

Thirteen of them append a load of adm-zip to a shipped dcmjs build. The
fourteenth is the unmodified install. Each injection must be caught by the
resolve trace, the static specifier scan, or both. The unmodified install
must be caught by neither. The string-built createRequire case is the AC17
RED: the scan cannot see it, and the trace must.

The channels are derived from the injected text and from which file the
smoke run executes. A quoted specifier is a scan hit. A load that runs is
a trace hit. `require` in the ES build throws before resolution, so that
case is scan only. A string built with `.join` is trace only.
"""

from __future__ import annotations

import importlib.util
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

ES = "node_modules/dcmjs/build/dcmjs.es.js"
CJS = "node_modules/dcmjs/build/dcmjs.js"


def load(name: str):
    spec = importlib.util.spec_from_file_location(
        name + "_injections", ROOT / (name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


P = load("package_check")


def _append(rel: str, text: str):
    def mutate(consumer: Path) -> None:
        path = consumer / rel
        path.write_text(
            path.read_text(encoding="utf-8") + "\n" + text + "\n",
            encoding="utf-8")
    return mutate


# (name, mutator, channels that must fire). `trace` is
# `loads an unreachable package`. `scan` is `names an unreachable package`.
CASES = [
    ("green", None, frozenset()),
    ("es-static-import", _append(ES, 'import "adm-zip";'),
     frozenset({"trace", "scan"})),
    ("es-require", _append(ES, 'require("adm-zip");'),
     frozenset({"scan"})),
    ("es-createRequire", _append(
        ES, 'import { createRequire as __cr } from "node:module"; '
            '__cr(import.meta.url)("adm-zip");'),
     frozenset({"trace", "scan"})),
    ("es-dynamic-await", _append(ES, 'await import("adm-zip");'),
     frozenset({"trace", "scan"})),
    ("es-dynamic-noawait", _append(ES, 'import("adm-zip");'),
     frozenset({"trace", "scan"})),
    ("es-stringbuilt-dynamic",
     _append(ES, 'await import(["adm","zip"].join("-"));'),
     frozenset({"trace"})),
    ("es-stringbuilt-createRequire", _append(
        ES, 'import { createRequire as __cr } from "node:module"; '
            '__cr(import.meta.url)(["adm","zip"].join("-"));'),
     frozenset({"trace"})),
    ("es-subpath", _append(ES, 'import "adm-zip/adm-zip.js";'),
     frozenset({"trace", "scan"})),
    ("es-template", _append(ES, "await import(`adm-zip`);"),
     frozenset({"trace", "scan"})),
    ("cjs-require", _append(CJS, 'require("adm-zip");'),
     frozenset({"trace", "scan"})),
    ("cjs-subpath", _append(CJS, 'require("adm-zip/adm-zip.js");'),
     frozenset({"trace", "scan"})),
    ("cjs-template", _append(CJS, "require(`adm-zip`);"),
     frozenset({"trace", "scan"})),
    ("cjs-stringbuilt", _append(CJS, 'require(["adm","zip"].join("-"));'),
     frozenset({"trace"})),
]


def _channels(problems: list[str]) -> frozenset[str]:
    found = set()
    for item in problems:
        if "loads an unreachable package" in item:
            found.add("trace")
        if "names an unreachable package" in item:
            found.add("scan")
    return frozenset(found)


class InjectionTable(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls._tmp = tempfile.TemporaryDirectory()
        work = Path(cls._tmp.name)
        built = P.run(["npm", "run", "build"], P.ROOT)
        if built.returncode != 0:
            raise AssertionError(built.stdout or built.stderr)
        tarballs = []
        for name in P.PACKAGES:
            directory = work / "pack" / name
            directory.mkdir(parents=True)
            tarball, _names = P.tarball_paths(directory)
            tarballs.append(tarball)
        cls.tarballs = tarballs
        cls.template_work = work / "template"
        cls.template_work.mkdir()
        problems = P.check_consumer(cls.template_work, tarballs)
        if problems:
            raise AssertionError(
                "the unmodified consumer is not green:\n" + "\n".join(problems))
        cls.template = cls.template_work / "consumer"

    @classmethod
    def tearDownClass(cls) -> None:
        cls._tmp.cleanup()

    def variant(self, mutate) -> list[str]:
        work = Path(self._tmp.name) / "variant"
        shutil.rmtree(work, ignore_errors=True)
        work.mkdir()
        original = P.run

        def fake_run(cmd, cwd):
            if cmd[:2] == ["npm", "install"]:
                shutil.copytree(
                    self.template / "node_modules",
                    Path(cwd) / "node_modules")
                shutil.copy(
                    self.template / "package-lock.json",
                    Path(cwd) / "package-lock.json")
                if mutate is not None:
                    mutate(Path(cwd))
                return subprocess.CompletedProcess(cmd, 0, "", "")
            # The table is about the trace and the scan. Typecheck is the
            # consumer proof's other half and is not what these cases change.
            if len(cmd) >= 2 and Path(str(cmd[1])).name == "tsc":
                return subprocess.CompletedProcess(cmd, 0, "", "")
            return original(cmd, cwd)

        P.run = fake_run
        try:
            return P.check_consumer(work, self.tarballs)
        finally:
            P.run = original

    def test_each_review_case_is_caught_by_the_declared_channel(self) -> None:
        """AC17 RED is `es-stringbuilt-createRequire`, trace only."""
        self.assertEqual(len(CASES), 14)
        for name, mutate, expected in CASES:
            with self.subTest(name=name):
                problems = self.variant(mutate)
                self.assertFalse(
                    any("does not record" in item for item in problems),
                    problems)
                self.assertEqual(_channels(problems), expected, problems)
                if name == "es-stringbuilt-createRequire":
                    self.assertEqual(expected, frozenset({"trace"}))
                    self.assertTrue(problems)


if __name__ == "__main__":
    unittest.main()
