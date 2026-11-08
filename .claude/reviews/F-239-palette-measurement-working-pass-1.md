# AC27 root LLD note factual review, pass 1

Reviewer: Codex. Root authored lld-note-draft.md. I authored the adapter tests and setup driver, so this reviews a different author's factual writeup and independently recomputes its stated artifact properties. It does not independently approve my own harness or product change. Root executed the actual producer builds and two-row observation.

Result: zero defects, zero smells, zero nits in this bounded note.

Both named source identities match before-binary.json, after-binary.json and identity-results.json: 7678f744063b674c2fffec3547adb162cfd5d638 before and 8d142a48d22672824a1a6cf758f6f320d57e0633 after. Their direct build receipts run cargo build --release -p ocelli-oracle --bin ocelli-compare, both exit 0. I rehashed both preserved binaries and matched the receipt's 999d5e5c... and 68e5ec36... values.

I independently read and hashed all four synthetic candidate .raw files. Each is 1048576 bytes. The first pair's SHA-256 is 19341b4f4e8c7cf95501015c324e1964bfa5d70cbaf4cef0e1fe2ffa25472f0c. The second pair's is 06ddef8c9a7e4511f638314506b8f86fd6bc76bff930f5b8250d611a3a7a03f0. The root note copies both exactly. Both actual producer logs say written 2, omitted 0, unsupported 0, and identity-results.json records direct exits 0.

The R1 statement is independently checked against only the two manifest-hash-qualified synthetic files. pydicom metadata inspection confirms all three descriptors declare 16 bits, and every table entry is at most 255. The first file has 256 entries per channel and the second 65536. No clinical file, patient value or raw byte payload was printed. lld-synthetic-table-check.json and lld-raw-independent-check.json preserve those aggregate results and hashes. The source adapter directly calls the shared provider, projects its eleven input attributes and preserves decoded-RGB/non-palette sidecar bypass. The wording maps provider errors to palette-refused without incorrectly treating Ok(None) as such an error.

The note correctly limits the result to before/after native candidate identity on two selected synthetic rows. It does not claim a new Cornerstone draw, full corpus gate, independent pixel gold, size budget, performance win or whole-feature acceptance. The final corpus gate remains explicitly separate. I ran no producer, browser, GPU, native build or timing during this factual review.
