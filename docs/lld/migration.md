# Migration

**F-IDs that contributed:** F-108, F-106
**Last updated:** 2026-11-03

The library enables on a plain DOM element. `enableElement` in
`packages/core` registers one element and dispatches Ocelli event names on
it. The pump still drains the ring once per frame. Decision D1 keeps this
a new API. `cornerstone.enable` is not rewritten.

Stack viewports move first, then MPR, then 3D. A per-viewport flag and
shadow mode are later stories. This file does not implement them.

`attachSideBySide` stores one Ocelli element and one neighbour. The same
object cannot be both. Release disables only the Ocelli element. The
neighbour keeps listeners this package did not register. The other library
is not imported.

`rewriteListenerNames` rewrites a string literal that is the first argument
of `addEventListener` or `removeEventListener` when `CORNERSTONE_EVENT_MAP`
gives that cornerstone name an Ocelli name. `ELEMENT_ENABLED` becomes
`viewport.mounted`. `CAMERA_MODIFIED` stays, because that row does not name
an Ocelli event. A template literal stays. The function does not write a
file and it does not copy the 103-row map.
