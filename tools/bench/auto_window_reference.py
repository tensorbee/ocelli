"""Private page-ordered sampling reference. No printing, discovery or study execution.
Scalar arithmetic follows the separately reviewed F227 binary32 reference.
Numerical pool facts are for in-process use only, never log or serialize study results.
"""
from typing import NamedTuple
import contextlib
import numpy as np
from window_reference import reference_window
SAMPLE_CAP = 1 << 20
MAX_DECLARED_FRAMES = 16

class VerifierRefusal(Exception):
    def __init__(self, stage):
        super().__init__(stage)
        self.stage = stage

def refuse(stage):
    raise VerifierRefusal(stage) from None

def series_sample(images):
    if type(images) is not int or images <= 0:
        refuse("empty-series")
    k = min(images, MAX_DECLARED_FRAMES)
    return [0] if k == 1 else [i * (images - 1) // (k - 1) for i in range(k)]

def frame_stride(n_samples, budget):
    if n_samples <= 0 or budget <= 0:
        refuse("empty-frame")
    return (n_samples + budget - 1) // budget

def sample_positions(n_samples, stride):
    return np.arange(0, n_samples, stride, dtype=np.int64)

def nearest_ranks(n):
    if n <= 0:
        refuse("empty-pool")
    return (n + 99) // 100, (99 * n + 99) // 100

class FrameSamples(NamedTuple):
    stored: np.ndarray
    padding: tuple | None
    slope: float
    intercept: float

def _present(value):
    return value is not None and (not hasattr(value, "__len__") or len(value) != 0)

def _window_slots_present(value):
    """Only WC/WW: empty DS slots are absent, numeric zero remains present."""
    from pydicom.multival import MultiValue
    if isinstance(value, (MultiValue, list, tuple)):
        return any(slot is not None and slot != "" for slot in value)
    return value is not None and value != ""


def _macro(ds, index, name):
    for tag, offset in (("PerFrameFunctionalGroupsSequence", index),
                        ("SharedFunctionalGroupsSequence", 0)):
        groups = getattr(ds, tag, None)
        if groups is None:
            continue
        if offset >= len(groups):
            refuse("metadata-contract")
        seq = getattr(groups[offset], name, None)
        if seq is not None:
            if len(seq) != 1:
                refuse("metadata-contract")
            return seq[0]
    return ds

def _padding(ds):
    value = getattr(ds, "PixelPaddingValue", None)
    limit = getattr(ds, "PixelPaddingRangeLimit", None)
    if value is None:
        if limit is not None:
            refuse("invalid-padding")
        return None
    try:
        a, b = float(value), float(value if limit is None else limit)
        if not all(np.isfinite(x) and x == int(x) and abs(x) <= 2**24 for x in (a, b)):
            refuse("invalid-padding")
        return min(int(a), int(b)), max(int(a), int(b))
    except VerifierRefusal:
        raise
    except Exception:
        refuse("invalid-padding")

def _rescale_precedence(ds, index):
    item = _macro(ds, index, "PixelValueTransformationSequence")
    a, b = getattr(item, "RescaleSlope", None), getattr(item, "RescaleIntercept", None)
    if a is None and b is None:
        return 1.0, 0.0
    if a is None or b is None:
        refuse("metadata-contract")
    return float(a), float(b)

def read_frame_pydicom(path, frame_index):
    try:
        import pydicom
        ds = pydicom.dcmread(path)
        n = int(getattr(ds, "NumberOfFrames", 1))
        if type(frame_index) is not int or not 0 <= frame_index < n:
            refuse("decode-closed")
        if getattr(ds, "FloatPixelData", None) is not None or getattr(ds, "DoubleFloatPixelData", None) is not None:
            refuse("unsupported-contract")
        try:
            allocated, stored_bits, high, representation = (int(getattr(ds, tag)) for tag in
                ("BitsAllocated", "BitsStored", "HighBit", "PixelRepresentation"))
        except Exception:
            refuse("unsupported-contract")
        if (str(getattr(ds, "PhotometricInterpretation", "")) not in ("MONOCHROME1", "MONOCHROME2")
                or int(ds.SamplesPerPixel) != 1 or allocated not in (8, 16)
                or not 1 <= stored_bits <= allocated or high != stored_bits - 1
                or representation not in (0, 1)):
            refuse("unsupported-contract")
        voi = _macro(ds, frame_index, "FrameVOILUTSequence")
        if (_window_slots_present(getattr(voi, "WindowCenter", None))
                or _window_slots_present(getattr(voi, "WindowWidth", None))
                or _present(getattr(voi, "VOILUTSequence", None))):
            refuse("ineligible")
        if _present(getattr(ds, "ModalityLUTSequence", None)):
            refuse("unsupported-contract")
        arr = ds.pixel_array
        shape = (int(ds.Rows), int(ds.Columns))
        expected = shape if n == 1 else (n, *shape)
        if arr.shape != expected or not np.issubdtype(arr.dtype, np.integer):
            refuse("unsupported-contract")
        stored = (arr if n == 1 else arr[frame_index]).reshape(-1)
        return FrameSamples(stored, _padding(ds), *_rescale_precedence(ds, frame_index))
    except VerifierRefusal:
        raise
    except Exception:
        refuse("decode-closed")

def _to_modality(stored, slope, intercept):
    return np.add(np.multiply(stored.astype(np.float32), np.float32(slope), dtype=np.float32),
                  np.float32(intercept), dtype=np.float32)

def _total_sorted(values):
    bits = values.view(np.uint32)
    keys = np.where((bits & 0x80000000) != 0, ~bits, bits | 0x80000000)
    return values[np.argsort(keys, kind="stable")]

class SeriesPool:
    def __init__(self, declared_frames):
        if type(declared_frames) is not int or not 1 <= declared_frames <= 16:
            refuse("frame-count")
        self.declared, self.budget = declared_frames, SAMPLE_CAP // declared_frames
        self.added = self.skipped = self.counted_n = 0
        self.excluded_max = None
        self._counted = []

    def add_stored_frame(self, stored, padding, slope, intercept):
        try:
            if self.added + self.skipped >= self.declared:
                refuse("too-many-frames")
            arr = np.asarray(stored)
            if arr.ndim != 1 or not np.issubdtype(arr.dtype, np.integer):
                refuse("unsupported-contract")
            samples = arr[::frame_stride(arr.size, self.budget)]
            keep = samples != 0
            if padding is not None:
                if len(padding) != 2 or not all(np.isfinite(x) and x == int(x) and abs(x) <= 2**24 for x in padding):
                    refuse("invalid-padding")
                a, b = min(padding), max(padding)
                keep &= ~((samples >= a) & (samples <= b))
            with np.errstate(over="ignore", invalid="ignore"):
                mapped = _to_modality(samples, slope, intercept)
            if not np.isfinite(mapped).all():
                refuse("non-finite-modality")
            counted, excluded = mapped[keep], mapped[~keep]
            if excluded.size:
                m = float(_total_sorted(excluded)[-1])
                self.excluded_max = m if self.excluded_max is None else float(_total_sorted(np.array([self.excluded_max, m], dtype=np.float32))[-1])
            self._counted.append(counted)
            self.counted_n += counted.size
            self.added += 1
        except VerifierRefusal:
            raise
        except Exception:
            refuse("metadata-contract")

    def finish(self):
        if self.added == 0:
            refuse("empty-pool")
        result = {"stage": "pool-complete", "counted": int(self.counted_n), "skipped": self.skipped,
                  "excluded_max": self.excluded_max,
                  "window": {"stage": "window-contract", "window_blocked": True, "center": None, "width": None}}
        if self.counted_n:
            values = _total_sorted(np.concatenate(self._counted))
            a, b = nearest_ranks(self.counted_n)
            result.update(rank_low=a, rank_high=b, value_low=float(values[a-1]), value_high=float(values[b-1]))
        return result

def compute_series_window(records, reader=read_frame_pydicom):
    indices = series_sample(len(records))
    pool = SeriesPool(len(indices))
    stages = {}
    covered = True
    for i in indices:
        try:
            frame = reader(*records[i])
            pool.add_stored_frame(*frame)
        except VerifierRefusal as e:
            stage = e.stage
            pool.skipped += 1
            stages[stage] = stages.get(stage, 0) + 1
            covered &= stage == "ineligible"
        except Exception:
            pool.skipped += 1
            stages["decode-closed"] = stages.get("decode-closed", 0) + 1
            covered = False
    try:
        result = pool.finish()
    except VerifierRefusal as e:
        result = {"stage": e.stage, "counted": 0, "skipped": pool.skipped}
    result.update(record_count=len(records), sampled_count=len(indices), covered=covered,
                  skipped_stages=stages)
    if result["stage"] == "pool-complete" and covered:
        try:
            center, width, basis = reference_window(result.get("value_low"), result.get("value_high"),
                                                    excluded_max=result.get("excluded_max"))
            result["window"] = {"center": center, "width": width, "basis": basis, "window_blocked": False}
        except ValueError as e:
            stage = str(e)
            allowed = {"unverifiable-window", "empty-or-nonfinite-pool", "invalid-percentiles", "nonfinite-window"}
            result["stage"] = stage if stage in allowed else "window-contract"
    return result

def compare_observed_window(record, observed_center, observed_width):
    result = {"passed": False, "compared": False, "fields_compared": 0, "stage": "window-contract", "window_blocked": True}
    window = record.get("window", {})
    if not record.get("covered") or window.get("window_blocked", True):
        return result
    result["window_blocked"] = False
    try:
        if not isinstance(observed_center, str) or not isinstance(observed_width, str):
            return {**result, "stage": "observed-contract"}
        with np.errstate(all="ignore"):
            a, b = np.float32(observed_center), np.float32(observed_width)
        if not np.isfinite(a) or not np.isfinite(b) or b <= 0:
            return {**result, "stage": "observed-contract"}
        same = (a.view(np.uint32) == np.float32(window["center"]).view(np.uint32)
                and b.view(np.uint32) == np.float32(window["width"]).view(np.uint32))
        return {"passed": bool(same), "compared": True, "fields_compared": 2, "window_blocked": False,
                "stage": "verified" if same else "different-window"}
    except Exception:
        return {**result, "stage": "observed-contract"}


def _closed_result(stage, **counts):
    return {"passed": False, "stage": stage, "record_count": 0, "sampled_count": 0,
            "observation_count": 0, **counts}


def verify_mapping(mapping, input_root):
    """One caller-ordered series. Return only bool/count/closed-stage fields."""
    from pathlib import Path
    result = _closed_result("mapping-contract")
    try:
        if not isinstance(mapping, dict) or set(mapping) != {"records", "observed", "mode"}:
            return result
        rows, observed, mode = mapping["records"], mapping["observed"], mapping["mode"]
        if not isinstance(rows, list) or not rows or not isinstance(observed, list):
            return result
        if mode not in ("auto", "file-voi-control"):
            return result
        root = Path(input_root).resolve(strict=True)
        if not root.is_dir():
            return _closed_result("input-root-contract")
        records = []
        for row in rows:
            if not isinstance(row, dict) or set(row) != {"path", "frame"}:
                return result
            if not isinstance(row["path"], str) or type(row["frame"]) is not int or row["frame"] < 0:
                return result
            relative = Path(row["path"])
            if relative.is_absolute() or ".." in relative.parts:
                return _closed_result("input-root-contract")
            path = (root / relative).resolve(strict=True)
            if not path.is_relative_to(root) or not path.is_file():
                return _closed_result("input-root-contract")
            records.append((path, row["frame"]))
        facts = compute_series_window(records)
        counts = {"record_count": len(records), "sampled_count": len(series_sample(len(records))),
                  "observation_count": len(observed), "skipped_count": facts["skipped"]}
        if mode == "file-voi-control":
            passed = not observed and facts.get("skipped_stages") == {"ineligible": counts["sampled_count"]}
            return {**_closed_result("file-voi-control" if passed else "control-unverified", **counts), "passed": passed}
        if not observed or facts["stage"] != "pool-complete" or not facts.get("covered"):
            return _closed_result("calculation-unavailable", **counts)
        for observation in observed:
            if not isinstance(observation, dict) or set(observation) != {"center", "width"}:
                return _closed_result("observed-contract", **counts)
            checked = compare_observed_window(facts, observation["center"], observation["width"])
            if not checked["passed"]:
                return _closed_result(checked["stage"], **counts)
        return {**_closed_result("verified", **counts), "passed": True}
    except Exception:
        return result


@contextlib.contextmanager
def _quiet_diagnostics():
    """Serial CLI boundary. Discard diagnostics, never retain raw text in RAM."""
    import logging, os, sys, warnings
    disabled = logging.root.manager.disable
    saved = {}
    try:
        sys.stdout.flush()
        sys.stderr.flush()
        for descriptor in (1, 2):
            saved[descriptor] = os.dup(descriptor)
        with open(os.devnull, "w", encoding="utf-8") as sink:
            for descriptor in (1, 2):
                os.dup2(sink.fileno(), descriptor)
            logging.disable(logging.CRITICAL)
            with warnings.catch_warnings(), contextlib.redirect_stdout(sink), contextlib.redirect_stderr(sink):
                warnings.simplefilter("ignore")
                yield
    finally:
        try:
            for descriptor, previous in saved.items():
                os.dup2(previous, descriptor)
        finally:
            logging.disable(disabled)
            for previous in saved.values():
                os.close(previous)


def main(argv=None, summary_stream=None):
    """Closed CLI. No study values or foreign diagnostics reach output artifacts."""
    import json, os, sys
    args = sys.argv[1:] if argv is None else argv
    result = _closed_result("arguments-contract")
    target = None
    try:
        if len(args) != 6 or set(args[::2]) != {"--mapping", "--input-root", "--result"}:
            raise ValueError
        options = dict(zip(args[::2], args[1::2]))
        target = options["--result"]
        with _quiet_diagnostics():
            with open(options["--mapping"], encoding="utf-8") as handle:
                mapping = json.load(handle)
            result = verify_mapping(mapping, options["--input-root"])
    except Exception:
        result = _closed_result("input-contract")
    if target is not None:
        try:
            fd = os.open(target, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            with os.fdopen(fd, "w", encoding="utf-8") as handle:
                json.dump(result, handle, sort_keys=True)
                handle.write("\n")
        except Exception:
            result = _closed_result("result-contract")
    summary = (("PASS" if result["passed"] else "FAIL") + " records=" + str(result["record_count"])
               + " sampled=" + str(result["sampled_count"]) + " observations=" + str(result["observation_count"]))
    if summary_stream is None:
        print(summary)
    else:
        summary_stream.write(summary + "\n")
        summary_stream.flush()
    return 0 if result["passed"] else 1


def _entrypoint():
    """Keep native/Python diagnostics discarded through interpreter shutdown.

    Only this local duplicate can publish the deliberately closed summary.
    Native stdio may defer output until atexit, after a temporary guard ends.
    """
    import logging, os, sys, warnings
    summary_stream = None
    try:
        summary_stream = os.fdopen(os.dup(1), "w", encoding="utf-8")
        sink = open(os.devnull, "w", encoding="utf-8")
        os.dup2(sink.fileno(), 1)
        os.dup2(sink.fileno(), 2)
        # Intentionally remain open and installed through process shutdown.
        sys.stdout = sys.stderr = sink
        logging.disable(logging.CRITICAL)
        warnings.simplefilter("ignore")
        return main(summary_stream=summary_stream)
    except BaseException:
        if summary_stream is not None:
            try:
                summary_stream.write("FAIL records=0 sampled=0 observations=0\n")
                summary_stream.flush()
            except Exception:
                pass
        return 1
    finally:
        if summary_stream is not None:
            summary_stream.close()


if __name__ == "__main__":
    raise SystemExit(_entrypoint())
