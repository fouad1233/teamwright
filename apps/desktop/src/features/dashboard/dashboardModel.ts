import type {
  CalendarDate,
  Color,
  Conversation,
  Member,
  Project,
  Sprint,
  SprintReport,
  Status,
  StatusCategory,
  Task,
} from '@/shared/api/types';
import { calendarDateOfDay, dayNumberOf, mondayOf } from '@/shared/lib/days';
import { calendarDateOf } from '@/shared/lib/format';

import { advisorConversation } from '../advisor/advisorModel';
import { countWaitingForManager } from '../office/officeModel';
import { daysLeft } from '../sprints/sprintModel';
import { allTokens, runningTurn, tokensOfTurns } from '../work/workModel';

/**
 * Pure functions for the dashboard of a project (decision 0020): what it
 * counts, and the sentences that say where the project stands. They are
 * worked out by rules from what the interface already knows. No model is
 * asked and no token is spent, so the page can be opened as often as the
 * manager likes.
 *
 * Every rule follows the category of a status, never its name: managers
 * name their statuses freely.
 */

/** The category of the status a task is in, or nothing for a status that is gone. */
function categoryOf(project: Project, task: Task): StatusCategory | undefined {
  return project.statuses.find((status) => status.id === task.statusId)?.category;
}

/** Whether a task is finished: in a status of the category done. */
export function isFinished(project: Project, task: Task): boolean {
  return categoryOf(project, task) === 'done';
}

/** Whether a task is late: not finished, and due before today. */
export function isLate(project: Project, task: Task, today: CalendarDate): boolean {
  // Dates in the form year-month-day compare correctly as text.
  return !isFinished(project, task) && task.dueOn !== null && task.dueOn < today;
}

/** The plain numbers of the dashboard. */
export interface DashboardCounts {
  /** Every task of the project, subtasks included. */
  total: number;
  /** The tasks that are not finished. */
  open: number;
  /** The tasks that are begun and not finished: in a status that is active or in review. */
  inWork: number;
  finished: number;
  late: number;
  /** The tasks that are not finished and are given to nobody. */
  givenToNobody: number;
  /** The things that wait for the manager in this project. */
  waiting: number;
}

export function countsOf(
  project: Project,
  tasks: readonly Task[],
  conversations: readonly Conversation[],
  today: CalendarDate,
): DashboardCounts {
  const categories = tasks.map((task) => categoryOf(project, task));
  const finished = categories.filter((category) => category === 'done').length;
  return {
    total: tasks.length,
    open: tasks.length - finished,
    inWork: categories.filter((category) => category === 'active' || category === 'review').length,
    finished,
    late: tasks.filter((task) => isLate(project, task, today)).length,
    givenToNobody: tasks.filter((task) => task.assignee === null && !isFinished(project, task))
      .length,
    waiting: countWaitingForManager(conversations),
  };
}

/** The tasks at risk: not finished and past their due date, the one late longest first. */
export function tasksAtRisk(project: Project, tasks: readonly Task[], today: CalendarDate): Task[] {
  return tasks
    .filter((task) => isLate(project, task, today))
    .sort((first, second) => byDueDate(first, second) || first.number - second.number);
}

function byDueDate(first: Task, second: Task): number {
  return (first.dueOn ?? '').localeCompare(second.dueOn ?? '');
}

/** How many days a task is late: from its due date to today. */
export function daysLate(task: Pick<Task, 'dueOn'>, today: CalendarDate): number {
  return task.dueOn === null ? 0 : Math.max(0, dayNumberOf(today) - dayNumberOf(task.dueOn));
}

/** "1 day late", "9 days late". */
export function lateInWords(days: number): string {
  return `${count(days, 'day', 'days')} late`;
}

/** One status of the workflow with its tasks counted. */
export interface StatusShare {
  status: Status;
  count: number;
}

