import { beforeEach, describe, expect, it } from 'vitest';

import type { Conversation, Member, PlanLimit, Project, Task } from '../types';
import { DemoBackend } from './demoBackend';
import { testEnvironment } from './testing';

describe('the limit of the Claude plan in the DemoBackend', () => {
  let environment: ReturnType<typeof testEnvironment>;
  let backend: DemoBackend;
  let project: Project;
  let engineer: Member;
  let schematic: Task;

  beforeEach(async () => {
    environment = testEnvironment();
    backend = new DemoBackend(undefined, environment);
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'engineering' });
    engineer = await backend.addMember({
      projectId: project.id,
      name: 'Hardware engineer',
      kind: 'agent',
    });
    schematic = await backend.createTask({
      projectId: project.id,
      title: 'Draw the schematic',
      assignee: { kind: 'member', memberId: engineer.id },
    });
  });

  async function onlyConversation(): Promise<Conversation> {
    const [conversation] = await backend.listConversations(project.id);
    if (!conversation) {
      throw new Error('there is no conversation');
    }
    return conversation;
  }

  it('knows nothing of the plan before a turn reports it', async () => {
    expect(await backend.planLimit()).toEqual({ reached: null, windows: [], reportedAt: null });
  });

  it('reports the windows after a turn went through', async () => {
    await backend.startWork(schematic.id);
    environment.passAll();

    const known = await backend.planLimit();
    expect(known.reached).toBeNull();
    expect(known.windows.map((window) => window.kind)).toEqual(['five_hour', 'seven_day']);
    expect(known.reportedAt).not.toBeNull();
  });

  it('ends a running turn at the limit, ends new turns at once, and lifts at the reset', async () => {
    const told: PlanLimit[] = [];
    backend.onPlanLimitChanged((known) => told.push(known));
    await backend.startWork(schematic.id);
    environment.pass(100);

    backend.pretendPlanLimitReached(60_000);

    let turn = (await onlyConversation()).turns[0];
    expect(turn).toMatchObject({
      state: 'failed',
      planLimitReached: true,
      failure: 'The limit of the Claude plan was reached.',
    });
    const known = await backend.planLimit();
    expect(known.reached).toMatchObject({
      window: 'five_hour',
      resetsAt: '2026-11-11T09:01:00.100Z',
    });
    // The task stays in work: the specialist did not fail it.
    const task = (await backend.listTasks(project.id)).find((listed) => listed.id === schematic.id);
    expect(project.statuses.find((status) => status.id === task?.statusId)?.category).toBe(
      'active',
    );

    // Trying again before the reset ends the same way.
    await backend.reply((await onlyConversation()).id, 'Please go on where you stopped.');
    environment.pass(1_000);
    turn = (await onlyConversation()).turns[1];
    expect(turn).toMatchObject({ state: 'failed', planLimitReached: true });

    environment.pass(60_000);
    expect((await backend.planLimit()).reached).toBeNull();
    expect(told.at(-1)?.reached).toBeNull();

    // After the reset, work goes through.
    await backend.reply((await onlyConversation()).id, 'Please go on where you stopped.');
    environment.passAll();
    expect((await onlyConversation()).turns[2]?.state).toBe('finished');
  });

  it('keeps a limit without a known reset until it is lifted', async () => {
    backend.pretendPlanLimitReached(null);
    environment.passAll();
    expect((await backend.planLimit()).reached?.resetsAt).toBeNull();

    backend.liftPlanLimit();
    expect((await backend.planLimit()).reached).toBeNull();
  });
});
