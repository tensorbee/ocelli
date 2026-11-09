"""Synthetic CLI composition and privacy controls, no operator study access."""
import contextlib, io, json, logging, os, subprocess, tempfile, unittest, warnings
from pathlib import Path
from unittest.mock import patch
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import numpy as np
from pydicom.dataset import FileDataset, FileMetaDataset
from pydicom.uid import ExplicitVRLittleEndian
import auto_window_reference as ref


def fixture(path, windowed=False):
    meta = FileMetaDataset()
    meta.TransferSyntaxUID = ExplicitVRLittleEndian
    meta.MediaStorageSOPClassUID = "1.2.840.10008.5.1.4.1.1.7"
    meta.MediaStorageSOPInstanceUID = "1.2.826.0.1.3680043.10.543.227.1"
    ds = FileDataset(str(path), {}, file_meta=meta, preamble=b"\0" * 128)
    ds.SOPClassUID = meta.MediaStorageSOPClassUID
    ds.SOPInstanceUID = meta.MediaStorageSOPInstanceUID
    ds.Rows, ds.Columns, ds.SamplesPerPixel = 1, 2, 1
    ds.PhotometricInterpretation = "MONOCHROME2"
    ds.BitsAllocated = ds.BitsStored = 16
    ds.HighBit, ds.PixelRepresentation = 15, 0
    ds.PixelData = np.array([7, 7], dtype="<u2").tobytes()
    if windowed:
        ds.WindowCenter, ds.WindowWidth = "7", "1"
    ds.save_as(path, enforce_file_format=True)


