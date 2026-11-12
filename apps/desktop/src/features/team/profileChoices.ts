import type { Choice } from '@/shared/ui/Menu';

import { EFFORTS, MODEL_NAMES, effortLabel, modelLabel } from './teamModel';

/**
 * The value that stands, in a menu, for leaving a choice to the program
 * that runs the specialists. A menu chooses among texts, and "nothing" is
 * not one.
 */
export const LEFT_TO_THE_PROGRAM = 'default';

/** The efforts a menu offers: the default, and the known ones, the least first. */
export const EFFORT_CHOICES: Choice<string>[] = [
  { value: LEFT_TO_THE_PROGRAM, label: 'The default' },
  ...EFFORTS.map((effort) => ({ value: effort, label: effortLabel(effort) })),
];

/**
 * The models the menu offers: the default, the short names, and the model
 * that is chosen now if it is none of those.
 */
export function modelChoices(current: string | null): Choice<string>[] {
  const names =
    current === null || MODEL_NAMES.includes(current) ? MODEL_NAMES : [...MODEL_NAMES, current];
  return [
    { value: LEFT_TO_THE_PROGRAM, label: 'The default of Claude Code' },
    ...names.map((name) => ({ value: name, label: modelLabel(name) })),
  ];
}
