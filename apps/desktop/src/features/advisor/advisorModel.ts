import type {
  ApprovedProposal,
  Conversation,
  Id,
  Interview,
  InterviewAnswer,
  InterviewQuestion,
  Member,
  Project,
  Proposal,
  ProposedMove,
  ProposedOwner,
  ProposedTask,
  Task,
} from '@/shared/api/types';
import { formatEstimate, taskReference } from '@/shared/lib/format';

/**
 * The conversation with the Advisor that a project shows: the one that was
 * started last. Earlier ones stay on record.
 */
export function advisorConversation(
  conversations: readonly Conversation[],
): Conversation | undefined {
  return conversations
    .filter((conversation) => conversation.isWithAdvisor)
    .reduce<Conversation | undefined>(
      (latest, conversation) =>
        latest && latest.startedAt >= conversation.startedAt ? latest : conversation,
      undefined,
    );
}

/**
 * The interview that is open in a conversation: the one its latest turn
 * ended with, if the manager has not answered it yet.
 */
export function openInterview(conversation: Conversation): Interview | undefined {
  const latest = conversation.turns.at(-1);
  return latest?.state !== 'running' && latest?.interview && latest.interview.answers === null
    ? latest.interview
    : undefined;
}

/**
 * The proposal that is open in a conversation: the one its latest turn
 * ended with, if the manager has not approved it.
 */
export function openProposal(conversation: Conversation): Proposal | undefined {
  const latest = conversation.turns.at(-1);
  return latest?.state !== 'running' && latest?.proposal && latest.proposal.approvedAt === null
    ? latest.proposal
    : undefined;
}

/** What the manager has put down for one question of an interview so far. */
export interface AnswerDraft {
  /** The option that is picked, `own words` for an answer in words alone, or nothing yet. */
  pick: number | 'own words' | undefined;
  note: string;
}

/** Nothing put down yet, for every question. */
export function emptyAnswers(questions: readonly InterviewQuestion[]): AnswerDraft[] {
  return questions.map(() => ({ pick: undefined, note: '' }));
}

/**
 * Whether a question is answered: an option is picked, or the manager
 * wrote something. A question without options is answered in words.
 */
export function isAnswered(draft: AnswerDraft): boolean {
  return typeof draft.pick === 'number' || draft.note.trim() !== '';
}

/** The answers as the backend takes them. */
export function answersOf(drafts: readonly AnswerDraft[]): InterviewAnswer[] {
  return drafts.map((draft) => ({
    option: typeof draft.pick === 'number' ? draft.pick : null,
    note: draft.note,
  }));
}

/**
 * How a question was answered, in words: the option that was chosen, and
 * what the manager added.
 */
export function answerInWords(question: InterviewQuestion, answer: InterviewAnswer): string {
  const chosen = answer.option === null ? undefined : question.options[answer.option]?.label;
  if (chosen === undefined) {
    return answer.note;
  }
  return answer.note === '' ? chosen : `${chosen}. ${answer.note}`;
}

/**
 * What approving a proposal sends: its members and its tasks as they read
 * now, with every change the manager made. The changes are stored as they
 * are made, so the proposal itself holds them.
 */
export function draftOf(proposal: Proposal): ApprovedProposal {
  return structuredClone({
    members: proposal.members,
    tasks: proposal.tasks,
    moves: proposal.moves,
  });
}

/** A task of a plan that another task of the same plan could wait for. */
export interface WaitChoice {
  /** Its place in the plan, from 0. */
  place: number;
  title: string;
}

/**
 * The tasks of a plan that a task could be made to wait for: the others,
 * without those it waits for already. Whether waiting for one would close
 * a circle is for the backend to say.
 */
export function waitChoices(tasks: readonly ProposedTask[], place: number): WaitChoice[] {
  const waiting = tasks[place];
  return tasks.flatMap((task, other) =>
    other === place || waiting?.waitsFor.includes(other) === true
      ? []
      : [{ place: other, title: task.title }],
  );
}

/** The tasks a task of a plan waits for, by their places, in the order of the plan. */
export function waitedFor(tasks: readonly ProposedTask[], place: number): WaitChoice[] {
  const waiting = tasks[place];
  return [...(waiting?.waitsFor ?? [])]
    .sort((first, second) => first - second)
    .flatMap((other) => {
      const task = tasks[other];
      return task ? [{ place: other, title: task.title }] : [];
    });
}

