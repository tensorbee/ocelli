# Migration

**F-IDs that contributed:** F-108, F-106, F-107, F-109
**Last updated:** 2026-11-03

The library enables on a plain DOM element. `enableElement` in
`packages/core` registers one element and dispatches Ocelli event names on
it. The pump still drains the ring once per frame. Decision D1 keeps this
a new API. `cornerstone.enable` is not rewritten.

Stack viewports move first, then MPR, then 3D. `setKindEnabled` keeps
that order. `shadowDivergence` is the count the caller reads before a
flag flips.

`attachSideBySide` stores one Ocelli element and one neighbour. The same
object cannot be both. Release disables only the Ocelli element. The
neighbour keeps listeners this package did not register. The other library
is not imported.

`setKindEnabled` stores three kind flags. They start off. Stack turns on
before MPR, and MPR turns on before volume. `enableFlagged` enables an
element only while that kind is on. Turning the kind off disables the
elements this module enabled. A mount that turns the kind off does not
leave the element dispatching. The other library is not imported.

`shadowDivergence` counts samples whose absolute difference is greater
than an integer limit from 0 to 255. Unequal lengths return nothing. A
limit outside that range returns nothing. The buffers are not written.
The function does not render and does not import the other library.

`rewriteListenerNames` rewrites a string literal that is the first argument
of `addEventListener` or `removeEventListener` when `CORNERSTONE_EVENT_MAP`
gives that cornerstone name an Ocelli name. `ELEMENT_ENABLED` becomes
`viewport.mounted`. `CAMERA_MODIFIED` stays, because that row does not name
an Ocelli event. A template literal stays. The function does not write a
file and it does not copy the 103-row map.
