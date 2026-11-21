import { BackendError } from '../backend';
import type { Id, MemoryEntry, MemoryKind, Moment, Project, Task } from '../types';

/**
 * The memory of the projects of the demo: decisions, facts and contracts.
 *
 * The real application keeps it in its database and follows the rules of
 * the domain and the application (decision 0036). The demo keeps it here
 * and follows the same rules: an entry never changes in place but is
 * replaced by a newer one, a search finds the current entries in which
 * every word begins a word, the newest first, and the brief of a task
 * carries what belongs to it within a budget.
 */

/** The most characters of a title. */
const MAX_TITLE_LENGTH = 120;
/** The most characters of a text. */
const MAX_TEXT_LENGTH = 20_000;
/** The most characters of a search. */
const MAX_SEARCH_LENGTH = 200;
/** The most words of a search. */
const MOST_SEARCH_WORDS = 12;
/** The most characters the memory takes in a brief. */
const MEMORY_IN_BRIEF_LENGTH = 3000;
/** The most entries in a brief. */
const MOST_IN_BRIEF = 8;
/** The most characters of the text of one entry in a brief. */
const BRIEF_TEXT_LENGTH = 500;

/** What a new entry says and where it comes from. */
export type NewEntry = Omit<MemoryEntry, 'id' | 'recordedAt' | 'replacedBy'>;

export class PretendedMemory {
  private entries: { projectId: Id; entry: MemoryEntry }[];

  /** Takes in the entries a workspace starts with. */
  constructor(entries: readonly { projectId: Id; entry: MemoryEntry }[] = []) {
    this.entries = structuredClone([...entries]);
  }

  /** Every entry of a project, current and replaced, the newest first. */
  of(projectId: Id): MemoryEntry[] {
    return structuredClone(
      this.entries
        .filter((kept) => kept.projectId === projectId)
        .map((kept) => kept.entry)
        // Of entries recorded at one moment, the one recorded last comes first.
        .reverse()
        .sort(newestFirst),
    );
  }

  /** The current entries of a project that fit the words, the newest first. */
  search(projectId: Id, query: string): MemoryEntry[] {
    const current = this.of(projectId).filter((entry) => entry.replacedBy === null);
    if (query.trim() === '') {
      return current;
    }
    const words = searchWords(query);
    return current.filter((entry) => fits(words, entry.title, entry.text));
  }

  find(entryId: Id): { projectId: Id; entry: MemoryEntry } | undefined {
    return this.entries.find((kept) => kept.entry.id === entryId);
  }

  /** Records a new entry, checked by the rules of an entry. */
  record(projectId: Id, entry: NewEntry, id: Id, at: Moment): MemoryEntry {
    const recorded: MemoryEntry = {
      ...entry,
      ...checkedContent(entry.title, entry.text),
      id,
      recordedAt: at,
      replacedBy: null,
    };
    this.entries.push({ projectId, entry: recorded });
    return structuredClone(recorded);
  }

  /**
   * Writes the manager's words as a newer entry that replaces an entry, and
   * marks the older one as replaced.
   */
  replace(entryId: Id, title: string, text: string, id: Id, at: Moment): MemoryEntry {
    const older = this.find(entryId);
    if (!older) {
      throw new BackendError('not_found', 'there is no such memory entry');
    }
    if (older.entry.replacedBy !== null) {
      throw new BackendError(
        'rule_broken',
        'this memory entry was replaced by a newer one already',
      );
    }
    const newer = this.record(
      older.projectId,
      {
        kind: older.entry.kind,
        title,
        text,
        source: 'manager',
        memberId: null,
        conversationId: null,
        turnId: null,
        taskId: older.entry.taskId,
      },
      id,
      at,
    );
    older.entry.replacedBy = newer.id;
    return newer;
  }

  /**
   * Records the contract of a handoff as the contract of its task, and
   * marks the earlier contracts of the task as replaced by it.
   */
  recordContract(projectId: Id, entry: NewEntry, id: Id, at: Moment): void {
    const earlier = this.entries.filter(
      (kept) =>
        kept.projectId === projectId &&
        kept.entry.kind === 'contract' &&
        kept.entry.taskId === entry.taskId &&
        kept.entry.replacedBy === null,
    );
    const newer = this.record(projectId, entry, id, at);
    for (const kept of earlier) {
      kept.entry.replacedBy = newer.id;
    }
  }

  /** How many facts a turn has remembered. */
  factsOfTurn(turnId: Id): number {
    return this.entries.filter(
      (kept) => kept.entry.source === 'specialist' && kept.entry.turnId === turnId,
    ).length;
  }

  remove(entryId: Id): void {
    if (!this.find(entryId)) {
      throw new BackendError('not_found', 'there is no such memory entry');
    }
    this.entries = this.entries.filter((kept) => kept.entry.id !== entryId);
  }

  /** Forgets the memory of a project that is deleted. Entries outlive their tasks. */
  forgetProject(projectId: Id): void {
    this.entries = this.entries.filter((kept) => kept.projectId !== projectId);
  }
}

function newestFirst(first: MemoryEntry, second: MemoryEntry): number {
  return second.recordedAt.localeCompare(first.recordedAt);
}

/** The words of a text, in small letters: runs of letters and digits. */
function wordsOf(text: string): string[] {
  return text
    .toLowerCase()
    .split(/[^\p{L}\p{N}]+/u)
    .filter((word) => word !== '');
}

