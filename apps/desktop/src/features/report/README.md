# Report

A kind of view that sets one page about the project in the middle of the
view, like a printed sheet, to hand to somebody. It is a page the view keeps
(group `pages`), found by the table of kinds through `kind.tsx`.

- **Nothing is worked out twice.** Every figure and sentence on the page is
  the dashboard's (`features/dashboard/dashboardModel.ts`): the counts, the
  sentence of where the project stands, this week, the tasks at risk, the
  work by holder, the usage table, the running sprint with its burndown
  (`features/sprints/BurndownChart`). `reportModel.ts` only arranges them:
  which parts show, the shares of the one bar, the tasks in work (the
  dashboard's `isInWork`), what comes next (this week's due tasks that are
  not late), and lists cut at eight lines.
- **What it keeps.** JSON in the view's content: the manager's introduction
  and the parts left out. Content that is missing or damaged reads as a new
  report; a part it does not know is dropped, and a part added later shows.
  A new report leaves out only what the team used. Changes are written about
  a second after the last one, and at once when the view is left
  (`useReportSettings`).
- **Paper in both themes.** The sheet carries `data-paper`, which gives it
  the paper tokens (`shared/styles/tokens.css`): at night the ground stays
  dark and the page stays paper. Ink and hairlines only; the gold is used for
  the mark over the title and the today line of the burndown.
- **Printing.** "Print or save as PDF" calls the print of the window. A copy
  of the sheet, read-only, lives in a portal straight in the body; the print
  style sheet hides everything else in the document and the copy keeps the
  printer's margins, with no shadow or fixed size. A part and a row of a
  table are not cut across pages where that can be avoided. Nothing is sent
  anywhere and no link is made.
