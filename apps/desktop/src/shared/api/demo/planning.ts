import { BackendError } from '../backend';
import type {
  ApprovedProposal,
  Conversation,
  Interview,
  InterviewAnswer,
  InterviewQuestion,
  Proposal,
  ProposalChange,
  ProposedTask,
} from '../types';

/**
 * The interview that is open in a conversation: the one its latest turn
 * ended with, if the manager has not answered it yet.
 */
export function openInterviewOf(conversation: Conversation): Interview | undefined {
  const latest = conversation.turns.at(-1);
  return latest?.state !== 'running' && latest?.interview && latest.interview.answers === null
    ? latest.interview
    : undefined;
}

/**
 * The proposal that is open in a conversation: the one its latest turn
 * ended with, if the manager has not approved it. A proposal of an earlier
 * turn is not open: the conversation has gone on since.
 */
export function openProposalOf(conversation: Conversation): Proposal | undefined {
  const latest = conversation.turns.at(-1);
  return latest?.state !== 'running' && latest?.proposal && latest.proposal.approvedAt === null
    ? latest.proposal
    : undefined;
}

/**
 * The answers to an interview as the Advisor reads them: every question
 * again, with the option that was chosen and what the manager added, or
 * with the manager's own words.
 */
export function answersInWords(
  questions: readonly InterviewQuestion[],
  answers: readonly InterviewAnswer[],
): string {
  const blocks = questions.map((question, place) => {
    const answer = answers[place];
    const note = answer?.note ?? '';
    const option = answer?.option ?? null;
    const chosen = option === null ? undefined : question.options[option];
    const lines = [`${(place + 1).toString()}. ${question.question}`];
    if (chosen) {
      lines.push(`   Answer: ${chosen.label}`);
      if (note !== '') {
        lines.push(`   The manager adds: ${note}`);
      }
    } else {
      lines.push(`   Answer: ${note}`);
    }
    return lines.join('\n');
  });
  return ['The manager answered your questions.', ...blocks].join('\n\n');
}

/**
 * Whether some tasks of a proposal wait for each other in a circle, so
 * that none of them could ever start.
 */
export function waitInCircle(tasks: readonly ProposedTask[]): boolean {
  // A task is looked at once. While the tasks it waits for are followed, it
  // is on the way; meeting a task that is on the way closes a circle.
  const state = new Map<number, 'on the way' | 'clear'>();
  const closesCircle = (place: number): boolean => {
    if (state.get(place) === 'clear') {
      return false;
    }
    if (state.get(place) === 'on the way') {
      return true;
    }
    state.set(place, 'on the way');
    const found = (tasks[place]?.waitsFor ?? []).some(closesCircle);
    state.set(place, 'clear');
    return found;
  };
  return tasks.some((_, place) => closesCircle(place));
}

/**
 * What a task of a plan waits for: the tasks it names and, as on the
 * board, its parts, because a task with parts starts only once its parts
 * are finished.
 */
function waitsWithParts(tasks: readonly ProposedTask[], place: number): number[] {
  const parts = tasks.flatMap((task, part) => (task.partOf === place ? [part] : []));
  return [...(tasks[place]?.waitsFor ?? []), ...parts];
}

/**
 * Whether some tasks of a proposal are parts of each other in a circle, so
 * that no task is the top of them.
 */
export function partsInCircle(tasks: readonly ProposedTask[]): boolean {
  return tasks.some((_, place) => {
    const seen = new Set<number>();
    for (
      let whole = tasks[place]?.partOf ?? null;
      whole !== null;
      whole = tasks[whole]?.partOf ?? null
    ) {
      if (whole === place) {
        return true;
      }
      if (seen.has(whole)) {
        return false;
      }
      seen.add(whole);
    }
    return false;
  });
}

/**
 * Whether some tasks of a proposal wait for each other in a circle once a
 * task with parts waits for its parts, as it does on the board.
 */
export function partsWaitInCircle(tasks: readonly ProposedTask[]): boolean {
  return waitInCircle(
    tasks.map((task, place) => ({ ...task, waitsFor: waitsWithParts(tasks, place) })),
  );
}

/**
 * Whether one task of a plan waits for another, directly or through other
 * tasks, counting that a task waits for its parts.
 */
function waitsOn(tasks: readonly ProposedTask[], waiting: number, target: number): boolean {
  const seen = new Set<number>();
  const toVisit = [waiting];
  for (let current = toVisit.pop(); current !== undefined; current = toVisit.pop()) {
    if (seen.has(current)) {
      continue;
    }
    seen.add(current);
    for (const earlier of waitsWithParts(tasks, current)) {
      if (earlier === target) {
        return true;
      }
      toVisit.push(earlier);
    }
  }
  return false;
}

