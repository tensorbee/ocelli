# F-245 review, config pair pass 1

**Reviewed**: `examples/folder-viewer/src/dicomweb.rs` in the dirty worker `/private/tmp/ocelli-s47-f245-glm` (HEAD `d85406a5eed7d4340b3b4e6db961631569bff99a`), file sha256 `ddc08880bde3afc949c4bcac55f1c5cb044d4b435ab7e31e9778a53932aeec84` verified before and after the pass. Scope is the URL pair parser only, lines 394 to 558 and the three `viewer_dicomweb_pair_*` tests at lines 1213 to 1375, diffed against `/private/tmp/ocelli-s46-codex/f245-implementation/claude-dicomweb-rs-before-pair-v3.rs`. The rest of the file is unchanged and out of scope.
**Author / reviewer**: author GLM. Reviewer Claude (root independent reviewer, subagent). The reviewer wrote none of the parser or its tests.
**Result**: 2 defects, 1 smell, 2 nitpicks

## Acceptance challenged

Plan line 272 (exactly two full URLs, both absent means local only, partial or malformed pair refused generically with no echo, HTTP and HTTPS only, any host, no user info, no query or fragment, path allowed). Plan line 274 (only the studies origin is allowed to connect, no wildcards, no other directive changes). The derived `connect_origin` was challenged against WHATWG URL host parsing and origin serialization, and against the CSP3 `host-source` grammar and its IP address note, using independently chosen inputs rather than the tests' own cases.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `shasum -a 256 examples/folder-viewer/src/dicomweb.rs` | worker file | now, before and after | 0, `ddc08880...ec84` both times |
| `CARGO_BUILD_JOBS=2 cargo test -p ocelli-folder-viewer --lib viewer_dicomweb_pair` | worker root | now | 0, 3 passed, 151 filtered |
| standalone probe `/private/tmp/claude-501/f245-pair-probe`, `cargo run -q` | lines 394 to 558 copied verbatim, `uriparse = "=0.6.4"`, rustc 1.97.1 | now | 0, 52 inputs printed with their origin or refusal |
| same probe, `cargo test -q --lib` | lines 1213 to 1375 copied verbatim | now | 0, 3 passed |
| 8 mutants on the probe copy, restored and `cmp` clean | probe copy only, never the worker | now | see Verified clean and N2 |
| GLM v4 mutant receipt | `claude-native-config-pair-v4-mutants.json` | prior receipt | numeric label, host case and default port mutants red, IPv4 noncanonical mutant exit 0 (later shown unreachable, removed in v5) |
| CSP3 editor's draft, host-source ABNF and IP note | https://w3c.github.io/webappsec-csp/ | now, fetched | `host-char = ALPHA / DIGIT / "-"`, and "only `127.0.0.1` will actually match a URL when used in a source expression" |

## Defects

### D1, hexadecimal IPv4 host forms are accepted and serialized as names

**Where**: `examples/folder-viewer/src/dicomweb.rs:479` to `:490`
**What**: the ends-in-a-number guard only treats an all-decimal last label as numeric. Probe output: `http://0X7F000001:8042/s` gives origin `http://0x7f000001:8042`, `http://0x/s` gives `http://0x`, `http://a.0x10/s` gives `http://a.0x10`.
**Why it is wrong**: WHATWG URL "ends in a number checker" also returns true when the last label is `0x` or `0X` followed by zero or more hex digits, then runs the IPv4 parser. `0x7f000001` becomes `127.0.0.1` and `0x` becomes `0.0.0.0`, so the browser origin is `http://127.0.0.1:8042`, not the derived one. The CSP source names a host the fetch never targets, so the connect is blocked. `a.0x10` fails host parsing entirely, the same unfetchable class v4 closed for `999.999.999.999`. This violates the slice's own contract that the origin equals the browser serialization.
**Evidence**: probe run above, exit 0, the three origins as quoted.
**Scope and regression**: fails closed, no widening. Smallest regression is adding `http://0x7f000001/studies`, `http://0x/studies` and `http://a.0x10/studies` to `faulty_urls`, with the guard extended to the WHATWG definition (last label, after the existing empty-label refusal, is all decimal digits, or `0x`/`0X` plus only hex digits).

### D2, IPv6 hosts are accepted but the origin is not a CSP3 host-source, and mapped forms serialize differently from WHATWG

