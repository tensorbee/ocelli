# Developer setup

Everything runs natively. There is no container, because Ocelli needs a real
GPU for WebGPU and a real browser for the differential oracle, and a container
gives it neither.

## Prerequisites

| Tool | Version | Why |
|------|---------|-----|
| Rust | **1.97.1** | Pinned in `rust-toolchain.toml`. `rustup` picks it up |
| `wasm32-unknown-unknown` | | The wasm target, installed by the toolchain file |
| wasm-pack | 0.15+ | Builds `crates/ocelli-wasm` |
| Node | 24+ | The TypeScript workspaces |
| Python | 3.12+ | The guard scripts, and corpus and fixture work |
| pandoc | | Cuts `docs/hld/` from the authored `.docx` |
| A WebGPU-capable browser | | Chrome or Edge. Required for the oracle |

```bash
rustup toolchain install 1.97.1
cargo install wasm-pack
python3 -m pip install openpyxl 'pydicom[all]' numpy
brew install pandoc dcmtk           # macOS
npm ci
```

`dcmtk` is optional but useful for corpus triage. See the `dicom-tooling`
skill.

## First run

```bash
git config core.hooksPath .githooks    # once per clone. Not optional
bin/ocelli.sh gate --floor             # everything that needs no GPU
```

**Enable the hooks.** They are what refuse a staged DICOM and what write the
provenance trailer. A clone without them can commit patient data and can push
a head CI will reject.

## The private source documents

The authored `.docx` and the backlog `.xlsx` are held outside the repository.
Record where they live, once per clone:

```bash
python3 scripts/source_dir.py --set /path/to/source-documents
python3 scripts/source_dir.py --check
```

Without them the `docs` gate **skips with a stated reason** rather than passing
or failing, and says which of the three sources it resolved the path from. You
only need them to regenerate or verify `docs/hld/` and the backlog.

**That folder holds three things, not two.** The `.docx`, the `.xlsx`, and
`redactions.json`, which carries the commercial product names stripped from the
published specification. The rules live with the private source rather than in
this repository, because a map of what was redacted still contains what was
redacted.

So **a folder holding only the two documents is not the private source folder**,
and pointing this setting at one is worse than leaving it unset. It was worse
silently until `split_hld.py` was taught to refuse: with the rules absent the
redaction step became the identity function and a regenerate wrote the product
names back into `docs/hld/`, with normal-looking output and nothing said.
`scripts/split_hld.py` now stops instead, and `docs/runbooks/guard-verification.md`
carries the probe that proves it.

One more thing to expect on a machine that is not the author's. The `docs` gate
asserts byte-identity against a freshly converted document, so a **different
pandoc version reports drift that is not drift**. pandoc 3.11 emits horizontal
rules where the version that generated the tracked files did not, which shows up
as a dozen changed files whose formulas, signatures and tables are all
identical. Diff before believing it, and never regenerate to make the gate
green.

## The corpus

Not in git. See `corpus/README.md`.

```bash
export OCELLI_CORPUS_DIR=/path/to/corpus
bin/ocelli.sh corpus
```

Until it exists, `/verify` records `corpus=absent` honestly. That is permitted
during early development and refused at `/release`.

## The gates

```bash
bin/ocelli.sh gate --list     # what each one covers, and which needs a GPU
bin/ocelli.sh gate --floor    # what CI runs
bin/ocelli.sh gate --all      # everything, including corpus and oracle
```

## Running the example viewer

```bash
bin/ocelli.sh wasm            # produces crates/ocelli-wasm/pkg
npm run dev                   # http://localhost:5173
```

A clean clone has no built core, so the viewer starts in its "core not built"
state rather than failing. That is deliberate.

## Why some things are the way they are

**Why no container.** WebGPU needs a real device. The oracle needs a real
browser. Containerising the Rust half alone would give two build trees, two
toolchain installs and a split mental model, for parity on the half that was
never the problem.

**Why the toolchain is pinned exactly.** `wgpu` is pinned exactly too, for the
reason in HLD section 15.2: agents reliably emit wgpu 0.19-era pipeline code,
and a caret range lets that compile against something subtly different from
what the shader expects. A floating toolchain reintroduces the same class of
surprise from below.

**Why `cargo check` and not `cargo build`.** The release profile is
`opt-level = "z"` with fat LTO and one codegen unit, which is slow on purpose
and only meaningful when measuring size. Iterate with `check`.

**Why the wasm module is not committed.** It is a build artefact, it is several
megabytes, and a stale one in the tree is worse than none: the example viewer
would show a core that is not the one the source builds.

## Troubleshooting

**`cargo` wants the network and you have none.** The skeleton has no external
dependencies on purpose, so `cargo check --workspace --offline` works from a
clean clone. Once real dependencies land, `cargo fetch` once.

**The oracle says it is not installed.** It is built by F-010. Nothing else in
the port should start before it works (`docs/hld/25-first-ten-files.md`).

**A gate fails and you did not touch that area.** Check whether it was red
before your change: run the same gate against the sprint base in a throwaway
worktree and compare failures by name. Do not assume.

**`git push` is refused.** The head carries no verification evidence. Run
`bin/ocelli.sh gate --all`, then commit again so the commit-msg hook can write
the trailer. See `docs/hld/DEVIATIONS.md` D-04 for why this exists.