/** The words of a search, each once, refused when there are none or too many. */
export function searchWords(query: string): string[] {
  if (query.trim().length > MAX_SEARCH_LENGTH) {
    throw new BackendError(
      'rule_broken',
      `a search has at most ${MAX_SEARCH_LENGTH.toString()} characters`,
    );
  }
  const words = [...new Set(wordsOf(query))];
  if (words.length === 0) {
    throw new BackendError('rule_broken', 'the search must not be empty');
  }
  if (words.length > MOST_SEARCH_WORDS) {
    throw new BackendError(
      'rule_broken',
      `a search has at most ${MOST_SEARCH_WORDS.toString()} words, but this one has ${words.length.toString()}`,
    );
  }
  return words;
}

/** Whether every word of a search begins a word of the title or the text. */
export function fits(words: readonly string[], title: string, text: string): boolean {
  const found = [...wordsOf(title), ...wordsOf(text)];
  return words.every((wanted) => found.some((word) => word.startsWith(wanted)));
}

/** A title and a text checked as the domain checks them. */
function checkedContent(title: string, text: string): { title: string; text: string } {
  const tidyTitle = title.trim();
  const tidyText = text.trim();
  if (tidyTitle === '') {
    throw new BackendError('rule_broken', 'the title of a memory entry must not be empty');
  }
  if (/[\r\n]/.test(tidyTitle)) {
    throw new BackendError('rule_broken', 'the title of a memory entry is one line');
  }
  if ([...tidyTitle].length > MAX_TITLE_LENGTH) {
    throw new BackendError(
      'rule_broken',
      `the title of a memory entry has at most ${MAX_TITLE_LENGTH.toString()} characters`,
    );
  }
  if (tidyText === '') {
    throw new BackendError('rule_broken', 'the text of a memory entry must not be empty');
  }
  if ([...tidyText].length > MAX_TEXT_LENGTH) {
    throw new BackendError('rule_broken', 'the text of a memory entry is too long');
  }
  return { title: tidyTitle, text: tidyText };
}

/** A text of at most `max` characters, ended with an ellipsis if it was cut. */
export function shorten(text: string, max: number): string {
  const characters = [...text];
  if (characters.length <= max) {
    return text;
  }
  return `${characters
    .slice(0, max - 1)
    .join('')
    .trimEnd()}…`;
}

const KIND_IN_WORDS: Record<MemoryKind, string> = {
  decision: 'Decision',
  fact: 'Fact',
  contract: 'Contract',
};

/**
 * What the memory says to a specialist starting on a task, as a part of its
 * brief: the demo's copy of `memory_in_words` in the application crate.
 */
export function memoryInWords(
  project: Project,
  task: Task,
  blockers: readonly Task[],
  entries: readonly MemoryEntry[],
  tasks: readonly Task[],
): string | undefined {
  const current = entries.filter((entry) => entry.replacedBy === null).sort(newestFirst);
  if (current.length === 0) {
    return undefined;
  }
  const ofBlocker = (entry: MemoryEntry) => blockers.some((blocker) => blocker.id === entry.taskId);
  const handedOverAlready = (entry: MemoryEntry) => ofBlocker(entry) && entry.source === 'handoff';
  const groups: ((entry: MemoryEntry) => boolean)[] = [
    (entry) => entry.taskId === task.id,
    (entry) => ofBlocker(entry) && !handedOverAlready(entry),
    (entry) => entry.kind === 'contract' && !handedOverAlready(entry),
    (entry) => entry.taskId === null,
  ];
  const chosen = new Set<Id>();
  const parts: string[] = [];
  let used = 0;
  let isFull = false;
  for (const belongs of groups) {
    for (const entry of current.filter(belongs)) {
      if (isFull || chosen.has(entry.id)) {
        continue;
      }
      if (chosen.size >= MOST_IN_BRIEF) {
        isFull = true;
        continue;
      }
      const part = entryInWords(project, entry, tasks);
      const length = [...part].length;
      if (used + length > MEMORY_IN_BRIEF_LENGTH) {
        isFull = true;
        continue;
      }
      used += length;
      chosen.add(entry.id);
      parts.push(part);
    }
  }
  const left = current.length - chosen.size;
  if (parts.length === 0) {
    return `The project keeps a memory of decisions, facts and contracts (${current.length.toString()} ${
      current.length === 1 ? 'entry' : 'entries'
    }). Search it with the tool search_memory before you ask the manager something that may have been decided already.`;
  }
  const paragraphs = [
    'From the memory of the project, what bears on this task. Search the memory with the tool search_memory before you ask the manager something that may have been decided already.',
    ...parts,
  ];
  if (left > 0) {
    paragraphs.push(
      `${left.toString()} more ${left === 1 ? 'entry is' : 'entries are'} in the memory; search for ${
        left === 1 ? 'it' : 'them'
      }.`,
    );
  }
  return paragraphs.join('\n\n');
}

function entryInWords(project: Project, entry: MemoryEntry, tasks: readonly Task[]): string {
  const about: string[] = [];
  const task = tasks.find((other) => other.id === entry.taskId);
  if (task) {
    about.push(`${project.key}-${task.number.toString()}: ${task.title}`);
  }
  about.push(entry.recordedAt.slice(0, 10));
  return `${KIND_IN_WORDS[entry.kind]}: ${entry.title} (${about.join(', ')})\n${shorten(
    entry.text,
    BRIEF_TEXT_LENGTH,
  )}`;
}