/** One row of the plan as the review shows it: a task, set in as deep as it lies. */
export interface PlanRow {
  /** The place of the task in the plan, from 0. */
  place: number;
  /** How many tasks it lies under: 0 for a task of its own, 1 for a part. */
  depth: number;
  /** How many parts it has, directly. */
  parts: number;
}

/**
 * The rows of a plan in the order the review shows them, like subtasks on
 * the board: each task of its own in the order of the plan, and under it
 * its parts, in the order of the plan, each with its own parts under it.
 * The parts of a folded task are left out. A task whose whole is not in
 * the plan is shown as a task of its own, so that nothing is lost.
 */
export function planRows(
  tasks: readonly ProposedTask[],
  folded: ReadonlySet<number> = new Set(),
): PlanRow[] {
  const isTop = (task: ProposedTask, place: number) =>
    task.partOf === null || task.partOf === place || tasks[task.partOf] === undefined;
  const partsOf = (whole: number) =>
    tasks.flatMap((task, place) => (task.partOf === whole && !isTop(task, place) ? [place] : []));
  const rows: PlanRow[] = [];
  const placed = new Set<number>();
  const add = (place: number, depth: number) => {
    if (placed.has(place)) {
      return;
    }
    placed.add(place);
    const parts = partsOf(place);
    rows.push({ place, depth, parts: parts.length });
    if (!folded.has(place)) {
      for (const part of parts) {
        add(part, depth + 1);
      }
    }
  };
  tasks.forEach((task, place) => {
    if (isTop(task, place)) {
      add(place, 0);
    }
  });
  return rows;
}

/** Whether a task of a plan lies below another, as its part or further down. */
function liesBelow(tasks: readonly ProposedTask[], place: number, whole: number): boolean {
  const seen = new Set<number>();
  for (
    let above = tasks[place]?.partOf ?? null;
    above !== null;
    above = tasks[above]?.partOf ?? null
  ) {
    if (above === whole) {
      return true;
    }
    if (seen.has(above)) {
      return false;
    }
    seen.add(above);
  }
  return false;
}

/**
 * The tasks of a plan that a task could be made a part of: the others,
 * without the one it is a part of already and without its own parts, at
 * any depth. Whether that breaks another rule is for the backend to say.
 */
export function partOfChoices(tasks: readonly ProposedTask[], place: number): WaitChoice[] {
  const task = tasks[place];
  return tasks.flatMap((other, whole) =>
    whole === place || task?.partOf === whole || liesBelow(tasks, whole, place)
      ? []
      : [{ place: whole, title: other.title }],
  );
}

/** How many tasks of a plan are a part of another one, of the plan or of the project. */
export function countParts(tasks: readonly ProposedTask[]): number {
  return tasks.filter((task) => task.partOf !== null || task.partOfTask !== null).length;
}

/**
 * The tasks of the project a proposal moves under a task of the proposal,
 * in the order of the proposal: "moves here", under that task.
 */
export function movesUnder(moves: readonly ProposedMove[], place: number): ProposedMove[] {
  return moves.filter((move) => move.partOfTask === null && move.partOf === place);
}

/**
 * The tasks of the project a proposal moves under another task of the
 * project, or makes tasks of their own: those that go under no task of the
 * proposal.
 */
export function movesElsewhere(moves: readonly ProposedMove[]): ProposedMove[] {
  return moves.filter((move) => move.partOfTask !== null || move.partOf === null);
}

/** The key of the choice "a task of its own" in a picker of wholes. */
export const OWN_TASK = 'own';

/** The key of a task of the proposal in a picker of wholes. */
export function keyOfPlace(place: number): string {
  return `place:${place.toString()}`;
}

/** The key of a task of the project in a picker of wholes. */
export function keyOfTask(taskId: Id): string {
  return `task:${taskId}`;
}

/** Where a key of a picker of wholes puts a task. */
export function wholeOfKey(key: string): { partOf: number | null; partOfTask: Id | null } {
  if (key.startsWith('place:')) {
    return { partOf: Number(key.slice('place:'.length)), partOfTask: null };
  }
  if (key.startsWith('task:')) {
    return { partOf: null, partOfTask: key.slice('task:'.length) };
  }
  return { partOf: null, partOfTask: null };
}

