# Themes

The window has ten themes ([decision 0028](../decisions/0028-themes.md)).
Each is a set of values for the design tokens; no component knows which
one is shown. They are chosen on the page Computers and kept on that
computer only.

This page says what each palette is for and on what ground. It makes no
claim that a colour makes anybody work better or faster: no study shows
that. What the studies do show is narrower, about reading, glare, sleep
and colour vision, and that is what the palettes are built on.

## The grounds

- **Contrast.** The Web Content Accessibility Guidelines (WCAG 2) of the
  W3C measure the contrast of two colours as a ratio from 1 to 21. Body
  text needs 4.5 to 1 for the level AA and 7 to 1 for the level AAA; a
  mark that is not text, such as a status dot or a focus ring, needs 3 to
  1. Every theme meets AA for every word on its ground and its surfaces,
  and 3 to 1 for every mark. Clear meets AAA for every word.
- **Not white on black.** Many readers have some astigmatism. For them a
  bright letter on a black ground spreads into a glow (halation), so
  light text on a dark ground is harder to read than the contrast ratio
  says. The dark themes therefore never set pure white on pure black, and
  none goes past 15 to 1 between ink and ground.
- **Light text is not best everywhere.** Studies of reading on screens
  (the "positive polarity" findings of Buchner and Baumgartner, and of
  Piepenbrock and others) found dark text on a light ground read more
  accurately than the reverse, in light and dim rooms alike, and most of
  all for small text. A light ground also matches a bright room, so the
  eye need not adapt between screen and surroundings. Paper and Clear are
  light for those reasons.
- **Evening light.** Light of short wavelength, blue, in the evening holds
  back melatonin and delays sleep (the work of Chang, Czeisler and others
  on reading from light-emitting screens). A dark palette for the evening
  is therefore warm and dim: little blue in its large fields, and ink that
  is not bright. A screen palette is only a small part of the light in a
  room, so Dusk helps a little, not much.
- **Colour is never alone.** About one man in twelve sees colour
  differently. WCAG asks that colour never be the only carrier of meaning,
  and Teamwright already puts a mark or a word beside every status colour.
  Even goes further: its tints are the hues of Okabe and Ito's palette for
  colour-blind readers, set on a ladder of lightness so that any two
  differ by at least 1.2 to 1 in lightness alone, and part even in grey.

## The palettes

| Theme | Mode | What it does |
|---|---|---|
| Lamplight | Follows the system | The default: night or paper, warm neutrals and one gold. Unchanged. |
| Paper | Light | A warm off-white ground with ink of dark umber, not black: the sharpest reading in a bright room, without the glare of pure white. |
| Dusk | Dark | Warm and dim, with less blue than red in every large field and ink dimmer than Lamplight's, for the evening. |
| Slate | Dark | A neutral grey ground, never black, and contrast held near 9 to 1 rather than 15, for long sessions in a dim room. |
| Clear | Light | The highest contrast: every word, in ink or in a tint, at 7 to 1 or more (AAA). |
| Even | Dark | Status tints told apart without colour vision, by lightness as well as hue. |
| Blossom | Light | A warm ground with plum blossoms in faint rose line. |
| Wing | Dark | A night sky with swallows in flight. |
| Fern | Light | A pale sage ground with fronds of fern. |
| Tide | Dark | Deep sea water with rings of waves, laid row over row. |

The special palettes are palettes first: each meets the same contrast as
the plain ones. Their patterns are drawn by
[`tools/design/theme-patterns.mjs`](../../tools/design/theme-patterns.mjs)
as line drawings in the manner of a block print, and lie on the ground of
the window at a contrast of a few hundredths, stronger only above the foot
of the sidebar and in a band under an empty state.

## The ratios

Measured by `apps/desktop/src/shared/theme/contrast.ts` from
`themes.css`; `themeContrast.test.ts` fails if any falls below what is
promised above. A wash or a rule is laid over its ground before it is
measured.

Ink on the ground and on a surface, and the other words on a surface:

| Theme | Ink | Soft ink | Faint ink | Gold words |
|---|---|---|---|---|
| Paper | 12.7 / 14.1 | 8.3 | 5.6 | 7.1 |
| Dusk | 9.8 / 9.1 | 6.8 | 5.1 | 7.5 |
| Slate | 9.1 / 8.0 | 6.2 | 5.3 | 6.4 |
| Clear | 18.1 / 19.4 | 13.5 | 8.7 | 9.5 |
| Even | 14.4 / 13.1 | 8.4 | 5.8 | 8.7 |
| Blossom | 13.4 / 14.6 | 8.5 | 5.6 | 7.8 |
| Wing | 13.2 / 12.1 | 7.7 | 5.3 | 8.0 |
| Fern | 12.9 / 14.2 | 8.1 | 5.4 | 7.0 |
| Tide | 13.9 / 12.6 | 8.3 | 6.0 | 8.0 |

Each status tint as a mark on the ground, and as a word on a surface:

| Theme | Gray | Blue | Purple | Amber | Green | Teal | Pink | Red |
|---|---|---|---|---|---|---|---|---|
| Paper | 3.2 / 6.7 | 4.0 / 6.0 | 4.4 / 6.6 | 3.5 / 6.1 | 3.4 / 5.5 | 3.7 / 5.9 | 3.9 / 6.0 | 4.1 / 6.3 |
| Dusk | 5.0 / 7.6 | 5.4 / 8.1 | 5.7 / 8.5 | 7.0 / 8.7 | 6.8 / 9.0 | 6.2 / 8.7 | 5.8 / 8.1 | 5.1 / 7.4 |
| Slate | 4.7 / 6.6 | 5.3 / 6.9 | 5.1 / 6.8 | 6.2 / 7.3 | 6.0 / 7.4 | 5.9 / 7.5 | 5.1 / 6.8 | 4.5 / 6.1 |
| Clear | 4.9 / 9.5 | 5.4 / 9.0 | 6.2 / 9.8 | 4.4 / 8.6 | 5.0 / 8.3 | 5.4 / 8.8 | 5.6 / 8.9 | 5.6 / 9.0 |
| Even | 7.4 / 10.2 | 3.4 / 7.8 | 5.1 / 9.4 | 13.5 / 13.3 | 6.1 / 9.4 | 9.0 / 10.9 | 10.9 / 12.1 | 4.2 / 8.7 |
| Blossom | 3.1 / 6.9 | 4.2 / 6.5 | 4.7 / 7.1 | 3.0 / 6.1 | 3.7 / 5.9 | 3.9 / 6.1 | 3.8 / 6.0 | 4.3 / 6.5 |
| Wing | 5.3 / 8.0 | 6.1 / 8.4 | 6.0 / 8.4 | 7.3 / 8.9 | 6.8 / 8.8 | 6.7 / 8.9 | 6.0 / 8.4 | 5.3 / 7.4 |
| Fern | 3.0 / 6.5 | 4.4 / 6.5 | 4.8 / 7.2 | 3.1 / 6.2 | 3.7 / 6.0 | 4.0 / 6.3 | 4.1 / 6.3 | 4.2 / 6.4 |
| Tide | 5.6 / 8.5 | 6.2 / 8.5 | 6.0 / 8.4 | 7.7 / 9.3 | 7.2 / 9.1 | 7.3 / 9.4 | 6.2 / 8.6 | 5.6 / 7.6 |

In Even the tints, darkest to lightest, are blue, red, purple, green,
gray, teal, pink and amber, each at least 1.2 times as light as the one
before.

## What was not measured

The ratios are those of the colours as written. A screen, its brightness
and the light of the room change what an eye receives, and none of that
was measured. No palette was tried with readers.
