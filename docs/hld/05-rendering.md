<!-- Originally converted by scripts/split_hld.py from Ocelli-HLD.docx.
     This tracked Markdown file is authoritative.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the tracked Markdown wins. -->

# Rendering

**Source**: bootstrap import from `Ocelli-HLD.docx`, section 7. This tracked Markdown is authoritative.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

## 7. Rendering

- **Two capability tiers, one codebase.** Tier A is WebGPU: compute shaders, storage buffers, 3D textures to 2048. Tier B is WebGL2 through wgpu's downlevel profile: fragment shaders only, no compute, no storage buffers, a conservative 3D-texture floor of 256. Every feature declares the tier it needs; the tier resolves once at startup.

- **Volume rendering runs on both tiers**, because a 3D-texture ray-cast in a fragment shader is tier-B legal. Anything wanting compute — GPU segmentation, histogram passes, compute-based resampling — is tier A only and must degrade, not fail.

- **Bricking above 256 MiB.** A 512×512×600 sixteen-bit CT series is roughly 300 MB against a guaranteed maximum buffer size of 256 MiB, so chunked upload is the normal path, not an optimisation.

- **One submit per frame.** The render graph tracks dirty viewports and issues a single submission across all of them, driven by requestAnimationFrame inside the render worker.

- **Blend modes are shader variants** — composite, MIP, MinIP, average — selected by specialisation constant rather than branching per fragment.
