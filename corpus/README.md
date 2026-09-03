# The golden corpus

The corpus is **not in git**, deliberately. `corpus/manifest.tsv` is, and it is
what makes the corpus verifiable without being present.

## Where it lives

```bash
export OCELLI_CORPUS_DIR=/path/to/your/corpus
python3 scripts/corpus_check.py            # verify against the manifest
```

Default when the variable is unset: `corpus/data/`, which is gitignored.

## Why it is not committed

Three reasons, in order of how much they bind:

1. **Redistribution terms are not ours to assume.** A TCIA collection carries
   its own licence and citation requirement per collection. Re-hosting it
   inside a repository that will be public changes who is redistributing it.
   The manifest records the licence and its URL per case so the question has
   an answer without the bytes being here.
2. **PHI risk is not zero just because a set is labelled de-identified.**
   Burned-in pixel annotation is a real and common failure, which is why
   HLD E22.3 is a story. A repository that never contains DICOM cannot leak
   one. `.githooks/pre-commit` refuses a staged `.dcm` for that reason.
3. **Size.** A corpus large enough to be worth rendering is larger than a
   repository should be.

## What the manifest guarantees

Every row carries `sha256`, so a local corpus is either the corpus the
tolerance policy was written against or it is not, and `scripts/corpus_check.py`
says which. A silently different corpus is how a green suite stops meaning
anything.

Columns: `path`, `modality`, `transfer_syntax`, `category`, `source`,
`licence`, `licence_url`, `sha256`, `url`.

`url` may be empty for a case obtained out of band. The row is still checked
for presence and digest, it simply cannot be fetched. `scripts/corpus_check.py
--fetch` downloads only the rows that carry one.

## Adding a case

Every field bug becomes a permanent fixture (HLD section 11, story E2.6). To
add one:

1. Put the file under `$OCELLI_CORPUS_DIR`.
2. `python3 scripts/corpus_check.py --add <file> --modality CT --category ...`
   appends the row with a computed digest.
3. Commit the manifest row, never the file.

The tolerance the case is compared under comes from the policy in
`docs/hld/22-testing-and-tolerance.md` section 25.1. It is not set per case.
A tolerance change is a pull request with a rationale, reviewed like code.
