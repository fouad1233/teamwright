import type {
  Conversation,
  Interview,
  InterviewAnswer,
  InterviewQuestion,
  Proposal,
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
