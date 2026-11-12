# 0019. Put people in the office, drawn as pixel characters

- Status: accepted
- Date: 2026-11-12

## Context

[Decision 0018](0018-the-office-as-a-plan-of-a-workshop.md) drew the
office as a workshop in plan: a room, a bench for everybody, a lamp lit
where work goes on. It left out characters that walk about and pixel art,
for two reasons: movement that does not mean work breaks
[decision 0015](0015-light-where-something-happens.md), and figures with
faces would be a second style next to Joinery.

The owner has seen the room and wants people in it: for each member a
character, such as a software engineer or a mechanical engineer, with a
default character for any role the program does not know; the manager
and the Advisor too. In the owner's words it should "give the feeling that
these are people working in offices, like a real game", and the people may
walk. The same owner threw away an earlier interface because it looked
machine-made, so the people must look drawn by a person, and the room
must stay calm in a program that is open all day.

## Decision

- **Keep the room, add the people.** The plan of the workshop, the
  benches, the lamps, the gold corners, the title block and every word on
  a bench stay as decision 0018 has them. The people are drawn on one
  canvas laid over the room. It catches no pointer and is hidden from
  screen readers; the words on the bench still say everything.
- **Original pixel characters, drawn in code.** Every figure is a grid of
  letters written by hand, 16 pixels by 24, and painted at a whole number
  of device pixels per pixel, so it is never blurred. No image files, no
  fonts, nothing traced. The colors are data in one module, warm and
  limited, tuned to night and paper; the shirt is the member's own
  pigment and the gold is the brass of the theme, read from the tokens.
  A line of umber holds each figure at night, a line of ink on paper.
- **A character for every kind of role.** The manager, the Advisor, a
  software engineer, a hardware engineer, a mechanical engineer, a
  designer, a test engineer, a technical writer, a researcher, and a
  default. A role is read from the words of the member's name, or else of
  its responsibility. What tells roles apart is what they wear and what
  stands on their bench, never a label: headphones and a laptop; a loupe
  and a soldering iron; an apron and a caliper; a beret and a drawing
  tablet; a lab coat and an instrument; a cardigan, a notebook and books;
  glasses, papers and a chart; the manager's gold jacket and a tray of
  letters; the Advisor's long coat, gray hair, glasses and the plan.
- **Two members of one role still differ, and stay the same.** Skin, hair
  and the way it is worn come from the member's identifier.
- **They do what the bench says.** At work, the figure sits bent over its
  work, in a loop of four frames, and its things move: smoke from the
  iron, light from the screen, a trace on the instrument. Waiting for the
  manager, it sits turned to the room with a hand raised, inside the gold
  corners. Free, it sits idle and glances aside now and then, and now and
  then gets up and walks to a shared place: the window, the table with
  the plan, or the coffee corner, in a band at the foot of the room. Its
  chair and its things stay on the bench. The manager and the Advisor do
  the same.
- **Aisles make the room walkable.** An aisle runs in front of every row
  of benches and down the right side of the room; the partition stops
  short of the right wall to let it through. People walk in right angles
  along the aisles, so nobody crosses a bench, a wall or a word. One
  person walks for the pleasure of it at a time, and nobody goes to a
  place another holds.
- **A portrait at the bench.** The head and shoulders of the character
  take the place of the initials on the bench, and next to the name in
  the Team view. Smaller places keep the initials.

## Why idle people may move

Decision 0015 keeps motion for where something happens. The owner asked
for a room that feels alive, and a room where only the working move reads
as empty. So the free may move, under three conditions that keep 0015's
purpose:

- it is slow and small: a walk at an unhurried pace, a glance, never more
  than one person on the way at once, nothing that flashes or bounces;
- it never carries the meaning: the lamp and the corners still say who
  works and who waits, and the figure at work and the figure that waits
  are drawn so in a still picture too;
- it stops when motion is turned down. With `prefers-reduced-motion`
  nobody walks and nothing loops: everybody sits at the bench in the
  still pose of what the bench says.

## Consequences

- The room is taller: every aisle is 80 pixels deep, and the band of
  shared places takes 112 more at the foot. A team of eight needs to
  scroll on a small screen sooner than before.
- One loop draws the whole room, about fifteen times a second, and paints
  nothing when nothing has moved. It stops while the window is hidden. It
  measures the room only when the room changes size, and then sends
  everybody back to their bench.
- On a screen scaled by a fraction, such as 125 percent, the figures are
  drawn a little smaller than designed rather than blurred.
- The characters are a second visual language after all. They are kept
  apart from Joinery's: they live in the office and in the Team view only,
  never on the board, and they are drawn in the palette of the program.

## Alternatives considered

- **Figures that only sit.** Calm, but the owner asked for people who walk
  and a room that feels like a game; a room of seated figures reads as a
  photograph.
- **Free walking anywhere in the room.** Lively, but people would cross
  words and each other. Aisles in right angles keep every word clear and
  suit a plan drawn on a grid.
- **Pictures drawn elsewhere and loaded as files.** Quicker to make look
  rich, but they would not follow the theme or the member's color, and
  they would not be the program's own.
- **Characters on the cards of the board.** Left out: at twenty pixels a
  face does not read, and the board is where the manager works, not where
  the team is watched.
