# F-X017 wasm view lint review, pass 1

**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1. A renamed typed-array constructor escaped the rule

The initial rule recognized a construction only when the callee identifier
was one of the twelve standard constructor names. This left the following
route green even though TypeScript still knew the constructed value was a
typed array:

```typescript
const RenamedView = Uint8Array;
new RenamedView(wasm.memory.buffer);
```

That contradicted the plan's claim that every typed-array construction over
wasm memory is refused. The repair now asks TypeScript for the callee's
construct signatures and checks the fully qualified symbol of each return
type. The executable alias probe includes this route and expects eight
refusals rather than seven.

## Smells

None.

## Nitpicks

None.

## Other checks

The review confirmed that computed `buffer` access and all eleven typed-array
classes plus `DataView` share the same type-aware receiver path. It also
confirmed that the two allowance lists disable both halves of the rule and
that the self-check refuses a missing enforcement block, weakened severity,
or surplus allowance.

The implementation write set was corrected to include the lint gate runner,
semantic test, budget and LLD index. The generated guard runbook did not move
and is not claimed as a changed path.
