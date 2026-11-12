import type { Id, Member, MemberKind, Task } from '@/shared/api/types';

const KIND_LABELS: Record<MemberKind, string> = {
  agent: 'AI specialist',
  human: 'Person',
};

/** What a kind of member is called for people. */
export function kindLabel(kind: MemberKind): string {
  return KIND_LABELS[kind];
}

/** The members of a team by their identifier. */
export function indexMembers(team: readonly Member[]): Map<Id, Member> {
  return new Map(team.map((member) => [member.id, member]));
}

/**
 * The two letters that stand for a member where there is no room for the
 * name: the first letters of its first two words, or the first two letters
 * of a name that is one word.
 */
export function initialsOf(name: string): string {
  const words = name.split(/\s+/).filter((word) => word !== '');
  const [first = '', second] = words;
  // Spreading a text gives its characters, also those that take two code units.
  const letters = second === undefined ? [...first].slice(0, 2) : [[...first][0], [...second][0]];
  return letters.join('').toUpperCase();
}

/** The tasks that are given to one member of the team. */
export function tasksOfMember(tasks: readonly Task[], memberId: Id): Task[] {
  return tasks.filter(
    (task) => task.assignee?.kind === 'member' && task.assignee.memberId === memberId,
  );
}

/** The tasks that the manager keeps. */
export function tasksOfManager(tasks: readonly Task[]): Task[] {
  return tasks.filter((task) => task.assignee?.kind === 'manager');
}