/** One place a task can be put under, as a picker lists it. */
export interface WholeChoice {
  key: string;
  /** "3" for a task of the proposal, "AQN-12" for a task of the project. */
  reference: string;
  title: string;
}

/** A task of the project and every task below it, as their identifiers. */
function subtreeOf(board: readonly Task[], root: Id): Set<Id> {
  const below = new Set<Id>([root]);
  for (let sizeBefore = 0; sizeBefore !== below.size;) {
    sizeBefore = below.size;
    for (const task of board) {
      if (task.parentId !== null && below.has(task.parentId)) {
        below.add(task.id);
      }
    }
  }
  return below;
}

/**
 * Where a moved task of the project can go: under a task of the proposal,
 * under another task of the project that is not one of its own parts, or
 * on its own. Where it goes now is left out. Whether a choice breaks
 * another rule is for the backend to say.
 */
export function moveChoices(
  proposal: Pick<Proposal, 'tasks'>,
  project: Project,
  board: readonly Task[],
  move: ProposedMove,
): WholeChoice[] {
  const below = subtreeOf(board, move.taskId);
  const now =
    move.partOfTask !== null
      ? keyOfTask(move.partOfTask)
      : move.partOf !== null
        ? keyOfPlace(move.partOf)
        : OWN_TASK;
  const choices: WholeChoice[] = [
    { key: OWN_TASK, reference: '', title: 'A task of its own' },
    ...proposal.tasks.map((task, place) => ({
      key: keyOfPlace(place),
      reference: (place + 1).toString(),
      title: task.title,
    })),
    ...[...board]
      .sort((first, second) => first.number - second.number)
      .filter((task) => !below.has(task.id))
      .map((task) => ({
        key: keyOfTask(task.id),
        reference: taskReference(project, task),
        title: task.title,
      })),
  ];
  return choices.filter((choice) => choice.key !== now);
}

/**
 * The tasks of the project a new task of a proposal can be made a part of,
 * besides the tasks of the proposal: all of them but the one it is a part
 * of already. Whether that breaks a rule is for the backend to say.
 */
export function boardWholeChoices(
  project: Project,
  board: readonly Task[],
  task: ProposedTask,
): WholeChoice[] {
  return [...board]
    .sort((first, second) => first.number - second.number)
    .filter((other) => other.id !== task.partOfTask)
    .map((other) => ({
      key: keyOfTask(other.id),
      reference: taskReference(project, other),
      title: other.title,
    }));
}

/** One of the people a proposed task can be given to. */
export interface OwnerChoice {
  /** What the menu of owners knows the choice by. */
  value: string;
  label: string;
  owner: ProposedOwner | null;
}

/** The value of the choice "nobody" in the menu of owners. */
const NOBODY = 'nobody';
/** The value of the choice "the manager" in the menu of owners. */
const MANAGER = 'manager';

function valueOfMember(name: string): string {
  return `member:${name.toLowerCase()}`;
}

/**
 * Whom a task of a proposal can be given to: the manager, the members that
 * are proposed, the AI specialists and people the team already has, and
 * nobody. A name that is on the team and in the proposal is listed once.
 */
export function ownerChoices(draft: ApprovedProposal, team: readonly Member[]): OwnerChoice[] {
  const names = new Map<string, string>();
  for (const name of [...draft.members, ...team].map((member) => member.name)) {
    if (!names.has(name.toLowerCase())) {
      names.set(name.toLowerCase(), name);
    }
  }
  return [
    { value: MANAGER, label: 'You', owner: { kind: 'manager' } },
    ...[...names.values()].map((name): OwnerChoice => ({
      value: valueOfMember(name),
      label: name,
      owner: { kind: 'member', name },
    })),
    { value: NOBODY, label: 'Nobody', owner: null },
  ];
}

/** The value by which the menu of owners knows the owner of a task. */
export function valueOfOwner(owner: ProposedOwner | null): string {
  if (owner === null) {
    return NOBODY;
  }
  return owner.kind === 'manager' ? MANAGER : valueOfMember(owner.name);
}

