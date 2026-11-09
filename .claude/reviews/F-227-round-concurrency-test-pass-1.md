# F-227 series-round concurrency control, bounded independent review

Reviewed 2026-11-09 by root Codex. Result: 0 defects, 0 smells, 0 nitpicks in final patch `85a3db3b0e7a1e026a925cca30cfbcbb6a74354386b989ef56403ab3c776b07c`, based on canonical 9a4456b6. The f225 Codex participant authored the adopted repair independently of the separately preserved GLM implementation. Root authored neither. This is a functional concurrency-test repair, not new timing acceptance or completion of F-227.

## Failure and scope

Qualification CI run 42590597505 failed the original test with an empty HTTP reply. Its helper has an unchanged five-second socket bound. Empty data alone does not distinguish EOF from timeout. The unchanged local source passed its selected test in 2.82 seconds, so no local reproduction of the Ubuntu failure is claimed. The old large sixteen-frame 512-square inputs used work duration to encourage overlap.

The repair uses sixteen small 16-square frames per request and test-only coordination around actual admission. Four real HTTP requests still must return 200 and the maximum admitted round count still must equal the literal two. Production permit, decode, window, budget and HTTP timeout code is unchanged. All new occupancy instrumentation is cfg(test), isolated by the actual unique viewer generation.

## Consumer and cleanup challenge

An admitted RoundCount increments active occupancy while holding its real permit, then parks on a test-owned condition. A WaitingRound is counted only from the actual full-permit admission loop. The test waits until all four attempts are accounted for, then unconditionally drops the held-round guard before joining client threads. Releasing as soon as two entries appear could hide an oversized limit. Counting both admitted and actually waiting requests rejects that false pass. The exact expected occupancy is two active and two waiting.

Root inspected lock ordering and guard lifetimes. The admission witness takes the test-state lock while holding the admission lock. No inspected path nests those locks in the opposite order. Condition waits release their associated mutex. Waiting guards drop as admission returns before the caller increments its running-round count. The held-round guard releases on ordinary return, setup timeout and unwinding. There is no join-before-release deadlock. The two-second rendezvous is a functional setup bound inside the unchanged five-second transport bound, not a widened performance tolerance.

## Direct verification

The private f227-round-concurrency-controller evidence directory preserves immutable old and final sources, binaries and command receipts. First clippy exited 101 on collapsible_if. The equivalent let-chain correction is the only difference between root's original source review and the final patch. Final formatter exited 0. Final targeted clippy exited 0 in 1.733011 seconds. All 116 folder-viewer library tests passed with two test threads, exit 0 in 22.237750 seconds. This checks generation isolation under concurrent tests without claiming corpus or GPU acceptance.

On the final source, cap-one, cap-three and cap-four mutations each completed all four HTTP checks and then failed the original exact-peak assertion with the corresponding wrong count. Their direct exits were 101 in 1.158695, 1.167395 and 1.136090 seconds. Removing the actual waiting witness failed the four-attempt rendezvous in 3.321351 seconds, after releasing and joining. Restored source passed the selected test, exit 0 in 1.189999 seconds. These are semantic controls, not compile failures or timeout increases.

## Limits and ownership

The separate GLM first implementation and its raw tool history remain preserved. Its ten-second hold and release ordering were not adopted or credited for the tested Codex fallback. Later model review is advisory only. This patch supplies no study-window observation, idle timing, Linux CI success or F-227 completion. Final canonical feature verification and a separately authorized qualification run remain required. The permanent regression now challenges the actual admission consumer. Existing workflow already requires such lifecycle controls, so no new gate exception or process rule is introduced.
