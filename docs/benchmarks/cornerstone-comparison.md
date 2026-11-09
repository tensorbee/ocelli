<!-- Written by bin/ocelli.sh bench --compare-cornerstone --publish. Do not edit. -->

# Comparative cost of the local folder viewer and cornerstone3D 5.8.2

The two sides do different work in different places. The Ocelli folder viewer parses and decodes each file in a native process on this machine and sends 8-bit grey frames over loopback to the page, which draws them on a 2D canvas on the CPU. The cornerstone3D page parses and decodes in the browser, in dicom-image-loader's WebAssembly workers, and renders through WebGL. Memory and CPU are therefore counted over every process each side runs: for Ocelli the native process and every Chromium process, and for cornerstone3D every Chromium process, including the GPU process. This measures the local folder viewer example, not the Ocelli browser core, which has no viewer page to measure yet.

cornerstone3D 5.8.2 is the reference this project learns from and validates against. This page records cost on one machine, for one set of inputs, at each side's shipped defaults. It is a measurement, not a ranking.

The tier: Ocelli tier C on the CPU, cornerstone3D on ANGLE (Apple, ANGLE Metal Renderer: Apple M4 Max, Unspecified Version). The browser ran headless.

## The machine

- CPU: Apple M4 Max, 16 cores
- memory: 51539607552 bytes (49152.0 MiB)
- operating system: darwin 27.0.0 (arm64)

## Versions

| tool or package | version |
| --- | --- |
| chromium | 151.0.7922.34 |
| playwright | 1.62.1 |
| @cornerstonejs/core | 5.8.2 |
| @cornerstonejs/dicom-image-loader | 5.8.2 |

## The defaults each side ran at

| side | default | value |
| --- | --- | --- |
| Ocelli | server cache bytes | 268435456 |
| Ocelli | page grey cache bytes | 67108864 |
| Ocelli | decode permits | 2 |
| cornerstone3D | max cache size bytes | 3221225472 |
| cornerstone3D | decode workers | 8 |

## The workloads

| workload | state | inputs |
| --- | --- | --- |
| synthetic CT | selected | 1693 |
| synthetic DX | selected | 1 |
| pseudo full | selected | 201 |
| pseudo CT | selected | 200 |
| pseudo DX | selected | 1 |

## The measurements

### page ready, milliseconds

| workload | Ocelli median | Ocelli p25 to p75 | cornerstone3D median | cornerstone3D p25 to p75 | ratio | trials |
| --- | --- | --- | --- | --- | --- | --- |
| session/cold | 270.9998 | 269.6875 to 271.4170 | 137.2510 | 135.5940 to 141.8579 | 1.97 | 9 |
| session/warm | unavailable | unavailable | unavailable | unavailable | - | 0 |

- session/warm: a warm page load measures the browser cache, not either library
- session/warm: a warm page load measures the browser cache, not either library

### open, milliseconds

| workload | Ocelli median | Ocelli p25 to p75 | cornerstone3D median | cornerstone3D p25 to p75 | ratio | trials |
| --- | --- | --- | --- | --- | --- | --- |
| synthetic-ct/cold | 998.8000 | 995.2000 to 1004.6000 | 716.8000 | 711.9000 to 718.1000 | 1.39 | 9 |
| synthetic-ct/warm | 1010.4000 | 1006.6000 to 1016.6000 | 718.7000 | 710.1000 to 720.6000 | 1.41 | 9 |
| pseudo-full/cold | 147.0000 | 144.8000 to 148.7000 | 106.7000 | 105.6000 to 107.8000 | 1.38 | 9 |
| pseudo-full/warm | 147.9000 | 147.5000 to 148.3000 | 110.0000 | 109.2000 to 111.0000 | 1.34 | 9 |

### first image, milliseconds

| workload | Ocelli median | Ocelli p25 to p75 | cornerstone3D median | cornerstone3D p25 to p75 | ratio | trials |
| --- | --- | --- | --- | --- | --- | --- |
| synthetic-ct/cold | 8.8000 | 8.5000 to 9.5000 | 25.3000 | 23.8000 to 26.2000 | 0.348 | 9 |
| synthetic-ct/warm | 8.8000 | 8.7000 to 10.9000 | 24.4000 | 24.1000 to 25.0000 | 0.361 | 9 |
| synthetic-dx/cold | 33.2000 | 32.9000 to 33.8000 | 52.9000 | 52.6000 to 53.6000 | 0.628 | 9 |
| synthetic-dx/warm | 30.0000 | 29.2000 to 32.3000 | 52.6000 | 52.0000 to 54.2000 | 0.570 | 9 |
| pseudo-ct/cold | 9.2000 | 9.1000 to 11.3000 | 25.1000 | 24.6000 to 25.6000 | 0.367 | 9 |
| pseudo-ct/warm | 9.3000 | 8.8000 to 10.5000 | 24.8000 | 24.3000 to 26.0000 | 0.375 | 9 |
| pseudo-dx/cold | 32.9000 | 32.2000 to 33.8000 | 52.9000 | 52.2000 to 54.2000 | 0.622 | 9 |
| pseudo-dx/warm | 29.1000 | 28.4000 to 29.8000 | 51.9000 | 51.8000 to 52.1000 | 0.561 | 9 |

