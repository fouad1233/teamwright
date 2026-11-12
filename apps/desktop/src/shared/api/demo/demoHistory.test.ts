import { beforeEach, describe, expect, it } from 'vitest';

import type { Project, Task, TaskChange } from '../types';
import { DemoBackend, type DemoEnvironment } from './demoBackend';
import { changesBetween, historyOf } from './taskHistory';

/** An environment whose time and identifiers the test controls. */
function testEnvironment(): DemoEnvironment & { advanceHours(hours: number): void } {
  let now = new Date('2026-11-11T09:00:00.000Z');
  let identifiers = 0;
  return {
    now: () => now,
    newId: () => {
      identifiers += 1;
      return `id-${identifiers.toString()}`;
    },
    // Nothing a simulated specialist does happens in these tests.
    later: () => undefined,
    advanceHours: (hours) => {
      now = new Date(now.getTime() + hours * 60 * 60 * 1000);
    },
  };
}

function kinds(history: TaskChange[]): string[] {
  return history.map((change) => change.kind);
}

describe('the history of a task in the DemoBackend', () => {
  let environment: ReturnType<typeof testEnvironment>;
  let backend: DemoBackend;
  let project: Project;
  let task: Task;

  beforeEach(async () => {
    environment = testEnvironment();
    backend = new DemoBackend(undefined, environment);
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'engineering' });
    task = await backend.createTask({ projectId: project.id, title: 'Draw the schematic' });
  });

  it('begins with the task being opened', async () => {
    expect(await backend.listTaskHistory(task.id)).toEqual([
      { at: '2026-11-11T09:00:00.000Z', kind: 'created' },
    ]);
  });

  it('lists every change with the moment it was made, the oldest first', async () => {
    const inProgress = project.statuses[1];
    environment.advanceHours(1);
    await backend.renameTask(task.id, 'Draw the whole schematic');
    environment.advanceHours(1);
    await backend.moveTask(task.id, inProgress?.id ?? '', 0);

    expect(await backend.listTaskHistory(task.id)).toEqual([
      { at: '2026-11-11T09:00:00.000Z', kind: 'created' },
      {
        at: '2026-11-11T10:00:00.000Z',
        kind: 'renamed',
        from: 'Draw the schematic',
        to: 'Draw the whole schematic',
      },
      {
        at: '2026-11-11T11:00:00.000Z',
        kind: 'moved',
        fromStatusId: project.statuses[0]?.id,
        toStatusId: inProgress?.id,
      },
    ]);
  });

  it('knows every kind of change a manager can make', async () => {
    const engineer = await backend.addMember({
      projectId: project.id,
      name: 'Hardware engineer',
      kind: 'agent',
    });
    const pins = await backend.createTask({ projectId: project.id, title: 'Define the pin map' });

    await backend.describeTask(task.id, 'Keep the board small.');
    await backend.assignTask(task.id, { kind: 'member', memberId: engineer.id });
    await backend.prioritizeTask(task.id, 'high');
    await backend.estimateTask(task.id, 480);
    await backend.rescheduleTask(task.id, '2026-11-10', '2026-11-14');
    await backend.addTaskDependency(task.id, pins.id);
    await backend.removeTaskDependency(task.id, pins.id);

    const history = await backend.listTaskHistory(task.id);
    expect(kinds(history)).toEqual([
      'created',
      'described',
      'assigned',
      'prioritized',
      'estimated',
      'rescheduled',
      'now_waits_for',
      'no_longer_waits_for',
    ]);
    expect(history[2]).toMatchObject({ from: null, to: { kind: 'member', memberId: engineer.id } });
    expect(history[3]).toMatchObject({ from: 'normal', to: 'high' });
    expect(history[4]).toMatchObject({ from: null, to: 480 });
    expect(history[5]).toMatchObject({
      from: { startOn: null, dueOn: null },
      to: { startOn: '2026-11-10', dueOn: '2026-11-14' },
    });
    expect(history[6]).toMatchObject({ taskId: pins.id });
  });

  it('does not list a move up or down within a status', async () => {
    await backend.createTask({ projectId: project.id, title: 'Route the board' });

    await backend.moveTask(task.id, project.statuses[0]?.id ?? '', 1);

    expect(kinds(await backend.listTaskHistory(task.id))).toEqual(['created']);
  });

  it('lists what happens to a task through something else', async () => {
    const engineer = await backend.addMember({
      projectId: project.id,
      name: 'Hardware engineer',
      kind: 'agent',
    });
    const inRevision = project.statuses[2];
    await backend.assignTask(task.id, { kind: 'member', memberId: engineer.id });
    await backend.moveTask(task.id, inRevision?.id ?? '', 0);

    // The member leaves, and the status the task is in is removed.
    await backend.removeMember(engineer.id);
    await backend.removeStatus(project.id, inRevision?.id ?? '', project.statuses[1]?.id);

    const history = await backend.listTaskHistory(task.id);
    expect(kinds(history)).toEqual(['created', 'assigned', 'moved', 'assigned', 'moved']);
    expect(history[3]).toMatchObject({ to: null });
    expect(history[4]).toMatchObject({ toStatusId: project.statuses[1]?.id });
  });

  it('is gone with the task', async () => {
    await backend.deleteTask(task.id);

    await expect(backend.listTaskHistory(task.id)).rejects.toMatchObject({ code: 'not_found' });
  });
});

describe('changesBetween and historyOf', () => {
  const before: Task = {
    id: 'task',
    projectId: 'project',
    number: 1,
    title: 'Draw the schematic',
    description: '',
    statusId: 'todo',
    priority: 'normal',
    assignee: null,
    parentId: null,
    blockedBy: ['a'],
    estimateMinutes: null,
    sprintId: null,
    points: null,
    startOn: null,
    dueOn: null,
    rank: 0,
    createdAt: '2026-11-11T09:00:00.000Z',
    updatedAt: '2026-11-11T09:00:00.000Z',
    completedAt: null,
  };

  it('find nothing when only the rank and the time of the change differ', () => {
    const after = { ...before, rank: 512, updatedAt: '2026-11-11T10:00:00.000Z' };

    expect(changesBetween(before, after)).toEqual([]);
  });

  it('list several changes of one moment in a fixed order', () => {
    const after: Task = {
      ...before,
      title: 'Draw it',
      statusId: 'done',
      blockedBy: ['b'],
      updatedAt: '2026-11-11T10:00:00.000Z',
    };

    expect(kinds(changesBetween(before, after))).toEqual([
      'renamed',
      'moved',
      'now_waits_for',
      'no_longer_waits_for',
    ]);
  });

  it('have nothing to say about a task without versions', () => {
    expect(historyOf([])).toEqual([]);
  });
});
