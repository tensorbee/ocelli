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

- **Colour and ultrasound:** perceptual difference below a stated threshold, because chroma subsampling and YBR conversion legitimately differ.

- **Geometry:** world coordinates within 1e-6 mm; canvas coordinates within a quarter pixel.

- A tolerance change is a pull request with a rationale, reviewed like code.
