# Themes

The window has thirteen themes ([decision 0028](../decisions/0028-themes.md)).
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
| Rose | Light | A white ground with pink for what is chosen, focused or pressed, and a faint pink head. |
| Delft | Light | A white ground with the blue of glazed tiles for the same marks. |
| Poppy | Light | A white ground with red for the same marks; trouble keeps a darker, cooler red. |
| Blossom | Light | A warm ground with plum blossoms in faint rose line. |
| Wing | Dark | A night sky with swallows in flight. |
| Fern | Light | A pale sage ground with fronds of fern. |
| Tide | Dark | Deep sea water with rings of waves, laid row over row. |

## One colour and white

Rose, Delft and Poppy ([decision 0028](../decisions/0028-themes.md), note of
22 November 2026) are three light themes, each a white ground with one
colour. They claim nothing about the eyes beyond what any light palette
does (see "Light text is not best everywhere" above): ink of a very dark
shade of the colour, never black, on white.

- **Where the colour is.** In the focus ring and the marks of the gold of
  the other themes; in the line under the active tab; in a faint wash on the
  header of a project and a line under it; in the hover and the pressed
  state, so that the chosen place in the sidebar and the chosen button of a
  pair are tinted; and in the faint tint of the sidebar. Everything else is
  white or ink.
- **Where it is not.** Not on the ground, not on the cards, and not on the
  tints of the statuses, which are the same eight in the three themes. A
  status is still told by its mark and its word as well as its colour, and no
  tint equals the colour of the theme.
- **Red is not trouble.** Poppy's red is a warm vermilion. Trouble (a date
  gone by, a limit passed, a refusal) is a cooler, darker crimson, with a
  word beside it as everywhere. The hover in Poppy is a neutral wash, not red,
  so that rows and the weekends of the Gantt chart do not look flagged. In
  Rose and Delft trouble is the red it is in the other light themes.

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
| Rose | 16.7 / 17.2 | 8.9 | 5.8 | 6.7 |
| Delft | 15.5 / 15.9 | 7.6 | 5.3 | 7.0 |
| Poppy | 16.4 / 16.7 | 9.3 | 5.9 | 6.6 |
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
| Rose | 3.3 / 6.8 | 4.6 / 6.3 | 5.0 / 7.0 | 4.0 / 6.4 | 3.9 / 5.9 | 4.2 / 6.2 | 4.4 / 6.4 | 4.7 / 6.7 |
| Delft | 3.3 / 6.8 | 4.6 / 6.3 | 5.1 / 7.0 | 4.0 / 6.4 | 3.9 / 5.9 | 4.2 / 6.2 | 4.4 / 6.4 | 4.7 / 6.7 |
| Poppy | 3.4 / 6.8 | 4.6 / 6.3 | 5.1 / 7.0 | 4.0 / 6.4 | 3.9 / 5.9 | 4.2 / 6.2 | 4.5 / 6.4 | 4.7 / 6.7 |
| Blossom | 3.1 / 6.9 | 4.2 / 6.5 | 4.7 / 7.1 | 3.0 / 6.1 | 3.7 / 5.9 | 3.9 / 6.1 | 3.8 / 6.0 | 4.3 / 6.5 |
| Wing | 5.3 / 8.0 | 6.1 / 8.4 | 6.0 / 8.4 | 7.3 / 8.9 | 6.8 / 8.8 | 6.7 / 8.9 | 6.0 / 8.4 | 5.3 / 7.4 |
| Fern | 3.0 / 6.5 | 4.4 / 6.5 | 4.8 / 7.2 | 3.1 / 6.2 | 3.7 / 6.0 | 4.0 / 6.3 | 4.1 / 6.3 | 4.2 / 6.4 |
| Tide | 5.6 / 8.5 | 6.2 / 8.5 | 6.0 / 8.4 | 7.7 / 9.3 | 7.2 / 9.1 | 7.3 / 9.4 | 6.2 / 8.6 | 5.6 / 7.6 |

For the three themes of one colour and white the tests also measure the
colour itself and the red of trouble:

| Theme | Colour as a mark on the ground | Colour as a word | Trouble as a word | Words on the head wash |
|---|---|---|---|---|
| Rose | 4.6 | 6.7 | 6.5 | at least 4.5 |
| Delft | 4.8 | 7.0 | 6.5 | at least 4.5 |
| Poppy | 4.7 | 6.6 | 8.8 | at least 4.5 |

Every word (ink, soft ink, faint ink, the colour as a word and trouble) is
held to 4.5 to 1 on the ground, the surfaces, the faint tinted sidebar
ground, and on the head, hover and pressed washes laid over the ground. The
colour as a mark is held to 3 to 1 on each of those grounds. The red of
trouble is a red between 330 and 20 degrees of hue and at least 20 degrees
from the colour of the theme; in Poppy it is also at least 1.2 times darker
in luminance than the colour as a word.

In Even the tints, darkest to lightest, are blue, red, purple, green,
gray, teal, pink and amber, each at least 1.2 times as light as the one
before.

## What was not measured

The ratios are those of the colours as written. A screen, its brightness
and the light of the room change what an eye receives, and none of that
was measured. No palette was tried with readers.
