import { describe, expect, it } from 'vitest';

import { DemoBackend } from './demoBackend';
import { testEnvironment } from './testing';
import { contextAfter, contextOf, SIMULATED_WINDOW } from './usage';

describe('what the DemoBackend says about tokens and contexts', () => {
  it('fills a context turn by turn and makes room before it runs out', () => {
    const first = contextAfter(null, 38_400);
    expect(first).toEqual({ held: 38_400, window: SIMULATED_WINDOW, isNearlyFull: false });
    expect(contextAfter(first, 130_000).isNearlyFull).toBe(true);
    // 38,400 and 160,000 would pass 95 % of the window: the session makes room.
    const madeRoom = contextAfter(first, 160_000);
    expect(madeRoom.held).toBeLessThan(first.held + 160_000);
    expect(madeRoom.isNearlyFull).toBe(false);
    expect(contextOf(10, null).isNearlyFull).toBe(false);
  });

  it('counts what a specialist spent and how full its context is after a turn', async () => {
    const environment = testEnvironment();
    const backend = new DemoBackend(undefined, environment);
    const project = await backend.createProject({ name: 'Sensor', workflow: 'engineering' });
    const engineer = await backend.addMember({
      projectId: project.id,
      name: 'Hardware engineer',
      kind: 'agent',
    });
    const task = await backend.createTask({
      projectId: project.id,
      title: 'Draw the schematic',
      assignee: { kind: 'member', memberId: engineer.id },
    });

    expect((await backend.projectUsage(project.id)).members).toEqual([]);
    await backend.startWork(task.id);
    const running = await backend.projectUsage(project.id);
    expect(running.members[0]?.usage.turns).toBe(0);
    expect(running.members[0]?.usage.context).toBeNull();

    environment.passAll();

    const [conversation] = await backend.listConversations(project.id);
    expect(conversation?.context?.held).toBe(38_400);
    expect(conversation?.turns[0]?.context).toEqual(conversation?.context);
    const usage = await backend.projectUsage(project.id);
    const [member] = usage.members;
    expect(member?.memberId).toBe(engineer.id);
    expect(member?.usage.turns).toBe(1);
    expect(member?.usage.allTokens).toBe(42 + 880 + 21_500 + 6400);
    expect(member?.usage.lastModel).toBe('simulated');
    expect(member?.usage.context?.held).toBe(38_400);
    expect(usage.advisor).toBeNull();
    expect(usage.total.allTokens).toBe(member?.usage.allTokens);
    expect(usage.total.context).toBeNull();
  });
});
