import type { Conversation, Id, Member, Moment, Project, Step, Task } from '@/shared/api/types';
import { formatEstimate, taskReference } from '@/shared/lib/format';

import { advisorConversation, advisorWaitsFor } from '../advisor/advisorModel';
import { statusOf } from '../tasks/taskModel';
import { kindLabel, profileInWords, tasksOfManager, tasksOfMember } from '../team/teamModel';
import {
  allTokens,
  durationInWords,
  formatTokens,
  openDecision,
  runningTurn,
  tokensOfTurns,
  whatMemberIsWorkingOn,
  whatMemberWaitsFor,
} from '../work/workModel';

/**
 * Pure functions for the office (decision 0018): what each place in the
 * room shows, where the places stand, and the words for all of it. The
 * components of the office only draw what is worked out here.
 */

/**
 * What a place is doing now. Only `working` has its lamp lit, and only
 * `waiting` is held by the gold corners (decision 0015).
 */
export type PlaceState = 'working' | 'waiting' | 'free' | 'person' | 'manager';

/** Who sits at a place, which decides the figure drawn for it. */
export type PlaceFigure =
  { kind: 'manager' } | { kind: 'advisor' } | { kind: 'member'; member: Member };

/** Where pressing a place leads. */
export type PlaceTarget =
  | { kind: 'task'; taskId: Id }
  | { kind: 'chat'; memberId: Id }
  | { kind: 'advisor' }
  | { kind: 'inbox' };

/** A task as a place names it: its reference and its title. */
export interface PlaceSubject {
  /** The reference of the task, such as `SSN-6`, or nothing for work that is not a task. */
  reference: string | null;
  title: string;
}

/** A counted fact of a place, with the word that says what is counted. */
export interface Measure {
  label: string;
  value: string;
}

/** Everything one place of the office shows. */
export interface Place {
  /** What tells this place from the others, for the list it is drawn from. */
  key: string;
  figure: PlaceFigure;
  name: string;
  /** What the one who sits here is: "AI specialist, Sonnet", "Person", "Manager". */
  role: string;
  state: PlaceState;
  /** The state in a word or two: "At work", "Waits for you", "Free". */
  stateInWords: string;
  /** The task the place is busy with or waits about, if there is one. */
  subject: PlaceSubject | null;
  /** One line more: the last thing done, what is waited for, or what is left. */
  detail: string | null;
  /** The facts that are counted: how long, how big, how many tokens this turn. */
  measures: Measure[];
  /** What the place has used in this project, or for the manager what everybody has. */
  usage: Measure;
  /** Where pressing the place leads, or nothing if it leads nowhere. */
  target: PlaceTarget | null;
  /** What pressing it does, in words, for the keyboard and for screen readers. */
  actionInWords: string | null;
}

/** The whole room: the front, where the plan is made, and the floor, where the team works. */
export interface Office {
  /** The manager's place and the Advisor's. */
  front: Place[];
  /** One place for every member of the team, in the order they joined. */
  floor: Place[];
  summary: OfficeSummary;
}

/** The room counted up, for the title block. */
export interface OfficeSummary {
  working: number;
  waiting: number;
  free: number;
  people: number;
  /** Every token the specialists and the Advisor used in this project. */
  used: string;
}

/** What the office is drawn from. All of it is already known to the interface. */
export interface OfficeSources {
  project: Project;
  members: readonly Member[];
  tasks: readonly Task[];
  conversations: readonly Conversation[];
  /** The moment the office is drawn at, to count how long work has been going on. */
  now: Moment;
}

/** The longest stretch of a specialist's own words that a place quotes. */
const LONGEST_QUOTE = 72;

/**
 * What the common tools of the program do, as a verb that the thing they
 * work on follows ("Reads README.md"), and as a phrase for when nothing is
 * named.
 */
const TOOL_WORDS: Readonly<Record<string, { verb: string; alone: string }>> = {
  Read: { verb: 'Reads', alone: 'Reads a file' },
  Write: { verb: 'Writes', alone: 'Writes a file' },
  Edit: { verb: 'Edits', alone: 'Edits a file' },
  MultiEdit: { verb: 'Edits', alone: 'Edits a file' },
  NotebookEdit: { verb: 'Edits', alone: 'Edits a notebook' },
  Bash: { verb: 'Runs', alone: 'Runs a command' },
  Grep: { verb: 'Searches for', alone: 'Searches the files' },
  Glob: { verb: 'Looks for', alone: 'Looks for files' },
  WebFetch: { verb: 'Reads', alone: 'Reads a page of the web' },
  WebSearch: { verb: 'Searches the web for', alone: 'Searches the web' },
  TodoWrite: { verb: 'Plans', alone: 'Plans its steps' },
  Task: { verb: 'Hands over', alone: 'Hands a part to a helper' },
};

