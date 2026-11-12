//! Content Security Policy for the folder viewer's trusted page literals.
//!
//! Inline script and style text, including import maps, is hashed with the
//! workspace SHA-256 and encoded with RFC 4648 base64. Each distinct page's
//! policy is cached for the process lifetime. The configured form with extra
//! origins is built per context and not cached. Production calls use the
//! three embedded page constants.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, PoisonError};

use sha2::{Digest, Sha256};

/// The per-page policy cache. Keyed by the page constant itself.
static POLICY_CACHE: OnceLock<Mutex<HashMap<&'static str, &'static str>>> = OnceLock::new();

/// The policy for one embedded page constant.
///
/// The same page always yields the same policy, and after the first call the
/// cached value is returned, so repeated calls for one page never recompute.
pub(crate) fn page_policy(page: &'static str) -> &'static str {
    let cache = POLICY_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(&policy) = cache.get(page) {
        return policy;
    }
    let policy: &'static str = Box::leak(configured_main_policy(page, &[]).into_boxed_str());
    cache.insert(page, policy);
    policy
}

/// Builds the per-page policy for the page with the given already validated
/// origins appended to connect-src. The caller must validate the origins.
/// The empty-origin form is what page_policy caches.
pub(crate) fn configured_main_policy(page: &str, origins: &[String]) -> String {
    let (scripts, styles) = collect_inline_bodies(page);
    let mut policy = String::from("default-src 'none'");
    policy.push_str("; script-src 'self'");
    for script in &scripts {
        push_hash(&mut policy, script);
    }
    policy.push_str("; style-src 'self'");
    for style in &styles {
        push_hash(&mut policy, style);
    }
    policy.push_str("; img-src 'self' data:");
    policy.push_str("; connect-src 'self'");
    for origin in origins {
        policy.push(' ');
        policy.push_str(origin);
    }
    policy.push_str("; base-uri 'none'");
    policy.push_str("; form-action 'none'");
    policy.push_str("; frame-ancestors 'none'");
    policy
}

fn push_hash(policy: &mut String, body: &str) {
    policy.push_str(" 'sha256-");
    policy.push_str(&sha256_base64(body.as_bytes()));
    policy.push('\'');
}

/// SHA-256 over the exact body bytes, RFC 4648 section 4 encoded.
fn sha256_base64(data: &[u8]) -> String {
    base64_encode(&Sha256::digest(data))
}

/// The RFC 4648 section 4 alphabet, Table 1.
const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// RFC 4648 section 4, the standard alphabet with canonical padding: pad
/// bits are zero, so every input has exactly one encoding.
fn base64_encode(data: &[u8]) -> String {
    let mut encoded = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let high = usize::from(chunk.first().copied().unwrap_or(0));
        let mid = usize::from(chunk.get(1).copied().unwrap_or(0));
        let low = usize::from(chunk.get(2).copied().unwrap_or(0));
        let group = (high << 16) | (mid << 8) | low;
        encoded.push(char::from(digit((group >> 18) & 0x3F)));
        encoded.push(char::from(digit((group >> 12) & 0x3F)));
        if chunk.len() > 1 {
            encoded.push(char::from(digit((group >> 6) & 0x3F)));
        } else {
            encoded.push('=');
        }
        if chunk.len() > 2 {
            encoded.push(char::from(digit(group & 0x3F)));
        } else {
            encoded.push('=');
        }
    }
    encoded
}

/// One digit of Table 1. `index` is a masked 6-bit value, at most 63, and
/// `BASE64` has 64 entries, so `.get` cannot miss. The fallback is
/// unreachable and would fail every RFC section 10 vector if taken.
fn digit(index: usize) -> u8 {
    BASE64.get(index).copied().unwrap_or(b'?')
}

