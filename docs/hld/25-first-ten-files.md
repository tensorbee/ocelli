<!-- Originally converted by scripts/split_hld.py from Ocelli-HLD.docx.
     This tracked Markdown file is authoritative.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the tracked Markdown wins. -->

# The first ten files

**Source**: bootstrap import from `Ocelli-HLD.docx`, section 28. This tracked Markdown is authoritative.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

## 28. The first ten files

In this order. The goal of the first two weeks is to diff one windowed 2D image against cornerstone3D — everything below serves that.

| **\#** | **File** | **Why here** |
|----|----|----|
| 1 | crates/ocelli-core/src/space.rs | Coordinate spaces and transforms; everything downstream depends on them |
| 2 | crates/ocelli-core/src/value.rs | Stored, Modality and Display newtypes |
| 3 | crates/ocelli-pixel/src/lut.rs | The LUT chain plus the §18.3 fixtures. Written before any shader |
| 4 | tools/oracle/ | The differential harness. Nothing else should start before this works |
| 5 | crates/ocelli-codec/src/registry.rs | The decoder trait and registry |
| 6 | crates/ocelli-dicom/src/parse.rs | Parse and dispatch over dicom-rs |
| 7 | crates/ocelli-render/src/caps.rs | Tier detection, so tier assumptions are explicit from the start |
| 8 | crates/ocelli-render/src/device.rs | Device init and loss recovery |
| 9 | crates/ocelli-wasm/src/ring.rs | The event ring; one of two files permitted unsafe |
| 10 | packages/core/src/session.ts | The boundary's JavaScript side, including the bulk-write pattern |
