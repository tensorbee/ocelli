# F-245 viewer DICOMweb URL pair parser review, pass 2

**Reviewed**: `examples/folder-viewer/src/dicomweb.rs` aa2cd7bf798a2606aebc49d05feef2bf7d1eed59d276a500ba4f80910ae0bff2, parser items and the three `viewer_dicomweb_pair_*` tests only, against pass 1 at ddc08880. Worker plan 8a6e7e6960bf8e73 with one appended amendment to the startup URL policy paragraph.

**Author / reviewer**: GLM authored the v7 repair, the v8 compile fix and the amendment sentence. Claude coordinated, ran the Chromium observation and independently reviewed. Pass 1 was a separate Claude subagent.

**Result**: 0 defects, 0 smells, 2 nitpicks.

## Pass 1 dispositions

D1, hex IPv4 forms. Fixed. The last-label rule now treats a label as a number when it is all ASCII digits or `0x`/`0X` followed by ASCII hex digits, which is the WHATWG ends-in-a-number test. `http://0X7F000001:8042/s`, `http://0x/s` and `http://a.0x10/s` are refused in both positions. The `label[2..]` slice cannot panic because it follows a two-byte ASCII prefix check.

D2, IPv6. Resolved by refusal and amendment, from an actual observation rather than the CSP text. Chromium 151.0.7922.34 raised a CSP violation for a fetch to `http://[::1]:9/x` under `connect-src http://[::1]:9`, and the same for `[2001:db8::1]`. IPv4 literals and a DNS name matched without violation, and two mismatched-host controls violated, which proves the listener (`claude-csp-ip-probe/out.json`). An IPv6 URL would have passed startup and then been blocked on every fetch. It is now refused with the fixed generic error, and the former IPv6 positives moved to the refusal list.

S1, non-loopback IPv4 under CSP3 matching. Not reproduced. Chromium matched `192.0.2.10`. Firefox and Safari were not observed and that remains an external observation gap, not a claim.

N1, invalid punycode such as `xn--a` accepted. Carried as a nitpick below.

N2, two unreachable guards. Fixed, both removed.

## Findings

**N1** `dicomweb.rs:416`. An invalid A-label such as `http://xn--a/s` is accepted although a browser cannot resolve it. It fails closed at fetch time and introduces no CSP source beyond the configured host.

**N2** `dicomweb.rs:419` and `:507`. IPv6 is refused twice, in `is_allowed_host` and again in the origin match, because both matches must be exhaustive. Mutating only the first site leaves the tests green. Mutating both turns `viewer_dicomweb_pair_rejects_faulty_urls_identically` red, so the refusal is covered. The second arm is unreachable but required by the compiler.

## Amendment review

The appended sentences refuse IPv6 literal hosts and numeric last labels at startup and give the observed Chromium non-match as the IPv6 reason. They narrow "any host" without touching user info, query, fragment, path or the generic no-echo refusal. No acceptance criterion, tolerance or baseline changes. The plan owner and a further reader should still confirm the narrowing, since IPv6-only DICOMweb services cannot be configured in this viewer.

## Evidence

| Command | Exit and observation |
| --- | --- |
| `cargo test -p ocelli-folder-viewer --lib viewer_dicomweb_pair`, two jobs | 0, 3 passed |
| `cargo clippy -p ocelli-folder-viewer --all-targets` | 0, only dead-code warnings for the not yet called startup items |
| `cargo fmt -p ocelli-folder-viewer --check` | 0 after formatting |
| hex-last-label mutant | 101, faulty-URL test fails |
| IPv6 accepted at `is_allowed_host` only | 0, see N2 |
| IPv6 accepted at both sites | 101, faulty-URL test fails |
| Chromium 151 CSP probe, five sources plus two mismatch controls | 0, as described above |

Receipts: `claude-native-config-pair-v8-mutants.json`, `claude-native-config-pair-v7-application.json`, `claude-native-config-pair-v8-application.json` in the evidence root.

## Still open

No startup caller exists, so the parser is reached only by tests. Startup binding, the configured policy on `/` and `/index.html`, and the `/dicweb/config` reply are the next slice.
