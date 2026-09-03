# Contributing to Ocelli

Thank you for looking. Two things will surprise you if you have contributed to
other projects, so they are first.

## 1. Never send us patient data

**No DICOM enters this repository, ever.** Not a fixture, not a reduced case,
not an anonymised one.

The pre-commit hook refuses a staged DICOM by magic bytes as well as by
suffix, because a file called `anon001` with no extension is a very normal way
to receive one. There is no allowlist and no escape hatch.

A dataset labelled de-identified can still carry burned-in pixel annotation.
A repository that never contains DICOM cannot leak one, which is why the rule
is absolute rather than careful.

Test images live outside the repository, described by `corpus/manifest.tsv`.
See `corpus/README.md`.

## 2. Read the source policy before you write code

`docs/SOURCE-POLICY.md` records which third-party projects may be **read** and
which may be depended on, and the distinction matters more here than usual.
Translating source into Rust is a translation, which is an exclusive right of
the copyright holder, so a copyleft licence constrains reading and not only
linking.

**Two well-regarded projects are out of bounds to open**, purely on licence
grounds. If you have recently read either, say so rather than contributing to
the affected area. That is not an accusation, it is how a clean-room position
stays defensible.

`scripts/source_provenance_check.py` enforces this on every commit.

## The rhythm

```text
/design F-XXX  ->  /start-feature  ->  /implement-feature
               ->  /microscope     ->  /verify  ->  /complete-feature
```

`.claude/WORKFLOW.md` is the full process and it wins on any procedural
question. `docs/hld/` is the specification and it wins on what to build.

Work is tracked as F-IDs in `docs/sprints/BACKLOG.md`. Pick one that is
`pending` and whose dependencies are `done`.

## Before you open a pull request

```bash
git config core.hooksPath .githooks   # once per clone, not optional
npm ci
bin/ocelli.sh gate --floor            # everything needing no GPU
```

`bin/ocelli.sh gate --list` shows what each gate covers. A gate that cannot
run reports **skipped**, never passed.

## The standard that is higher than you expect

**The dangerous defect here is not the crash, it is the pixel that is quietly
wrong.** At the centre of a soft-tissue CT window the two DICOM VOI LUT
functions differ by 0.32 of 255: invisible in a screenshot, immediate in a
pixel diff.

So, for any change touching pixel or geometry arithmetic:

- **Write the fixture first**, with expected values taken from the DICOM
  standard and the section cited in a comment. Not from what the code produces.
  A test written after the implementation, by the same reasoning, asserts that
  the implementation is itself.
- **Mutate one constant, re-run, confirm the test goes red**, then revert. A
  test that passes both ways is not a test.
- **Do not change a tolerance to make something pass.** That is a design
  decision, reviewed like code.
- **Every `as` cast is a review item.** The lints deny the lossy ones. Do not
  add an `allow` to move on.

## Things that will be asked in review

- Which capability tier does this work on, and what does it do on the others?
  There are three: WebGPU, WebGL2, and CPU. "Not applicable" is a fine answer,
  an omitted answer is not.
- Does a feature that cannot run report **unavailable**, rather than quietly
  producing a different result?
- Is `wasm-bindgen` still confined to one crate?

## Licence

Contributions are dual-licensed under MIT or Apache-2.0, matching the project.
See `LICENSE`.