### scroll, presented frames per second

| workload | Ocelli median | Ocelli p25 to p75 | cornerstone3D median | cornerstone3D p25 to p75 | ratio | trials |
| --- | --- | --- | --- | --- | --- | --- |
| synthetic-ct/cold | unavailable | unavailable | unavailable | unavailable | - | 0 |
| synthetic-ct/warm | 96.64 | 96.13 to 96.76 | 48.37 | 48.37 to 48.38 | 2.00 | 5 |
| pseudo-ct/cold | unavailable | unavailable | unavailable | unavailable | - | 0 |
| pseudo-ct/warm | 96.65 | 96.64 to 96.76 | 48.38 | 48.38 to 48.38 | 2.00 | 5 |

- synthetic-ct/cold: scroll is measured after the first image
- synthetic-ct/cold: scroll is measured after the first image
- pseudo-ct/cold: scroll is measured after the first image
- pseudo-ct/cold: scroll is measured after the first image

### window change, milliseconds

| workload | Ocelli median | Ocelli p25 to p75 | cornerstone3D median | cornerstone3D p25 to p75 | ratio | trials |
| --- | --- | --- | --- | --- | --- | --- |
| synthetic-ct/cold | unavailable | unavailable | unavailable | unavailable | - | 0 |
| synthetic-ct/warm | 7.7000 | 7.6000 to 8.4000 | 14.6000 | 11.0000 to 16.1000 | 0.527 | 9 |
| synthetic-dx/cold | unavailable | unavailable | unavailable | unavailable | - | 0 |
| synthetic-dx/warm | 18.1000 | 17.1000 to 19.1000 | 13.2000 | 10.9000 to 14.2000 | 1.37 | 9 |
| pseudo-ct/cold | unavailable | unavailable | unavailable | unavailable | - | 0 |
| pseudo-ct/warm | 7.6000 | 7.6000 to 7.7000 | 16.1000 | 16.1000 to 16.1000 | 0.472 | 9 |
| pseudo-dx/cold | unavailable | unavailable | unavailable | unavailable | - | 0 |
| pseudo-dx/warm | 18.7000 | 18.1000 to 19.0000 | 13.6000 | 13.0000 to 14.0000 | 1.38 | 9 |

- synthetic-ct/cold: a window change needs a drawn image
- synthetic-ct/cold: a window change needs a drawn image
- synthetic-dx/cold: a window change needs a drawn image
- synthetic-dx/cold: a window change needs a drawn image
- pseudo-ct/cold: a window change needs a drawn image
- pseudo-ct/cold: a window change needs a drawn image
- pseudo-dx/cold: a window change needs a drawn image
- pseudo-dx/cold: a window change needs a drawn image

### peak memory, MiB, from the exact bytes the JSON records

Memory sampling pauses at least 250 ms between completed collection rounds. Actual sample timestamps remain in raw evidence.

| workload | Ocelli median | Ocelli p25 to p75 | cornerstone3D median | cornerstone3D p25 to p75 | ratio | trials |
| --- | --- | --- | --- | --- | --- | --- |
| session/cold, sampled concurrent footprint lower bound | 1046.8 | 1044.5 to 1048.2 | 1280.1 | 1279.2 to 1283.2 | 0.818 | 3 |
| session/cold, sum of per-process peaks upper bound | 1084.8 | 1084.7 to 1085.7 | 1337.6 | 1336.5 to 1344.9 | 0.811 | 3 |
| session/warm, sampled concurrent footprint lower bound | unavailable | unavailable | unavailable | unavailable | - | 0 |

- session/warm: this condition is not applicable
- session/warm: this condition is not applicable

### session CPU, seconds

| workload | Ocelli median | Ocelli p25 to p75 | cornerstone3D median | cornerstone3D p25 to p75 | ratio | trials |
| --- | --- | --- | --- | --- | --- | --- |
| session/cold | 3.35 | 3.33 to 3.35 | 1.90 | 1.90 to 1.91 | 1.76 | 3 |
| session/warm | unavailable | unavailable | unavailable | unavailable | - | 0 |

- session/warm: this condition is not applicable
- session/warm: this condition is not applicable

## The pixel agreement check

Every timing figure above was taken in a run whose pixel check passed. One frame of each side's pseudo CT workload, at centre 40, width 400, LINEAR, compared through HLD 25.1's monochrome rule and bias bound.

- within one LSB: 1.0000 of pixels
- pixels over the maximum: 0
- maximum absolute difference: 1
- signed mean bias over the informative region: -0.0586
- informative fraction: 0.4668
- low information: no
- the verdict passed: yes

## What the numbers do not show

- one machine and one browser build
- loopback only and no network
- a warm operating system file cache
- different default budgets on the two sides
- a comparator page written for this measurement rather than an application such as OHIF
- the matched prefetch policy
- a comparator-only adapter wraps valid VOI strings for loader5.8.2, with pinned package bytes and cache and worker defaults unchanged
- uncompressed inputs only so no codec cost
- a 2D canvas against WebGL
- the tier: Ocelli tier C on the CPU, cornerstone3D on the named WebGL renderer

The input identity is d87461fd59a9dc875b6b5ffe96f20ff7640059f8cd69c04c89adc97a7ff5aa58, the SHA-256 of the sorted list of per-file content digests.
