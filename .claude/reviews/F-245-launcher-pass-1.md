# F-245 launcher review, pass 1

**Reviewed**: bin/folder-viewer-dev.mjs SHA-256 `bb2b799593dac9ec6e88a2fc374b40610a8753c1b04c4da512774733a6865cd1`, its current synthetic test and completed GLM own review at launcher-own-microscope-low-v1/content-parsed.json.
**Author / reviewer**: GLM launcher and test, independent Codex root review.
**Result**: 2 defects, 0 smells, 1 nonblocking nitpick. Partial launcher slice only, not full F-245 review or approval.

## D1, direct-child signaling does not establish build-tree cleanup

The current launcher records only current and calls current.kill. The actual npm phase can create a script child, and Cargo can create compiler children. The current tests exercise only the final backend and can kill an enclosing fixture group during teardown, masking this missing product behavior. The approved launcher must clean its owned subprocesses, including an interrupted build, rather than merely observe the direct parent disappear. The passing backend INT/TERM and wrong-signal mutant controls remain useful, but do not establish build-phase cleanup.

Root checked the actual script and [Node 24.13.0 child-process contract](https://nodejs.org/download/release/v24.13.0/docs/api/child_process.html). On POSIX, detached spawn creates an owned process group, and descendants are not guaranteed to stop merely because the immediate parent is killed. Preserve direct-child spawning of the selected backend while bounding cleanup to the groups the launcher itself creates. Never enumerate or signal unrelated processes by name or port.

## D2, ignored stop signals can retain the launcher indefinitely

After forwarding INT or TERM the current awaitExit has no stop deadline or escalation. The selected child can keep running after handling the signal, so the owned cleanup contract is not established. Add an explicit finite shutdown policy, with a real ignored-signal fixture and a build-parent-exits/child-remains fixture. The test's external watchdog must fail on timeout rather than count timeout as successful cleanup.

## Lead disposition for GLM remediation

For current POSIX targets, create and retain an owned group per build/backend phase. Forward the first requested INT or TERM to that group, allow 2000 ms grace, then SIGKILL only that still-owned group if needed, and allow at most 2000 ms for cleanup confirmation. Subsequent stop requests must not restart or extend the deadline. Preserve the original request's launcher exit 130 or 143. Stop before starting another phase once shutdown is requested. Confirm the group is gone, including after the direct child closes, and cancel timers/listeners once settled. No generic gate signal-manager change is authorized.

Do not apply negative-PID signaling on Windows. Inspect and preserve the existing platform contract and disclose any unverified platform behavior rather than claiming POSIX proof there. A platform-specific cleanup candidate must use only the launcher-created process identity. This disposition does not waive an existing host acceptance row or authorize a general process killer.

GLM must add meaningful controls for interrupted npm and cargo phases, a descendant that ignores the first signal, normal forwarding, and cleanup after an early failure. Keep custom-target artifact selection controls and exercise core/cargo nonzero exits plus missing, malformed and duplicate binary artifacts. Update test teardown to track exact newly owned groups as well, because detached product groups will no longer be inside the fixture launcher's group. Preserve unrelated-process sentinels. A fresh actual launcher/npm/browser run remains owed by the full story.

## Own findings adjudicated

The raw own report's missing ENOENT detail is not itself a success-path defect: current error handling returns nonzero. Fixed stage-specific diagnostics are sufficient, with no raw error-object requirement. The exit-code-to-signal mapping currently has only its two named inputs and is covered by the actual wrong-signal control, so its alternative representation is a preference. The module-state timing concern does not demonstrate a current JavaScript interleaving failure. Cargo output size and optional error-code distinctions do not require new policy here.

The one nit is optional clearer diagnostics for an unexpected child termination. No extra error taxonomy is requested. Root has not yet reproduced the two new cleanup controls, so the source-confirmed gaps and author-attributed successful controls are distinct. Have GLM implement and run its own microscope, then send this slice for fresh independent review. Other F-245 slices remain unapproved.
