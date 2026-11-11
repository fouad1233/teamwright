# 0008. Measure usage and budgets in tokens

- Status: accepted
- Date: 2026-11-11

## Context

A manager needs to see what each team member has used and to cap it. Most
users will run Teamwright on a Claude subscription, where there is no price
per request. Claude Code does report a dollar figure, but it is an estimate
computed locally from token counts at list prices. Tools that enforce budgets
in dollars have been seen to record zero for subscription users, so their
caps never trigger.

What Claude Code reports reliably after every turn is the token count per
model: input, output, cache read and cache write. For subscribers it can also
report how much of the five-hour and weekly plan limits is used.

## Decision

- Tokens are the unit for budgets, limits and dashboards.
- Budgets can be set per task, per team member and per project. The
  coordinator stops assigning work when a budget is exhausted and asks the
  manager.
- Claude Code's dollar estimate is shown next to the tokens and labeled as an
  estimate.
- Plan limits are shown when Claude Code reports them and are otherwise
  marked as unknown, never guessed.

## Consequences

- Budgets work the same on a subscription and with an API key.
- Token counts from different models are not equally expensive, so views
  break usage down by model.
- A budget stops new work. It cannot interrupt a model call that is already
  running, so a turn can overshoot by its own size.

## Alternatives considered

- **Dollar budgets.** Meaningless on a subscription.
- **No budgets, only reporting.** Leaves the manager to notice a runaway
  task by watching.
