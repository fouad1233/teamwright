import type {
  DecisionOption,
  Id,
  PermissionKind,
  PermissionRequest,
  PermissionScope,
} from '@/shared/api/types';

/**
 * How the interface speaks of permissions (decision 0038). What may be
 * allowed is decided by the application; this only puts it in words and
 * finds what stands.
 */

/** What a permission allows, in words: "run the command `npm test`". */
export function permissionInWords(kind: PermissionKind, what: string): string {
  switch (kind) {
    case 'command':
      return `Run the command ${what}`;
    case 'command_prefix':
      return `Run commands that start with ${what}`;
    case 'web_domain':
      return `Fetch web pages from ${what}`;
  }
}

/** What a kind of permission is, in a few words, for the label above it. */
export function kindInWords(kind: PermissionKind): string {
  switch (kind) {
    case 'command':
      return 'Command';
    case 'command_prefix':
      return 'Commands that start with';
    case 'web_domain':
      return 'Web domain';
  }
}

/** Whether a kind of permission lets commands run on this computer. */
export function runsCommands(kind: PermissionKind): boolean {
  return kind === 'command' || kind === 'command_prefix';
}

/**
 * What the manager reads before allowing a command: one plain sentence
 * about what it means.
 */
export const COMMANDS_RUN_AS_YOU =
  'A command runs on this computer with your rights, like a command you type yourself. Allow only what you would run.';

/**
 * The first two words of a command: the program and what it is asked to do,
 * which is the narrowest prefix that names a family of commands. Nothing
 * for a command of one word.
 */
export function prefixOf(command: string): string | null {
  const words = command.trim().split(/\s+/);
  return words.length >= 2 ? words.slice(0, 2).join(' ') : null;
}

/** One of the answers to a request: allowed for a while, or refused. */
export type PermissionChoice = PermissionScope | 'refuse';

/**
 * The answers the manager can give, in order, as options with what each
 * leads to. Allowing for the task is offered only when the conversation is
 * about one.
 */
export function choicesFor(
  request: PermissionRequest,
  specialist: string,
): { choice: PermissionChoice; option: DecisionOption }[] {
  const choices: { choice: PermissionChoice; option: DecisionOption }[] = [
    {
      choice: 'once',
      option: {
        label: 'Allow once',
        consequence: 'For the next turn only. Asked for again, it comes back here.',
      },
    },
  ];
  if (request.taskId !== null) {
    choices.push({
      choice: 'task',
      option: {
        label: 'Allow for this task',
        consequence: `For every turn of ${specialist} on this task.`,
      },
    });
  }
  choices.push(
    {
      choice: 'always',
      option: {
        label: `Always allow ${specialist}`,
        consequence:
          'For every turn in this project, on this computer, until you take it back in Team.',
      },
    },
    {
      choice: 'refuse',
      option: {
        label: 'Refuse',
        consequence: `${specialist} goes on without it, and says what could not be done.`,
      },
    },
  );
  return choices;
}

/**
 * The permissions that stand for a specialist: allowed for a task or for
 * always, and not taken back. They are what its later turns are started
 * with, on the computer they were allowed on.
 */
export function standingPermissionsOf(
  requests: readonly PermissionRequest[],
  memberId: Id,
): PermissionRequest[] {
  return requests.filter(
    (request) =>
      request.memberId === memberId &&
      request.answer?.decision === 'allowed' &&
      request.answer.scope !== 'once' &&
      request.answer.withdrawnAt === null,
  );
}

/** How long a standing permission holds, in words. */
export function scopeInWords(scope: PermissionScope | null): string {
  switch (scope) {
    case 'once':
      return 'Once';
    case 'task':
      return 'For one task';
    case 'always':
      return 'Always';
    case null:
      return '';
  }
}