/// The inline element bodies in document order: scripts first, then styles.
fn collect_inline_bodies(page: &str) -> (Vec<&str>, Vec<&str>) {
    let mut scripts = Vec::new();
    let mut styles = Vec::new();
    let bytes = page.as_bytes();
    let mut i = 0;
    while let Some(byte) = bytes.get(i).copied() {
        if byte != b'<' {
            i += 1;
            continue;
        }
        let Some((kind, name_end)) = tag_kind(bytes, i + 1) else {
            i += 1;
            continue;
        };
        let Some(start) = start_tag_end(bytes, name_end) else {
            i += 1;
            continue;
        };
        let Some(close) = closing_tag(bytes, start.after, kind) else {
            i = start.after;
            continue;
        };
        // A src script is external, so it contributes no hash. A style
        // element has no src attribute to check. Either way the scan
        // resumes after the closing tag, so a body is never rescanned.
        if kind == TagKind::Script && start.has_src {
            i = close.resume;
            continue;
        }
        if let Some(body) = page.get(start.after..close.body_end) {
            match kind {
                TagKind::Script => scripts.push(body),
                TagKind::Style => styles.push(body),
            }
        }
        i = close.resume;
    }
    (scripts, styles)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TagKind {
    Script,
    Style,
}

/// The element kind and the index just past its name, for a start tag whose
/// `<` sits at `at - 1`. Anything else is `None`, so `<scriptx>` is not
/// `<script`.
fn tag_kind(bytes: &[u8], at: usize) -> Option<(TagKind, usize)> {
    if !bytes.get(at).copied()?.is_ascii_alphabetic() {
        return None;
    }
    let mut end = at;
    while let Some(byte) = bytes.get(end).copied() {
        if is_html_space(byte) || byte == b'/' || byte == b'>' {
            break;
        }
        end += 1;
    }
    let name = bytes.get(at..end)?;
    let kind = if name.eq_ignore_ascii_case(b"script") {
        TagKind::Script
    } else if name.eq_ignore_ascii_case(b"style") {
        TagKind::Style
    } else {
        return None;
    };
    Some((kind, end))
}

struct StartTag {
    /// Index just past the start tag's `>`.
    after: usize,
    /// Whether a `src` attribute appeared, in any placement or quote.
    has_src: bool,
}

/// Walk a start tag's attributes from just past its name to its `>`. A
/// quoted value is consumed whole, so a `>` inside it does not end the tag
/// and the text `src=` inside it is not an attribute.
fn start_tag_end(bytes: &[u8], from: usize) -> Option<StartTag> {
    let mut j = from;
    let mut has_src = false;
    loop {
        while let Some(byte) = bytes.get(j).copied() {
            if !is_html_space(byte) {
                break;
            }
            j += 1;
        }
        match bytes.get(j).copied()? {
            b'>' => {
                return Some(StartTag {
                    after: j + 1,
                    has_src,
                });
            }
            b'/' => j += 1,
            _ => {
                let name_start = j;
                while let Some(byte) = bytes.get(j).copied() {
                    if is_html_space(byte) || byte == b'=' || byte == b'>' || byte == b'/' {
                        break;
                    }
                    j += 1;
                }
                let is_src = bytes
                    .get(name_start..j)
                    .is_some_and(|name| name.eq_ignore_ascii_case(b"src"));
                while let Some(byte) = bytes.get(j).copied() {
                    if !is_html_space(byte) {
                        break;
                    }
                    j += 1;
                }
                if bytes.get(j).copied() == Some(b'=') {
                    j += 1;
                    while let Some(byte) = bytes.get(j).copied() {
                        if !is_html_space(byte) {
                            break;
                        }
                        j += 1;
                    }
                    match bytes.get(j).copied() {
                        Some(quote @ (b'"' | b'\'')) => {
                            j += 1;
                            while let Some(byte) = bytes.get(j).copied() {
                                j += 1;
                                if byte == quote {
                                    break;
                                }
                            }
                        }
                        Some(b'>') => {}
                        Some(_) => {
                            while let Some(byte) = bytes.get(j).copied() {
                                if is_html_space(byte) || byte == b'>' {
                                    break;
                                }
                                j += 1;
                            }
                        }
                        None => {}
                    }
                }
                has_src |= is_src;
            }
        }
    }
}

struct Closing {
    /// Index of the `<` that opens the closing tag.
    body_end: usize,
    /// Index just past the closing tag's `>`.
    resume: usize,
}

/// The first closing tag for `kind` at or after `from`.
fn closing_tag(bytes: &[u8], from: usize, kind: TagKind) -> Option<Closing> {
    let name: &[u8] = match kind {
        TagKind::Script => b"script",
        TagKind::Style => b"style",
    };
    let mut k = from;
    while let Some(byte) = bytes.get(k).copied() {
        if byte == b'<' && bytes.get(k + 1).copied() == Some(b'/') {
            let mut end = k + 2;
            while let Some(byte) = bytes.get(end).copied() {
                if is_html_space(byte) || byte == b'>' {
                    break;
                }
                end += 1;
            }
            if bytes
                .get(k + 2..end)
                .is_some_and(|found| found.eq_ignore_ascii_case(name))
            {
                let mut resume = end;
                while let Some(byte) = bytes.get(resume).copied() {
                    resume += 1;
                    if byte == b'>' {
                        break;
                    }
                }
                return Some(Closing {
                    body_end: k,
                    resume,
                });
            }
        }
        k += 1;
    }
    None
}

/// HTML ASCII whitespace, WHATWG HTML's space, tab, LF, FF and CR.
const fn is_html_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | 0x0C | b'\r')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_origin_list_preserves_the_existing_policy_bytes() {
        // The cached default for COMBINED is the unchanged self-only shape.
        let cached = page_policy(COMBINED);
        let configured = configured_main_policy(COMBINED, &[]);
        assert_eq!(configured, cached);
    }

    #[test]
    fn a_configured_origin_changes_only_connect_src_and_keeps_the_page_hashes() {
        let origins = vec![String::from("https://studies.example:8443")];
        let configured = configured_main_policy(COMBINED, &origins);
        // Every other directive is byte-identical to the cached page policy.
        for name in [
            "default-src",
            "script-src",
            "style-src",
            "img-src",
            "base-uri",
            "form-action",
            "frame-ancestors",
        ] {
            assert_eq!(
                directive(&configured, name),
                directive(page_policy(COMBINED), name)
            );
        }
        // The page's own independent hash gold is still present.
        assert!(configured.contains(HASH_BOOT_CALL));
        assert!(configured.contains(HASH_H1_RULE));
        // connect-src gains exactly the given origin after 'self'.
        assert_eq!(
            directive(&configured, "connect-src"),
            "connect-src 'self' https://studies.example:8443"
        );
        // No forbidden source and no wildcard entered anywhere.
        assert!(!configured.contains("unsafe-eval"));
        assert!(!configured.contains("wasm-unsafe-eval"));
        assert!(!configured.contains("blob:"));
        assert!(!configured.contains('*'));
    }

    #[test]
    fn configured_origins_never_enter_the_cached_default_and_the_pointer_stays_stable() {
        let before = page_policy(NO_INLINE);
        let configured = configured_main_policy(
            NO_INLINE,
            &[
                String::from("https://studies.example:8443"),
                String::from("http://studies.example:9000"),
            ],
        );
        // The helper accepted both origins, in order.
        assert_eq!(
            directive(&configured, "connect-src"),
            "connect-src 'self' https://studies.example:8443 http://studies.example:9000"
        );
        // The cached default policy is untouched: still self-only.
        let after = page_policy(NO_INLINE);
        assert!(std::ptr::eq(before, after));
        assert_eq!(directive(after, "connect-src"), "connect-src 'self'");
        assert!(!after.contains("studies.example"));
    }

    use sha2::{Digest, Sha256};

    // Fixtures. Each is an exact page literal. The expected hashes beside
    // them were computed with Python hashlib and base64 over the body text,
    // and with RFC 4648 section 10 and FIPS 180-4 for the primitives, never
    // from this module's own output.

    /// Two distinct inline scripts, one per line.
    const TWO_SCRIPTS: &str = "<!doctype html><html><body>\n<script>alpha()</script>\n<script>beta()</script>\n</body></html>";

    /// Two style blocks.
    const TWO_STYLES: &str = "<html><head><style>body { color: black }</style><style>p { margin: 0 }</style></head></html>";

    /// An import map and a module script, the F-213 shape.
    const IMPORTMAP_AND_MODULE: &str = r#"<html><head>
