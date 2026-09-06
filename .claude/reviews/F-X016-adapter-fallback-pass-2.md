# F-X016 review, pass 2

**Reviewed**: remediated working tree against `1e18647`
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Remediation verified

- The private `attempt_candidates` function now owns the same ordering and
  continuation loop used by the concrete wgpu path.
- Its two current instantiations are the concrete wgpu device request and the
  deterministic tests. No mock trait or forwarding wrapper was introduced.
- The first-failure test observes attempts `[0, 1]`, the second adapter opening,
  and the first adapter's diagnostic evidence surviving.
- The exhausted test observes attempts `[0, 1]` and both failures in order.
- An independent early-`break` mutation after the first failed attempt made
  both focused tests fail at exit 101 with observed attempts `[0]`. After the
  mutation was reverted, both focused tests passed at exit 0.

## Full feature verified clean

- Candidate order is every A candidate before every B candidate, then device
  type, then enumeration order for exact ties. `Backend::Noop` is excluded.
- Classification and override clamping use only the adapter whose device
  opened. A failed A candidate cannot construct tier A after a B fallback.
- All failed adapter identities and diagnostic reasons survive into evidence.
- Probing stops at the first successful device and applies the existing
  benchmark and software-renderer evidence to that adapter.
- Allocation remains confined to startup probing. No render-loop allocation,
  unsafe code, wasm boundary change, pixel transfer, LUT duplication, or
  tolerance change was added.
- Public native fixtures and the totality property match the new explicit
  probe outcome model.
