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
    case 'web_domain':
      return `Fetch web pages from ${what}`;
  }
}

/** What a kind of permission is, in a few words, for the label above it. */
export function kindInWords(kind: PermissionKind): string {
  switch (kind) {
    case 'command':
      return 'Command';
    case 'web_domain':
      return 'Web domain';
  }
}

/** Whether a kind of permission lets commands run on this computer. */
export function runsCommands(kind: PermissionKind): boolean {
  return kind === 'command';
}

/**
 * What the manager reads before allowing a command: what it really means,
 * and what is meant to hold it in. The sandbox is the one every turn with a
 * command is started in: it is to let the command change files only in the
 * folder of the project, keep it out of the rest of the home folder (Claude
 * Code opens the git settings again), and let it reach only the web domains
 * the manager allowed. It has not been seen working in this version, and
 * the card says so. With the shell there, Claude Code also runs its own
 * read-only commands without asking (decision 0038, note of 22 November
 * 2026).
 */
export const WHAT_A_COMMAND_MEANS = [
  'This specialist can change the files this command runs (scripts, build files, tests). Allowing it lets the specialist run any program as you.',
  'It runs in a sandbox meant to let it change files only in the folder of the project, keep it out of the rest of your home folder except your git settings, and reach no web domain you have not allowed. That sandbox has not been proven in this version of Teamwright.',
  'With any allowed command the specialist can also read, list and search files inside the folders of the project with the shell.',
] as const;

/** One of the answers to a request: allowed for a while, or refused. */
export type PermissionChoice = PermissionScope | 'refuse';

/**
 * The answers the manager can give, in order, as options with what each
 * leads to. Allowing for the task is offered only when the conversation is
 * about one; allowing always only for a web domain, as a command is never
 * allowed always.
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
        consequence: runsCommands(request.kind)
          ? `For every turn of ${specialist} about this task that you start yourself: Start work, a reply, Try again. Never in a turn the coordinator starts on its own.`
          : `For every turn of ${specialist} about this task.`,
      },
    });
  }
  if (!runsCommands(request.kind)) {
    choices.push({
      choice: 'always',
      option: {
        label: `Always allow ${specialist}`,
        consequence:
          'For every turn in this project, on this computer, until you take it back in Team.',
      },
    });
  }
  choices.push({
    choice: 'refuse',
    option: {
      label: 'Refuse',
      consequence: `${specialist} goes on without it, and says what could not be done.`,
    },
  });
  return choices;
}

/**
 * The permissions that stand for a specialist: allowed for a task or for
 * always, and not taken back. They are what its later turns are started
 * with, on the computer they were allowed on. One that is no longer valid
 * is among them, so that it can be taken back.
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
