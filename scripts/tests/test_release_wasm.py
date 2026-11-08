"""The release comparison's refusals, driven red and green.

docs/RELEASE.md item 6, the F-237 review's D3. The comparison is pure over
its inputs, so the cold build is stubbed by construction: each test hands
`compare_problems` and `verify` a parsed build id and the digests a cold
build would produce. The tarball round trip runs on a real tarball built in
the test's own temporary directory, which is what pins the wiring between
the pack, the build id it stages and the bytes it stages beside it.

The stage tests run the real `packages/wasm/scripts/stage.mjs` under node in
a scratch repository whose `bin/ocelli.sh` exits 0, so the git state the
stage records is read off a real index without a wasm toolchain. The guards
job that runs this suite pins node for the same reason the `bench` gate has
it. F-237 review pass 2, D1: a staged change records and the gate takes it,
an unstaged one refuses.
"""

from __future__ import annotations

import hashlib
import importlib.util
import io
import json
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

_spec = importlib.util.spec_from_file_location(
    "release_wasm", ROOT / "release_wasm.py")
release_wasm = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(release_wasm)

_spec = importlib.util.spec_from_file_location(
    "package_check", ROOT / "package_check.py")
package_check = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(package_check)

TAG_SHA = "1" * 40
OTHER_SHA = "0" * 40
TAG_TREE = "2" * 40
MODULE = b"\x00" * 8
DIGEST = hashlib.sha256(MODULE).hexdigest()
COLD = {"dist/ocelli_wasm_bg.wasm": DIGEST,
        "dist/decode/ocelli_wasm_bg.wasm": DIGEST}


def build_id(git_sha: str = TAG_SHA, tree: str = TAG_TREE,
             dirty: bool = False) -> dict:
    return {"version": "0.1.0", "git_sha": git_sha, "tree": tree,
            "dirty": dirty,
            "modules": {"dist/ocelli_wasm_bg.wasm": DIGEST,
                        "dist/decode/ocelli_wasm_bg.wasm": DIGEST}}


def pack(self: unittest.TestCase, modules: dict[str, bytes],
         identity: bytes | None) -> Path:
    """A real tarball with the given module bytes and build id.

    The tarball lands in a temporary directory removed when the test ends.
    """
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w") as archive:
        entries = list(modules.items())
        if identity is not None:
            entries.append(("dist/build-id.json", identity))
        for name, data in entries:
            info = tarfile.TarInfo("package/" + name)
            info.size = len(data)
            archive.addfile(info, io.BytesIO(data))
    directory = tempfile.mkdtemp(prefix="ocelli-release-test-")
    self.addCleanup(shutil.rmtree, directory, True)
    path = Path(directory) / "ocelli-wasm-0.1.0.tgz"
    path.write_bytes(buffer.getvalue())
    return path


