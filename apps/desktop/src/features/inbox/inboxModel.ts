import type { Conversation, Moment, PermissionRequest } from '@/shared/api/types';

/** One thing in the inbox: a conversation that waits, or a request for a permission. */
export type InboxItem =
  | { kind: 'conversation'; conversation: Conversation; at: Moment }
  | { kind: 'permission'; request: PermissionRequest; at: Moment };

/**
 * The conversations that wait and the open requests for a permission
 * together, the one asked longest ago first.
 */
export function inOrderOfAsking(
  conversations: readonly Conversation[],
  requests: readonly PermissionRequest[],
  askedAt: (conversation: Conversation) => Moment,
): InboxItem[] {
  const items: InboxItem[] = [
    ...conversations.map((conversation) => ({
      kind: 'conversation' as const,
      conversation,
      at: askedAt(conversation),
    })),
    ...requests.map((request) => ({ kind: 'permission' as const, request, at: request.askedAt })),
  ];
  return items.sort((first, second) => first.at.localeCompare(second.at));
}

/**
 * Says how many things wait for the manager: "1 thing waits for you". A
 * thing is a question or a request for a permission of a specialist, or
 * questions or a proposal of the Advisor.
 */
export function waitingInWords(count: number): string {
  if (count === 0) {
    return 'Nothing waits for you';
  }
  return count === 1 ? '1 thing waits for you' : `${count.toString()} things wait for you`;
}
