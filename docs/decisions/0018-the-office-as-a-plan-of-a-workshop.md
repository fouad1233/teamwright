# 0018. Show the team at work as a workshop drawn in plan

- Status: accepted
- Date: 2026-11-12

## Context

From the first day the owner asked for "a game-like office view": one
place where the manager sees, at a glance, who is working, on what, for how
long, and at what cost. The roadmap names it "The office: specialists as
characters with activity, time left and usage".

Everything the view needs is already known to the interface. The team says
who is there. The conversations say who is working at this moment, what the
running turn did last, when it started, and what every finished turn used.
An open decision, an open interview or an open proposal says who waits for
the manager. The tasks say what is given to whom and how big it is. The
Team view and the board already read these sources; the office reads the
same ones and asks the backend for nothing new.

"Game-like" invites characters that walk about, pixel art and cartoon
faces. That would be a second visual language next to Joinery, and it would
make the most watched screen of the program the least honest one.

## Decision

- **The office is a tab of a project, after Team.** It shows the place of
  every member of the team (AI specialists and people), the manager's place,
  and the Advisor's place.
- **It is a workshop drawn in plan, late in the evening.** A sheet with a
  margin, a room drawn in hairlines, a bench for everybody, and a lamp over
  each bench. The manager and the Advisor sit at the front of the room,
  where the plan is made; the team works on the floor. Every bench carries a
  reference on the grid of the drawing, such as B2, the way the parts of a
  plate are numbered. A title block in the corner sums up the room, and a
  legend says what the lamp and the corners mean.
- **The arrangement is worked out, not chosen.** The number of benches in a
  row follows from how many members there are and how much room the window
  gives: one member sits alone, four sit two by two, eight sit four by two.
  A shorter last row is centred. Nobody is dragged around.
- **A bench shows who it is and what it is doing now**, in words:
  - at work: the task (reference and title) or "a question of yours", the
    last thing done in the running turn, how long the turn has been going,
    counted live, and the size of the task if it has one;
  - waits for you: the decision it waits for and on which task; for the
    Advisor, answers to its questions or a word about its proposal;
  - free: that it is free, and how many of the tasks given to it have not
    been started;
  - a person: how many tasks given to them are not finished.

  And what it has used in this project so far, in tokens, written the way
  the rest of the program writes them.
- **The light is decision 0015 drawn as a room.** A lamp is lit over a
  bench where a specialist or the Advisor is working, and nowhere else. A
  bench that waits for the manager is held by the four gold corners of a
  waiting card and says "Waits for you". Every light stands next to words
  that say the same thing. With motion turned down nothing moves, and the
  room still reads.
- **No invented numbers.** The program does not know how long a task will
  still take, so the office shows no countdown and no bar that pretends to.
  The "time left" of the roadmap is answered with what is known: how long
  the work has taken so far, and the size the task was given. A turn's
  tokens are known only when it ends, so a running turn shows them only if
  they are known.
- **A bench leads on.** A specialist that works or waits opens its task, or
  its conversation if the work is a question of the manager's. A free
  specialist opens its conversation. The Advisor opens its page. The
  manager's place opens the inbox. Every bench that leads somewhere is a
  button and can be reached with the keyboard.
- **The arithmetic and the words live in `officeModel.ts`**, with tests.
  The components only draw.

## Consequences

- The office costs nothing to keep up: it follows the conversations the
  interface already receives, and redraws once a second only while somebody
  is working, so that the time counts.
- A team of more than eight still fits: the rows grow downwards.
- The office and the Team view say some things twice. The Team view is for
  changing the team; the office is for watching it.
- Plan limits, which the vision promises next to the office, are not shown,
  because the program does not know them yet.

## Alternatives considered

- **Characters that walk about.** Lively in a demonstration and tiring in a
  program that is open all day. Movement that does not mean work going on
  breaks decision 0015, and figures with faces would be a second style.
- **A free-form room the manager arranges.** Pleasant once, but it needs
  positions stored for every project and every computer, and a manager with
  eight members would spend time on furniture. It can come later on top of
  the worked-out arrangement.
- **A dashboard of numbers.** Honest and quick to build, but it is the
  Team view again with more columns. The owner asked to see the team at
  work, not to read a report about it.

## Left out of this version

- Moving benches, and grouping them by discipline.
- Plan limits and what remains of them.
- A history of the day: who worked when.
- Tokens of a turn while it runs, which the program does not report until
  the turn ends.
