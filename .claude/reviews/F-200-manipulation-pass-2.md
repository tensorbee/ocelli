# F-200 manipulation, pass 2

**Reviewed**: staged tree 274dd369fad071c02a52eeacd120529816084679

**Result**: 0 defects, 0 smells, 2 nitpicks

The page calls `windowDrag`, `windowRegion`, `rotationDrag`, `orientationQuadrant`, `orientationMarker`, `magnifyCircle`, `scaleLength`, `cropPlanes`, `packCommands` and `describeError`. The panel test changes the colour map to grey. Packed lengths on the page come from that report. The warning line is `describeError` of the caller code. One to one, histogram and four corners read unavailable.

Pass 1 was 0 defects, 2 smells and 2 nitpicks. The first smell was a panel test that left the colour map at its default. The second was an unrecorded rename from `s44-panels.test.tsx`. Both are closed. Two nitpicks remain. The design still has both sentences about a refused window and replacing the whole report. The tracker row names the flip mutation and not the colour map mutation.

Reviewer Claude Code, read only, with no shell. This pass did not run the tests. The host ran eslint, tsc and the manipulation tests. Ignoring the colour map select went red and was restored. A vertical flip flag change went red and was restored.
