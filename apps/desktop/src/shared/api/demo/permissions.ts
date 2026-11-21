import { BackendError } from '../backend';
import type {
  Id,
  Moment,
  NewPermissionAnswer,
  PermissionKind,
  PermissionRequest,
  PermissionScope,
} from '../types';

/**
 * The permission requests of the demo (decision 0038).
 *
 * The demo runs nothing, so a permission allows nothing here. It keeps the
 * requests and the answers as the application does, so that the inbox, the
 * conversation and the team show them as they will be. The rules a
 * permission must keep are checked with the same words as the application
 * says them, though less thoroughly: the application is the judge.
 */

/** Characters a command may not contain: they would join, redirect or widen it. */
const FORBIDDEN_IN_COMMANDS = /[;&|<>`$*?(){}[\]\\~!#%^\n\r]/;

/** Programs that are never allowed. */
const NEVER_PROGRAMS = new Set(['claude', 'teamwright', 'sudo', 'su', 'doas', 'pkexec', 'runas']);

/** Programs that change files, which a specialist does with its file tools. */
const FILE_CHANGING_PROGRAMS = new Set([
  'rm',
  'rmdir',
  'mv',
  'cp',
  'mkdir',
  'touch',
  'ln',
  'chmod',
  'chown',
  'dd',
  'tee',
]);

/** Programs that run whatever follows them: never a prefix. */
const RUNS_ANYTHING = new Set([
  'sh',
  'bash',
  'zsh',
  'pwsh',
  'powershell',
  'cmd',
  'python',
  'python3',
  'node',
  'env',
  'xargs',
  'npx',
  'curl',
  'wget',
  'ssh',
  'find',
]);

/**
 * Checks a rule as the manager would allow it, and returns its text
 * tidied: single spaces in a command, small letters in a domain.
 */
export function checkedRule(kind: PermissionKind, what: string): string {
  const text = what.trim();
  if (text === '') {
    throw new BackendError('rule_broken', 'the permission must not be empty');
  }
  const notPossible = (reason: string) =>
    new BackendError('rule_broken', `this permission cannot be given: ${reason}`);
  if (kind === 'web_domain') {
    const domain = text.toLowerCase();
    const host = domain.startsWith('*.') ? domain.slice(2) : domain;
    const labels = host.split('.');
    if (!labels.every((label) => /^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/.test(label))) {
      throw notPossible('a web domain is a host name such as docs.rs, without "https://"');
    }
    if (labels.length < 2) {
      throw notPossible('a web domain has at least two parts, such as docs.rs');
    }
    const last = labels.at(-1) ?? '';
    if (/^\d+$/.test(last) || last === 'localhost' || last === 'local') {
      throw notPossible('an address of this computer or of a local network can never be allowed');
    }
    return domain;
  }
  if (FORBIDDEN_IN_COMMANDS.test(text)) {
    throw notPossible('a command may not join, redirect or widen commands');
  }
  const words = text.split(/\s+/);
  const lower = text.toLowerCase();
  if (words.some((word) => word.startsWith('/') || word.split('/').includes('..'))) {
    throw notPossible('a command may name only paths inside the folder of the project');
  }
  if (lower.includes('.claude') || lower.includes('teamwright')) {
    throw notPossible(
      "a command may not touch the settings of the agent program or Teamwright's own data",
    );
  }
  const program = (words[0] ?? '').toLowerCase();
  if (NEVER_PROGRAMS.has(program)) {
    throw notPossible(`"${program}" can never be allowed`);
  }
  if (FILE_CHANGING_PROGRAMS.has(program)) {
    throw notPossible(`"${program}" changes files; a specialist changes files with its file tools`);
  }
  if (kind === 'command_prefix') {
    if (words.length < 2) {
      throw notPossible('a prefix names the program and what it is asked to do, such as "npm run"');
    }
    if (RUNS_ANYTHING.has(program)) {
      throw notPossible(`"${program}" runs whatever follows it, so it is allowed only exactly`);
    }
  }
  return words.join(' ');
}

/** What a rule allows, in words: "run the command `npm test`". */
export function permissionInWords(kind: PermissionKind, what: string): string {
  switch (kind) {
    case 'command':
      return `run the command \`${what}\``;
    case 'command_prefix':
      return `run commands that start with \`${what}\``;
    case 'web_domain':
      return `fetch web pages from ${what}`;
  }
}

