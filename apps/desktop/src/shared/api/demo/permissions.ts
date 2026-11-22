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

/**
 * Characters a command may not contain: they would join, redirect, quote,
 * escape or widen it, so that the shell would read another command than the
 * one checked.
 */
const FORBIDDEN_IN_COMMANDS = /[;&|<>`$*?(){}[\]\\~!#%^"']/;

/** Programs that are never allowed, anywhere in a command. */
const NEVER_PROGRAMS = new Set([
  'claude',
  'teamwright',
  'sudo',
  'su',
  'doas',
  'pkexec',
  'runas',
  'gsudo',
]);

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
  'chgrp',
  'dd',
  'tee',
  'truncate',
  'shred',
  'install',
  'sed',
]);

/**
 * Programs and words of the shell that only start another, or run a file in
 * the shell itself: a command never starts with one.
 */
const WRAPPERS = new Set([
  'env',
  'xargs',
  'eval',
  'exec',
  'nohup',
  'timeout',
  'time',
  'nice',
  'ionice',
  'watch',
  'stdbuf',
  'setsid',
  'command',
  'builtin',
  'noglob',
  'nocorrect',
  'flock',
  'busybox',
  'toybox',
  'chrt',
  'taskset',
  'setpriv',
  'unshare',
  'sg',
  'script',
  '.',
  'source',
  'strace',
  'ltrace',
  'nsenter',
  'chroot',
  'prlimit',
  'numactl',
  'setarch',
  'runuser',
  'systemd-run',
  'fakeroot',
  'faketime',
  'unbuffer',
  'rlwrap',
  'caffeinate',
]);

/** The name of a program as it is compared. */
function programName(word: string): string {
  const name = (word.split('/').at(-1) ?? word).toLowerCase();
  return name.endsWith('.exe') ? name.slice(0, -4) : name;
}

/** Whether a word names a path outside the folder of the project. */
function leavesTheFolder(word: string): boolean {
  // A slash only right after a letter, a digit or . _ - of a name.
  for (let place = 0; place < word.length; place += 1) {
    if (word[place] === '/' && (place === 0 || !/[A-Za-z0-9._-]/.test(word[place - 1] ?? ''))) {
      return true;
    }
  }
  // An option glued to a path: -o/etc/x.
  const beforeSlash = word.split('/')[0] ?? '';
  if (word.includes('/') && beforeSlash.startsWith('-') && /^[A-Za-z-]+$/.test(beforeSlash)) {
    return true;
  }
  // A drive, at the start, after = or a comma, or glued to a one-letter option.
  if (/(^|[=,])[A-Za-z]:/.test(word) || /^-[A-Za-z][A-Za-z]:/.test(word)) {
    return true;
  }
  return word.includes(':/');
}

/**
 * Checks a rule as the manager would allow it, and returns its text
 * tidied: single spaces in a command, small letters in a domain. The same
 * rules as the application's, in the same words, though the application is
 * the judge.
 */
export function checkedRule(kind: PermissionKind, what: string): string {
  const text = what.replace(/^ +| +$/g, '');
  if (text === '') {
    throw new BackendError('rule_broken', 'the permission must not be empty');
  }
  const notPossible = (reason: string) =>
    new BackendError('rule_broken', `this permission cannot be given: ${reason}`);
  if ((kind as string) !== 'command' && kind !== 'web_domain') {
    throw new BackendError('invalid_request', `"${kind}" is not a kind of permission`);
  }
  if (kind === 'web_domain') {
    const domain = text.toLowerCase();
    if (domain.includes('*')) {
      throw notPossible(
        'a permission names one host, such as docs.rs or api.example.com, not all the hosts under a name; ask for each host on its own',
      );
    }
    const labels = domain.split('.');
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
  if (/[^\x20-\x7e]/.test(text)) {
    throw notPossible(
      'a command is one line of plain keyboard characters, with spaces between its words',
    );
  }
  if (FORBIDDEN_IN_COMMANDS.test(text)) {
    throw notPossible(
      'a command may not quote, join, redirect or widen commands, so that what runs is not what you read',
    );
  }
  const words = text.split(' ').filter((word) => word !== '');
  const first = words[0] ?? '';
  if (first.includes('=')) {
    throw notPossible('a command may not start by setting a variable');
  }
  if (words.some((word) => word.startsWith('='))) {
    throw notPossible(
      'a word of a command may not start with =, which some shells turn into the full path of a program',
    );
  }
  if (words.some((word) => word.includes('..'))) {
    throw notPossible('a command may not contain .., which reaches into a folder above');
  }
  if (words.some(leavesTheFolder)) {
    throw notPossible(
      'a command may name only paths inside the folder of the project: no absolute path, no drive and no home folder, also not after = or an option',
    );
  }
  const lower = text.toLowerCase();
  if (lower.includes('.claude') || lower.includes('teamwright')) {
    throw notPossible(
      "a command may not touch the settings of the agent program or Teamwright's own data",
    );
  }
  const program = programName(first);
  if (WRAPPERS.has(program)) {
    throw notPossible(`a command may not start with ${program}, which only starts another program`);
  }
  if (FILE_CHANGING_PROGRAMS.has(program)) {
    throw notPossible(`${program} changes files; a specialist changes files with its file tools`);
  }
  const never = words
    .flatMap((word) => word.split(/[=,:]/))
    .map(programName)
    .find((name) => NEVER_PROGRAMS.has(name));
  if (never !== undefined) {
    throw notPossible(`${never} can never be allowed`);
  }
  return words.join(' ');
}

/** What a rule allows, in words: "run the command `npm test`". */
export function permissionInWords(kind: PermissionKind, what: string): string {
  switch (kind) {
    case 'command':
      return `run the command \`${what}\``;
    case 'web_domain':
      return `fetch web pages from ${what}`;
  }
}

/**
 * Checks how long a rule is to hold: a command once or for a task, never
 * always.
 */
export function checkedScope(kind: PermissionKind, scope: PermissionScope): void {
  if (kind === 'command' && scope === 'always') {
    throw new BackendError(
      'rule_broken',
      'this permission cannot be given: a command can be allowed once or for this task, never always',
    );
  }
}

/** Why a specialist can be given no command, in the application's words. */
export const SECRET_IN_REACH =
  'this permission cannot be given: this specialist has a connector that is given a secret in an environment variable, and a command could read it; take that connector from the specialist first';

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
            noLongerValid: false,
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
            noLongerValid: false,
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
    answer.noLongerValid = false;
    return structuredClone(request);
  }
}
