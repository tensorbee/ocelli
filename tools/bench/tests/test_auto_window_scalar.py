import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
"""Hand-derived controls from the F227 plan and PS3.3 C.11.2.1.3.2.

Expected numbers are not read from the product implementation or its output.
"""
import unittest
import numpy as np
import window_reference as ref
from window_reference import F, _search, display, reference_window, step


def bits(value):
    return int(F(value).view(np.uint32))


class WindowReferenceTests(unittest.TestCase):
    def test_piecewise_linear_exact(self):
        # DICOM formula at c=40,w=400: lower=-160, upper=240, middle=127.5.
        self.assertEqual([display(F(x), F(40), F(400)) for x in (-160, 40, 240)],
                         [F(0), F(127.5), F(255)])

    def test_background_and_constant(self):
        self.assertEqual(reference_window(excluded_max=0), (F(.5), F(1), "background"))
        self.assertEqual(reference_window(7, 7), (F(7), F(1), "constant"))

    def test_symmetric_margin(self):
        # Moving +/-1 inward16 steps subtracts16*2^-24 from each magnitude.
        # Their difference is 2-2^-19, and the midpoint is exactly zero.
        c, w, basis = reference_window(-1, 1)
        self.assertEqual((bits(c), bits(w), basis), (0, 0x3ffffff0, "percentile"))

    def test_four_point_margin_not_only_original_pair(self):
        # q=2^-24. Original:1-17q,1+34q. Inward16:1-q,1+2q.
        # Width3q/center1 maps moved low near42.5, so it must not pass.
        # Width2q is the first geometric candidate whose two moved ends pass.
        q = F(2 ** -24)
        low, high = F(1 - 17 * float(q)), F(1 + 34 * float(q))
        self.assertEqual((display(low, F(1), F(3*q)), display(high, F(1), F(3*q))),
                         (F(0), F(255)))
        self.assertGreater(display(F(1-q), F(1), F(3*q)), F(0))
        c, w, basis = reference_window(low, high)
        self.assertEqual((bits(c), bits(w), basis), (0x3f800000, 0x34000000, "percentile"))

    def test_constant_widens_from_hand_inequality(self):
        # At32768,16 upward steps span.0625. Require255*.0625/w<.5,
        # hence w>31.875. Candidates17 fails and33 is first passing.
        self.assertEqual(reference_window(32768, 32768), (F(32768), F(33), "constant"))

    def test_subnormal_half_rounds_before_midpoint(self):
        q = F(2 ** -149)
        c, w, basis = reference_window(F(3*q), F(4*q))
        self.assertEqual((bits(c), bits(w), basis), (3, 0x3f800000, "constant"))

    def test_numeric_zero_steps_and_equal_sign(self):
        q = F(2 ** -149)
        self.assertEqual(step(F(-0.), np.inf), q)
        self.assertEqual(step(F(0.), -np.inf), -q)
        self.assertEqual(bits(reference_window(F(-0.), F(-0.))[0]), 0x80000000)

    def test_narrowing_uses_downward_spacing_at_power_of_two(self):
        # From1, the downward neighbor is1-2^-24, not1-2^-23.
        c, w = _search(F(1), False, lambda _: F(0), lambda _, x: x < F(1))
        self.assertEqual((bits(c), bits(w)), (0, 0x3f7fffff))

    def test_last_geometric_candidate_is_available(self):
        # v=2^35-32768. Its16 neighbors either way are32768 away.
        # At width2^24:255*32768/width=255/512=.498046875<.5.
        # The preceding width1+2^23 fails.1+2^24 rounds to2^24.
        c, w, basis = reference_window(F(2**35-32768), F(2**35-32768))
        self.assertEqual((bits(c), bits(w), basis), (0x50fffff0, 0x4b800000, "constant"))

    def test_overflowing_comparison_bound_is_not_failed_output(self):
        # c=w=x=MAX. Upper c+w/2 overflows, while x-c is exactly0.
        with np.errstate(all="ignore"):
            maximum = F(np.finfo(np.float32).max)
            self.assertEqual(display(maximum, maximum, maximum), F(127.5))

    def test_background_center_recomputed_when_width_changes(self):
        # At2^23, width1 half=.5 leaves rounded center at the maximum.
        # First wider width1+2^-23 has half>.5 and rounds center to maximum+1.
        try:
            c, w, basis = reference_window(excluded_max=F(2**23))
        except ValueError:
            self.fail("background candidate unexpectedly refused")
        self.assertEqual((bits(c), bits(w), basis), (0x4b000001, 0x3f800001, "background"))

    def test_closed_refusals(self):
        maximum = F(np.finfo(np.float32).max)
        cases = [
            ({"excluded_max": np.nan}, "empty-or-nonfinite-pool"),
            ({"low": np.nan, "high": 1}, "invalid-percentiles"),
            ({"low": 1, "high": 0}, "invalid-percentiles"),
            ({"low": -maximum, "high": maximum}, "nonfinite-window"),
            # At2^63,16 upward binary32 steps span2^44. The strict
            # half-grey check needs w>510*2^44. The largest of the
            # base plus48changed candidates is2^24, so all must refuse.
            ({"low": F(2**63), "high": F(2**63)}, "unverifiable-window"),
        ]
        for arguments, stage in cases:
            with self.subTest(stage=stage, arguments=arguments):
                with self.assertRaisesRegex(ValueError, "^" + stage + "$"):
                    ref.reference_window(**arguments)


if __name__ == "__main__":
    unittest.main()
