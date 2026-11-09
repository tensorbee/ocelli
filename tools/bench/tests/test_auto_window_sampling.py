import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
"""Source-only synthetic tests for the independent AC16 verifier. Pure path
only: no filesystem writes, no study access, no product imports, and no
numerical output beyond pass/fail. Expected sampling, ranks and pooled values
come only from the approved plan; the final center/width contract
(window-contract) is blocked and is never asserted as an expected value.
"""
import numpy as np
import unittest

from auto_window_reference import (SeriesPool, VerifierRefusal,
                                compare_observed_window, compute_series_window,
                                frame_stride, nearest_ranks, sample_positions,
                                series_sample)

def fold(frames, slopes, paddings=None, intercepts=None):
    """Pure fold of synthetic frames through SeriesPool; no filesystem."""
    pool = SeriesPool(len(frames))
    for i, stored in enumerate(frames):
        pool.add_stored_frame(stored,
                              paddings[i] if paddings else None,
                              slopes[i],
                              intercepts[i] if intercepts else 0.0)
    return pool.finish()


class SamplingTests(unittest.TestCase):
    def test_series_sample_exact_indices_and_empty_refusal(self):
        assert series_sample(1) == [0]
        assert series_sample(2) == [0, 1]
        assert series_sample(16) == list(range(16))
        assert series_sample(17) == [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16]
        assert series_sample(267) == [0, 17, 35, 53, 70, 88, 106, 124, 141, 159, 177,
                                      195, 212, 230, 248, 266]
        assert series_sample(175) == [0, 11, 23, 34, 46, 58, 69, 81, 92, 104, 116,
                                      127, 139, 150, 162, 174]
        assert nearest_ranks(150) == (2, 149)
        with unittest.TestCase().assertRaises(VerifierRefusal):
            series_sample(0)

    def test_own_slopes_pool_is_low2_high196_not_old_final_width(self):
        unit = np.arange(1, 101, dtype=np.int64)
        record = fold([unit, unit], (1.0, 2.0))
        assert record["counted"] == 200
        assert record["rank_low"] == 2 and record["rank_high"] == 198
        assert record["value_low"] == 2.0 and record["value_high"] == 196.0
        assert record["window"]["window_blocked"] and record["window"]["width"] is None

    def test_per_frame_padding_and_stored_zero_exclusion(self):
        frame_a = np.array([0] * 50 + list(range(1, 51)), dtype=np.int64)
        frame_b = np.array([-2000] * 20 + list(range(51, 101)), dtype=np.int64)
        record = fold([frame_a, frame_b], (1.0, 1.0),
                      paddings=(None, (-2000, -2000)))
        assert record["counted"] == 100
        assert record["value_low"] == 1.0 and record["value_high"] == 99.0
        stored_zero = np.array([0] * 50 + list(range(1000, 1100)), dtype=np.int64)
        record = fold([stored_zero], (1.0,), intercepts=(-1024.0,))
        assert record["value_low"] == -24.0 and record["value_high"] == 74.0

    def test_nearest_rank_single_frame_150(self):
        record = fold([np.arange(1, 151, dtype=np.int64)], (1.0,))
        assert record["rank_low"] == 2 and record["rank_high"] == 149
        assert record["value_low"] == 2.0 and record["value_high"] == 149.0

    def test_endpoint_inclusion_and_cap_split(self):
        count = (1 << 19) + 1
        budget = (1 << 20) // 2
        stride = frame_stride(count, budget)
        assert stride == 2
        positions = sample_positions(count, stride)
        assert positions[0] == 0 and positions[-1] == count - 1
        assert positions.size == 262145
        frame_a = np.empty(count, dtype=np.int64)
        frame_a[1::2] = 60000
        frame_a[0::2] = (np.arange((count + 1) // 2) % 100) + 1
        record = fold([frame_a, np.arange(1, 101, dtype=np.int64)], (1.0, 1.0))
        assert record["counted"] == 262245
        assert record["value_low"] == 1.0 and record["value_high"] == 99.0
        assert frame_stride(1 << 20, (1 << 20) // 1) == 1
        assert sample_positions(1 << 20, 1).size == 1 << 20

    def test_constant7_and_allzero_tracked_distinct(self):
        seven = fold([np.full(16, 7, dtype=np.int64)], (1.0,))
        zero = fold([np.zeros(16, dtype=np.int64)], (1.0,))
        assert seven["counted"] == 16 and seven["value_low"] == 7.0
        assert zero["counted"] == 0 and zero["excluded_max"] is not None
        assert zero["window"]["window_blocked"] and seven["window"]["width"] is None

    def test_skipped_sample_not_replaced_and_cap_preserved(self):
        pool = SeriesPool(3)
        assert pool.budget == (1 << 20) // 3
        pool.add_stored_frame(np.arange(1, 101, dtype=np.int64), None, 1.0, 0.0)
        pool.skipped += 1  # an ineligible sampled frame contributes nothing
        pool.add_stored_frame(np.arange(101, 201, dtype=np.int64), None, 1.0, 0.0)
        record = pool.finish()
        assert record["skipped"] == 1 and record["counted"] == 200

    def test_overflow_closed_and_pool_unharmed(self):
        pool = SeriesPool(2)
        with unittest.TestCase().assertRaises(VerifierRefusal) as refusal:
            pool.add_stored_frame(np.array([2], dtype=np.int64), None, 3.0e38, 0.0)
        assert refusal.exception.stage == "non-finite-modality"
        pool.add_stored_frame(np.arange(1, 101, dtype=np.int64), None, 1.0, 0.0)
        assert pool.finish()["counted"] == 100

    def test_closed_stages_reveal_no_canary_path_or_value(self):
        path = "/canary/study/patient-x.dcm"

        def leaking_reader(p, f):
            raise RuntimeError(f"leak {p} code=424242")

        record = compute_series_window([(path, 0)], reader=leaking_reader)
        assert record["skipped"] == 1 and record["stage"] == "empty-pool"
        text = repr(record).lower()
        assert "canary" not in text and "patient-x" not in text and "424242" not in text

    def test_blocked_comparator_cannot_pass_even_with_observed_numbers(self):
        outcome = compare_observed_window({"stage": "pool-complete"}, 25.0, 49.0)
        assert outcome["passed"] is False and outcome["compared"] is False
        assert outcome["stage"] == "window-contract" and outcome["window_blocked"] is True
        text = repr(outcome)
        assert "25" not in text and "49" not in text

if __name__ == "__main__":
    unittest.main()
