import { BackendError } from '../backend';
import type {
  ConnectorStart,
  Id,
  KitChoice,
  Library,
  LibraryConnector,
  LibrarySkill,
  Member,
  MemberKit,
  Moment,
  NewConnector,
} from '../types';

/**
 * The library of skills and connectors of the demo (decision 0039). The
 * rules are a short copy of those of the domain crate, enough for the demo
 * to refuse what the program refuses. The simulated specialist ignores
 * what it is given.
 *
 * Like the program, the demo keeps what the manager confirmed "on this
 * computer" (note of 22 November 2026): a connector added here, or a kit
 * set here, is confirmed; the sample library holds one connector and one
 * tool that pretend to have arrived from another computer and wait.
 */

/** The folders the demo pretends the manager can choose as skills. */
export const SAMPLE_SKILL_FOLDERS: Record<string, { name: string; description: string } | string> =
  {
    'C:\\Users\\you\\skills\\datasheet-reading': {
      name: 'datasheet-reading',
      description:
        'Reads a component data sheet and pulls out the limits, the pinout and the timing.',
    },
    'C:\\Users\\you\\skills\\release-notes': {
      name: 'release-notes',
      description: 'Writes release notes from a list of changes.',
    },
    'C:\\Users\\you\\skills\\deploy-helper':
      'it runs a command when it is used (with !` or ```!), and a specialist runs no commands',
  };

const NAME = /^[a-z0-9]+(-[a-z0-9]+)*$/;
const TOOL = /^[A-Za-z0-9_][A-Za-z0-9_-]{0,63}$/;
const VARIABLE = /^[A-Za-z_][A-Za-z0-9_]{0,63}$/;
const SECRET_WORDS = ['token', 'key', 'secret', 'password', 'auth', 'signature'];

function rule(message: string): never {
  throw new BackendError('rule_broken', message);
}

function checkVariable(name: string): void {
  if (!VARIABLE.test(name)) {
    rule(`"${name}" is not the name of an environment variable, such as PARTS_TOKEN`);
  }
  if (/^(claude|anthropic)/i.test(name)) {
    rule(`the variable ${name} belongs to Claude Code and is never handed to a connector`);
  }
}