**Where**: `examples/folder-viewer/src/dicomweb.rs:417`, `:500` to `:504`, test expectation at `:1289`
**What**: every IPv6 literal is accepted and emitted as `scheme://[addr]`. The test pins `https://[2001:db8::1]:8042` as correct. Separately, `http://[::ffff:1.2.3.4]/s` and `http://[::FFFF:0102:0304]/s` both give `http://[::ffff:1.2.3.4]` because std `Ipv6Addr` Display writes IPv4-mapped addresses in dotted form.
**Why it is wrong**: CSP3 `host-part` is `1*host-char *( "." 1*host-char )` with `host-char = ALPHA / DIGIT / "-"`, so `[` `]` and `:` inside the host make the source expression invalid and a conforming parser drops it. The connect is then blocked for every accepted IPv6 configuration. For the mapped case, WHATWG IPv6 serialization is always eight hex pieces with compression, giving `[::ffff:102:304]`, so even a browser that parsed bracketed sources would not match the dotted text.
**Evidence**: probe run, exit 0. Mutant `ipv6-refused` turns the accept test red, which shows the test asserts the accepted behavior rather than guarding against it.
**Scope and regression**: fails closed, no wildcard or extra source. The plan says "any host", so the contract owner must choose between refusing IPv6 at startup with the generic message (and moving the IPv6 cases to `faulty_urls`) or recording a deviation that IPv6 studies URLs are accepted but unusable under the CSP. Either way the current test asserting acceptance as success is the wrong pin. Not confirmed in a real browser.

## Smells

### S1, non-loopback IPv4 origins rely on matching that CSP3 says does not happen

**Where**: `examples/folder-viewer/src/dicomweb.rs:417`, `:499`
**What**: `http://192.0.2.10:8042/studies` is accepted with origin `http://192.0.2.10:8042`.
**Why it is a smell**: CSP3 states that although IP addresses match the grammar, only `127.0.0.1` will actually match a URL in a source expression. A conforming user agent blocks the studies fetch for any other IPv4 host. Unconfirmed in Chromium or Firefox, which may compare the literal. The mechanism is concrete: a LAN PACS configured by address would pass startup and fail at the first QIDO request with a CSP violation rather than a startup refusal. Needs either a browser receipt or a recorded limitation.

## Nitpicks

### N1, invalid punycode labels are accepted

`http://xn--a/s` gives `http://xn--a`. WHATWG domain-to-ASCII decodes `xn--` labels and fails on invalid Punycode, so the host is unfetchable. Fails closed at fetch time, `dicomweb.rs:415` to `:433`.

### N2, two guards are equivalent mutants

`!last_label.is_empty()` at `:484` is unreachable because `is_allowed_host` already refused empty labels, and mutant `empty-label-guard-removed` stays green. `scheme_str.to_ascii_lowercase()` at `:496` is redundant because uriparse 0.6.4 `Scheme::try_from` normalizes registered schemes to the lowercase variant (`scheme.rs:1074` asserts `"HTTP"` becomes `"http"`), and mutant `scheme-case-kept` stays green. Harmless defence, but neither line is a tested guard.

## Verified clean

- No value or parser error is echoed. Every refusal path builds `io::Error` from the constant at `:398`, and the tests compare `to_string()` to it for every faulty input in both positions.
- The wado URL goes through the same `validate_full_http_url`, its origin discarded per plan line 274. Mutant `wado-unvalidated` is red.
- Scheme case (`Http://`, `HTTPS://`), host ASCII case, default ports (`:80` http, `:443` https, `https://h:80` kept), ports `:0`, `:65535`, `:0080`, `:00000000000000000080`, empty port `http://h:/` all match WHATWG origin serialization. `:65536` and `::80` refused.
- Trailing dot, `*`, `_`, `%`, non-ASCII `bücher`, zone ids `%25eth0`, IPvFuture, backslash, leading or trailing space, tab, empty userinfo `@h`, `h@`, `http:h`, `http:////h` are all refused. Refusal is stricter than WHATWG for some of these, which fails closed.
- Paths with `//`, `%2F`, `;`, `,`, `'self'`, `%0d%0a` are accepted and do not affect the origin, and only the origin reaches the CSP.
- Labels over 63 and hosts over 253 octets are accepted, which matches WHATWG with `beStrict` false and CSP3 has no length limit, so this is not a contract issue.
- Mutants `userinfo-allowed`, `query-allowed`, `charset-star` and `fixed-message-studies-only` are all red. GLM's v4 receipt shows numeric-label, host-case and default-port mutants red.
- No `as` cast in the slice. Allocations are one `String` per URL plus `to_string` temporaries at startup only, outside any render loop.

## Limitations and handoff

- No caller exists yet, so the dead-code warnings are expected. Startup wiring, `/dicweb/config` and `configured_main_policy` consumption are open coverage, not reviewed here.
- No browser was run, so D2's CSP outcome and S1 rest on the CSP3 text, not an observed violation report.
- Next owner is GLM for D1, D2 and S1, then a fresh independent pass on the changed lines.