class CompareProblems(unittest.TestCase):
    def test_a_matching_build_is_accepted(self) -> None:
        self.assertEqual(release_wasm.compare_problems(
            build_id(), TAG_SHA, TAG_TREE, COLD), [])

    def test_a_mismatched_digest_is_refused(self) -> None:
        cold = {"dist/ocelli_wasm_bg.wasm": "2" * 64,
                "dist/decode/ocelli_wasm_bg.wasm": DIGEST}
        problems = release_wasm.compare_problems(
            build_id(), TAG_SHA, TAG_TREE, cold)
        self.assertEqual(len(problems), 1)
        self.assertIn("cold build of the tagged source", problems[0])
        self.assertIn("dist/ocelli_wasm_bg.wasm", problems[0])

    def test_a_wrong_sha_is_refused(self) -> None:
        problems = release_wasm.compare_problems(
            build_id(git_sha=OTHER_SHA), TAG_SHA, TAG_TREE, COLD)
        self.assertEqual(len(problems), 1)
        self.assertIn("not built from the tagged source", problems[0])

    def test_a_dirty_tree_build_is_refused(self) -> None:
        problems = release_wasm.compare_problems(
            build_id(dirty=True), TAG_SHA, TAG_TREE, COLD)
        self.assertEqual(len(problems), 1)
        self.assertIn("does not record a clean tree", problems[0])

    def test_a_build_id_without_the_dirty_field_is_refused(self) -> None:
        # F-237 review pass 2, N2. `is not False` fails closed for a build id
        # with no dirty field, and this input is what pins that direction:
        # loosening the check to `is True` turns this red while every other
        # test here stays green.
        raw = build_id()
        del raw["dirty"]
        problems = release_wasm.compare_problems(raw, TAG_SHA, TAG_TREE, COLD)
        self.assertEqual(len(problems), 1)
        self.assertIn("does not record a clean tree", problems[0])

    def test_a_staged_index_tree_is_refused(self) -> None:
        # The tree the build id records has to be the tag commit's. An index
        # that was staged over the tag is not a clean checkout of it, even
        # where the SHA still matches, and this is the fact release_wasm.py
        # refuses on that the packages gate cannot see.
        problems = release_wasm.compare_problems(
            build_id(tree="3" * 40), TAG_SHA, TAG_TREE, COLD)
        self.assertEqual(len(problems), 1)
        self.assertIn("is not the tagged source", problems[0])

    def test_a_build_id_without_modules_is_refused(self) -> None:
        problems = release_wasm.compare_problems(
            {"version": "0.1.0", "git_sha": TAG_SHA, "tree": TAG_TREE,
             "dirty": False},
            TAG_SHA, TAG_TREE, COLD)
        self.assertEqual(len(problems), 3)
        self.assertIn("records no modules object", problems[0])


class VerifyTarball(unittest.TestCase):
    def test_the_verified_tarball_is_accepted(self) -> None:
        identity = json.dumps(build_id()).encode()
        tarball = pack(self, dict.fromkeys(release_wasm.MODULES, MODULE),
                       identity)
        self.assertEqual(release_wasm.verify(tarball, TAG_SHA, TAG_TREE,
                                             COLD), [])

    def test_tarball_bytes_that_differ_from_the_cold_build_are_refused(
            self) -> None:
        # The identity records the cold build's digests and the staged bytes
        # are something else, which is an identity written without a rebuild.
        identity = json.dumps(build_id()).encode()
        tarball = pack(self, {"dist/ocelli_wasm_bg.wasm": b"\x01" * 8,
                              "dist/decode/ocelli_wasm_bg.wasm": MODULE},
                       identity)
        problems = release_wasm.verify(tarball, TAG_SHA, TAG_TREE, COLD)
        self.assertTrue(any("would not be publishing the compared bytes"
                            in problem for problem in problems))

    def test_a_tarball_without_the_build_id_is_refused(self) -> None:
        tarball = pack(self, dict.fromkeys(release_wasm.MODULES, MODULE),
                       None)
        with self.assertRaises(RuntimeError) as caught:
            release_wasm.verify(tarball, TAG_SHA, TAG_TREE, COLD)
        self.assertIn("dist/build-id.json", str(caught.exception))


