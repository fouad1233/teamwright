import type { Conversation, Id, Member, Turn } from '@/shared/api/types';

import { advisorConversation, openInterview } from '../advisor/advisorModel';
import { openDecision, runningTurn, whatMemberIsWorkingOn } from '../work/workModel';

/**
 * Pure functions of the chat: which conversations it shows, how their turns
 * are laid out as one talk in the order of time, and what happens to a
 * message the manager writes to one of the team.
 *
 * A conversation belongs to a specialist (`memberId`) and is about a task
 * (`taskId`) or about nothing in particular. Or it is with the Advisor
 * (`isWithAdvisor`), which is not a member and has no task. The chat shows
 * every conversation that is not work on a task.
 */

/** The value in the chooser that stands for the Advisor, which is not a member. */
export const ADVISOR = 'advisor';

/** Whom a message is for: the id of a specialist, or the Advisor. */
export type Recipient = string;

/** Whom a conversation is with, as a recipient. Nothing for a specialist who has left the team. */
export function recipientOf(conversation: Conversation): Recipient | null {
  return conversation.isWithAdvisor ? ADVISOR : conversation.memberId;
}

/** Whether a conversation belongs in the chat: everything that is not work on a task. */
export function isTalk(conversation: Conversation): boolean {
  return conversation.taskId === null;
}

/** A stretch of the talk: turns that follow each other in one conversation. */
export interface Run {
  /** Identifies the run among the others: the id of its first turn. */
  key: string;
  conversation: Conversation;
  /** Its turns, the oldest first. */
  turns: Turn[];
  /** Whether the run begins its conversation, in which the session remembers nothing earlier. */
  startsConversation: boolean;
  /** Whether no later run belongs to the same conversation. */
  isLastOfConversation: boolean;
}

/**
 * The turns of every conversation of the chat in the order they were
 * started, cut into runs wherever the conversation changes. Two
 * conversations that went on at the same time therefore alternate, as they
 * did.
 */
export function runsOfTalk(conversations: readonly Conversation[]): Run[] {
  const placed = conversations
    .filter(isTalk)
    .flatMap((conversation) =>
      conversation.turns.map((turn, place) => ({ conversation, turn, place })),
    );
  // Moments are written the same way throughout, so their text sorts in time.
  // A tie keeps the order of the turns within their conversation.
  placed.sort(
    (first, second) =>
      first.turn.startedAt.localeCompare(second.turn.startedAt) ||
      (first.conversation === second.conversation ? first.place - second.place : 0),
  );

  const runs: Run[] = [];
  for (const { conversation, turn, place } of placed) {
    const last = runs.at(-1);
    if (last?.conversation.id === conversation.id) {
      last.turns.push(turn);
    } else {
      runs.push({
        key: turn.id,
        conversation,
        turns: [turn],
        startsConversation: place === 0,
        isLastOfConversation: true,
      });
    }
  }

  const seen = new Set<Id>();
  for (let place = runs.length - 1; place >= 0; place -= 1) {
    const run = runs[place];
    if (run) {
      run.isLastOfConversation = !seen.has(run.conversation.id);
      seen.add(run.conversation.id);
    }
  }
  return runs;
}

/** Someone a message can be written to. */
export interface Addressee {
  recipient: Recipient;
  name: string;
  /** The specialist, or nothing for the Advisor. */
  member: Member | null;
}

/** What the Advisor is called in the chat. */
export const ADVISOR_NAME = 'Advisor';

/**
 * Whom the manager can write to: the AI specialists of the team, in its
 * order, and the Advisor once a plan has been started with it. A human on
 * the team is not asked through the program. A conversation with the
 * Advisor begins with a brief in the Advisor itself, not with a message.
 */
export function addressees(
  team: readonly Member[],
  conversations: readonly Conversation[],
): Addressee[] {
  const specialists = team
    .filter((member) => member.kind === 'agent')
    .map((member) => ({ recipient: member.id, name: member.name, member }));
  return advisorConversation(conversations)
    ? [...specialists, { recipient: ADVISOR, name: ADVISOR_NAME, member: null }]
    : specialists;
}

/**
 * Whom the field is for when the page opens: whoever the talk ended with,
 * if the message can still go to them, or else the first who can be written to.
 */
