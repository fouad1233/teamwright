# Chat

A kind of view that shows the manager's conversations with the team that are
not work on a task, as one talk in the order of time, with a field at its
foot to write to one of them. It is a page (group `pages`), found by the
table of kinds through `kind.tsx`. It keeps nothing of its own and cannot be
filtered: everything it shows is the conversations of the project.

- **What it shows.** A conversation belongs to a specialist and is about a
  task or about nothing in particular, or it is with the Advisor, which has
  neither. The chat shows every conversation without a task: the questions
  the manager asked a specialist (the same ones the conversation panel of the
  team shows) and the talk with the Advisor. Work on a task stays in the
  panel of that task.
- **One talk.** `runsOfTalk` lays every turn out in the order it started and
  cuts the line into runs wherever the conversation changes, so two
  conversations that went on at the same time alternate as they did. A quiet
  rule with the name starts each run, says "New conversation" where the
  session begins afresh, and holds "Working" and "Stop" while a turn runs.
  The turns are drawn by the program's own `TurnView`; a question a
  specialist asked is answered under it with the program's `DecisionCard`.
  What the Advisor asked or proposed is named in a line that leads to the
  Advisor, where there is room for it.
- **The field.** Whom the message is for (the AI specialists, and the Advisor
  once a plan was started with it, since a plan begins with a brief in the
  Advisor itself), the text, and "Send" (or Ctrl and Enter). `howToSend`
  applies the rules the program has: a message continues the latest
  conversation with that specialist on this computer, or starts one; nothing
  goes out while the one written to is at work (a specialist works on one
  thing at a time, a task or a question), while a specialist waits for a
  decision, while the Advisor waits for answers to its questions, or when the
  conversation with the Advisor runs on another computer. The line under the
  field says which of these it is. The context meter of the conversation a
  message would continue stands beside the chooser.