class StageRecordedIdentity(unittest.TestCase):
    """The build id `stage.mjs` writes, read off a real index.

    F-237 review pass 2, D1. The stage runs `bin/ocelli.sh` for the two
    builds, so this drives it in a scratch repository whose runner is a stub
    that exits 0 and whose `pkg` out-dirs are dummies: the git state is real,
    the wasm build is not, and that is the half this suite is about. Each
    test then packs the staged dist and hands it to the gate's own identity
    check with `package_check.ROOT` pointed at the scratch repository, so
    the gate reads the scratch repository's HEAD and index and not this
    one's.
    """

    GLUE = ["ocelli_wasm.js", "ocelli_wasm_bg.wasm", "ocelli_wasm.d.ts",
            "ocelli_wasm_bg.wasm.d.ts"]

    def scratch_repository(self) -> Path:
        root = Path(tempfile.mkdtemp(prefix="ocelli-stage-test-"))
        self.addCleanup(shutil.rmtree, root, True)
        self.git(root, "init", "-q", "-b", "main")
        self.git(root, "config", "user.name", "ocelli stage test")
        self.git(root, "config", "user.email", "stage@ocelli.invalid")
        self.git(root, "config", "commit.gpgsign", "false")
        stub = root / "bin" / "ocelli.sh"
        stub.parent.mkdir(parents=True)
        stub.write_text("#!/bin/sh\nexit 0\n")
        stub.chmod(0o755)
        for module in ("pkg", "pkg-decode"):
            directory = root / "crates" / "ocelli-wasm" / module
            directory.mkdir(parents=True)
            for name in self.GLUE:
                (directory / name).write_bytes(b"\x00" * 8)
        for name in ("THIRD_PARTY_LICENSES", "THIRD_PARTY_NOTICES"):
            (root / name).write_text("notice\n")
        package = root / "packages" / "wasm"
        (package / "scripts").mkdir(parents=True)
        (package / "package.json").write_text(
            json.dumps({"name": "@ocelli/wasm", "version": "0.1.0"}) + "\n")
        shutil.copy2(ROOT / ".." / "packages" / "wasm" / "scripts" /
                     "stage.mjs", package / "scripts" / "stage.mjs")
        (root / "README.md").write_text("tracked\n")
        self.git(root, "add", "-A", "-f")
        self.git(root, "commit", "-q", "-m", "base")
        return root

    def git(self, root: Path, *args: str) -> str:
        done = subprocess.run(["git", *args], cwd=root, capture_output=True,
                              text=True)
        if done.returncode != 0:
            raise AssertionError(f"git {' '.join(args)} failed in the "
                                 f"scratch repository: {done.stderr.strip()}")
        return done.stdout.strip()

    def stage_and_pack(self, root: Path) -> Path:
        """Run the real stage under the stub runner and tar what it staged."""
        stage = root / "packages" / "wasm" / "scripts" / "stage.mjs"
        done = subprocess.run(["node", str(stage)], cwd=root,
                              capture_output=True, text=True)
        if done.returncode != 0:
            raise AssertionError(f"stage.mjs failed: {done.stderr.strip()}")
        dist = root / "packages" / "wasm" / "dist"
        identity = (dist / "build-id.json").read_bytes()
        modules = {"dist/ocelli_wasm_bg.wasm":
                   (dist / "ocelli_wasm_bg.wasm").read_bytes(),
                   "dist/decode/ocelli_wasm_bg.wasm":
                   (dist / "decode" / "ocelli_wasm_bg.wasm").read_bytes()}
        return pack(self, modules, identity)

    def identity_problems(self, root: Path, tarball: Path) -> list[str]:
        saved = package_check.ROOT
        package_check.ROOT = root
        try:
            return package_check.wasm_identity_from_tarball(tarball, "0.1.0")
        finally:
            package_check.ROOT = saved

    def test_a_staged_change_records_and_the_gate_takes_it(self) -> None:
        root = self.scratch_repository()
        (root / "README.md").write_text("staged\n")
        self.git(root, "add", "README.md")
        tarball = self.stage_and_pack(root)
        with tarfile.open(tarball) as archive:
            recorded = json.loads(
                archive.extractfile("package/dist/build-id.json").read())
        self.assertEqual(recorded["git_sha"],
                         self.git(root, "rev-parse", "HEAD"))
        self.assertEqual(recorded["tree"], self.git(root, "write-tree"))
        self.assertIs(recorded["dirty"], False)
        self.assertEqual(self.identity_problems(root, tarball), [])

    def test_an_unstaged_change_records_dirty_and_the_gate_refuses_it(
            self) -> None:
        root = self.scratch_repository()
        (root / "README.md").write_text("unstaged\n")
        tarball = self.stage_and_pack(root)
        with tarfile.open(tarball) as archive:
            recorded = json.loads(
                archive.extractfile("package/dist/build-id.json").read())
        self.assertIs(recorded["dirty"], True)
        problems = self.identity_problems(root, tarball)
        self.assertTrue(any("does not record a tree without unstaged "
                            "tracked changes" in problem
                            for problem in problems))


if __name__ == "__main__":
    unittest.main()