/** The answer as the specialist reads it, the request of its next turn. */
export function permissionAnswerInWords(
  request: PermissionRequest,
  answer: NewPermissionAnswer,
): string {
  const note = answer.note.trim();
  let said: string;
  if (answer.decision === 'allow') {
    const howLong: Record<PermissionScope, string> = {
      once: 'for this turn only',
      task: 'for every turn of yours on this task',
      always: 'for every turn of yours in this project, until the manager takes it back',
    };
    said = `Permission: the manager allowed you to ${permissionInWords(answer.kind, answer.what.trim())}, ${howLong[answer.scope]}. It is available now. Go on with your work.`;
  } else {
    said = `Permission: the manager refused your request to ${permissionInWords(request.kind, request.what)}. Do not try it another way. Go on without it, and say in your report what could not be done.`;
  }
  return note === '' ? said : `${said}\n\nThe manager adds: ${note}`;
}

/** The requests of the demo, with their answers. */
export class PretendedPermissions {
  private readonly requests = new Map<Id, PermissionRequest>();

  constructor(seeded: readonly PermissionRequest[] = []) {
    for (const request of seeded) {
      this.requests.set(request.id, structuredClone(request));
    }
  }

  /** Records a request a simulated specialist made. */
  record(request: PermissionRequest): PermissionRequest {
    this.requests.set(request.id, structuredClone(request));
    return structuredClone(request);
  }

  /** The requests of a project, the oldest first. */
  of(projectId: Id): PermissionRequest[] {
    return [...this.requests.values()]
      .filter((request) => request.projectId === projectId)
      .sort((first, second) => first.askedAt.localeCompare(second.askedAt))
      .map((request) => structuredClone(request));
  }

  /** The open requests of these projects, the oldest first. */
  openIn(projectIds: ReadonlySet<Id>): PermissionRequest[] {
    return [...this.requests.values()]
      .filter((request) => request.answer === null && projectIds.has(request.projectId))
      .sort((first, second) => first.askedAt.localeCompare(second.askedAt))
      .map((request) => structuredClone(request));
  }

  /** The request, if it waits for an answer. */
  requireOpen(id: Id): PermissionRequest {
    const request = this.requests.get(id);
    if (!request) {
      throw new BackendError('not_found', 'there is no such permission request');
    }
    if (request.answer !== null) {
      throw new BackendError('rule_broken', 'the request has been answered already');
    }
    return request;
  }

  /** Records the manager's answer to an open request. */
  answer(id: Id, answer: NewPermissionAnswer, at: Moment): PermissionRequest {
    const request = this.requireOpen(id);
    request.answer =
      answer.decision === 'allow'
        ? {
            decision: 'allowed',
            scope: answer.scope,
            kind: answer.kind,
            what: checkedRule(answer.kind, answer.what),
            note: answer.note.trim(),
            answeredAt: at,
            onThisComputer: true,
            withdrawnAt: null,
          }
        : {
            decision: 'refused',
            scope: null,
            kind: null,
            what: null,
            note: answer.note.trim(),
            answeredAt: at,
            onThisComputer: true,
            withdrawnAt: null,
          };
    return structuredClone(request);
  }

  /** Takes back a permission allowed for a task or for always. */
  withdraw(id: Id, at: Moment): PermissionRequest {
    const request = this.requests.get(id);
    if (!request) {
      throw new BackendError('not_found', 'there is no such permission request');
    }
    const answer = request.answer;
    if (answer?.decision !== 'allowed' || answer.scope === 'once' || answer.withdrawnAt !== null) {
      throw new BackendError('rule_broken', 'there is no allowed permission to take back');
    }
    answer.withdrawnAt = at;
    return structuredClone(request);
  }
}