/** The tasks of every status, in the order of the workflow. Statuses without tasks are kept. */
export function workByStatus(project: Project, tasks: readonly Task[]): StatusShare[] {
  return project.statuses.map((status) => ({
    status,
    count: tasks.filter((task) => task.statusId === status.id).length,
  }));
}

/** Somebody tasks can be given to, with what is given to them. */
export interface Holder {
  key: string;
  kind: 'manager' | 'member' | 'nobody';
  member: Member | null;
  name: string;
  open: number;
  finished: number;
}

/**
 * The work of everybody who can be given a task: the manager first, then
 * every member in the order they joined, then the tasks given to nobody.
 * The line for nobody is left out when nothing is given to nobody.
 */
export function workByHolder(
  project: Project,
  tasks: readonly Task[],
  members: readonly Member[],
): Holder[] {
  const tally = (given: readonly Task[]) => {
    const finished = given.filter((task) => isFinished(project, task)).length;
    return { open: given.length - finished, finished };
  };
  const manager: Holder = {
    key: 'manager',
    kind: 'manager',
    member: null,
    name: 'You',
    ...tally(tasks.filter((task) => task.assignee?.kind === 'manager')),
  };
  const team = members.map((member): Holder => ({
    key: member.id,
    kind: 'member',
    member,
    name: member.name,
    ...tally(
      tasks.filter(
        (task) => task.assignee?.kind === 'member' && task.assignee.memberId === member.id,
      ),
    ),
  }));
  const nobody: Holder = {
    key: 'nobody',
    kind: 'nobody',
    member: null,
    name: 'Nobody',
    ...tally(tasks.filter((task) => task.assignee === null)),
  };
  return [manager, ...team, ...(nobody.open + nobody.finished > 0 ? [nobody] : [])];
}

/** The largest number of tasks any one holder has, so that every bar shares one scale. */
export function largestHolding(holders: readonly Holder[]): number {
  return Math.max(1, ...holders.map((holder) => holder.open + holder.finished));
}

/** The days of the week today is in: from its Monday to its Sunday. */
export function weekOf(today: CalendarDate): { first: CalendarDate; last: CalendarDate } {
  const monday = mondayOf(dayNumberOf(today));
  return { first: calendarDateOfDay(monday), last: calendarDateOfDay(monday + 6) };
}

/** What happens to the tasks this week. */
export interface Week {
  /** Finished since Monday, the one finished last first. */
  finished: Task[];
  /** Not finished, and due by Sunday or already late, the one due first first. */
  due: Task[];
}

/**
 * The tasks of this week. A task counts as finished this week if it is in
 * a status of the category done and was finished on a day of this week,
 * where the user is.
 */
export function thisWeek(project: Project, tasks: readonly Task[], today: CalendarDate): Week {
  const { first, last } = weekOf(today);
  const finishedOn = (task: Task) =>
    task.completedAt === null ? null : calendarDateOf(new Date(task.completedAt));
  const finished = tasks
    .filter((task) => {
      const day = finishedOn(task);
      return isFinished(project, task) && day !== null && day >= first && day <= today;
    })
    .sort((a, b) => (b.completedAt ?? '').localeCompare(a.completedAt ?? ''));
  const due = tasks
    .filter((task) => !isFinished(project, task) && task.dueOn !== null && task.dueOn <= last)
    .sort((a, b) => byDueDate(a, b) || a.number - b.number);
  return { finished, due };
}

/** What a specialist, the Advisor or somebody who left has used, in tokens. */
export interface Usage {
  key: string;
  kind: 'advisor' | 'member' | 'former';
  member: Member | null;
  name: string;
  tokens: number;
}

/** What the project used, and who used it. */
export interface ProjectUsage {
  total: number;
  /** The Advisor, then every AI specialist in the order they joined, then those who left. */
  users: Usage[];
}

/**
 * Who used how many tokens in the project. A person uses none and is not
 * listed. Conversations whose specialist has left the team are counted
 * together, and only if they used anything.
 */