class CliTests(unittest.TestCase):
    def test_composed_constant_exact_unrounded_bits(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); fixture(root / "synthetic.dcm")
            m = {"mode": "auto", "records": [{"path": "synthetic.dcm", "frame": 0}],
                 "observed": [{"center": "7", "width": "1"}]}
            self.assertTrue(ref.verify_mapping(m, root)["passed"])
            m["observed"][0]["center"] = "7.000000476837158203125"
            self.assertEqual(ref.verify_mapping(m, root)["stage"], "different-window")
            m["observed"][0]["center"] = 7
            self.assertEqual(ref.verify_mapping(m, root)["stage"], "observed-contract")

    def test_file_voi_control_does_not_claim_auto(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); fixture(root / "synthetic.dcm", windowed=True)
            m = {"mode": "file-voi-control", "records": [{"path": "synthetic.dcm", "frame": 0}], "observed": []}
            r = ref.verify_mapping(m, root)
            self.assertTrue(r["passed"]); self.assertEqual(r["stage"], "file-voi-control")
            m["mode"] = "auto"; m["observed"] = [{"center": "7", "width": "1"}]
            self.assertFalse(ref.verify_mapping(m, root)["passed"])

    def test_root_escape_is_closed(self):
        with tempfile.TemporaryDirectory() as d:
            m = {"mode": "auto", "records": [{"path": "../private-canary", "frame": 0}],
                 "observed": [{"center": "7", "width": "1"}]}
            self.assertEqual(ref.verify_mapping(m, d)["stage"], "input-root-contract")

    def test_closed_cli_and_private_result(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); fixture(root / "synthetic.dcm")
            mapping = root / "map.json"; result = root / "result.json"
            mapping.write_text(json.dumps({"mode": "auto", "records": [{"path": "synthetic.dcm", "frame": 0}],
                                          "observed": [{"center": "7", "width": "1"}]}))
            argv = ["--mapping", str(mapping), "--input-root", str(root), "--result", str(result)]
            capture = io.StringIO()
            with contextlib.redirect_stdout(capture): code = ref.main(argv)
            self.assertEqual(code, 0)
            self.assertEqual(capture.getvalue(), "PASS records=1 sampled=1 observations=1\n")
            r = json.loads(result.read_text()); self.assertTrue(r["passed"])
            self.assertEqual(os.stat(result).st_mode & 0o777, 0o600)
            before = result.read_bytes()
            with contextlib.redirect_stdout(io.StringIO()): self.assertEqual(ref.main(argv), 1)
            self.assertEqual(result.read_bytes(), before)

    def test_foreign_diagnostics_do_not_escape_cli(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); (root / "synthetic.dcm").touch()
            mapping = root / "map.json"; result = root / "result.json"
            mapping.write_text(json.dumps({"mode": "auto", "records": [{"path": "synthetic.dcm", "frame": 0}],
                                          "observed": [{"center": "7", "width": "1"}]}))
            def foreign(_):
                print("SYNTHETIC_PRIVATE_CANARY")
                print("SYNTHETIC_PRIVATE_CANARY", file=sys.stderr)
                logging.warning("SYNTHETIC_PRIVATE_CANARY")
                warnings.warn("SYNTHETIC_PRIVATE_CANARY")
                raise RuntimeError("SYNTHETIC_PRIVATE_CANARY")
            capture, err = io.StringIO(), io.StringIO(); previous = logging.root.manager.disable
            leaked = []
            class Collector(logging.Handler):
                def emit(self, record): leaked.append(record.getMessage())
            collector = Collector(); logging.getLogger().addHandler(collector)
            try:
                with patch("pydicom.dcmread", side_effect=foreign), contextlib.redirect_stdout(capture), contextlib.redirect_stderr(err):
                    code = ref.main(["--mapping", str(mapping), "--input-root", str(root), "--result", str(result)])
            finally:
                logging.getLogger().removeHandler(collector)
            self.assertEqual(leaked, [])
            self.assertEqual(code, 1); self.assertEqual(err.getvalue(), "")
            self.assertNotIn("CANARY", capture.getvalue() + result.read_text())
            self.assertEqual(logging.root.manager.disable, previous)
    def test_native_diagnostics_are_discarded_and_streams_restored(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); fixture(root / "synthetic.dcm")
            mapping = root / "map.json"; result = root / "result.json"
            mapping.write_text(json.dumps({"mode": "auto", "records": [{"path": "synthetic.dcm", "frame": 0}],
                                          "observed": [{"center": "7", "width": "1"}]}))
            program = """import os,sys
from unittest.mock import patch
import auto_window_reference as ref
original_out,original_err=sys.stdout,sys.stderr
def foreign(_):
    os.write(1,b'SYNTHETIC_PRIVATE_CANARY\\n')
    os.write(2,b'SYNTHETIC_PRIVATE_CANARY\\n')
    raise RuntimeError('SYNTHETIC_PRIVATE_CANARY')
with patch('pydicom.dcmread',side_effect=foreign):
    code=ref.main(sys.argv[1:])
assert sys.stdout is original_out and sys.stderr is original_err
sys.stdout.flush();sys.stderr.flush()
os.write(1,b'RESTORED-STDOUT\\n')
os.write(2,b'RESTORED-STDERR\\n')
raise SystemExit(code)
"""
            env = {**os.environ, "PYTHONPATH": str(Path(__file__).resolve().parents[1]),
                   "PYTHONDONTWRITEBYTECODE": "1"}
            child = subprocess.run([sys.executable, "-c", program, "--mapping", str(mapping),
                                    "--input-root", str(root), "--result", str(result)],
                                   env=env, capture_output=True, text=True)
            self.assertEqual(child.returncode, 1)
            self.assertNotIn("CANARY", child.stdout + child.stderr + result.read_text())
            self.assertEqual(child.stdout, "FAIL records=1 sampled=1 observations=1\nRESTORED-STDOUT\n")
            self.assertEqual(child.stderr, "RESTORED-STDERR\n")
            success_result = root / "success.json"
            success_program = """import os,sys
import auto_window_reference as ref
original_out,original_err=sys.stdout,sys.stderr
code=ref.main(sys.argv[1:])
assert sys.stdout is original_out and sys.stderr is original_err
sys.stdout.flush();sys.stderr.flush()
os.write(1,b'RESTORED-STDOUT\\n')
os.write(2,b'RESTORED-STDERR\\n')
raise SystemExit(code)
"""
            success = subprocess.run([sys.executable, "-c", success_program, "--mapping", str(mapping),
                                      "--input-root", str(root), "--result", str(success_result)],
                                     env=env, capture_output=True, text=True)
            self.assertEqual(success.returncode, 0)
            self.assertEqual(success.stdout, "PASS records=1 sampled=1 observations=1\nRESTORED-STDOUT\n")
            self.assertEqual(success.stderr, "RESTORED-STDERR\n")

    def test_actual_pydicom_all_empty_window_slots_are_auto(self):
        from pydicom import dcmread
        for slots in ([], ["", ""]):
            with tempfile.TemporaryDirectory() as d:
                root = Path(d); path = root / "synthetic.dcm"; fixture(path)
                ds = dcmread(path)
                ds.WindowCenter = slots; ds.WindowWidth = slots
                ds.save_as(path, enforce_file_format=True)
                mapping = {"mode": "auto", "records": [{"path": "synthetic.dcm", "frame": 0}],
                           "observed": [{"center": "7", "width": "1"}]}
                self.assertTrue(ref.verify_mapping(mapping, root)["passed"])
                mapping["mode"] = "file-voi-control"; mapping["observed"] = []
                self.assertFalse(ref.verify_mapping(mapping, root)["passed"])

    def test_all_empty_pydicom_slots_do_not_certify_file_voi_control(self):
        from pydicom import dcmread
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); path = root / "synthetic.dcm"; fixture(path)
            ds = dcmread(path); ds.WindowCenter = ["", ""]; ds.WindowWidth = ["", ""]
            ds.save_as(path, enforce_file_format=True)
            mapping = {"mode": "file-voi-control", "records": [{"path": "synthetic.dcm", "frame": 0}],
                       "observed": []}
            self.assertFalse(ref.verify_mapping(mapping, root)["passed"])

    def test_typed_none_window_slots_remain_absent(self):
        from pydicom import dcmread
        from pydicom.dataelem import DataElement
        from pydicom.multival import MultiValue
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "synthetic.dcm"; fixture(path)
            ds = dcmread(path)
            for tag in (0x00281050, 0x00281051):
                ds[tag] = DataElement(tag, "DS", MultiValue(lambda x: x, [None, None]), already_converted=True)
            with patch("pydicom.dcmread", return_value=ds):
                frame = ref.read_frame_pydicom(path, 0)
            self.assertEqual(frame.stored.tolist(), [7, 7])

    def test_real_zero_and_mixed_window_slots_remain_ineligible(self):
        from pydicom import dcmread
        for slots in ([0], ["", 7]):
            with tempfile.TemporaryDirectory() as d:
                root = Path(d); path = root / "synthetic.dcm"; fixture(path)
                ds = dcmread(path); ds.WindowCenter = slots; ds.WindowWidth = slots
                ds.save_as(path, enforce_file_format=True)
                mapping = {"mode": "file-voi-control", "records": [{"path": "synthetic.dcm", "frame": 0}],
                           "observed": []}
                self.assertTrue(ref.verify_mapping(mapping, root)["passed"])
                mapping["mode"] = "auto"; mapping["observed"] = [{"center": "7", "width": "1"}]
                self.assertFalse(ref.verify_mapping(mapping, root)["passed"])

    def test_nonempty_voi_sequence_is_not_empty_window_absence(self):
        from pydicom import dcmread
        from pydicom.dataset import Dataset
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); path = root / "synthetic.dcm"; fixture(path)
            ds = dcmread(path); ds.VOILUTSequence = [Dataset()]
            ds.save_as(path, enforce_file_format=True)
            with self.assertRaises(ref.VerifierRefusal) as caught:
                ref.read_frame_pydicom(path, 0)
            self.assertEqual(caught.exception.stage, "ineligible")

    def test_actual_entrypoint_holds_guard_through_native_stdio_shutdown(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); fixture(root / "synthetic.dcm")
            mapping = root / "map.json"; result = root / "failure.json"
            mapping.write_text(json.dumps({"mode": "auto", "records": [{"path": "synthetic.dcm", "frame": 0}],
                                          "observed": [{"center": "7", "width": "1"}]}))
            script = Path(ref.__file__).resolve()
            program = """import atexit,ctypes,logging,os,runpy,sys
from unittest.mock import patch
libc=ctypes.CDLL(None)
libc.puts.argtypes=[ctypes.c_char_p]
libc.puts.restype=ctypes.c_int
def foreign(_):
    libc.puts(b'SYNTHETIC_BUFFERED_PRIVATE_CANARY')
    os.write(1,b'SYNTHETIC_DIRECT_PRIVATE_CANARY\\n')
    os.write(2,b'SYNTHETIC_DIRECT_PRIVATE_CANARY\\n')
    atexit.register(lambda: libc.puts(b'SYNTHETIC_ATEXIT_PRIVATE_CANARY'))
    atexit.register(lambda: print('SYNTHETIC_ATEXIT_PRIVATE_CANARY'))
    atexit.register(lambda: logging.warning('SYNTHETIC_ATEXIT_PRIVATE_CANARY'))
    raise RuntimeError('SYNTHETIC_PRIVATE_CANARY')
script=sys.argv.pop(1)
with patch('pydicom.dcmread',side_effect=foreign):
    runpy.run_path(script,run_name='__main__')
"""
            env = {**os.environ, "PYTHONPATH": str(script.parent), "PYTHONDONTWRITEBYTECODE": "1"}
            command = [sys.executable, "-c", program, str(script), "--mapping", str(mapping),
                       "--input-root", str(root), "--result", str(result)]
            child = subprocess.run(command, env=env, capture_output=True, text=True)
            self.assertEqual(child.returncode, 1)
            self.assertEqual(child.stdout, "FAIL records=1 sampled=1 observations=1\n")
            self.assertEqual(child.stderr, "")
            self.assertNotIn("CANARY", child.stdout + child.stderr + result.read_text())
            success_result = root / "success.json"
            success = subprocess.run([sys.executable, str(script), "--mapping", str(mapping),
                                      "--input-root", str(root), "--result", str(success_result)],
                                     env=env, capture_output=True, text=True)
            self.assertEqual(success.returncode, 0)
            self.assertEqual(success.stdout, "PASS records=1 sampled=1 observations=1\n")
            self.assertEqual(success.stderr, "")
            self.assertTrue(json.loads(success_result.read_text())["passed"])



if __name__ == "__main__":
    unittest.main()
