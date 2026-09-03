# Spike gates, project-added

`docs/hld/A-spike-gates.md` carries A1 to A6 and is cut from the authored
document, so it cannot be hand-edited. Gates decided after the HLD was written
live here, in the same form and with the same authority: **each can end or
reshape the work it gates.**

`/spike` runs a gate from either file. Answers land in `docs/spikes/`.

---

## A7, is tier C worth building, and where does its value actually come from?

**Created by**: deviation D-07, `docs/hld/DEVIATIONS.md`.
**Gates**: F-X003 and F-X004. Answer it before either is designed.
**Status**: **RESOLVED, outcome `Pass`.** See `docs/spikes/A7-tier-c.md`.
Every sub-question is decided. The two remaining figures, whether lavapipe
resolves under the pinned wgpu and what the incumbent viewer costs, are
measurements and are acceptance criteria on F-X002 and F-X003.

> **A7.1 is answered and it went the other way.** Deployments are assumed to
> span GPU-capable clients and GPU-less ones. The GPU-less class, which
> includes virtualised desktops without passthrough and builds where
> acceleration is disabled by policy, is where GPU access is least reliable,
> so the no-GPU population is not a rounding error and tier C is not a
> defensive fallback.
> The paragraph below about WebGL2 breadth is left in place because it was
> the reasoning that framed the gate, and seeing why it was wrong is worth
> more than deleting it. It was reasoning about consumer browsers, and it
> does not transfer to a virtual desktop.

### Why this is a gate and not an assumption

Tier C was added because HLD §7 leaves a machine with neither WebGPU nor
WebGL2 rendering nothing at all. That is a real hole. But the honest question
is how big it is, and **the obvious justification is probably the weakest one**:

WebGL2 support is very broad. A browser that has neither WebGPU nor WebGL2 is
rare, and getting rarer. If tier C is justified only by that population, it may
not be worth four engineer-weeks.

There are two other justifications and they may be stronger. This gate exists
to find out which, before the design commits to a shape that serves the wrong
one.

### The three questions

**A7.1, how large is the no-GPU browser population, really?**
Measure rather than assume. Check WebGL2 and WebGPU availability against the
browser and OS matrix F-114 (E19.1) will certify, including locked-down
enterprise and virtual-desktop environments, which is where a radiology
deployment actually lives and where GPU access is most often absent or
blacklisted. A managed clinical estate is the plausible case, not a
consumer browser.

**A7.2, does a usable software adapter exist for the render tests?**
`wgpu` with `force_fallback_adapter: true` against lavapipe or equivalent, on a
CI runner. If it works, F-X002 makes pipeline construction, bind-group layouts,
shader compilation and the tier-B variants all testable with no GPU, which
widens what CI can cover under the accepted D-04 arrangement.

**Note the trap:** a software adapter is a *test* mechanism, not a *reference*.
Its output is not an oracle. The reference is cornerstone3D through the
harness, per §11. A software adapter proves the pipeline is constructible and
the shader compiles. It does not prove a pixel is right.

**A7.3, is CPU stack rendering fast enough to be worth shipping?**
**Revised by the A7.1 answer.** Measure on a representative GPU-less session
and report three numbers, not one:

1. Window and level drag latency, and scroll latency, on a 512x512 16-bit
   series.
2. **CPU cost per session**, as a fraction of one vCPU, while doing it. On a
   shared host the economics are sessions per host, so a renderer that burns a
   core reduces density for everyone on it. This is the number a platform team
   asks for and the one that decides whether the application is allowed at
   all.
3. Frame change rate under a cine loop, because on a remoted session every
   changed frame has to be encoded and shipped.

State the numbers. "Usable" is not a measurement.

### What a pass and a fail look like

Written before the work, per `/spike` step 1.

| Outcome | Meaning |
|---------|---------|
| **Pass** | A7.1 passes on its own, and A7.3 shows interactive stack rendering is achievable inside a defensible CPU budget. Build F-X001 to F-X004 as planned |
| **Constrained** | A7.1 passes but A7.3 shows CPU stack rendering costs too much per session for the estate. Tier C still ships, but the default becomes reduced: lower interactive resolution during a drag, full resolution on settle. **A degraded mode is still tier C, and it is still declared** |
| **Fail** | CPU stack rendering cannot be made viable at any quality. Drop F-X003 and F-X004, keep F-X001 and F-X002 so a no-GPU session at least reports UNAVAILABLE honestly instead of showing a blank canvas |

**A7.1 is now answered as a pass, so `Fail` no longer means "revert D-07".**
Even in the worst case the tier must exist, because the alternative is a
clinical user on a GPU-less session seeing a viewport that silently does
nothing.
Reporting unavailable is a feature. A blank canvas is a defect.

### A7.1b, answered

Whether to assume **GPU passthrough** on the GPU-less class. Answered: assume
not. With passthrough most such sessions would resolve to tier A or B and tier
C would serve only the policy-disabled tail. Without it, tier C is a primary
clinical rendering path. Planning for the harder case costs little and the
reverse does not.
