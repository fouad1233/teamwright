# 0028. Ten themes as sets of the design tokens, chosen per computer, crossed into smoothly

- Status: accepted
- Date: 2026-11-18

## Context

The owner asked for a choice of theme in the settings: palettes that are
good for the eyes and for long work, and "special" palettes with patterns
such as flowers or birds, with the look the program has now as the
default, and a change from one theme to another that is very smooth.

The interface already drew everything from design tokens (decision 0015):
a component never names a colour. A theme can therefore be nothing more
than another set of values for the same names.

## Decision

1. **Ten themes, one of them the default.** Lamplight is the look of
   November 2026, in `tokens.css`, unchanged; it alone follows the light or
   dark setting of the system. Five plain palettes (Paper, Dusk, Slate,
   Clear, Even) and four special palettes (Blossom, Wing, Fern, Tide) are
   one fixed mode each, in `themes.css`, under `data-theme` on the root.
   Every theme sets every token of colour, so nothing of Lamplight shows
   through, and a sheet marked `data-paper` (the report) stays paper.
2. **Honest about what a palette does.** No palette claims to make anybody
   productive. Each says its mode and what it does, and
   [the themes](../design/themes.md) give the grounds. The contrast of
   every pair that carries meaning is measured from the style sheet itself
   by a pure function with tests, so the document and the program cannot
   drift apart.
3. **Patterns drawn in code, behind the content.** The patterns of the
   special palettes are SVG in a token, drawn by
   `tools/design/theme-patterns.mjs`: line drawings in the manner of a
   block print, in a tile whose motifs are drawn again across its edges so
   the repeat has no seam. Nothing is fetched and there are no picture
   files. A pattern lies on the ground of the window at a very low
   contrast, a little stronger above the foot of the sidebar and in a band
   under the words of an empty state, never under a line of text that is
   dense.
4. **A fact about one computer.** The theme is kept like the choice about
   notifications (decision 0026): in `meta` under the key `theme`, written
   without the recorder, so never logged and never synced. Two computers of
   one manager may well sit in rooms with different light. A name the
   program does not know reads as Lamplight. The window also remembers its
   theme in `localStorage` (`teamwright.theme`), and a small script in
   `public/` sets it before anything is drawn, so the window does not open
   in Lamplight and then turn; the backend's answer then decides.
5. **Chosen on the page Computers.** The settings of this computer are
   there. The chooser is one group of radio buttons; each theme is a
   swatch drawn inside `data-theme-preview`, so it shows the theme's own
   tokens whatever theme the window has.
6. **A smooth crossing.** The whole window crosses over in 400 ms on a
   curve eased at both ends: a view transition where the web view has one
   (WebView2 does), an eased transition of every colour where it does not,
   and at once with motion turned down. This is longer than the 300 ms that
   decision 0015 allows a moving part, on purpose: the whole window changes
   at once, nothing moves, and a quicker cross-fade of a full screen reads
   as a flash.

## Consequences

- A new token of colour must be given a value in every theme, or a test
  fails. A component still never names a colour.
- What draws with its own colours (the office and the portraits, on
  canvas) listens for the window's look through `subscribeToWindowLook`,
  not for the system's setting.
- The patterns cost about 95 KB of style sheet, most of it the fern. They
  are text and compress well; nothing is loaded from elsewhere.
- Lamplight's two modes keep following the system. A manager who wants
  Lamplight's night in a light room has Dusk or Slate.

## Note of 22 November 2026: three themes of one colour and white

The founder asked for three more: "pink and white, blue and white, red and
white". They are Rose, Delft and Poppy, and they make thirteen themes. The
decision above stands; this is what was added and why.

1. **Light, and only light.** A theme of "colour and white" has a white
   ground, so each is one mode, light, in `themes.css` like the others. The
   ground is white or a breath of the colour (never above a few per cent),
   the surfaces are pure white, and no pattern is drawn. They are a third
   kind in the chooser, `colour`, with a group of their own, "One colour and
   white", between the plain and the special palettes.
2. **The colour carries the marks, not the ground.** The colour is the
   gold of the other themes (`--brass`: focus ring, marks, the count of what
   waits), and also three new tokens in `tokens.css`, left bare in
   Lamplight and the plain and special palettes: `--head-wash` and `--head-edge` give the header of a project
   a wash of the colour and a line under it, and `--tab-line` makes the line
   under the active tab the colour instead of ink. The hover and the pressed
   state are washes of the colour (a neutral one in Poppy, see 4), so the
   chosen item of the sidebar and the chosen switch of a pair are tinted. The
   sunken ground, which is the sidebar, is a faint tint of the colour. The main button stays ink, which is a very
   dark shade of the colour's own family; it needs no colour to be seen.
3. **Statuses do not depend on the theme's colour.** The eight status tints
   are the same in all three, as in Paper, with the same ratios, so a blue
   status in Delft is still told from the blue of the theme by its place and
   its mark (a status is a mark and a word, never a colour alone). A test
   checks that no tint equals the theme's colour.
4. **Trouble in the red theme.** The interface uses red for late dates,
   limits that are exceeded and refusals. In Poppy the theme's red is a warm
   vermilion and used only for marks, lines and washes; trouble is a darker,
   cooler crimson (a hue at least 20 degrees apart and at least 1.2 times
   darker in luminance, so the two part in grey as well), and it was already a word or
   a mark with a word, never a colour alone. The hover is a neutral wash in
   Poppy, so that the weekends of the Gantt chart and the rows under the
   pointer do not turn pink-red. In all three, trouble is a red at least 20
   degrees of hue from the colour of the theme; a test checks it.
5. **Names.** Rose, Delft (the blue of the glazed tiles of that town) and
   Poppy: one plain word each, like the others.

The ratios are in [the themes](../design/themes.md). Looked at in a browser
on the demo (the board, the dashboard, the Gantt chart, the office and the
page Computers in all three; a task panel and a dialog in Delft and Poppy). Not
tried in the installed program.

