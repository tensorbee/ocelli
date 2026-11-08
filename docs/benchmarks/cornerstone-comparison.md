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
| session/cold | 273.4233 | 271.5259 to 277.2151 | 136.6970 | 135.6563 to 138.7402 | 2.00 | 9 |
| session/warm | unavailable | unavailable | unavailable | unavailable | - | 0 |

- session/warm: a warm page load measures the browser cache, not either library
- session/warm: a warm page load measures the browser cache, not either library

### open, milliseconds

| workload | Ocelli median | Ocelli p25 to p75 | cornerstone3D median | cornerstone3D p25 to p75 | ratio | trials |
| --- | --- | --- | --- | --- | --- | --- |
| synthetic-ct/cold | 999.9000 | 991.6000 to 1008.8000 | 730.5000 | 719.3000 to 732.9000 | 1.37 | 9 |
| synthetic-ct/warm | 1015.3000 | 1014.7000 to 1018.0000 | 718.2000 | 712.2000 to 719.4000 | 1.41 | 9 |
| pseudo-full/cold | 146.3000 | 145.7000 to 148.9000 | 107.8000 | 105.6000 to 109.8000 | 1.36 | 9 |
| pseudo-full/warm | 147.6000 | 145.7000 to 148.4000 | 110.9000 | 110.0000 to 111.4000 | 1.33 | 9 |

### first image, milliseconds

| workload | Ocelli median | Ocelli p25 to p75 | cornerstone3D median | cornerstone3D p25 to p75 | ratio | trials |
| --- | --- | --- | --- | --- | --- | --- |
| synthetic-ct/cold | 8.6000 | 7.2000 to 9.2000 | 24.5000 | 24.1000 to 25.2000 | 0.351 | 9 |
| synthetic-ct/warm | 8.8000 | 7.4000 to 10.1000 | 24.0000 | 23.7000 to 24.5000 | 0.367 | 9 |
| synthetic-dx/cold | 32.7000 | 31.9000 to 33.7000 | 53.4000 | 52.0000 to 54.8000 | 0.612 | 9 |
| synthetic-dx/warm | 29.0000 | 28.8000 to 29.2000 | 52.2000 | 51.4000 to 52.8000 | 0.556 | 9 |
| pseudo-ct/cold | 10.3000 | 9.2000 to 11.1000 | 24.6000 | 24.3000 to 25.9000 | 0.419 | 9 |
| pseudo-ct/warm | 9.0000 | 8.7000 to 10.2000 | 25.6000 | 23.9000 to 27.4000 | 0.352 | 9 |
| pseudo-dx/cold | 33.1000 | 32.4000 to 33.9000 | 52.6000 | 52.4000 to 53.3000 | 0.629 | 9 |
| pseudo-dx/warm | 28.4000 | 27.9000 to 29.4000 | 51.6000 | 51.1000 to 52.1000 | 0.550 | 9 |

### scroll, presented frames per second

| workload | Ocelli median | Ocelli p25 to p75 | cornerstone3D median | cornerstone3D p25 to p75 | ratio | trials |
| --- | --- | --- | --- | --- | --- | --- |
| synthetic-ct/cold | unavailable | unavailable | unavailable | unavailable | - | 0 |
| synthetic-ct/warm | 96.16 | 96.14 to 96.74 | 48.37 | 48.36 to 48.39 | 1.99 | 5 |
| pseudo-ct/cold | unavailable | unavailable | unavailable | unavailable | - | 0 |
| pseudo-ct/warm | 96.73 | 96.69 to 96.75 | 48.38 | 48.37 to 48.38 | 2.00 | 5 |

- synthetic-ct/cold: scroll is measured after the first image
- synthetic-ct/cold: scroll is measured after the first image
- pseudo-ct/cold: scroll is measured after the first image
- pseudo-ct/cold: scroll is measured after the first image

### window change, milliseconds

| workload | Ocelli median | Ocelli p25 to p75 | cornerstone3D median | cornerstone3D p25 to p75 | ratio | trials |
| --- | --- | --- | --- | --- | --- | --- |
| synthetic-ct/cold | unavailable | unavailable | unavailable | unavailable | - | 0 |
| synthetic-ct/warm | 9.3000 | 7.7000 to 9.6000 | 13.9000 | 13.7000 to 15.6000 | 0.669 | 9 |
| synthetic-dx/cold | unavailable | unavailable | unavailable | unavailable | - | 0 |
| synthetic-dx/warm | 18.1000 | 17.4000 to 20.1000 | 14.1000 | 12.6000 to 14.9000 | 1.28 | 9 |
| pseudo-ct/cold | unavailable | unavailable | unavailable | unavailable | - | 0 |
| pseudo-ct/warm | 7.6000 | 7.6000 to 7.8000 | 16.1000 | 16.0000 to 16.2000 | 0.472 | 9 |
| pseudo-dx/cold | unavailable | unavailable | unavailable | unavailable | - | 0 |
| pseudo-dx/warm | 19.4000 | 18.4000 to 20.1000 | 13.1000 | 12.5000 to 13.6000 | 1.48 | 9 |

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
| session/cold, sampled concurrent footprint lower bound | 1044.7 | 1041.7 to 1048.8 | 1275.5 | 1266.2 to 1277.5 | 0.819 | 3 |
| session/cold, sum of per-process peaks upper bound | 1082.1 | 1080.0 to 1085.5 | 1369.9 | 1368.7 to 1375.0 | 0.790 | 3 |
| session/warm, sampled concurrent footprint lower bound | unavailable | unavailable | unavailable | unavailable | - | 0 |

- session/warm: this condition is not applicable
- session/warm: this condition is not applicable

### session CPU, seconds

| workload | Ocelli median | Ocelli p25 to p75 | cornerstone3D median | cornerstone3D p25 to p75 | ratio | trials |
| --- | --- | --- | --- | --- | --- | --- |
| session/cold | 3.38 | 3.37 to 3.38 | 1.94 | 1.92 to 1.97 | 1.74 | 3 |
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
