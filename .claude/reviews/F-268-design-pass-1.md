# F-268 design microscope, pass 1

Reviewer: Codex, independent of GLM design authorship. Reviewed private proposed plan SHA256 `864732703755f6ba8f7ff65f4d64be3233088b615acfc0042159f70243235864`, exact patch `c73e826e86583683837e4b9a02c6301dae41b2f46e51220a520db3743827e760`, canonical draft and actual diagnostic source `47356703d47a8b61ce8f9eba514c493ba306de88`. No implementation is authorized by this pass.

Result: 1 defect, 0 smells, 1 nitpick. The actual source-bound doc fixes, public API preservation, provider map and proposed floor placement are otherwise coherent.

## Defect

D1. Required lint flags lack a concrete effective-environment policy. Explicit `RUSTDOCFLAGS` does not take precedence over `CARGO_ENCODED_RUSTDOCFLAGS`. Root executed a tiny dependency-free Cargo documentation control with a real public-to-private link. The proposed pair of denied lints exits 101 normally, but exits 0 when inherited encoded flags allow those same lints. This is an actual composition counterexample, not a workspace acceptance run or product mutation. Proof and raw logs are in `/private/tmp/ocelli-s46-codex/f268-workspace-doc-placement-resume-glm/root-flag-precedence-proof.json`.

The [Cargo configuration reference](https://doc.rust-lang.org/cargo/reference/config.html#buildrustdocflags) lists encoded flags before plain flags. The existing workspace diagnostic already unset encoded flags in its child. That measured setup is not yet the proposed gate's stated policy. Specify authoritative required lint flags and child-local handling of overriding encoded flags, without changing the parent environment. Require the actual future gate to catch a planted link even when the caller supplies encoded allow flags, followed by healthy recovery. Do not weaken a lint, substitute a source grep or claim that merely printing flags enforces them.

## Nitpick

The Tests table labels the planted-link controls unit tests. They execute Cargo and rustdoc with filesystem I/O and belong to integration under the workflow taxonomy. Relabel the row, with no new test obligation.

## Checked and accepted design direction

The actual native diagnostic exited 101 with 11 located errors across six crates. Ten positions link to private targets and one unresolved position names `LutChain::new`. The seven named source files match the actual location inventory. The historical draft's disputed count is not promoted to a current measurement.

Keep private helper references as code identifiers, preserving meaningful numerical and refusal prose. The existing public constructor can be fully qualified as `ocelli_pixel::LutChain::new`. Root checked the public struct and constructor in `ocelli-pixel/src/lut.rs` and its root reexport. Do not expose private helpers, change API visibility, rename functions or add doc-only imports to hide the problem.

The gate belongs in the floor because its default selected native documentation build needs neither GPU nor corpus. Root checked the actual GATES registry, runner floor exclusion rule and CI Rust job. A new named gate needs an actual unconditional CI invocation. No exclusion-list change follows from adding an included gate. Workflow, affected-command and adapter synchronization changes still require their own review and separate policy commit. This design decision does not implement or adopt that policy change.

Mandatory healthy, private-link and unresolved-link controls must execute real rustdoc and preserve own exit statuses, source, target, tool and effective flag identities. The actual baseline proves host/default-selection diagnostics, not every wasm cfg or feature combination. No pixel arithmetic or rendered parity is claimed.

The author's final raw review labels two wording preferences as smells. They have no identified runtime or maintenance failure mechanism after the named reviewer was supplied, so root treats them as nonblocking style suggestions. Earlier invented commands, malformed table clauses, unsupported source concerns and capped outputs remain preserved. They are not an accepted clean author pass. Root's independent finding above is the blocking design issue.

GLM owns the bounded plan amendment and its own review. Root will review that exact amendment before approving design or starting isolated implementation. No product source, new gate, workflow policy, canonical delivery status, integration or publication changes follow from this review.