<script type="importmap">{"imports": {}}</script>
<script type="module">import "./mod.js";</script>
</head></html>"#;

    /// Four src placements and quotes that contribute nothing, then one
    /// inline script that must still be hashed.
    const SRC_EXCLUDED: &str = r#"<html><body>
<script src="one.js"></script>
<script type="module" src='two.js'></script>
<script   defer   src=three.js ></script>
<script data-x="src=four.js" src="five.js"></script>
<script>inline()</script>
</body></html>"#;

    /// A `data-src` attribute and a `src=` text inside an unrelated value.
    /// Neither is a `src` attribute, so both bodies are hashed.
    const DATA_SRC_KEPT: &str = r#"<html><body>
<script data-src="not-a-src.js">kept()</script>
<script data-alt="src=inside.js">also()</script>
</body></html>"#;

    /// A script body with leading, inner and trailing whitespace to keep.
    const WHITESPACE_PAGE: &str = "<html><body>\n<script>\n  boot( 1 );\n</script>\n</body></html>";

    /// A `>` inside a quoted attribute value, in an inline script and in a
    /// src value. Only the first is inline, and neither quote may end the
    /// start tag early.
    const QUOTED_GT: &str = r#"<html><body><script data-label="a>b">gt()</script><script src="a>b.js"></script></body></html>"#;

    /// Only an external script. Neither directive carries a hash.
    const NO_INLINE: &str =
        r#"<html><head><script src="only.js"></script></head><body></body></html>"#;

    /// One style and one inline script, with a src script beside them. The
    /// exact policy string is asserted over this page.
    const COMBINED: &str = r#"<!doctype html><html><head><style>h1 { font-size: 2em }</style></head><body><script>boot()</script><script src="app.js"></script></body></html>"#;

    /// sha256 of `alpha()`.
    const HASH_ALPHA: &str = "'sha256-Gj+13GKCJNmXiKyVaEHz9N/OCYYGqhslxVRHPUGJnXY='";
    /// sha256 of `beta()`.
    const HASH_BETA: &str = "'sha256-8r/qxGqE4n3ShsC8FSFXFwSt9D+xEyurp1Nq2WAMOUs='";
    /// sha256 of the first TWO_STYLES body.
    const HASH_COLOR_RULE: &str = "'sha256-9nTC/7dB2f1JpbRTvOrSQbb3pM4m9KB3UwcuNN8sxso='";
    /// sha256 of the second TWO_STYLES body.
    const HASH_MARGIN_RULE: &str = "'sha256-7IWhgxoOGTB5NQzjGk9OPt0FwhVeBvZGoWgF9xLLh8c='";
    /// sha256 of the import map body.
    const HASH_IMPORT_MAP: &str = "'sha256-CJt/FpW3RQlGvqBeO/YK74xrTm1ctS6rLMJu1bV4fMc='";
    /// sha256 of the module script body.
    const HASH_MODULE_IMPORT: &str = "'sha256-K+N3h5LBAXLREHzjjrl336dr938Hd3ObUwASq0+Nryc='";
    /// sha256 of `inline()`.
    const HASH_INLINE_CALL: &str = "'sha256-fyoFIuhHbxPnDkt80oY6VkZ+KzhXiXlo1mBKEtnhf54='";
    /// sha256 of `kept()`.
    const HASH_KEPT_CALL: &str = "'sha256-/4E1Lb6rdDabql+IbVmlMOqdo8q8Hvk2EwcX73SpIXY='";
    /// sha256 of `also()`.
    const HASH_ALSO_CALL: &str = "'sha256-TTvED0u/Y9ZdXsyrR052qxcVIXYOVe6COIvrFU7hAmw='";
    /// sha256 of the WHITESPACE_PAGE body, newlines and inner spaces kept.
    const HASH_INDENTED_BODY: &str = "'sha256-5iuIV8cQF7mCpUpvlfxFC0yU5gVjr74mr+eiWn+FaI0='";
    /// sha256 of `boot( 1 );`, the same body with its whitespace stripped.
    /// The policy must not carry this one.
    const HASH_TRIMMED_BODY: &str = "'sha256-4aqdI3DUSwhbhF16OOhCFcVbhTluFFIQKCJcc6Zemwg='";
    /// sha256 of `gt()`.
    const HASH_GT_CALL: &str = "'sha256-+hrVMDvE8X3zf+15AhuCuLPhgXAftj1aIMpctEw3MGM='";
    /// sha256 of `boot()`.
    const HASH_BOOT_CALL: &str = "'sha256-MeZS89WlF0u+o0hCvHTBt4q1WHU+U+sJKgbdRUc36mY='";
    /// sha256 of the COMBINED style body.
    const HASH_H1_RULE: &str = "'sha256-x70MBaGtAZLOSBdc7kWyILwfVa+bBKqg5Y/n5byHnOo='";

    /// One directive's text out of a policy. Base64 carries no space or
    /// semicolon, so splitting on the directive separator is exact.
    fn directive<'a>(policy: &'a str, name: &str) -> &'a str {
        policy
            .split("; ")
            .find(|directive| directive.starts_with(name))
            .unwrap_or("")
    }

    #[test]
    fn base64_encodes_every_rfc_4648_section_10_vector() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn sha256_of_abc_matches_fips_180_4() {
        // The digest carries +, / and trailing =, so the standard alphabet
        // and the canonical padding are pinned with it.
        assert_eq!(
            sha256_base64(b"abc"),
            "ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0="
        );
        let hex: String = Sha256::digest(b"abc")
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            hex,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn two_distinct_inline_scripts_give_two_script_hashes_in_order() {
        let script_src = directive(page_policy(TWO_SCRIPTS), "script-src");
        assert_eq!(
            script_src,
            format!("script-src 'self' {HASH_ALPHA} {HASH_BETA}")
        );
    }

    #[test]
    fn more_than_one_style_block_gives_two_style_hashes() {
        assert_eq!(
            directive(page_policy(TWO_STYLES), "style-src"),
            format!("style-src 'self' {HASH_COLOR_RULE} {HASH_MARGIN_RULE}")
        );
        assert_eq!(
            directive(page_policy(TWO_STYLES), "script-src"),
            "script-src 'self'"
        );
    }

    #[test]
    fn an_importmap_and_a_module_script_are_both_hashed() {
        let script_src = directive(page_policy(IMPORTMAP_AND_MODULE), "script-src");
        assert_eq!(
            script_src,
            format!("script-src 'self' {HASH_IMPORT_MAP} {HASH_MODULE_IMPORT}")
        );
    }

    #[test]
    fn a_src_script_is_excluded_in_every_placement_and_quote() {
        // src first, src after type and single quoted, src unquoted with
        // extra whitespace, and src beside another attribute whose value
        // contains the text src=. Only the plain inline script remains.
        let script_src = directive(page_policy(SRC_EXCLUDED), "script-src");
        assert_eq!(script_src, format!("script-src 'self' {HASH_INLINE_CALL}"));
    }

    #[test]
    fn data_src_and_quoted_src_text_are_not_a_src_attribute() {
        let script_src = directive(page_policy(DATA_SRC_KEPT), "script-src");
        assert_eq!(
            script_src,
            format!("script-src 'self' {HASH_KEPT_CALL} {HASH_ALSO_CALL}")
        );
    }

    #[test]
    fn the_hash_covers_the_exact_body_text_with_its_whitespace() {
        let script_src = directive(page_policy(WHITESPACE_PAGE), "script-src");
        assert_eq!(
            script_src,
            format!("script-src 'self' {HASH_INDENTED_BODY}")
        );
        assert!(!script_src.contains(HASH_TRIMMED_BODY));
    }

    #[test]
    fn a_gt_inside_a_quoted_attribute_value_does_not_end_the_start_tag() {
        // A scan that ended the start tag at the first > would hash
        // b">gt() or treat the src value's > as the tag end.
        let script_src = directive(page_policy(QUOTED_GT), "script-src");
        assert_eq!(script_src, format!("script-src 'self' {HASH_GT_CALL}"));
    }

    #[test]
    fn a_page_with_only_external_scripts_keeps_the_directives() {
        assert_eq!(
            page_policy(NO_INLINE),
            "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; \
             connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'"
        );
    }

    #[test]
    fn the_policy_has_the_exact_shape() {
        assert_eq!(
            page_policy(COMBINED),
            "default-src 'none'; script-src 'self' \
             'sha256-MeZS89WlF0u+o0hCvHTBt4q1WHU+U+sJKgbdRUc36mY='; style-src 'self' \
             'sha256-x70MBaGtAZLOSBdc7kWyILwfVa+bBKqg5Y/n5byHnOo='; img-src 'self' data:; \
             connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'"
        );
    }

    #[test]
    fn each_directive_receives_only_its_own_hashes() {
        let policy = page_policy(COMBINED);
        let script_src = directive(policy, "script-src");
        let style_src = directive(policy, "style-src");
        assert!(script_src.contains(HASH_BOOT_CALL));
        assert!(!script_src.contains(HASH_H1_RULE));
        assert!(style_src.contains(HASH_H1_RULE));
        assert!(!style_src.contains(HASH_BOOT_CALL));
    }

    #[test]
    fn the_policy_carries_no_forbidden_source() {
        for policy in [
            page_policy(TWO_SCRIPTS),
            page_policy(COMBINED),
            page_policy(NO_INLINE),
        ] {
            assert!(!policy.contains("unsafe-inline"));
            assert!(!policy.contains("unsafe-eval"));
            assert!(!policy.contains("unsafe-hashes"));
            assert!(!policy.contains("blob:"));
            assert!(!policy.contains("wasm-unsafe-eval"));
            assert!(!policy.contains('*'));
            assert_eq!(policy.matches("data:").count(), 1);
            assert!(policy.contains("img-src 'self' data:"));
        }
    }

    #[test]
    fn repeated_calls_for_one_page_reuse_the_cached_policy() {
        let first = page_policy(TWO_SCRIPTS);
        let second = page_policy(TWO_SCRIPTS);
        assert_eq!(first, second);
        // Same address, so the cached value is returned rather than a
        // recomputed copy.
        assert!(std::ptr::eq(first, second));
    }

    #[test]
    fn different_pages_never_share_a_cached_header() {
        let scripts = page_policy(TWO_SCRIPTS);
        let external = page_policy(NO_INLINE);
        assert_ne!(scripts, external);
        assert!(std::ptr::eq(page_policy(TWO_SCRIPTS), scripts));
        assert!(std::ptr::eq(page_policy(NO_INLINE), external));
        assert!(page_policy(TWO_SCRIPTS).contains(HASH_ALPHA));
        assert!(!page_policy(NO_INLINE).contains(HASH_ALPHA));
    }
}