/** Whether a task of a plan lies below another, as its part or further down. */
function liesBelow(tasks: readonly ProposedTask[], part: number, whole: number): boolean {
  const seen = new Set<number>();
  for (
    let above = tasks[part]?.partOf ?? null;
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
 * The members and the tasks of a proposal after one change of the
 * manager's, as the domain of the real application makes it. Titles,
 * names and owners are checked afterwards, with the rest of the plan.
 */
export function proposalAfter(
  proposal: Pick<Proposal, 'members' | 'tasks'>,
  change: ProposalChange,
): ApprovedProposal {
  const members = structuredClone(proposal.members);
  const tasks = structuredClone(proposal.tasks);
  const taskAt = (place: number): ProposedTask => {
    const task = Number.isInteger(place) ? tasks[place] : undefined;
    if (!task) {
      throw new BackendError('rule_broken', `the proposal has no task ${(place + 1).toString()}`);
    }
    return task;
  };
  const memberAt = (place: number) => {
    const member = Number.isInteger(place) ? members[place] : undefined;
    if (!member) {
      throw new BackendError('rule_broken', `the proposal has no member ${(place + 1).toString()}`);
    }
    return member;
  };
  switch (change.kind) {
    case 'retitle':
      taskAt(change.task).title = change.title;
      break;
    case 'describe':
      taskAt(change.task).description = change.description;
      break;
    case 'estimate':
      taskAt(change.task).estimateMinutes = change.estimateMinutes;
      break;
    case 'prioritize':
      taskAt(change.task).priority = change.priority;
      break;
    case 'giveTo':
      taskAt(change.task).owner = change.owner;
      break;
    case 'addWait': {
      const other = taskAt(change.waitsFor);
      const task = taskAt(change.task);
      if (change.task === change.waitsFor) {
        throw new BackendError(
          'rule_broken',
          `"${task.title}" waits for a task that is not another task of the proposal`,
        );
      }
      if (waitsOn(tasks, change.waitsFor, change.task)) {
        throw new BackendError(
          'rule_broken',
          `"${task.title}" cannot wait for "${other.title}", which waits for it already`,
        );
      }
      if (!task.waitsFor.includes(change.waitsFor)) {
        task.waitsFor.push(change.waitsFor);
      }
      break;
    }
    case 'removeWait': {
      taskAt(change.waitsFor);
      const task = taskAt(change.task);
      task.waitsFor = task.waitsFor.filter((earlier) => earlier !== change.waitsFor);
      break;
    }
    case 'addTask': {
      const partOf = change.partOf ?? null;
      if (partOf !== null) {
        taskAt(partOf);
      }
      tasks.push({
        title: change.title,
        description: '',
        owner: null,
        estimateMinutes: null,
        priority: 'normal',
        waitsFor: [],
        partOf,
      });
      break;
    }
    case 'makePartOf': {
      const task = taskAt(change.task);
      if (change.partOf !== null) {
        const whole = taskAt(change.partOf);
        if (change.partOf === change.task) {
          throw new BackendError(
            'rule_broken',
            `"${task.title}" is a part of a task that is not another task of the proposal`,
          );
        }
        if (liesBelow(tasks, change.partOf, change.task)) {
          throw new BackendError(
            'rule_broken',
            `"${task.title}" cannot be a part of "${whole.title}", which is a part of it already`,
          );
        }
      }
      task.partOf = change.partOf;
      break;
    }
    case 'removeTask': {
      const whole = taskAt(change.task).partOf;
      tasks.splice(change.task, 1);
      const movedUp = (place: number) => (place > change.task ? place - 1 : place);
      for (const task of tasks) {
        task.waitsFor = task.waitsFor.filter((earlier) => earlier !== change.task).map(movedUp);
        // A part of the task that is taken out stays in the plan, because
        // its work is still to be done. It goes where the task was.
        const partOf = task.partOf === change.task ? whole : task.partOf;
        task.partOf = partOf === null ? null : movedUp(partOf);
      }
      break;
    }
    case 'removeMember': {
      const gone = memberAt(change.member).name.toLowerCase();
      members.splice(change.member, 1);
      for (const task of tasks) {
        if (task.owner?.kind === 'member' && task.owner.name.toLowerCase() === gone) {
          task.owner = null;
        }
      }
      break;
    }
    case 'setProfile': {
      const member = memberAt(change.member);
      member.model = change.model;
      member.effort = change.effort;
      break;
    }
  }
  return { members, tasks };
}
