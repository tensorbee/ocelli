# F-X010 review, pass 2

**Reviewed**: complete staged implementation against `408c866`
**Reviewer**: independent agent, did not write the implementation
**Result**: 0 defects, 0 smells, 0 nitpicks

Pass 1 predates the substantive probe remediation and is not treated as
evidence for this pass.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Probe equivalence and refusal precedence

The initial feature state disturbed 16 legacy `ci-floor` probes. Six are
restored by the staged blocked-first refusal precedence:

- `event-gated`
- `no-arm-command-gate-behind-a-condition`
- `step-keys-in-the-other-order`
- `quoted-if-key`
- `flow-mapping-run-before-if`
- `quoted-event-key`

An adversarial sandbox mutation moved the named-invocation decision back in
front of the blocked decision. All six then lost their declared condition or
event refusal. Four were intercepted by the visible multi-command message and
the two no-arm cases were intercepted by the older unnamed-work message. This
shows the staged precedence is load-bearing and that these probes still test
reachability rather than command-arm spelling.

Ten more legacy probes required direct builder or expectation changes. The
three invisible-work shapes, `work-inside-an-if`,
`work-behind-the-command-builtin`, and `background-operator-in-an-arm`, now
leave exactly one visible command in `bench`, retain invisible work, and
remove the named CI invocation. The new multi-command rule is therefore not
available and each probe reaches the older extractor-vocabulary refusal. The
`narrowed-arm-command` shape likewise leaves one visible command and no
invisible work. It reaches the no-running-command refusal because its argument
vector is no longer exact.

The remaining six parser probes correctly produce a two-command visible `fmt`
arm and expect the new named-invocation refusal:

- `arm-comment-holding-a-terminator`
- `arm-terminator-inside-a-quote`
- `comment-after-a-substitution`
- `heredoc-delimiter-backslash-quoted`
- `heredoc-delimiter-quoted-with-a-hyphen`
- `continuation-after-an-arm-comment`

That refusal does not mask their parser defense. In each sandbox, emulating
the historical truncation by retaining only the command already run directly
by CI made `covers` return true. The named requirement also disappeared
because the broken parse reported only one visible command. Their red result
therefore depends on preserving the planted command through the parser.

The separately reshaped
`gate-with-an-unextractable-arm-command` probe reports one visible command,
invisible work, and no named invocation. It remains independent of F-X010's
multi-command rule. The four new F-X010 probes also discriminate their stated
boundaries: exact commands split across steps, reversed, or split across jobs
are refused, while a descriptive named gate step in the existing area job is
accepted.

## Connection and drift checks

`test_guard_readers.py` is named explicitly in the `guards` gate. CI invokes
`bin/ocelli.sh gate guards`, and the complete connected gate passed. The
workflow migration keeps `backlog` in its existing area job and delegates its
two ordered commands to the runner. The `corpus`, `guards-deep`, and `oracle`
floor exclusions are unchanged.

The approved plan's measured write-set corrections account for the CI
workflow and generated runbook. The LLD describes the implemented visible
multi-command, single-command, invisible-work, precedence, and shared-selector
contracts. No HLD file, tolerance, deviation, GPU path, pixel path, unsafe
allowance, wasm-bindgen boundary, sprint record, verification record, commit,
or integration state is changed.

## Independent checks

```text
git diff --cached --check
sh -n bin/ocelli.sh
python3 -m unittest scripts.tests.test_guard_readers scripts.tests.test_guard_catalogue
python3 scripts/ci_floor_check.py
python3 scripts/guard_census.py --check-runbook
python3 scripts/guard_probe.py --only <the 16 initially failing legacy probes, the separately reshaped unextractable probe, and the four new F-X010 probes>
bin/ocelli.sh gate guards
```

The combined unit run passed 129 tests. The CI-floor control passed all 25
floor gates with the three declared exclusions. The targeted probe run drove
20 refusal probes red for their declared reason and kept one accept probe
green. The complete connected `guards` gate drove 152 refusal probes red,
kept 23 accept probes green, preserved the one declared G-04 known defect, and
passed its 54, 4, and 75 test groups. The census and generated runbook agree at
630 refusals, 61 files, and 238 probes.