/** The first sentence of a text, shortened to fit one line of a place. */
function firstSentence(text: string): string {
  const flat = text.replace(/\s+/g, ' ').trim();
  const end = /[.!?](\s|$)/.exec(flat);
  const sentence = end ? flat.slice(0, end.index + 1) : flat;
  if (sentence.length <= LONGEST_QUOTE) {
    return sentence;
  }
  const cut = sentence.slice(0, LONGEST_QUOTE);
  const lastSpace = cut.lastIndexOf(' ');
  return `${(lastSpace > 0 ? cut.slice(0, lastSpace) : cut).replace(/[,;:]$/, '')}…`;
}

/**
 * One step of a turn in a few words: "Reads README.md", "Runs npm test", or
 * the first sentence of what the specialist wrote, in quotation marks.
 * A notice of the program says nothing about the work and gives nothing.
 */
export function stepInWords(step: Step): string | null {
  switch (step.kind) {
    case 'text': {
      const sentence = firstSentence(step.text);
      return sentence === '' ? null : `“${sentence}”`;
    }
    case 'tool': {
      const words = TOOL_WORDS[step.tool];
      const summary = step.summary.trim();
      if (!words) {
        return summary === '' ? `Uses ${step.tool}` : `Uses ${step.tool} on ${summary}`;
      }
      return summary === '' ? words.alone : `${words.verb} ${summary}`;
    }
    case 'denied':
      return `Was not allowed to use ${step.tool}`;
    case 'notice':
      return null;
  }
}

/**
 * The last thing done in a turn that can be said in words. A turn that has
 * done nothing yet is starting.
 */
export function lastThingDone(steps: readonly Step[]): string {
  for (let index = steps.length - 1; index >= 0; index -= 1) {
    const step = steps[index];
    const words = step ? stepInWords(step) : null;
    if (words !== null) {
      return words;
    }
  }
  return 'Starting';
}

function count(number: number, one: string, many: string): string {
  return `${number.toString()} ${number === 1 ? one : many}`;
}

/** A number of tokens in words: "21.5k tokens", or "nothing yet" for none. */
export function usageInWords(tokens: number): string {
  return tokens === 0 ? 'nothing yet' : `${formatTokens(tokens)} tokens`;
}

/** Every token the finished turns of some conversations used. */
function tokensOf(conversations: readonly Conversation[]): number {
  return allTokens(tokensOfTurns(conversations.flatMap((conversation) => conversation.turns)));
}

/** The task a conversation is about, as a place names it. */
function subjectOf(
  project: Project,
  tasks: readonly Task[],
  conversation: Conversation,
): PlaceSubject {
  const task =
    conversation.taskId === null
      ? undefined
      : tasks.find((candidate) => candidate.id === conversation.taskId);
  return task
    ? { reference: taskReference(project, task), title: task.title }
    : { reference: null, title: 'A question of yours' };
}

/** Where a conversation leads: to its task, or to the conversation itself. */
function targetOf(conversation: Conversation, memberId: Id): PlaceTarget {
  return conversation.taskId === null
    ? { kind: 'chat', memberId }
    : { kind: 'task', taskId: conversation.taskId };
}

function actionOf(target: PlaceTarget, subject: PlaceSubject, name: string): string {
  if (target.kind === 'task') {
    return `Open ${subject.reference ?? 'the task'}, ${subject.title}`;
  }
  return `Open the conversation with ${name}`;
}

/** The facts of a running turn: how long it has been going, the size of its task, its tokens. */
function measuresOfWork(
  conversation: Conversation,
  tasks: readonly Task[],
  now: Moment,
): Measure[] {
  const turn = runningTurn(conversation);
  if (!turn) {
    return [];
  }
  const measures: Measure[] = [{ label: 'For', value: durationInWords(turn.startedAt, now) }];
  const task = tasks.find((candidate) => candidate.id === conversation.taskId);
  if (task?.estimateMinutes != null) {
    measures.push({ label: 'Size', value: formatEstimate(task.estimateMinutes) });
  }
  if (turn.tokens) {
    measures.push({ label: 'This turn', value: usageInWords(allTokens(turn.tokens)) });
  }
  return measures;
}

