# F-196 review, pass 1

**Reviewed**: uncommitted F-196 on `sprint/s43` before the odd-length, column-limit, template and referenced-UID fixes. HEAD was `2115db4`.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 2 defects, 2 smells, 1 nitpick

## Acceptance challenged

AC1 holds for the title Key image and the referenced CT SOP. AC3 held for the even 2 by 2 fixture and did not hold for an odd count. AC4 held for a short buffer. AC5 held in jsdom. The live page was not opened.

## Evidence

Claude read the plan, the writers, the tests and `docs/lld/key-object.md`. The implementer had already seen a template-constant mutation and a zero-pixel mutation go red. Those mutations did not isolate the reader.

## Findings

D1. `readSecondaryCapture` returned nothing for every odd pixel count. dcmjs pads an odd OB value with one zero byte under PS3.5 section 8.1.1, so a 3 by 3 image came back as 10 bytes and the length check failed.

D2. Columns or rows above 65535 were written as a wrapped unsigned short. 65536 columns were stored as 0.

S1. Deleting the template-identifier check left the TID 1500 test green, because that document also has a different SOP class. Nothing rewrote only the template.

S2. The referenced instance had to be a `2.25` UID. A key object flags an existing image, and that instance UID is not required to be synthetic. The plan limits `2.25` to the document study, series and instance.

N1. A title of one space was written and then read as nothing, because dcmjs drops trailing spaces.

## Remediation

The following pass reviews the repaired tree. Odd counts drop one trailing zero pad. Columns and rows above 65535 are refused. A rewritten template of 1500 is refused on its own. A non-`2.25` referenced instance round-trips. A title of only spaces is refused.
