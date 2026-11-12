# 0015. Let light fall only where something happens or waits for the manager

- Status: accepted
- Date: 2026-11-12

## Context

The board was flat and still. The design philosophy said that nothing
glows and nothing fades into anything else, and brass was the one colour
that meant "look here". Two states that matter most to the manager were
hard to find on a full board: a task a specialist is working on showed a
small brass block, and a task that waits for the manager's decision showed a
hollow square of seven pixels.

The owner asked for cards that are lit from one corner and fade "like a
sun", in a palette of four colours called Onyx and Gilded: obsidian
`#12100E`, burnished gold `#C9A96E`, smoky parchment `#E8E0D0` and charred
umber `#3D2F25`. Three directions were built as live pages and compared, in
both themes and with motion turned down:

- **A, Gilded sun.** Every card lit from a fixed sun above its top left
  corner.
- **B, Lamplight.** Cards stay flat. Gold appears only as light: under the
  pointer, travelling round the edge of a card a specialist works on, and
  as a small sun in the corner of that card.
- **C, Gilt joint.** No gradient. Fine joints at the corners of every card,
  which turn gold when the card means something.

The argument against A on the board: light on every card says nothing. Gold
spent on sixty cards stops meaning "look here", it sits in the same warm
range as the amber and red marks of blockers and priorities, and twelve
suns in a lane make stripes. The idea is good on one surface, not on all of
them. B keeps the meaning of gold and stays calm with many cards. C is the
clearest sign for a decision that waits, and the most Teamwright of the
three.

The owner chose Lamplight.

## Decision

- **Light is allowed where something is happening or is the manager's to
  do, and nowhere else.** This replaces "nothing glows" in the design
  philosophy.
- **The palette moves from graphite and brass to night and gilt.** The
  grounds, surfaces and inks take the Onyx and Gilded values. The names of
  the tokens stay. Gold that colours words gets a token of its own,
  `--brass-text`, because on paper the gold of a mark is too light to be
  read.
- **A card at rest is flat**, as before.
- **Under the pointer**, a soft light follows it over a card and over the
  primary button. It is a pleasure and carries no information: nothing may
  depend on it.
- **A card a specialist works on** has a point of light that travels round
  its edge and a small sun in its corner. The brass block next to the
  assignee stays, so a screenshot and reduced motion still say "at work".
- **A card that waits for the manager** is held by four gold corners, taken
  from direction C, and says "Waits for you" in gold.
- **The decision card** in the task panel and in the inbox carries the full
  gilded sun of direction A. It is one card at a time, and it is the one
  thing that is the manager's to do. It keeps its rule of brass.
- **Movement comes from a small set of named curves and durations.** Things
  arrive by slowing down and leave by speeding up. Leaving is quicker than
  arriving. Nothing bounces. Nothing takes longer than 300 ms except work
  that really goes on. Only position and opacity move; the one exception is
  the room a lane makes for a new card. With reduced motion, nothing moves
  across the screen: states change by a short fade, and nothing repeats.

## Consequences

- Every light stands next to a shape or a word that says the same thing, so
  the meaning survives a screenshot, reduced motion and colour blindness.
- The point of light on the edge repaints its card on every frame. It may
  be on the few cards that are being worked on, never on every card.
- It relies on `@property` and `mask-composite`, which WebView2 has. Where
  they are missing, the point of light stands still or the edge is a plain
  gold line. WebKit on macOS has not been checked.
- Text must reach 4.5 to 1 and marks 3 to 1 under the lights as well. That
  moved three colours a little: the faint ink, the red of danger, and the
  amber of text on paper.
- The pictures in the documentation show the old palette until they are
  taken again.

## Alternatives considered

- **A, a sun on every card.** Warm and striking in a screenshot. It spends
  the one colour that means "look here" on everything, fights the status
  colours, and turns a full board into wallpaper.
- **C alone.** The quietest and the cheapest to draw. The owner wanted
  light, and C wins only on a second look.
- **Glass and blur, drifting gradients, springs that overshoot.** In
  fashion, expensive to draw behind many cards, and tiring in a program that
  is open all day.
