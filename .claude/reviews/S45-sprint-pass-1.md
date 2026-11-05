# S45 review, pass 1

**Reviewed**: sprint/s45 at d242505, tree deb89355c849e659b5ff5afe515605ddc6a2fecb, diff from main 3620e94
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 1 smell, 2 nitpicks

## Acceptance challenged

- **The page does not store preset numbers.** The three buttons carry only `data-preset="soft_tissue"`, `"lung"` and `"bone"` (`examples/folder-viewer/src/page.html:375-377`). `requestGrey` sends `X-Ocelli-Window-Preset` only in the `else` branch after the numeric override (`page.html:1150-1156`). A preset therefore never travels with a numeric window. The page writes the centre and width it shows from the reply headers (`page.html:1172-1173`). A W/L drag that starts from a preset also starts from those reply values (`page.html:1530-1532`). Ending that drag clears `activePreset` (`page.html:1628`). The server refuses a preset combined with a centre, a width or a function (`preview.rs:214-220`).
- **One LUT.** The preset path supplies `preset.center()`, `preset.width()` and `VoiFunction::Linear` to the same `VoiTransform::new` and `LutChain::new` that the file window uses (`preview.rs:552-589`). The only copy of the numbers is `WindowPreset` (`crates/ocelli-viewport/src/presentation.rs:32-51`).
- **Preset refusal.** A Modality LUT Sequence is refused before either branch (`preview.rs:539-546`). A VOI LUT Sequence is refused inside the preset branch (`preview.rs:555-557`). The numeric override still replaces a VOI LUT Sequence, as F-206 scoped it.
- **Hand-computed grey.** I recomputed every value in `preset_grey_matches_ps3_3_c_11` from PS3.3 C.11.2.1.2:
  - Lung, c' = -600.5 and w' = 1499: air 59.54 gives 60, body 236.46 gives 236, lung 93.56 gives 94, bone is above 149 so 255.
  - Bone, c' = 299.5: body 83.36 gives 83, bone 195.63 gives 196, air and lung are at or below -450 so 0.
  - Soft tissue: body 127.82 gives 128.
  
  All of them match `tests/preview.rs:977-995`.
- **Spacing order.** `lengthText` multiplies `dCol` by `facts.column` and `dRow` by `facts.row` (`page.html:1952-1956`). `facts.row` is read from `X-Ocelli-Row-Spacing`, which `serve.rs:256-259` fills from the first DS value. Per PS3.3 C.7.6.2.1.1 that first value is the adjacent row spacing, which is the vertical step. The pairing is correct.
- **Edge letters.** The right edge takes the letter of the row cosine and the bottom edge the letter of the column cosine (`preview.rs:1026-1028`). For `1\0\0\0\1\0` the letters are left R, right L, top A, bottom P. That agrees with PS3.3 C.7.6.1.1.1 and with AC7.
- **No patient name or id.** The test checks the bytes for (0010,0010) and (0010,0020) and checks inspect for empty `patientName` and `patientId` (`tests/preview.rs:871-898`). A grep of `synthetic.rs` for group 0x0010 finds only (0020,0010), (0028,0010), (7FE0,0010) and (0002,0010).
- **Marks.** The sentence at `page.html:414` says the marks are not DICOM SR, are not written to the file, and are gone on reload. Marks live in the `notes` map, and nothing in `requestGrey` posts them.
- **Wheel.** A horizontal swipe calls `preventDefault` and does nothing else. Ctrl calls `preventDefault` before checking whether an image is loaded. A wheel without Ctrl steps at most once per event, after 40 px of travel, and resets on a direction change or after 150 ms (`page.html:1440-1475`).

## Evidence

Everything below comes from reading the files. I ran no commands. The files read were the ones in the brief, plus:

- `docs/sprints/BACKLOG.md` rows for F-119 and F-202 to F-207
- `packages/core/src/dicomweb.ts`, `commands.ts` and `index.ts`
- `packages/react/src/OcelliViewport.tsx`
- `synthetic.rs` and `tests/preview.rs`, by grep

I did not see the diff itself. The finding attributions below rest on the file contents and on the story ledgers.

## Defects

None.

## Smells