/** The place of one member of the team. */
export function placeOfMember(
  member: Member,
  { project, tasks, conversations, now }: Omit<OfficeSources, 'members'>,
): Place {
  const own = conversations.filter((conversation) => conversation.memberId === member.id);
  const base = {
    key: member.id,
    figure: { kind: 'member', member } as const,
    name: member.name,
    role: [kindLabel(member.kind), profileInWords(member)].filter(Boolean).join(', '),
    usage: { label: 'Used', value: usageInWords(tokensOf(own)) },
  };
  const given = tasksOfMember(tasks, member.id);
  const categoryOf = (task: Task) => statusOf(project, task)?.category;

  if (member.kind === 'human') {
    const open = given.filter((task) => categoryOf(task) !== 'done').length;
    return {
      ...base,
      state: 'person',
      stateInWords: 'Person',
      subject: null,
      detail: open === 0 ? 'No open tasks' : `${count(open, 'task', 'tasks')} not finished`,
      measures: [],
      target: null,
      actionInWords: null,
    };
  }

  const work = whatMemberIsWorkingOn(own, member.id);
  if (work) {
    const subject = subjectOf(project, tasks, work);
    const target = targetOf(work, member.id);
    return {
      ...base,
      state: 'working',
      stateInWords: 'At work',
      subject,
      detail: lastThingDone(runningTurn(work)?.steps ?? []),
      measures: measuresOfWork(work, tasks, now),
      target,
      actionInWords: actionOf(target, subject, member.name),
    };
  }

  const waiting = whatMemberWaitsFor(own, member.id);
  const decision = waiting ? openDecision(waiting) : undefined;
  if (waiting && decision) {
    const subject = subjectOf(project, tasks, waiting);
    const target = targetOf(waiting, member.id);
    return {
      ...base,
      state: 'waiting',
      stateInWords: 'Waits for you',
      subject,
      detail: `Asks you to decide: ${firstSentence(decision.question)}`,
      measures: [],
      target,
      actionInWords: actionOf(target, subject, member.name),
    };
  }

  const notStarted = given.filter((task) => categoryOf(task) === 'todo').length;
  return {
    ...base,
    state: 'free',
    stateInWords: 'Free',
    subject: null,
    detail:
      notStarted === 0
        ? 'Nothing given to it waits to start'
        : `${count(notStarted, 'task', 'tasks')} given to it not started`,
    measures: [],
    target: { kind: 'chat', memberId: member.id },
    actionInWords: `Ask ${member.name} something`,
  };
}

/** The place of the Advisor, who plans the project with the manager. */
export function placeOfAdvisor(conversations: readonly Conversation[], now: Moment): Place {
  const withAdvisor = conversations.filter((conversation) => conversation.isWithAdvisor);
  const latest = advisorConversation(withAdvisor);
  const base = {
    key: 'advisor',
    figure: { kind: 'advisor' } as const,
    name: 'Advisor',
    role: 'Plans the project with you',
    subject: null,
    usage: { label: 'Used', value: usageInWords(tokensOf(withAdvisor)) },
    target: { kind: 'advisor' } as const,
    actionInWords: 'Open the Advisor',
  };

  const turn = latest ? runningTurn(latest) : undefined;
  if (turn) {
    return {
      ...base,
      state: 'working',
      stateInWords: 'At work',
      subject: { reference: null, title: 'The plan of the project' },
      detail: lastThingDone(turn.steps),
      measures: [{ label: 'For', value: durationInWords(turn.startedAt, now) }],
    };
  }

  const waitsFor = latest ? advisorWaitsFor(latest) : null;
  if (waitsFor !== null) {
    return {
      ...base,
      state: 'waiting',
      stateInWords: 'Waits for you',
      subject: { reference: null, title: 'The plan of the project' },
      detail: `${waitsFor.charAt(0).toUpperCase()}${waitsFor.slice(1)}`,
      measures: [],
    };
  }

  return {
    ...base,
    state: 'free',
    stateInWords: 'Free',
    detail: latest ? 'Ready to plan again' : 'Has not planned this project yet',
    measures: [],
  };
}

/**
 * How many things wait for the manager in a project: the decisions the
 * specialists ask for, and what the Advisor asks or proposes.
 */
export function countWaitingForManager(conversations: readonly Conversation[]): number {
  const decisions = conversations.filter(
    (conversation) => !conversation.isWithAdvisor && openDecision(conversation),
  ).length;
  const latest = advisorConversation(conversations);
  return decisions + (latest && advisorWaitsFor(latest) !== null ? 1 : 0);
}

/** The manager's own place. */
export function placeOfManager({
  project,
  tasks,
  conversations,
}: Omit<OfficeSources, 'members' | 'now'>): Place {
  const waiting = countWaitingForManager(conversations);
  const open = tasksOfManager(tasks).filter(
    (task) => statusOf(project, task)?.category !== 'done',
  ).length;
  return {
    key: 'manager',
    figure: { kind: 'manager' },
    name: 'You',
    role: 'Manager',
    state: 'manager',
    stateInWords:
      waiting === 0
        ? 'Nothing waits for you'
        : `${count(waiting, 'thing waits', 'things wait')} for you`,
    subject: null,
    detail:
      open === 0
        ? 'None of your own tasks is open'
        : `${count(open, 'task', 'tasks')} of your own open`,
    measures: [],
    usage: { label: 'The project', value: usageInWords(tokensOf(conversations)) },
    target: { kind: 'inbox' },
    actionInWords: 'Open the inbox',
  };
}

