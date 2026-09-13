# F-019 review, pass 2

**Reviewed**: staged worker tree after pass 1 remediation
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The pass 1 findings are resolved. Number of Frames now checks the preserved
IS byte length before removing permitted edge spaces, parses within the DICOM
signed 32-bit range, retains a distinct zero refusal, and accepts only positive
values. Regression fixtures cover overlong padded text and the first positive
value outside the IS range.

Both Basic and Extended Offset Table constructors now reject odd physical
Fragment Value lengths. Extended lengths still permit an odd encoded frame
length only when the even physical Fragment contains exactly one pad byte.
Fixtures cover both constructor refusals, exact pad removal, and excess-byte
refusal.

The full focused set passes 15 tests. The review also rechecked per-frame over
shared precedence, duplicate-source evidence, F-020 ownership, Item Tag offset
origin, final bounds, current Extended Offset Table one-Fragment rules, public
exports, LLD claims, and the absence of unsafe code. The earlier independent
Item-header mutation produced five frame-index failures and was reverted. The
remediated tree has no unstaged changes and `git diff --check` passes.
