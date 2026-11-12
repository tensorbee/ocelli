# F-245 review, pass 1

**Reviewed**: work/f-245-glm at 01b20005.
**Author / reviewer**: GLM through opencode, with lead amendments ed4f42c1 and 8d4c4f63. Review by Claude, a read-only subagent, composed by the Claude coordinator for S47.
**Result**: 4 defects, 3 smells, 6 nitpicks

D1 the DICOMweb browser suite was registered in no gate, with a false claim that F-268 owned registration. D2 build-targets.md claimed the guard catalogue unchanged. D3 the page never cancelled or released remote state on a source change. D4 two LLD sentences stale after npm run dev changed. Smells on launcher evidence, the AC6 tier matrix and write-map deviations, settled by lead amendment ed4f42c1. Security, CSP, native routes and the provider verified clean.
