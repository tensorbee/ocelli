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
| uv | 0.11+ | Creates the locked repository-local Python environment |
| A WebGPU-capable browser | | Chrome or Edge. Required for the oracle |

```bash
rustup toolchain install 1.97.1
cargo install wasm-pack
uv sync --locked
brew install dcmtk                  # macOS
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

## The specification and delivery plan

`docs/hld/`, `docs/sprints/BACKLOG.md`, `docs/sprints/SPRINT_PLAN.md` and
`docs/sprints/allocation.json` are the authoritative project sources. They were
sanitized and converted to tracked repository formats during bootstrap. A clone
does not need the original DOCX, XLSX or private redaction rules.

Changes to the HLD go through a reviewed design plan and a declared deviation
where implementation departs from the specification. Backlog, sprint plan and
allocation changes must keep `backlog` and `deviations` gates green.

## The corpus

The bytes live under ignored `corpus/data`. See `corpus/README.md`.

```bash
uv run scripts/populate_corpus.py
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
