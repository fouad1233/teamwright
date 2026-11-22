import { describe, expect, it } from 'vitest';

import type { Library, LibraryConnector } from '@/shared/api/types';

import {
  choiceOf,
  confirmationOf,
  isToolGiven,
  isToolWaiting,
  kitInWords,
  sameChoice,
  startInWords,
  toggleSkill,
  toggleTool,
  wordsOf,
} from './kitModel';

const parts: LibraryConnector = {
  id: 'c1',
  name: 'parts',
  start: { kind: 'program', program: 'npx', arguments: ['parts-server'], variables: [] },
  tools: ['search', 'get', 'order'],
  addedAt: '2026-11-21T00:00:00.000Z',
  missingVariables: [],
  problem: null,
  fingerprint: 'f1',
  confirmed: true,
};

const library: Library = {
  skills: [
    { id: 's1', name: 'pcb-review', description: 'x', addedAt: '', here: true, problem: null },
  ],
  connectors: [parts],
  kits: [
    { memberId: 'm1', skills: ['s1'], connectors: [{ connectorId: 'c1', tools: ['search'] }] },
  ],
  waitingTools: [],
};

describe('the kit of a specialist', () => {
  it('says what a specialist has in a few words, and nothing for nothing', () => {
    expect(kitInWords(library, 'm1')).toBe('pcb-review · parts (1 tool)');
    expect(kitInWords(library, 'm2')).toBeNull();
    expect(kitInWords(undefined, 'm1')).toBeNull();
  });

  it('gives a connector tool by tool, in the connector order, and drops it with its last tool', () => {
    let choice = choiceOf(undefined);
    choice = toggleTool(choice, parts, 'order');
    choice = toggleTool(choice, parts, 'search');
    expect(choice.connectors).toEqual([{ connectorId: 'c1', tools: ['search', 'order'] }]);
    expect(isToolGiven(choice, 'c1', 'order')).toBe(true);
    choice = toggleTool(toggleTool(choice, parts, 'search'), parts, 'order');
    expect(choice.connectors).toEqual([]);
  });

  it('gives and takes a skill, and compares choices whatever their order', () => {
    const choice = toggleSkill(choiceOf(undefined), 's1');
    expect(choice.skills).toEqual(['s1']);
    expect(toggleSkill(choice, 's1').skills).toEqual([]);
    expect(
      sameChoice({ skills: ['a', 'b'], connectors: [] }, { skills: ['b', 'a'], connectors: [] }),
    ).toBe(true);
  });

  it('says when part of a kit waits for confirmation on this computer', () => {
    const waiting: Library = {
      ...library,
      waitingTools: [{ memberId: 'm1', connectorId: 'c1', tools: ['search'] }],
    };
    expect(kitInWords(waiting, 'm1')).toBe(
      'pcb-review · parts (1 tool), part of it waiting for your confirmation on this computer',
    );
    expect(isToolWaiting(waiting, 'm1', 'c1', 'search')).toBe(true);
    expect(isToolWaiting(waiting, 'm1', 'c1', 'get')).toBe(false);
    expect(isToolWaiting(waiting, undefined, 'c1', 'search')).toBe(false);
    expect(confirmationOf(waiting.waitingTools)).toEqual({
      skills: [],
      connectors: [{ connectorId: 'c1', tools: ['search'] }],
    });
    const changed: Library = { ...library, connectors: [{ ...parts, confirmed: false }] };
    expect(kitInWords(changed, 'm1')).toContain('waiting for your confirmation');
  });

  it('reads words typed on a line and says how a connector starts', () => {
    expect(wordsOf(' search, get  order ')).toEqual(['search', 'get', 'order']);
    expect(startInWords(parts.start)).toBe('npx parts-server');
  });
});