/** The name of the owner of a proposed task, as the plan shows it. */
export function ownerInWords(owner: ProposedOwner | null): string {
  if (owner === null) {
    return 'Nobody';
  }
  return owner.kind === 'manager' ? 'You' : owner.name;
}

/** The places of the tasks a task waits for, as people count them: "1, 3". */
export function waitsForInWords(task: ProposedTask): string {
  return [...task.waitsFor]
    .sort((first, second) => first - second)
    .map((place) => (place + 1).toString())
    .join(', ');
}

/** How many tasks of a proposal are given to nobody. */
export function countWithoutOwner(tasks: readonly ProposedTask[]): number {
  return tasks.filter((task) => task.owner === null).length;
}

function count(number: number, one: string, many: string): string {
  return `${number.toString()} ${number === 1 ? one : many}`;
}

/** The size of a team in words: "3 specialists". */
export function teamInWords(members: number): string {
  return count(members, 'specialist', 'specialists');
}

/**
 * The size of a plan in words: "12 tasks, about 61h". The time is the sum
 * of the sizes the Advisor gave, and is left out if it gave none.
 */
export function planInWords(tasks: readonly ProposedTask[]): string {
  const minutes = tasks.reduce((sum, task) => sum + (task.estimateMinutes ?? 0), 0);
  const size = count(tasks.length, 'task', 'tasks');
  return minutes === 0 ? size : `${size}, about ${formatEstimate(minutes)}`;
}

/**
 * What a proposal holds, in a few words: "3 specialists and 12 tasks", or
 * "2 tasks and 6 moves" for a regrouping.
 */
export function proposalInWords(proposal: Pick<Proposal, 'members' | 'tasks' | 'moves'>): string {
  const pieces = [
    ...(proposal.members.length === 0 ? [] : [teamInWords(proposal.members.length)]),
    ...(proposal.tasks.length === 0 && proposal.moves.length > 0
      ? []
      : [count(proposal.tasks.length, 'task', 'tasks')]),
    ...(proposal.moves.length === 0 ? [] : [count(proposal.moves.length, 'move', 'moves')]),
  ];
  return pieces.length <= 1
    ? (pieces[0] ?? '')
    : `${pieces.slice(0, -1).join(', ')} and ${pieces.at(-1) ?? ''}`;
}

/**
 * Whether the team already has a member of a name. Such a member is not
 * added again when a proposal that names it is approved. Capital and small
 * letters count as the same.
 */
export function isOnTeam(name: string, team: readonly Member[]): boolean {
  return team.some((member) => member.name.toLowerCase() === name.toLowerCase());
}

/** How many members of a proposal are new to the team. */
export function countNewMembers(draft: ApprovedProposal, team: readonly Member[]): number {
  return draft.members.filter((member) => !isOnTeam(member.name, team)).length;
}

/**
 * What approving a proposal will do, in a sentence. A proposed member whose
 * name the team already has is not added again.
 */
export function approvalInWords(draft: ApprovedProposal, team: readonly Member[]): string {
  const added = countNewMembers(draft, team);
  const tasks = count(draft.tasks.length, 'task', 'tasks');
  const what = added === 0 ? tasks : `${teamInWords(added)} and ${tasks}`;
  const adds =
    added === 0 && draft.tasks.length === 0 && draft.moves.length > 0
      ? ''
      : `Adds ${what} to the project. `;
  const parts = countParts(draft.tasks) === 0 ? '' : 'The parts become subtasks of their tasks. ';
  const moves =
    draft.moves.length === 0
      ? ''
      : `Moves ${count(draft.moves.length, 'task that exists', 'tasks that exist')} with their parts, into the area and the sprint of the task each goes under. `;
  return `${adds}${parts}${moves}No work starts before you start it.`;
}

/**
 * What waits for the manager in a conversation with the Advisor, in a few
 * words, or nothing if nothing does.
 */
export function advisorWaitsFor(conversation: Conversation): string | null {
  const interview = openInterview(conversation);
  if (interview) {
    return `asks ${count(interview.questions.length, 'question', 'questions')}`;
  }
  const proposal = openProposal(conversation);
  return proposal ? `proposes ${proposalInWords(proposal)}` : null;
}
