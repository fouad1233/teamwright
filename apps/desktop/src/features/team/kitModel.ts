import type {
  ConnectorStart,
  Id,
  KitChoice,
  Library,
  LibraryConnector,
  MemberKit,
} from '@/shared/api/types';

/** What a specialist is given, by its member, or nothing. */
export function kitOf(library: Library | undefined, memberId: Id): MemberKit | undefined {
  return library?.kits.find((kit) => kit.memberId === memberId);
}

/** What a specialist has, in a few words for its row, or null for nothing. */
export function kitInWords(library: Library | undefined, memberId: Id): string | null {
  const kit = kitOf(library, memberId);
  if (!library || !kit) return null;
  const skills = kit.skills
    .map((id) => library.skills.find((skill) => skill.id === id)?.name)
    .filter((name): name is string => name !== undefined);
  const connectors = kit.connectors.flatMap((given) => {
    const connector = library.connectors.find((known) => known.id === given.connectorId);
    return connector ? [`${connector.name} (${toolsInWords(given.tools.length)})`] : [];
  });
  const parts = [...skills, ...connectors];
  return parts.length === 0 ? null : parts.join(' · ');
}

function toolsInWords(count: number): string {
  return count === 1 ? '1 tool' : `${count.toString()} tools`;
}

/** How a connector is started, in one line for people. */
export function startInWords(start: ConnectorStart): string {
  return start.kind === 'program' ? [start.program, ...start.arguments].join(' ') : start.url;
}

/** The variables a connector needs, by name. */
export function variablesOf(start: ConnectorStart): string[] {
  return start.kind === 'program'
    ? start.variables
    : start.headers.map((header) => header.variable);
}

/** The words a list typed on one line holds: separated by commas or spaces. */
export function wordsOf(text: string): string[] {
  return text
    .split(/[\s,]+/)
    .map((word) => word.trim())
    .filter((word) => word !== '');
}

/** The choice of a kit, to edit in the dialog of a member. */
export function choiceOf(kit: MemberKit | undefined): KitChoice {
  return {
    skills: [...(kit?.skills ?? [])],
    connectors: (kit?.connectors ?? []).map((given) => ({ ...given, tools: [...given.tools] })),
  };
}

/** The choice with a skill given or taken away. */
export function toggleSkill(choice: KitChoice, skillId: Id): KitChoice {
  return {
    ...choice,
    skills: choice.skills.includes(skillId)
      ? choice.skills.filter((id) => id !== skillId)
      : [...choice.skills, skillId],
  };
}

/**
 * The choice with one tool of a connector given or taken away. A connector
 * whose last tool is taken away is no longer given: a connector is never
 * given without a tool, nor whole.
 */
export function toggleTool(
  choice: KitChoice,
  connector: LibraryConnector,
  tool: string,
): KitChoice {
  const given = choice.connectors.find((entry) => entry.connectorId === connector.id);
  const tools = given?.tools ?? [];
  const next = tools.includes(tool)
    ? tools.filter((name) => name !== tool)
    : connector.tools.filter((name) => name === tool || tools.includes(name));
  const others = choice.connectors.filter((entry) => entry.connectorId !== connector.id);
  return {
    ...choice,
    connectors:
      next.length === 0 ? others : [...others, { connectorId: connector.id, tools: next }],
  };
}

/** Whether a tool of a connector is given in a choice. */
export function isToolGiven(choice: KitChoice, connectorId: Id, tool: string): boolean {
  return (
    choice.connectors.find((entry) => entry.connectorId === connectorId)?.tools.includes(tool) ??
    false
  );
}

/** Whether two choices give the same things. */
export function sameChoice(first: KitChoice, second: KitChoice): boolean {
  const key = (choice: KitChoice) =>
    JSON.stringify({
      skills: [...choice.skills].sort(),
      connectors: [...choice.connectors]
        .map((entry) => ({ id: entry.connectorId, tools: [...entry.tools].sort() }))
        .sort((a, b) => a.id.localeCompare(b.id)),
    });
  return key(first) === key(second);
}
