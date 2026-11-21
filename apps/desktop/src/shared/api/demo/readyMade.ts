import { BackendError } from '../backend';
import copy from '../readyMade.json';
import type { Effort, MemberKind, ReadyMadeSpecialist } from '../types';

/*
 * The ready-made specialists of the demo, and the rules about the
 * instructions of a member, as the program has them (decision 0031). The
 * set is the program's own: `readyMade.json` is a copy that a test of the
 * desktop shell keeps equal to the set in `crates/domain`.
 */

/** The ready-made specialists, in the order the manager is shown them. */
export const READY_MADE_SPECIALISTS: readonly ReadyMadeSpecialist[] = copy.specialists.map(
  (specialist) => ({ ...specialist, effort: specialist.effort as Effort }),
);

/** The most characters the instructions of a member can have. */
export const MEMBER_INSTRUCTIONS_LIMIT: number = copy.memberInstructionsLimit;

/** Finds a ready-made specialist by its key, or refuses with `not_found`. */
export function requireReadyMade(key: string): ReadyMadeSpecialist {
  const found = READY_MADE_SPECIALISTS.find((specialist) => specialist.key === key);
  if (found === undefined) {
    throw new BackendError('not_found', `there is no ready-made specialist ${key}`);
  }
  return found;
}

/**
 * The ready-made specialist a member's name stands for: one whose name is
 * the same, with capital and small letters and the space at both ends
 * counting for nothing.
 */
export function readyMadeNamed(name: string): ReadyMadeSpecialist | undefined {
  const wanted = name.trim().toLowerCase();
  return READY_MADE_SPECIALISTS.find((specialist) => specialist.name.toLowerCase() === wanted);
}

/**
 * The first name of the form "Name", "Name 2", "Name 3" and so on that
 * nobody on the team has, capital and small letters counting as the same.
 */
export function freeMemberName(namesOnTeam: readonly string[], name: string): string {
  const taken = new Set(namesOnTeam.map((other) => other.toLowerCase()));
  if (!taken.has(name.toLowerCase())) {
    return name;
  }
  let number = 2;
  while (taken.has(`${name} ${number.toString()}`.toLowerCase())) {
    number += 1;
  }
  return `${name} ${number.toString()}`;
}

/**
 * Checks and tidies the instructions a member is to get: every line break
 * becomes `\n`, the space at both ends goes, and the text must not be
 * longer than the limit. A person can only be given none.
 */
export function tidyInstructions(kind: MemberKind, name: string, raw: string): string {
  const instructions = raw.replace(/\r\n?/g, '\n').trim();
  const length = [...instructions].length;
  if (length > MEMBER_INSTRUCTIONS_LIMIT) {
    throw new BackendError(
      'rule_broken',
      `the instructions of a specialist can have at most ${MEMBER_INSTRUCTIONS_LIMIT.toString()} characters, but this one has ${length.toString()}`,
    );
  }
  if (kind !== 'agent' && instructions !== '') {
    throw new BackendError('no_specialist', `${name} is not an AI specialist`);
  }
  return instructions;
}
