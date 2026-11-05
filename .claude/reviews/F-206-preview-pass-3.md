# F-206 review, pass 3

**Reviewed**: working tree after the pass 2 comment fixes
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 0 smells, 1 nitpick

## Defects

None.

## Smells

None.

## Nitpicks

### N6

The new `FrameFacts` doc at `examples/folder-viewer/src/preview.rs:107` says "A token that is already a JSON number is kept." That is true only when the token is also finite. `clean_decimal` calls `parse_decimal_string` before it checks `json_number_token`, so `1e999` is a valid JSON number that fits in a 16-byte DS and is still refused, which leaves the pair `Unusable`. The word "finite" sits only on the second sentence, so a reader would assume the first branch has no such limit. The kept token is also the trimmed text and not the raw text. Suggested wording: "A finite token that is already a JSON number is kept after trimming. Any other finite DS is the parsed value. A non-finite token leaves the pair unusable." This does not block.

## Verified clean

- **N4 closed.** The doc no longer says the tokens are the file's decimal strings. It now matches `clean_decimal` for every finite input: `00.5` becomes `0.5`, `-.5` becomes `-0.5`, and `5E-1` is kept. Rust's `Display` for `f64` never writes an exponent, so the parsed branch always passes `json_number_token`.
- **N5 closed.** The comment at `tests/preview.rs:1199` now sits directly above the bad-token request on line 1201, and the phrase "refused from the header" is gone. The comment is true. `parse_header` rejects `soft` through `window_preset_from_token` at `serve.rs:463`, and that happens on line 315, before the body loop on lines 341 to 359. A non-empty body would therefore stay unread on the socket. The preset-plus-centre request at line 1151 sends a full body safely, because that combination is refused later in `preview_frame`, after the body has been read. So the comment's reasoning applies to this request only, which is what it now says.
- I re-opened no closed finding. Neither comment change made an earlier basis false.

COUNTS defects=0 smells=0 nitpicks=1
