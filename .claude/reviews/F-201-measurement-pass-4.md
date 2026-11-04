# F-201 measurement, pass 4

**Reviewed**: staged tree e32421873510dc69e2a39cf79c9ade469d610d98

**Result**: 0 defects, 0 smells, 3 nitpicks

The page calls `persistMeasurements`, `loadMeasurements`, `putAnnotation`, `annotationsForFrame`, `undoAnnotation`, `writeTid1500`, `readTid1500`, `keyImageMark`, `keyAction` and `createPointerDispatch`. The TID line is the number `readTid1500` returns. A spy that replaces that number with 3 makes the line 3 while the stored document stays 12.5 and the written number is 7. Echoing the written number leaves 7. Copying the loaded document leaves 12.5.

Pass 3 was 1 defect, 0 smells and 3 nitpicks. That defect was an echo of the written number that stayed green. It is closed. Three nitpicks remain. The page sentence about a mouse down describes the helper. The first event is read with a loop. The tracker names fewer mutations than the notes.

Reviewer Claude Code, read only, with no shell. This pass did not run the tests. The host ran eslint and the measurement tests. The echo mutation went red and was restored.
