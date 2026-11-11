"""Synthetic selected-frame, descriptor and privacy controls."""
import sys
import types
import unittest
from pathlib import Path
from unittest.mock import patch
import numpy as np
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import auto_window_reference as w
N=types.SimpleNamespace

def macro(slope,intercept): return N(PixelValueTransformationSequence=[N(RescaleSlope=slope,RescaleIntercept=intercept)])
def ds():
    return N(NumberOfFrames=2,Rows=1,Columns=2,SamplesPerPixel=1,BitsAllocated=16,BitsStored=16,HighBit=15,PixelRepresentation=1,
             PhotometricInterpretation="MONOCHROME2",pixel_array=np.array([[[1,2]],[[3,4]]],dtype=np.int16),
             PerFrameFunctionalGroupsSequence=[macro(1,0),macro(2,10)],
             SharedFunctionalGroupsSequence=[macro(4,20)],RescaleSlope=8,RescaleIntercept=40)


class AdapterTests(unittest.TestCase):
    def test_frame_index(self):
        data=ds()
        with patch("pydicom.dcmread",return_value=data):
            f=w.read_frame_pydicom("synthetic-only",1)
        assert (f.slope,f.intercept)==(2,10)
        assert np.array_equal(f.stored,np.array([3,4]))

    def test_per_frame_window(self):
        data=ds();data.PerFrameFunctionalGroupsSequence[1].FrameVOILUTSequence=[N(WindowCenter=10,WindowWidth=20)]
        with patch("pydicom.dcmread",return_value=data):
            try:w.read_frame_pydicom("synthetic-only",1)
            except w.VerifierRefusal as e:assert e.stage=="ineligible"
            else:raise AssertionError("per-frame window was pooled")

    def test_shared_then_top(self):
        data=ds();data.PerFrameFunctionalGroupsSequence=[N(),N()]
        with patch("pydicom.dcmread",return_value=data):
            f=w.read_frame_pydicom("synthetic-only",1)
            assert (f.slope,f.intercept)==(4,20)
            data.SharedFunctionalGroupsSequence=[N()]
            f=w.read_frame_pydicom("synthetic-only",1)
            assert (f.slope,f.intercept)==(8,40)

    def test_total_zero_order(self):
        p=w.SeriesPool(1);p.add_stored_frame(np.array([-1,1]),None,-0.,-0.)
        r=p.finish()
        assert np.float32(r["value_low"]).view(np.uint32)==0x80000000
        assert np.float32(r["value_high"]).view(np.uint32)==0

    def test_padding_closed_and_transaction(self):
        for padding in [(np.nan,0),(0,2**24+1)]:
            p=w.SeriesPool(1)
            try:p.add_stored_frame(np.array([1]),padding,1,0)
            except w.VerifierRefusal as e:assert e.stage=="invalid-padding"
            else:raise AssertionError("bad padding accepted")
            assert p.added==0 and p.counted_n==0
        p=w.SeriesPool(1);p.add_stored_frame(np.array([-2000,1,2]),(-1999,-2001),1,0)
        assert p.finish()["counted"]==2

    def test_metadata_privacy(self):
        p=w.SeriesPool(1)
        try:p.add_stored_frame(np.array([1]),None,"SYNTHETIC_PRIVATE_CANARY",0)
        except w.VerifierRefusal as e:assert str(e)=="metadata-contract"
        else:raise AssertionError("invalid slope accepted")

    def test_sample_skips_do_not_replace(self):
        calls=[]
        def reader(path,frame):
            calls.append(path)
            if path==14:raise RuntimeError("SYNTHETIC_PRIVATE_CANARY")
            return w.FrameSamples(np.array([path+1]),None,1,0)
        r=w.compute_series_window([(i,0) for i in range(17)],reader)
        assert calls==list(range(15))+[16] and r["skipped"]==1 and r["counted"]==15
    def test_descriptor_coverage_is_closed(self):
        for tag, value in (("BitsAllocated", 32), ("BitsStored", 0), ("BitsStored", 17),
                           ("HighBit", 14), ("PixelRepresentation", 2)):
            data = ds(); setattr(data, tag, value)
            with patch("pydicom.dcmread", return_value=data):
                with self.assertRaises(w.VerifierRefusal) as caught:
                    w.read_frame_pydicom("synthetic-only", 0)
            self.assertEqual(caught.exception.stage, "unsupported-contract")


if __name__ == "__main__":
    unittest.main()
