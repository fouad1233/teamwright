import type { Id, Member, MemoryEntry, MemoryKind } from '@/shared/api/types';

/** A choice of the kinds of entries to show: all of them, or one kind. */
export type KindChoice = 'all' | MemoryKind;

/** The kinds the manager chooses between, in the order they are offered. */
export const KIND_CHOICES: readonly { choice: KindChoice; label: string }[] = [
  { choice: 'all', label: 'All' },
  { choice: 'decision', label: 'Decisions' },
  { choice: 'fact', label: 'Facts' },
  { choice: 'contract', label: 'Contracts' },
];

/** What an entry is, as its label says it. */
export const KIND_LABELS: Record<MemoryKind, string> = {
  decision: 'Decision',
  fact: 'Fact',
  contract: 'Contract',
};

/** The name of a member, or what one who has left the team is called. */
function nameOf(members: readonly Member[], memberId: Id | null): string {
  return members.find((member) => member.id === memberId)?.name ?? 'a former teammate';
}

/**
 * Where an entry came from, in a few words for the manager: "Remembered by
 * Firmware engineer", "Decided by you when Hardware engineer asked".
 */
export function sourceInWords(entry: MemoryEntry, members: readonly Member[]): string {
  const who = nameOf(members, entry.memberId);
  switch (entry.source) {
    case 'manager':
      return 'Written by you';
    case 'specialist':
      return `Remembered by ${who}`;
    case 'decision_request':
      return `Decided by you when ${who} asked`;
    case 'handoff':
      return `Handed over by ${who}`;
  }
}

/** The entries in the order they are listed: the newest first. */
export function newestFirst(entries: readonly MemoryEntry[]): MemoryEntry[] {
  return [...entries].sort((first, second) => second.recordedAt.localeCompare(first.recordedAt));
}

/** What the place of memory lists. */
export interface Listing {
  /** The entries to show, the newest first. */
  shown: MemoryEntry[];
  /** How many current entries there are of each kind, among those found. */
  counts: Record<KindChoice, number>;
  /** How many entries newer ones replaced, of the kind chosen. Searching leaves them out. */
  replaced: number;
}

/**
 * The entries the place of memory lists. `found` holds the entries a
 * search found, or nothing when there is no search; the backend searched
 * by the rule the specialists search by. The current entries come first,
 * of the kind chosen, and the replaced ones only when asked for and when
 * nothing is searched.
 */
export function listingOf(
  all: readonly MemoryEntry[],
  found: readonly MemoryEntry[] | undefined,
  kind: KindChoice,
  showsReplaced: boolean,
): Listing {
  const current = found ?? all.filter((entry) => entry.replacedBy === null);
  const ofKind = (entry: MemoryEntry) => kind === 'all' || entry.kind === kind;
  const counts: Record<KindChoice, number> = { all: 0, decision: 0, fact: 0, contract: 0 };
  for (const entry of current) {
    counts.all += 1;
    counts[entry.kind] += 1;
  }
  const replaced =
    found === undefined ? all.filter((entry) => entry.replacedBy !== null && ofKind(entry)) : [];
  return {
    shown: newestFirst([...current.filter(ofKind), ...(showsReplaced ? replaced : [])]),
    counts,
    replaced: replaced.length,
  };
}

/** The facts a specialist remembered in one turn, the oldest first. */
export function rememberedInTurn(entries: readonly MemoryEntry[], turnId: Id): MemoryEntry[] {
  return newestFirst(
    entries.filter((entry) => entry.source === 'specialist' && entry.turnId === turnId),
  ).reverse();
}

/** The names of the tools of memory as a session of Claude Code calls them. */
const SEARCH_MEMORY_TOOL = 'mcp__teamwright__search_memory';
const REMEMBER_TOOL = 'mcp__teamwright__remember';

/**
 * The step of a tool as a conversation shows it: a short name in the
 * column of the tools and what it was used on. The tools of memory are
 * named in words; any other tool keeps its own name.
 */
export function toolStepInWords(tool: string, summary: string): { name: string; summary: string } {
  if (tool === SEARCH_MEMORY_TOOL) {
    return {
      name: 'Memory',
      summary: summary === '' ? 'searched the memory' : `searched for “${summary}”`,
    };
  }
  if (tool === REMEMBER_TOOL) {
    return { name: 'Memory', summary: summary === '' ? 'remembered a fact' : summary };
  }
  return { name: tool, summary };
}
