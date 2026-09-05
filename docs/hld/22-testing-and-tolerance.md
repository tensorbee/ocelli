<!-- Originally converted by scripts/split_hld.py from Ocelli-HLD.docx.
     This tracked Markdown file is authoritative.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the tracked Markdown wins. -->

# Testing and the tolerance policy

**Source**: bootstrap import from `Ocelli-HLD.docx`, section 25. This tracked Markdown is authoritative.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

## 25. Testing

| **Layer** | **What it proves** | **Where it comes from** |
|----|----|----|
| Unit and property | LUT arithmetic; geometry round-trips within epsilon | Hand-computed fixtures citing the DICOM section |
| Golden image | The rendered frame matches cornerstone3D | The oracle harness, over the corpus |
| Conformance | Each transfer syntax decodes correctly | Published DICOM test corpora |

```rust
proptest! {
    #[test]
    fn canvas_world_roundtrip(x in -1e4f64..1e4, y in -1e4f64..1e4) {
        let p = Pt::<Canvas>::new(x, y, 0.0);
        let t = viewport.canvas_to_world();
        let back = t.inverse().apply(t.apply(p));
        prop_assert!((back.x - p.x).abs() < 1e-6);
    }
}
```

### 25.1 Tolerance policy

Write it down once and hold it. Tuning tolerance per failure is how a suite stops meaning anything.

- **Monochrome 16-bit (CT, MR, CR, DR):** maximum absolute difference ≤ 1 LSB on at least 99.9% of pixels; zero pixels differing by more than 2.

- **Systematic bias, monochrome:** signed mean difference over the informative region within 0.1 of one display code, evaluated only where input identity, declared parameters and geometry already agree. The informative region and not the whole image rectangle, because a pixel clipped to black or white on both sides differs by nothing and cannot express a divergence, so counting it in the denominator hides one. Note what it cannot catch: the divergence between LINEAR and LINEAR_EXACT is `u / w` per pixel, so the bound fires only where the informative mean display code exceeds `0.1 * w`. That is content and not only width. No view with a window wider than 2550 can reach it for any content, and measured on the S03 corpus 20 of the 71 gating monochrome views cannot reach it either, the smallest blind window being 678. Added in S03 by operator decision through F-011's design plan. A maximum-difference bound cannot separate a systematic window-function divergence from rounding noise: LINEAR against LINEAR_EXACT at the soft-tissue window differs by 255 × (x + 160) / 159600, which peaks at 0.6375 of a display code and therefore never exceeds one code after quantisation to an 8-bit frame, so the bound above passes it everywhere. That divergence is one-sided and averages 0.32 of a code at the window centre, where the difference between two correct implementations averages zero. The bias bound is what makes the divergence §18.3 exists to warn about detectable by the oracle.

- **Colour and ultrasound:** perceptual difference below a stated threshold, because chroma subsampling and YBR conversion legitimately differ.

- **Geometry:** world coordinates within 1e-6 mm; canvas coordinates within a quarter pixel.

- A tolerance change is a pull request with a rationale, reviewed like code.