export function usageOf(
  members: readonly Member[],
  conversations: readonly Conversation[],
): ProjectUsage {
  const tokensOf = (some: readonly Conversation[]) =>
    allTokens(tokensOfTurns(some.flatMap((conversation) => conversation.turns)));
  const known = new Set(members.map((member) => member.id));
  const advisor: Usage = {
    key: 'advisor',
    kind: 'advisor',
    member: null,
    name: 'Advisor',
    tokens: tokensOf(conversations.filter((conversation) => conversation.isWithAdvisor)),
  };
  const specialists = members
    .filter((member) => member.kind === 'agent')
    .map((member): Usage => ({
      key: member.id,
      kind: 'member',
      member,
      name: member.name,
      tokens: tokensOf(conversations.filter((conversation) => conversation.memberId === member.id)),
    }));
  const formerTokens = tokensOf(
    conversations.filter(
      (conversation) =>
        !conversation.isWithAdvisor &&
        (conversation.memberId === null || !known.has(conversation.memberId)),
    ),
  );
  const former: Usage[] =
    formerTokens > 0
      ? [{ key: 'former', kind: 'former', member: null, name: 'Who left', tokens: formerTokens }]
      : [];
  return { total: tokensOf(conversations), users: [advisor, ...specialists, ...former] };
}

/** Who is at work now: the specialists with a running turn, and the Advisor. */
export function whoIsAtWork(
  members: readonly Member[],
  conversations: readonly Conversation[],
): string[] {
  const names = members
    .filter((member) =>
      conversations.some(
        (conversation) =>
          conversation.memberId === member.id && runningTurn(conversation) !== undefined,
      ),
    )
    .map((member) => member.name);
  const advisor = advisorConversation(conversations);
  return advisor && runningTurn(advisor) ? [...names, 'The Advisor'] : names;
}

/** The running sprint as the summary speaks of it. */
export interface SprintStanding {
  sprint: Sprint;
  /** Its report, once it has come. */
  report: SprintReport | null;
}

/** Everything the summary in words is worked out from. */
export interface StandingSources {
  project: Project;
  tasks: readonly Task[];
  members: readonly Member[];
  conversations: readonly Conversation[];
  /** The sprints of the project, or nothing while they are loading. */
  sprints: readonly Sprint[] | null;
  /** The report of the running sprint, if there is one and it has come. */
  report: SprintReport | null;
  today: CalendarDate;
}

/**
 * Where the project stands, in a few short sentences: how much is
 * finished, who is at work, what is late, how the running sprint stands,
 * and what waits for the manager. A sentence that would have nothing to
 * say is left out.
 */
export function standingInWords(sources: StandingSources): string[] {
  const { project, tasks, members, conversations, sprints, report, today } = sources;
  const counts = countsOf(project, tasks, conversations, today);
  const sentences = [
    progressInWords(counts),
    atWorkInWords(whoIsAtWork(members, conversations)),
    counts.late === 0 ? null : lateCountInWords(counts.late),
    sprints === null ? null : sprintInWords(sprints, report, today),
    counts.waiting === 0 ? null : waitingInWords(counts.waiting),
  ];
  return sentences.filter((sentence) => sentence !== null);
}

/** "12 of 17 tasks are finished, and 3 are in work." */
export function progressInWords(counts: Pick<DashboardCounts, 'total' | 'finished' | 'inWork'>) {
  const { total, finished, inWork } = counts;
  if (total === 0) {
    return 'The project has no tasks yet.';
  }
  if (total === 1) {
    if (finished === 1) {
      return 'Its one task is finished.';
    }
    return inWork === 1 ? 'Its one task is in work.' : 'Its one task is not begun yet.';
  }
  if (finished === total) {
    return `All ${total.toString()} tasks are finished.`;
  }
  const done =
    finished === 0
      ? `None of the ${total.toString()} tasks is finished`
      : `${finished.toString()} of ${total.toString()} tasks ${finished === 1 ? 'is' : 'are'} finished`;
  if (inWork === 0) {
    return `${done}, and none is in work.`;
  }
  return `${done}, and ${inWork.toString()} ${inWork === 1 ? 'is' : 'are'} in work.`;
}

