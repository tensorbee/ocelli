# Release policy

**F-IDs that contributed:** F-117
**Last updated:** 2026-11-03

`/release` is the only command that publishes. This page does not publish
and it does not create a `v` tag. The workspace version stays 0.1.0.

A stable claim is false while D-22 is open. It is also false unless the
certified row count equals the matrix row count and that count is an
integer of at least 1. A certified count short of the row count is not
enough. The current matrix has no certified row, and D-22 is open, so a
caller who passes those facts gets false.

`0.x` carries no stability promise. The removal window is null. A patch
does not schedule a removal, so its window is null. A name deprecated in
a published minor stays for one further minor. A major may remove a
name, so the window is 0. Any other channel is null.

Tiers A, B and C are not applicable. The functions do not render.