1. **The F-207 recovery claim does not match the page, and the as-built entry says nothing deviated.**
   - `.claude/plans/F-207-design.md:126` says "A failed preview leaves the previous grey image in place and shows the fault sentence."
   - On a cache miss, `showIndex` hides the stage before the fetch (`page.html:1055-1056`), and `showFault` hides it again (`page.html:1360-1361`). A refused preset therefore shows the fault sentence over an empty viewport, not the previous image.
   - `docs/sprints/AS_BUILT.md:5065` records "Deviations from the design plan. none."
   - The code's behaviour is the safer one. A stale grey image under an active Lung chip would be the quietly wrong picture. So the fix is the plan sentence or the as-built deviation line, not the code.
   - Failure scenario: open a file with a VOI LUT Sequence and press Lung. A reader who trusts the plan expects the earlier image to stay visible, and it does not.

## Nitpicks

1. **The README's list of fixed error cases does not name the preset refusal.** `README.md:219-221` lists a colour frame, a Modality LUT Sequence, and a VOI LUT Sequence without both a centre and a width. Since F-206, any preset on a VOI LUT Sequence file is also refused (`preview.rs:555-557`), and the README does not say so. This is an omission, not a false statement.
2. **The page's Length sentence leaves out the detector case.**
   - `page.html:467` names only millimetres with Pixel Spacing and pixels with no spacing.
   - The label prints "mm at the detector" for Imager Pixel Spacing (`page.html:1958`), and `README.md:208-210` documents that case.
   - The page text matches the approved plan's table word for word, so this is not a deviation. It is a gap against the sprint's "The README matches the page."

## Verified clean

- **F-119** is still S46, pending, and still the only story in the S46 table (`SPRINT_PLAN.md:670-676`, `BACKLOG.md:357`). Nothing replaced it.
- **M20 span.** It reads "S45 to S45, 2 stories, 13 engineer-weeks", and 5w plus 8w is 13. The S45 goal matches CURRENT_SPRINT and the M20 preamble.
- **F-202 to F-205** are still pending with no sprint, as CURRENT_SPRINT says.
- **Unchanged totals.** The Phase 1 and Phase 1.5 totals were not recalculated, as the brief said. I did not judge the generator preamble.
- **Grey cache.** `cacheKey` includes the preset token or the numeric window, plus the invert flag (`page.html:1089-1094`). Switching preset cannot serve a cached image from the old window.
- **Spacing headers.** These are sent only when a pair exists. Every token passes `json_number_token`, so `Number()` in `headerNumber` parses it. An empty source falls back to pixels.
- **Length labels.** `applyGrey` sets `shownFacts` before `drawGrey`, which calls `renderInk` (`page.html:1081-1082`, `1225-1228`). Length labels are not painted with the previous frame's spacing. The marks list shows no length text.
- **Static pages.** `/more` and `/use` are `include_str!` pages with no `<canvas>`. The routes are `serve.rs:118-123`.
- **Calls named in `/use`.** `viewportId`, `packCommands` and `DicomwebClient` are exported from `@ocelli/core`. The five `DicomwebClient` option names match `DicomwebClientOptions`. `OcelliViewport` takes `id`, `viewport` and an optional `session`.
- **`/synthetic/<n>`.** It parses a `u32` and returns 404 at 12 or above, or for a non-number.
- **Banned constructs.** There is no `unsafe` and no wasm-bindgen anywhere under `examples/folder-viewer`. `presentation.rs` contains no `unsafe`.
- **Ledgers.** CHANGELOG lines 25 and 26 and the two AS_BUILT "What was built" paragraphs match the code above.

## Limitations and handoff

- I could not see the diff, so I could not tell whether `README.md:219-221` predates S45. The nitpick stands either way, because F-206 added the case it leaves out.
- I did not check the tree hash against `git`.
- I did not run any tests or mutations. The intercept mutation is still recorded as not repeated after the later comment edits (`AS_BUILT.md:5041`).
- No one clicked through the live page. Gesture behaviour is confirmed only by reading the source.
- I did not reopen the F-206 FrameFacts nitpick or the three F-207 nitpicks. Nothing in this sprint makes any of them a defect.
- **Next step:** fix smell 1 by changing either the F-207 plan sentence or the as-built deviation line. It needs no code change.

COUNTS defects=0 smells=1 nitpicks=2
