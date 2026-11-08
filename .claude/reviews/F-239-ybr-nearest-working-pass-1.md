# F239 nearest inverse coefficients, independent repair review pass 1

**Reviewed**: exact tree06f98d497fcaee7f3755c4cfe1f5c8b9713ebb0f against
base7678f744063b674c2fffec3547adb162cfd5d638, only color.rs, its public
integration test file and the F239 plan amendment. Root authored the repair
and public tests. Codex f244_review authored no production edit or public
fixture code. **Result**: 0 defects, 0 smells, 0 nitpicks in this repair.
Whole-feature, cross-target, producer and browser acceptance remain separate.

## Independent arithmetic and roles

This reviewer independently transcribed the forward matrices from official
[PS3.3 2026d C.7.6.3](https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.7.6.3.html)
and independently opened the historical
[PS3.3 2017b section](https://dicom.nema.org/medical/dicom/2017b/output/chtml/part03/sect_C.7.6.3.html)
for PARTIAL422, retired in the current edition. The printed decimals and
offsets match the source. The historical partial equations also match the
current PARTIAL420 equations, without treating the retired name as current.

A separate Fraction Gauss-Jordan inversion checked the exact identity product.
A binary search over finite binary32 rational values selected nearest values
by exact distances and ties-to-even. Rust decimal literals were parsed as
exact rationals and independently rounded. Controls covered one, both tie
parities, a subnormal half-to-zero and a hand-computed2x2 inverse. No rounded
comment decimal or product output supplied gold.

All18 final frozen literals match this independent nearest-bit result. The
three corrected values are FULL red-Cb -870/23628469 to b81a6f2a, FULL
blue-Cr -3180/23628469 to b90d1ef6 and PARTIAL red-Cb
-310000/3261922071 to b8c74e0f. Remaining15 values are unchanged. The
source equations, offsets, arithmetic order, display scaling and byte policy
are unchanged. Comments now correctly distinguish nine-place approximations
from the exact fractions and direct binary32 rounding.

This reviewer supplied the independent narrow-window counterexample. Root
independently derived the matrix values earlier and authored all public
regressions and implementation. That mathematical contribution is disclosed,
not presented as authorship-independent approval of its own calculation.
The source repair was additionally checked against the entire independently
calculated18-entry table. A separate standalone GLM review remains isolated
on original e4a input and was not sent these results or the root repair.
Its unfinished output is not acceptance evidence in this report.

## Consumers and public regressions

The three public basis cases isolate one shifted component each. FULL inputs
[0,129,128] and [0,128,129] select the two corrected terms directly. The
PARTIAL422 packed pair [16,16,129,128] correctly uses both luminances and
shared chroma with the partial offsets. Its first output isolates red-Cb.
Expected bits are literal independently calculated gold, not product readback.
Checked fixture access and the existing require_ok helper preserve failures.

The fourth public test uses FULL [0,128,0], centre bits3f0468f7 and width1.
Independent strict binary32 evaluation gives old blue9248473/536870912 and
corrected4624251/268435456. The exact accepted centre-minus-one-half lies
between them. The unchanged width-one LINEAR rule therefore returns display
channel0 on old code and255 on corrected code. This establishes a real
nonidentity-window consequence, while the broader identity-window sweep's
zero-byte-change result does not prove absence under other windows. The
public test exercises ColorTransform and ColourWindow. No browser pixel
observation is claimed by this review.

## Actual checks and retained limits

Executed by this reviewer: only source/blob reads and small exact arithmetic,
direct exit0. All18 corrected literal bits checked against independent gold.
Production color.rs SHA256:
66e1ac40d4285910498b3d9653e4b363957f3dc2678209d9e5c59556bf552a25.
No test suite, compiler, browser, GPU, benchmark or oracle was run by this
reviewer. No source remediation was written.

Author receipts independently inspected: both old-code YBR selections exit101,
full pixel suite exit0 in9.583597 seconds, two initial test-lint attempts
exit101, final clippy exit0 in0.238449 seconds, and final focused seven YBR
tests exit0 in0.568331 seconds. The full pixel run precedes mechanical test
lint fixes. Those failures and source stages remain explicit, not erased by
the later green result. No tolerance, budget, API or guard was weakened.

Arithmetic and witness artifacts are in the existing private
f239-inverse-controller directory. The first witness helper attempt had a
Python syntax error before evaluation, followed by a corrected exact check.
This is a reviewer harness error, not a product failure. Root owns final
staged guards, cross-target/producer observations and integration evidence.