/** "Ada is at work.", "Ada and Ben are at work.", "4 are at work: …" */
export function atWorkInWords(names: readonly string[]): string | null {
  if (names.length === 0) {
    return null;
  }
  if (names.length === 1) {
    return `${names[0] ?? ''} is at work.`;
  }
  const last = names[names.length - 1] ?? '';
  return `${names.slice(0, -1).join(', ')} and ${last} are at work.`;
}

/** "1 task is late.", "2 tasks are late." */
export function lateCountInWords(late: number): string {
  return `${count(late, 'task is', 'tasks are')} late.`;
}

/** "1 thing waits for you.", "3 things wait for you." */
export function waitingInWords(waiting: number): string {
  return `${count(waiting, 'thing waits', 'things wait')} for you.`;
}

/**
 * How the running sprint stands: "Sprint 2 has 9 days left and 16 of 16
 * points to go." A project that uses no sprints gets no sentence; one that
 * uses them with none running is told so.
 */
export function sprintInWords(
  sprints: readonly Sprint[],
  report: SprintReport | null,
  today: CalendarDate,
): string | null {
  if (sprints.length === 0) {
    return null;
  }
  const active = sprints.find((sprint) => sprint.state === 'active');
  if (!active) {
    return 'No sprint is running.';
  }
  const time = active.endOn === null ? 'is running' : timeInWords(daysLeft(active.endOn, today));
  if (!report || report.sprintId !== active.id) {
    return `${active.name} ${time}.`;
  }
  const unit = report.unit === 'points' ? ['point', 'points'] : ['task', 'tasks'];
  const remaining = lastRemaining(report) ?? report.total;
  const work =
    report.total === 0
      ? 'nothing planned into it'
      : remaining === 0
        ? `all ${count(report.total, unit[0] ?? '', unit[1] ?? '')} done`
        : `${remaining.toString()} of ${count(report.total, unit[0] ?? '', unit[1] ?? '')} to go`;
  return `${active.name} ${time} and ${work}.`;
}

/** "has 9 days left", "ends today", "is 2 days past its end". */
function timeInWords(days: number): string {
  if (days === 0) {
    return 'ends today';
  }
  if (days > 0) {
    return `has ${count(days, 'day', 'days')} left`;
  }
  return `is ${count(-days, 'day', 'days')} past its end`;
}

/** The work that remained at the end of the last day that has come. */
function lastRemaining(report: SprintReport): number | null {
  let remaining: number | null = null;
  for (const day of report.days) {
    if (day.remaining !== null) {
      remaining = day.remaining;
    }
  }
  return remaining;
}

/** The sprints of a project the dashboard shows: the running one, and those that are over. */
export function sprintsToShow(sprints: readonly Sprint[]): {
  active: Sprint | undefined;
  completed: Sprint[];
} {
  return {
    active: sprints.find((sprint) => sprint.state === 'active'),
    completed: sprints.filter((sprint) => sprint.state === 'completed'),
  };
}

/** A share of a whole, from 0 to 1, for the width of a bar. Nothing of nothing is nothing. */
export function shareOf(part: number, whole: number): number {
  return whole <= 0 ? 0 : Math.min(1, Math.max(0, part / whole));
}

/** The tone of the status of a task, for the mark in front of it in a list. */
export function toneOf(project: Project, task: Task): Color {
  return project.statuses.find((status) => status.id === task.statusId)?.color ?? 'gray';
}

function count(number: number, one: string, many: string): string {
  return `${number.toString()} ${number === 1 ? one : many}`;
}
