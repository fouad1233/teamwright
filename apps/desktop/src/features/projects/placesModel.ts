/**
 * Says which files were copied for the specialists, by the names they have
 * now: "Copied sheet.pdf", "Copied sheet.pdf and notes.txt", "Copied 5
 * files".
 */
export function copiedInWords(names: readonly string[]): string {
  const [first, second] = names;
  if (first === undefined) {
    return 'Nothing was copied';
  }
  if (second === undefined) {
    return `Copied ${first}`;
  }
  return names.length === 2
    ? `Copied ${first} and ${second}`
    : `Copied ${names.length.toString()} files`;
}

/**
 * What full access gives and what it never gives, in the plain sentences
 * the folders dialog shows under the switch and asks the manager to confirm
 * (decision 0044).
 */
export const FULL_ACCESS_WORDS: readonly string[] = [
  'The specialists of this project may then read and change files anywhere your account can, not only in the folders above. A specialist can change or delete any of your files.',
  'Teamwright’s own data, the sync folder, the settings of Claude Code and your SSH and GnuPG keys stay closed, for reading too, and no command can be allowed.',
  'This choice is kept for this computer only. Your other computers keep their own.',
];

/**
 * What the folders dialog says about the coordinator while full access is
 * on, or nothing: the turns the coordinator starts by itself on this
 * computer have full access too, as every turn of the project here has.
 */
export function coordinatorWithFullAccess(
  fullAccess: boolean,
  coordinatorRunsHere: boolean,
): string | null {
  if (!fullAccess || !coordinatorRunsHere) {
    return null;
  }
  return 'The coordinator is on: the work it starts by itself has full access too.';
}

/** What the coordinator says while the project has full access on this computer. */
export const FULL_ACCESS_FOR_THE_COORDINATOR =
  'Full access is on in this project: the work the coordinator starts here has it too, with nobody watching.';
