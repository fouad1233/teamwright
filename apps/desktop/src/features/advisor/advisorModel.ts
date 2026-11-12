import type {
  ApprovedProposal,
  Conversation,
  Interview,
  InterviewAnswer,
  InterviewQuestion,
  Member,
  MemberProfile,
  Proposal,
  ProposedOwner,
  ProposedTask,
} from '@/shared/api/types';
import { formatEstimate } from '@/shared/lib/format';

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

/** What is left of a proposal while the manager changes it. */
export function draftOf(proposal: Proposal): ApprovedProposal {
  return structuredClone({ members: proposal.members, tasks: proposal.tasks });
}

function isOwnedBy(task: ProposedTask, name: string): boolean {
  return task.owner?.kind === 'member' && task.owner.name.toLowerCase() === name.toLowerCase();
}

/**
 * The proposal without one of its members. The tasks that were given to
 * that member stay, and are given to nobody: the work is still to be done.
 */
export function withoutMember(draft: ApprovedProposal, place: number): ApprovedProposal {
  const gone = draft.members[place];
  if (!gone) {
    return draft;
  }
  return {
    members: draft.members.filter((_, other) => other !== place),
    tasks: draft.tasks.map((task) =>
      isOwnedBy(task, gone.name) ? { ...task, owner: null } : task,
    ),
  };
}

/**
 * The proposal without one of its tasks. No task waits for it any more,
 * and the places the other tasks wait for are counted anew.
 */
export function withoutTask(draft: ApprovedProposal, place: number): ApprovedProposal {
  return {
    members: draft.members,
    tasks: draft.tasks
      .filter((_, other) => other !== place)
      .map((task) => ({
        ...task,
        waitsFor: task.waitsFor
          .filter((earlier) => earlier !== place)
          .map((earlier) => (earlier > place ? earlier - 1 : earlier)),
      })),
  };
}

/** The proposal with one of its tasks given to somebody else, or to nobody. */
export function withOwner(
  draft: ApprovedProposal,
  place: number,
  owner: ProposedOwner | null,
): ApprovedProposal {
  return {
    members: draft.members,
    tasks: draft.tasks.map((task, other) => (other === place ? { ...task, owner } : task)),
  };
}

/** The proposal with another model or effort for one of its members. */
export function withProfile(
  draft: ApprovedProposal,
  place: number,
  profile: Partial<MemberProfile>,
): ApprovedProposal {
  return {
    members: draft.members.map((member, other) =>
      other === place ? { ...member, ...profile } : member,
    ),
    tasks: draft.tasks,
  };
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

/** What a proposal holds, in a few words: "3 specialists and 12 tasks". */
export function proposalInWords(proposal: Pick<Proposal, 'members' | 'tasks'>): string {
  const tasks = count(proposal.tasks.length, 'task', 'tasks');
  return proposal.members.length === 0
    ? tasks
    : `${teamInWords(proposal.members.length)} and ${tasks}`;
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
  return `Adds ${what} to the project. No work starts before you start it.`;
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
