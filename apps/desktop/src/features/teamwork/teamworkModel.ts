import type { Handoff, Id, Member, MessageKind, TeamMessage, TeamTalk } from '@/shared/api/types';

/** What a turn of a conversation had to do with the team bus. */
export interface TalkOfTurn {
  /** The handoffs the turn published, the oldest first. */
  handoffs: Handoff[];
  /** The messages the turn sent to teammates, the oldest first. */
  sent: TeamMessage[];
  /** The messages of teammates that came with the turn, the oldest first. */
  arrived: TeamMessage[];
}

/** A turn that had nothing to do with the team bus. */
export const NO_TALK: TalkOfTurn = { handoffs: [], sent: [], arrived: [] };

/** What a turn published, sent and received, from everything of its project. */
export function talkOfTurn(talk: TeamTalk | undefined, turnId: Id): TalkOfTurn {
  if (!talk) {
    return NO_TALK;
  }
  const handoffs = talk.handoffs.filter((handoff) => handoff.turnId === turnId);
  const sent = talk.messages.filter((message) => message.turnId === turnId);
  const arrived = talk.messages.filter((message) => message.deliveredTurnId === turnId);
  return handoffs.length + sent.length + arrived.length === 0
    ? NO_TALK
    : { handoffs, sent, arrived };
}

/** Whether a turn had anything to do with the team bus. */
export function hasTalk(talk: TalkOfTurn): boolean {
  return talk.handoffs.length + talk.sent.length + talk.arrived.length > 0;
}

/**
 * The handoff of a task that counts: the latest it published. The others
 * are earlier versions of it.
 */
export function latestHandoffOf(
  handoffs: readonly Handoff[],
  taskId: Id,
): { latest: Handoff; earlier: number } | undefined {
  const ofTask = handoffs
    .filter((handoff) => handoff.taskId === taskId)
    .sort((first, second) => first.publishedAt.localeCompare(second.publishedAt));
  const latest = ofTask.at(-1);
  return latest ? { latest, earlier: ofTask.length - 1 } : undefined;
}

/** The messages between teammates, the latest first, as the manager reads them. */
export function messagesLatestFirst(messages: readonly TeamMessage[]): TeamMessage[] {
  return [...messages].sort((first, second) => second.sentAt.localeCompare(first.sentAt));
}

/** How many messages still wait for their receiver. */
export function waitingCount(messages: readonly TeamMessage[]): number {
  return messages.filter((message) => message.deliveredTurnId === null).length;
}

/** The kind of a message, as a label. */
export function kindInWords(kind: MessageKind): string {
  return kind === 'question' ? 'Question' : 'Note';
}

/** The name of a member, or what a member who has left the team is called. */
export function nameOf(members: readonly Member[], memberId: Id): string {
  return members.find((member) => member.id === memberId)?.name ?? 'A former teammate';
}