export function firstRecipient(
  runs: readonly Run[],
  offered: readonly Addressee[],
): Recipient | null {
  const lastRun = runs.at(-1);
  const last = lastRun ? recipientOf(lastRun.conversation) : null;
  if (last !== null && offered.some((addressee) => addressee.recipient === last)) {
    return last;
  }
  return offered[0]?.recipient ?? null;
}

/** The latest conversation of the chat with someone, if there is one. */
export function latestTalkWith(
  conversations: readonly Conversation[],
  recipient: Recipient,
): Conversation | undefined {
  if (recipient === ADVISOR) {
    return advisorConversation(conversations);
  }
  return conversations
    .filter(
      (conversation) =>
        isTalk(conversation) && !conversation.isWithAdvisor && conversation.memberId === recipient,
    )
    .reduce<Conversation | undefined>(
      (latest, conversation) =>
        latest && latest.startedAt > conversation.startedAt ? latest : conversation,
      undefined,
    );
}

/**
 * What becomes of a message to someone, by the rules the program has:
 *
 * - `ask` starts a new conversation with a specialist;
 * - `reply` continues the open one;
 * - `busy`: the one written to is at work (answering here, or on a task),
 *   and works on one thing at a time;
 * - `decision`: the specialist waits for a decision of the manager, which
 *   has to be made first;
 * - `interview`: the Advisor waits for answers to its questions, which are
 *   given in the Advisor;
 * - `elsewhere`: the conversation with the Advisor runs on another computer;
 * - `nobody`: there is nobody to write to.
 */
export type Sending =
  | { kind: 'ask' }
  | { kind: 'reply'; conversation: Conversation }
  | { kind: 'busy'; conversation: Conversation }
  | { kind: 'decision'; conversation: Conversation }
  | { kind: 'interview'; conversation: Conversation }
  | { kind: 'elsewhere'; conversation: Conversation }
  | { kind: 'nobody' };

/** What becomes of a message to someone now. */
export function howToSend(
  conversations: readonly Conversation[],
  recipient: Recipient | null,
): Sending {
  if (recipient === null) {
    return { kind: 'nobody' };
  }

  if (recipient === ADVISOR) {
    // One conversation with the Advisor runs in a project at a time.
    const working = conversations.find(
      (conversation) => conversation.isWithAdvisor && runningTurn(conversation),
    );
    if (working) {
      return { kind: 'busy', conversation: working };
    }
    const latest = advisorConversation(conversations);
    if (!latest) {
      return { kind: 'nobody' };
    }
    if (!latest.isHere) {
      return { kind: 'elsewhere', conversation: latest };
    }
    if (openInterview(latest)) {
      return { kind: 'interview', conversation: latest };
    }
    return { kind: 'reply', conversation: latest };
  }

  // A specialist works on one thing at a time, a task or a question.
  const working = whatMemberIsWorkingOn(conversations, recipient);
  if (working) {
    return { kind: 'busy', conversation: working };
  }
  // A conversation on another computer is continued there; here a new one starts.
  const latest = latestTalkWith(conversations, recipient);
  if (!latest?.isHere) {
    return { kind: 'ask' };
  }
  if (openDecision(latest)) {
    return { kind: 'decision', conversation: latest };
  }
  return { kind: 'reply', conversation: latest };
}

/** Whether a message can go out as things stand. */
export function canSend(sending: Sending): boolean {
  return sending.kind === 'ask' || sending.kind === 'reply';
}

/**
 * What the line of the field says about a message to someone: where it
 * goes, or why it cannot go now. `taskName` names the task a specialist
 * works on, by its key.
 */
export function sendingInWords(
  sending: Sending,
  name: string,
  taskName: (taskId: Id) => string,
): string {
  switch (sending.kind) {
    case 'ask':
      return `Starts a new conversation. ${name} begins without what was said before.`;
    case 'reply':
      return `Continues the conversation with ${name}.`;
    case 'busy':
      return sending.conversation.taskId === null
        ? `${name} is answering. You can write again when it is done.`
        : `${name} is working on ${taskName(sending.conversation.taskId)}. You can write when that is done.`;
    case 'decision':
      return `${name} waits for your decision above. Decide, and the conversation goes on.`;
    case 'interview':
      return 'The Advisor waits for your answers to its questions. Answer them in the Advisor.';
    case 'elsewhere':
      return 'This conversation with the Advisor runs on another computer. Continue it there.';
    case 'nobody':
      return 'The team has no AI specialist to write to yet.';
  }
}
