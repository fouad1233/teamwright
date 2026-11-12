import type { Effort, Id, Member, MemberKind, Task } from '@/shared/api/types';

const KIND_LABELS: Record<MemberKind, string> = {
  agent: 'AI specialist',
  human: 'Person',
};

/** What a kind of member is called for people. */
export function kindLabel(kind: MemberKind): string {
  return KIND_LABELS[kind];
}

/**
 * The models a specialist can be given by a short name, the most capable
 * first. A short name stands for the latest model of its family, so a
 * specialist follows the family when a newer model comes out.
 */
export const MODEL_NAMES: readonly string[] = ['fable', 'opus', 'sonnet', 'haiku'];

/** The efforts a specialist can be given, the least first. */
export const EFFORTS: readonly Effort[] = ['low', 'medium', 'high', 'xhigh', 'max'];

const EFFORT_LABELS: Record<Effort, string> = {
  low: 'Low',
  medium: 'Medium',
  high: 'High',
  xhigh: 'Extra high',
  max: 'Maximum',
};

/** What an effort is called for people. */
export function effortLabel(effort: Effort): string {
  return EFFORT_LABELS[effort];
}

/**
 * What a model is called for people. A short name gets a capital, as the
 * families are written. Any other name is the full name of a model and is
 * shown as it is.
 */
export function modelLabel(model: string): string {
  return MODEL_NAMES.includes(model) ? model.charAt(0).toUpperCase() + model.slice(1) : model;
}

/**
 * How a specialist runs, in a few words: "Sonnet, high effort". Nothing, if
 * both are left to the program that runs the specialists.
 */
export function profileInWords(member: Pick<Member, 'model' | 'effort'>): string | null {
  const parts = [
    member.model === null ? null : modelLabel(member.model),
    member.effort === null ? null : `${effortLabel(member.effort).toLowerCase()} effort`,
  ].filter((part) => part !== null);
  return parts.length === 0 ? null : parts.join(', ');
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
