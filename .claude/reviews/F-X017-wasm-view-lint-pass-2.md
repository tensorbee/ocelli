# F-X017 wasm view lint review, pass 2

**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Recheck of pass 1

The constructor-alias defect is closed. The rule resolves construct
signatures through TypeScript and recognizes the global return symbol, so a
renamed `Uint8Array` constructor is refused.

The semantic suite reports twelve constructor refusals and eight alias-route
refusals. Caller-owned `Uint8Array.buffer`, ordinary `ArrayBuffer`,
`bulk.ts`, and `panic.ts` remain green.

## Mutation evidence

Changing the accepted receiver symbol from `WebAssembly.Memory` to a name
that cannot resolve made both negative semantic groups fail, with zero
refusals observed instead of twelve and eight. Restoring the symbol returned
all four semantic groups to green.

Removing the node semantic-test command from the `lint` gate made
`scripts/guard_census.py` fail on the
`WASM_VIEW_TEST_REGISTRATION` strictness constant. Restoring the command
returned the census to green. The constructor set, full detection path,
self-check, remaining syntax rule and exact allowance list are independently
recorded constants.

## Verification

The following checks are green on the restored staged tree:

- `node --test scripts/tests/test_eslint_wasm_memory_view.mjs`
- `npm run lint`
- `npm run typecheck`
- `python3 scripts/guard_census.py --check-runbook`

The current implementation closes the measured wasm-memory view escapes with
no third production allowance and no false positive on ordinary buffers.
