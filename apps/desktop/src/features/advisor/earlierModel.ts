import type { ConversationsOfFolder, EarlierConversation } from '@/shared/api/types';

/**
 * How earlier conversations are shown: filtered by what the manager types,
 * and described in a few words so that the manager recognises them. What a
 * digest holds is decided in Rust (decision 0041); nothing here changes it.
 */

/**
 * The folders whose conversations match a filter, each with only those
 * conversations. A conversation matches when every word of the filter is
 * in its title, its first request or its folder, whatever the capitals. An
 * empty filter keeps everything.
 */
export function filterFolders(
  folders: readonly ConversationsOfFolder[],
  filter: string,
): ConversationsOfFolder[] {
  const words = filter.toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) {
    return [...folders];
  }
  return folders.flatMap((folder) => {
    const conversations = folder.conversations.filter((conversation) => {
      const text = [conversation.title ?? '', conversation.firstRequest ?? '', folder.folder]
        .join(' ')
        .toLowerCase();
      return words.every((word) => text.includes(word));
    });
    return conversations.length === 0 ? [] : [{ ...folder, conversations }];
  });
}

/** How many conversations the folders hold together. */
export function countOf(folders: readonly ConversationsOfFolder[]): number {
  return folders.reduce((count, folder) => count + folder.conversations.length, 0);
}

/** What a conversation is called in the list: its title, or else its first request. */
export function nameOf(conversation: EarlierConversation): string {
  return conversation.title ?? conversation.firstRequest ?? 'A conversation without a request';
}

/**
 * How long a conversation went, in words: "under an hour", "3 hours",
 * "2 days". Nothing when it is not known.
 */
export function durationInWords(startedAt: string | null, endedAt: string | null): string | null {
  if (startedAt === null || endedAt === null) {
    return null;
  }
  const hours = (Date.parse(endedAt) - Date.parse(startedAt)) / 3_600_000;
  if (!Number.isFinite(hours) || hours < 0) {
    return null;
  }
  if (hours < 1) {
    return 'under an hour';
  }
  if (hours < 24) {
    const whole = Math.round(hours);
    return whole === 1 ? '1 hour' : `${whole.toString()} hours`;
  }
  const days = Math.round(hours / 24);
  return days === 1 ? '1 day' : `${days.toString()} days`;
}

/** The size of a conversation's file in words: "12 KB", "3.4 MB". */
export function sizeInWords(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes.toString()} bytes`;
  }
  const kilobytes = bytes / 1024;
  if (kilobytes < 1024) {
    return `${Math.round(kilobytes).toString()} KB`;
  }
  const megabytes = kilobytes / 1024;
  return `${megabytes < 10 ? megabytes.toFixed(1) : Math.round(megabytes).toString()} MB`;
}

/** The day of a moment, for a list: "3 Nov 2026". */
export function dayOf(moment: string | null): string | null {
  if (moment === null) {
    return null;
  }
  return new Intl.DateTimeFormat('en-GB', { dateStyle: 'medium' }).format(new Date(moment));
}
