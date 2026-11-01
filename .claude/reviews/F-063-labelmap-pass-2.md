# F-063 review, pass 2

**Reviewed**: working tree on `sprint/s22` at `74814ba84870ce251dcc33d4eb40a9b286d577a8`. The implementation is uncommitted. `git rev-parse HEAD` was the same at the start and at the end. Nothing was staged. Nothing was committed. The only product edit in this pass was a temporary eight-neighbour step list in `crates/ocelli-seg/src/labelmap.rs`. It was restored. The file sha256 before that edit and after the restore is `cdd1e5eaa3635fc506e635c291b14927ad34ce01759016bbf52c5bbf7188fa44`. The test file sha256 stayed `a9d46e2395d33b4ab630dbee46752a48e8fd67e740c0787dd8eec1f1ebe1ce79`.

Start `git status --porcelain=v1`:

```
 M Cargo.lock
 M crates/ocelli-render/Cargo.toml
 M crates/ocelli-render/src/graph.rs
 M crates/ocelli-render/src/lib.rs
 M crates/ocelli-seg/src/lib.rs
?? .claude/reviews/F-063-labelmap-pass-1.md
?? crates/ocelli-render/shaders/labelmap.wgsl
?? crates/ocelli-render/src/labelmap.rs
?? crates/ocelli-render/tests/labelmap.rs
?? crates/ocelli-seg/src/labelmap.rs
?? crates/ocelli-seg/tests/
```

End `git status --porcelain=v1` adds only this review file:

```
 M Cargo.lock
 M crates/ocelli-render/Cargo.toml
 M crates/ocelli-render/src/graph.rs
 M crates/ocelli-render/src/lib.rs
 M crates/ocelli-seg/src/lib.rs
?? .claude/reviews/F-063-labelmap-pass-1.md
?? .claude/reviews/F-063-labelmap-pass-2.md
?? crates/ocelli-render/shaders/labelmap.wgsl
?? crates/ocelli-render/src/labelmap.rs
?? crates/ocelli-render/tests/labelmap.rs
?? crates/ocelli-seg/src/labelmap.rs
?? crates/ocelli-seg/tests/
```

**Author / reviewer**: the implementation and the new test were already in the working tree. This reviewer did not write them and is not pass 1. The author is not named by the tree. This is not a human review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The approved F-063 plan says the outline is the 4-connected boundary, one pixel. A neighbour outside the grid counts as background. An outline pixel keeps the segment RGB and uses alpha 255. It replaces the fill at that pixel. PS3.3 C.8.20.2 makes pixel 0 background and not a segment number. It does not define the neighbourhood.

Pass 1 left this shape unexecuted. A solid 3 by 3 of segment 1 cannot separate a 4-connected boundary from an 8-connected one. Every edge pixel already has an outside 4-neighbour, and the centre has no differing diagonal.

`a_diagonal_neighbour_does_not_make_an_outline` is a 3 by 3 plus of segment 1. Row-major indices are `[0, 1, 0, 1, 1, 1, 0, 1, 0]`.

```
0 1 0
1 1 1
0 1 0
```

The centre is index 4. Its orthogonal neighbours are indices 1, 3, 5 and 7, all segment 1, so it stays fill `(10, 20, 30, 128)`. Its diagonal neighbours are indices 0, 2, 6 and 8, all segment 0. An 8-connected outline paints that centre at alpha 255. Each arm is outline `(10, 20, 30, 255)`. Each corner is background and stays `(0, 0, 0, 0)`.

The restored walk is still the four steps `(1, 0)`, `(-1, 0)`, `(0, 1)` and `(0, -1)`. Replacing those with the same four plus `(1, 1)`, `(1, -1)`, `(-1, 1)` and `(-1, -1)` made the new test fail at `crates/ocelli-seg/tests/labelmap.rs:174`. Left was `Some([10, 20, 30, 255])` and right was `Some([10, 20, 30, 128])`. Exit 101. The other six tests stayed green. The new test cannot pass for that 8-connected outline. The four-step list was restored and the suite was green again.

`OUTLINE_ALPHA` is still 255 at line 85 and the outline branch still writes it at line 156. The mutated centre byte was 255, not the fill alpha 128, so the outline write is that constant.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD` and `git status --porcelain=v1` | `sprint/s22` working tree, start and end | Executed now | HEAD `74814ba84870ce251dcc33d4eb40a9b286d577a8` both times. End status adds only this review file. |
| `bin/ocelli.sh test ocelli-seg --test labelmap` | restored four-step presenter, before the mutation | Executed now | Exit 0. 7 passed, 0 failed. |
| Eight-neighbour `neighbour_differs` steps | same presenter, diagonals added to the four orthogonal steps | Executed now, then restored | Exit 101. Only `a_diagonal_neighbour_does_not_make_an_outline` failed, at `tests/labelmap.rs:174`. Left `Some([10, 20, 30, 255])`. Right `Some([10, 20, 30, 128])`. 6 passed, 1 failed. |
| `bin/ocelli.sh test ocelli-seg --test labelmap` | restored presenter, sha256 `cdd1e5eaa3635fc506e635c291b14927ad34ce01759016bbf52c5bbf7188fa44` | Executed now | Exit 0. 7 passed, 0 failed. |
| Restored `labelmap.rs` text | same path | Executed now | Line 85 is `const OUTLINE_ALPHA: u8 = 255`. Line 156 writes that constant. Line 177 is the four orthogonal steps. No eight-step list and no diagonal pair remain. |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`neighbour_differs` walks only `(1, 0)`, `(-1, 0)`, `(0, 1)` and `(0, -1)`. A step outside the grid returns boundary. An in-grid neighbour whose segment differs also returns boundary. The outline pixel keeps the segment RGB and stores `OUTLINE_ALPHA`. The centre of the plus matches on all four orthogonal samples, so the restored function returns not-a-boundary and `paint` keeps the LUT alpha 128.

The centre has no out-of-range neighbour even in the eight-neighbour set. All four corners are inside the 3 by 3 and are segment 0. The four orthogonal steps match, so they do not return early. The first diagonal then differs. That is why the mutation fails on the centre alpha and not on `Empty`, a corner, or an arm. The six older tests stayed green under the same steps. The solid 3 by 3 still does not see this difference. The new test does.

The new test asks `present_labelmap` for fill and outline together and checks all nine pixels. Segment 0 stays transparent and does not call `LabelColourLut::colour`. The expected centre is the plan's fill colour because the pixel is not a 4-connected boundary. It is not a copy of an 8-connected result. The mutation shows the assert rejects that result.

Pass 1 cited presenter lines 85 and 156, and the four orthogonal steps. Those lines still say the same thing. This pass did not change `ocelli-render` and did not rerun the GPU suite. The render labelmap sources were not the change under review.

## Limitations and handoff

Tier B was not opened. `show_labelmap` after recovery was not run. A target larger than the label was not drawn. Pass 1 already recorded those limits. This test does not make any of them false. wasm-pack was not run. The sprint gate was not run. Clippy was not rerun. The restored presenter hash matches the file this pass started from, which is the text pass 1 already clipped.

The plus arms also touch the outside of the grid. Their outline alpha is therefore also explained by an outside neighbour. This pass did not delete the in-grid mismatch check. The centre assert is the 4-versus-8 discriminator, and the eight-step mutation made that assert fail while the other rows stayed green.

This is a bounded review of the new test against the restored presenter. It is not a sprint-profile verification and it is not a completion record.