/** Checks a connector as the domain does. Never repeats an address. */
export function checkConnector(connector: NewConnector): void {
  const name = connector.name.trim();
  if (!NAME.test(name) || name.length > 40) {
    rule(
      'the name of a connector is lower-case letters, digits and single hyphens, at most 40 characters, such as parts-catalog',
    );
  }
  if (['teamwright', 'workspace', 'claude-in-chrome', 'computer-use'].includes(name)) {
    rule(`the name "${name}" belongs to a tool server of Teamwright or Claude Code`);
  }
  const start = connector.start;
  if (start.kind === 'program') {
    if (start.program.trim() === '') rule('a connector started as a program names the program');
    if (!isBareNameOrFullPath(start.program.trim())) {
      rule(
        'the program of a connector is a name found on the search path, such as npx, or a full path, such as /usr/local/bin/parts-server or C:\\Tools\\parts-server.exe; a relative path such as ./server or bin/server is refused, because it could name a file in the folder of the project',
      );
    }
    if ([start.program, ...start.arguments].some((text) => text.includes('$'))) {
      rule(
        'the program and its arguments cannot contain $; name the environment variables the connector needs instead',
      );
    }
    start.variables.forEach(checkVariable);
  } else {
    const url = start.url.trim();
    const local = /^http:\/\/(localhost|127\.0\.0\.1|\[::1\])([:/?#]|$)/.test(url);
    if (!url.startsWith('https://') && !local) {
      rule('the address of a connector starts with https://');
    }
    if (/\s|\$/.test(url)) rule('the address of a connector is one line without spaces or $');
    const authority = url.replace(/^https?:\/\//, '').split(/[/?#]/)[0] ?? '';
    if (authority === '' || authority.includes('@')) {
      rule('the address of a connector names a host and carries no user or password');
    }
    const query = url.split('?')[1]?.split('#')[0] ?? '';
    for (const parameter of query.split('&')) {
      const key = (parameter.split('=')[0] ?? '').toLowerCase();
      if (SECRET_WORDS.some((word) => key.includes(word))) {
        rule(
          'the address of a connector cannot carry a secret in its parameters; a secret goes in a header, from an environment variable',
        );
      }
    }
    for (const header of start.headers) checkVariable(header.variable);
  }
  const tools = connector.tools.map((tool) => tool.trim()).filter((tool) => tool !== '');
  if (tools.length === 0) {
    rule('a connector lists at least one of its tools, so that a specialist can be given it');
  }
  for (const tool of tools) {
    if (!TOOL.test(tool)) rule(`"${tool}" is not the name of a tool`);
  }
  if (new Set(tools).size !== tools.length) rule('a tool is listed twice');
}

/** Whether a program is a bare name or a full path, as the domain asks. */
function isBareNameOrFullPath(program: string): boolean {
  if (/^(\/|\\\\|[A-Za-z]:[\\/])/.test(program)) return true;
  return program !== '.' && program !== '..' && !/[/\\:]/.test(program);
}

/**
 * The fingerprint of the whole definition of a connector. The program uses
 * the SHA-256 of a canonical form; the demo only needs a text that changes
 * whenever the definition does.
 */
export function fingerprintOf(connector: LibraryConnector): string {
  return JSON.stringify([connector.id, connector.name, connector.start, connector.tools]);
}

/** The key of a tool of a connector given to a specialist. */
function toolKey(connectorId: Id, tool: string): string {
  return `${connectorId} ${tool}`;
}

/** The variables a connector needs. */
function variablesOf(start: ConnectorStart): string[] {
  return start.kind === 'program'
    ? start.variables
    : start.headers.map((header) => header.variable);
}

/** The libraries and kits of the demo, per project. */
export class PretendedLibrary {
  private readonly skills = new Map<Id, LibrarySkill[]>();
  private readonly connectors = new Map<Id, LibraryConnector[]>();
  private readonly kits = new Map<Id, MemberKit>();
  /** The variables the browser pretends are set. */
  private readonly variables = new Set(['PARTS_TOKEN', 'DRAWINGS_TOKEN']);
  /** The fingerprint of each connector the manager confirmed "on this computer". */
  private readonly confirmedDefinitions = new Map<Id, string>();
  /** The tools the manager confirmed "on this computer" as given to each specialist. */
  private readonly confirmedTools = new Map<Id, Set<string>>();

  constructor(
    private readonly newId: () => Id,
    private readonly now: () => Moment,
  ) {}

  /** Gives a project the sample library, and its first specialist a part of it. */
  seed(projectId: Id, specialist: Member | undefined): void {
    const skill: LibrarySkill = {
      id: this.newId(),
      ...(SAMPLE_SKILL_FOLDERS['C:\\Users\\you\\skills\\datasheet-reading'] as {
        name: string;
        description: string;
      }),
      addedAt: this.now(),
      here: true,
      problem: null,
    };
    const parts: LibraryConnector = {
      id: this.newId(),
      name: 'parts-catalog',
      start: {
        kind: 'program',
        program: 'parts-catalog-server',
        arguments: ['--stdio'],
        variables: ['PARTS_TOKEN'],
      },
      tools: ['search_parts', 'get_part', 'place_order'],
      addedAt: this.now(),
      missingVariables: [],
      problem: null,
      fingerprint: '',
      confirmed: true,
    };
    const tickets: LibraryConnector = {
      id: this.newId(),
      name: 'tickets',
      start: {
        kind: 'address',
        url: 'https://tickets.example.com/mcp',
        headers: [{ name: 'Authorization', variable: 'TICKETS_TOKEN', bearer: true }],
      },
      tools: ['read_ticket', 'list_tickets', 'comment'],
      addedAt: this.now(),
      missingVariables: ['TICKETS_TOKEN'],
      problem: null,
      fingerprint: '',
      confirmed: true,
    };
    // Added on another computer and taken in through the sync folder: it
    // waits until the manager confirms it here.
    const drawings: LibraryConnector = {
      id: this.newId(),
      name: 'drawings',
      start: {
        kind: 'program',
        program: 'uvx',
        arguments: ['drawings-mcp', '--read-only', '--root', 'C:\\Drawings'],
        variables: ['DRAWINGS_TOKEN'],
      },
      tools: ['find_drawing', 'read_drawing'],
      addedAt: this.now(),
      missingVariables: [],
      problem: null,
      fingerprint: '',
      confirmed: false,
    };
    this.skills.set(projectId, [skill]);
    this.connectors.set(projectId, [parts, tickets, drawings]);
    this.confirmedDefinitions.set(parts.id, fingerprintOf(parts));
    this.confirmedDefinitions.set(tickets.id, fingerprintOf(tickets));
    if (specialist) {
      this.kits.set(specialist.id, {
        memberId: specialist.id,
        skills: [skill.id],
        connectors: [
          { connectorId: parts.id, tools: ['search_parts', 'get_part'] },
          // Given on the other computer.
          { connectorId: drawings.id, tools: ['find_drawing'] },
        ],
      });
      this.confirmedTools.set(
        specialist.id,
        new Set([toolKey(parts.id, 'search_parts'), toolKey(parts.id, 'get_part')]),
      );
    }
  }

  library(projectId: Id, members: readonly Member[]): Library {
    const skills = [...(this.skills.get(projectId) ?? [])].sort((a, b) =>
      a.name.localeCompare(b.name),
    );
    const connectors = [...(this.connectors.get(projectId) ?? [])]
      .sort((a, b) => a.name.localeCompare(b.name))
      .map((connector) => {
        const fingerprint = fingerprintOf(connector);
        return {
          ...connector,
          missingVariables: variablesOf(connector.start).filter(
            (name) => !this.variables.has(name),
          ),
          fingerprint,
          confirmed: this.confirmedDefinitions.get(connector.id) === fingerprint,
        };
      });
    const kits = members.flatMap((member) => {
      const kit = this.kits.get(member.id);
      if (!kit) return [];
      return [
        {
          memberId: kit.memberId,
          skills: kit.skills.filter((id) => skills.some((skill) => skill.id === id)),
          connectors: kit.connectors.filter((given) =>
            connectors.some((connector) => connector.id === given.connectorId),
          ),
        },
      ];
    });
    const waitingTools = kits.flatMap((kit) => {
      const confirmed = this.confirmedTools.get(kit.memberId) ?? new Set<string>();
      return kit.connectors.flatMap((given) => {
        const tools = given.tools.filter(
          (tool) => !confirmed.has(toolKey(given.connectorId, tool)),
        );
        return tools.length > 0
          ? [{ memberId: kit.memberId, connectorId: given.connectorId, tools }]
          : [];
      });
    });
    return structuredClone({ skills, connectors, kits, waitingTools });
  }

  addSkill(projectId: Id, folder: string): void {
    const found = SAMPLE_SKILL_FOLDERS[folder];
    if (found === undefined) {
      // The demo has no disk: a folder it does not know is one that is not there.
      throw new BackendError('folder_missing', 'That folder does not exist.');
    }
    if (typeof found === 'string') rule(`this cannot be a skill: ${found}`);
    const skills = this.skills.get(projectId) ?? [];
    if (skills.some((skill) => skill.name === found.name)) {
      rule(
        `this cannot be a skill: the library already has the skill "${found.name}"; remove it first to add it again`,
      );
    }
    skills.push({ id: this.newId(), ...found, addedAt: this.now(), here: true, problem: null });
    this.skills.set(projectId, skills);
  }

  removeSkill(projectId: Id, skillId: Id): void {
    const skills = this.skills.get(projectId) ?? [];
    if (!skills.some((skill) => skill.id === skillId)) {
      throw new BackendError('not_found', `there is no skill with the identifier ${skillId}`);
    }
    this.skills.set(
      projectId,
      skills.filter((skill) => skill.id !== skillId),
    );
    for (const kit of this.kits.values()) {
      kit.skills = kit.skills.filter((id) => id !== skillId);
    }
  }

  addConnector(projectId: Id, connector: NewConnector): void {
    checkConnector(connector);
    const connectors = this.connectors.get(projectId) ?? [];
    const name = connector.name.trim();
    if (connectors.some((known) => known.name === name)) {
      rule(`the library already has a connector called ${name}`);
    }
    const added: LibraryConnector = {
      id: this.newId(),
      name,
      start: structuredClone(connector.start),
      tools: connector.tools.map((tool) => tool.trim()).filter((tool) => tool !== ''),
      addedAt: this.now(),
      missingVariables: [],
      problem: null,
      fingerprint: '',
      confirmed: true,
    };
    connectors.push(added);
    this.connectors.set(projectId, connectors);
    // Added here, through the interface: confirmed here as it is.
    this.confirmedDefinitions.set(added.id, fingerprintOf(added));
  }

  removeConnector(projectId: Id, connectorId: Id): void {
    const connectors = this.connectors.get(projectId) ?? [];
    if (!connectors.some((connector) => connector.id === connectorId)) {
      throw new BackendError(
        'not_found',
        `there is no connector with the identifier ${connectorId}`,
      );
    }
    this.connectors.set(
      projectId,
      connectors.filter((connector) => connector.id !== connectorId),
    );
    for (const kit of this.kits.values()) {
      kit.connectors = kit.connectors.filter((given) => given.connectorId !== connectorId);
    }
    this.confirmedDefinitions.delete(connectorId);
  }

  /**
   * Confirms "on this computer" a connector that arrived from another one,
   * exactly as the manager was shown it.
   */
  confirmConnector(projectId: Id, connectorId: Id, fingerprint: string): void {
    const connector = (this.connectors.get(projectId) ?? []).find(
      (known) => known.id === connectorId,
    );
    if (!connector) {
      throw new BackendError(
        'not_found',
        `there is no connector with the identifier ${connectorId}`,
      );
    }
    if (fingerprintOf(connector) !== fingerprint) {
      rule(
        `the connector ${connector.name} changed since you looked at it; look at it again before you confirm it`,
      );
    }
    this.confirmedDefinitions.set(connectorId, fingerprint);
  }

  /** Confirms "on this computer" the tools of a kit the manager was shown. */
  confirmKit(member: Member, seen: KitChoice): void {
    const kit = this.kits.get(member.id);
    if (!kit) return;
    const confirmed = this.confirmedTools.get(member.id) ?? new Set<string>();
    for (const given of kit.connectors) {
      const shown = seen.connectors.find((entry) => entry.connectorId === given.connectorId);
      for (const tool of given.tools) {
        if (shown?.tools.includes(tool)) confirmed.add(toolKey(given.connectorId, tool));
      }
    }
    this.confirmedTools.set(member.id, confirmed);
  }

  setKit(member: Member, choice: KitChoice): void {
    if (member.kind !== 'agent' && (choice.skills.length > 0 || choice.connectors.length > 0)) {
      rule('only an AI specialist is given skills and connectors');
    }
    const skills = this.skills.get(member.projectId) ?? [];
    for (const id of choice.skills) {
      if (!skills.some((skill) => skill.id === id)) {
        throw new BackendError('not_found', `there is no skill with the identifier ${id}`);
      }
    }
    const connectors = this.connectors.get(member.projectId) ?? [];
    for (const given of choice.connectors) {
      const connector = connectors.find((known) => known.id === given.connectorId);
      if (!connector) {
        throw new BackendError(
          'not_found',
          `there is no connector with the identifier ${given.connectorId}`,
        );
      }
      if (given.tools.length === 0) {
        rule('a specialist is given a connector with at least one of its tools');
      }
      const unknown = given.tools.find((tool) => !connector.tools.includes(tool));
      if (unknown !== undefined) {
        rule(`the connector ${connector.name} has no tool called ${unknown}`);
      }
    }
    this.kits.set(member.id, { memberId: member.id, ...structuredClone(choice) });
    // Chosen here: what the kit gives is confirmed here, and nothing else.
    this.confirmedTools.set(
      member.id,
      new Set(
        choice.connectors.flatMap((given) =>
          given.tools.map((tool) => toolKey(given.connectorId, tool)),
        ),
      ),
    );
  }

  /**
   * Whether a specialist was given a connector that is handed a secret in
   * an environment variable. Such a specialist is given no command
   * (decision 0038): a command could read the secret.
   */
  namesASecret(member: Member): boolean {
    const kit = this.kits.get(member.id);
    const connectors = this.connectors.get(member.projectId) ?? [];
    return (kit?.connectors ?? []).some((given) => {
      const connector = connectors.find((known) => known.id === given.connectorId);
      return connector !== undefined && variablesOf(connector.start).length > 0;
    });
  }

  /** Forgets everything of a project that is deleted. */
  forgetProject(projectId: Id): void {
    this.skills.delete(projectId);
    this.connectors.delete(projectId);
  }
}