/** The whole office of a project. */
export function describeOffice(sources: OfficeSources): Office {
  const floor = sources.members.map((member) => placeOfMember(member, sources));
  const front = [placeOfManager(sources), placeOfAdvisor(sources.conversations, sources.now)];
  const all = [...front, ...floor];
  return {
    front,
    floor,
    summary: {
      working: all.filter((place) => place.state === 'working').length,
      waiting: all.filter((place) => place.state === 'waiting').length,
      free: all.filter((place) => place.state === 'free').length,
      people: floor.filter((place) => place.state === 'person').length,
      used: usageInWords(tokensOf(sources.conversations)),
    },
  };
}

/** Whether anything in the office is going on, so that its time has to be counted. */
export function isAnybodyWorking(office: Office): boolean {
  return [...office.front, ...office.floor].some((place) => place.state === 'working');
}

/**
 * How many benches stand in a row of the floor when there is room enough:
 * one to three in a row, four as two by two, five and six in rows of three,
 * and from seven on in rows of four.
 */
export function idealColumns(count: number): number {
  if (count <= 3) {
    return Math.max(1, count);
  }
  if (count === 4) {
    return 2;
  }
  return count <= 6 ? 3 : 4;
}

/** The sizes the arrangement of the office is worked out from, in pixels. */
export interface OfficeMeasures {
  /** The width the office has in the window. */
  width: number;
  /** The width of one place. */
  placeWidth: number;
  /** The width of the title block beside the room. */
  asideWidth: number;
  /** The gap between two places, and between the room and the title block. */
  gap: number;
  /** The room the walls and the margin of the room take on both sides together. */
  roomInset: number;
}

/** How the office is arranged in the room it has. */
export interface OfficeArrangement {
  /** How many places stand in one row of the floor at most. */
  columns: number;
  /** Whether the title block stands beside the room, or under it when the window is narrow. */
  isAsideBeside: boolean;
}

/** The places at the front of the room: the manager's and the Advisor's. */
const FRONT_PLACES = 2;

/** How many places of a width fit side by side in a width. Always at least one. */
function fitting(width: number, placeWidth: number, gap: number): number {
  return Math.max(1, Math.floor((width + gap) / (placeWidth + gap)));
}

/**
 * Works out how the office stands in the window. The title block stands
 * beside the room only if the room keeps space for the two places of the
 * front side by side; otherwise it goes under the room. The
 * floor then takes as many benches in a row as the team calls for and the
 * room allows.
 */
export function arrangeOffice(memberCount: number, measures: OfficeMeasures): OfficeArrangement {
  const { width, placeWidth, asideWidth, gap, roomInset } = measures;
  const wanted = idealColumns(memberCount);
  const roomBeside = width - asideWidth - gap - roomInset;
  const isAsideBeside = fitting(roomBeside, placeWidth, gap) >= FRONT_PLACES;
  const room = isAsideBeside ? roomBeside : width - roomInset;
  return { columns: Math.min(wanted, fitting(room, placeWidth, gap)), isAsideBeside };
}

/**
 * Puts places into rows of at most `columns`. Every row is full but the
 * last, which the room centres.
 */
export function rowsOf<T>(items: readonly T[], columns: number): T[][] {
  const width = Math.max(1, columns);
  const rows: T[][] = [];
  for (let start = 0; start < items.length; start += width) {
    rows.push(items.slice(start, start + width));
  }
  return rows;
}

/** The letter of a row of the drawing, as on the margin of a plate: A, B, … Z, AA. */
export function rowLetter(index: number): string {
  const letters = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';
  const letter = letters[index % letters.length] ?? 'A';
  return index < letters.length
    ? letter
    : `${rowLetter(Math.floor(index / letters.length) - 1)}${letter}`;
}

/** The reference of a place on the grid of the drawing: "B2" for the second place of row B. */
export function placeReference(row: number, column: number): string {
  return `${rowLetter(row)}${(column + 1).toString()}`;
}

/** The summary of the room in words, for the line under the title. */
export function summaryInWords(summary: OfficeSummary): string {
  const parts = [
    summary.working > 0 ? `${summary.working.toString()} at work` : null,
    summary.waiting > 0 ? `${summary.waiting.toString()} waiting for you` : null,
  ].filter((part) => part !== null);
  return parts.length === 0 ? 'Nobody is at work' : parts.join(', ');
}
