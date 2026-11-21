import { describe, expect, it } from 'vitest';

import type { StatusCategory } from '../types';
import { DemoBackend } from './demoBackend';
import { sampleWorkspace } from './sampleWorkspace';
import { testEnvironment } from './testing';

/** A day of the clock of the tests, which starts on 11 November 2026 at nine. */
const DAY = 24 * 60 * 60 * 1000;

describe('what really happened to the tasks, in the DemoBackend', () => {
  it('reads the actual dates of a task from its versions and compares them with its plan', async () => {
    const environment = testEnvironment();
    const backend = new DemoBackend(undefined, environment);
    const project = await backend.createProject({ name: 'Air quality node', workflow: 'standard' });
    const statusOf = (category: StatusCategory) =>
      project.statuses.find((status) => status.category === category)?.id ?? '';
    const late = await backend.createTask({ projectId: project.id, title: 'Draw the schematic' });
    const unplanned = await backend.createTask({ projectId: project.id, title: 'Order the parts' });
    await backend.rescheduleTask(late.id, '2026-11-10', '2026-11-11');
    await backend.moveTask(late.id, statusOf('active'), 0);
    environment.pass(2 * DAY);
    await backend.moveTask(unplanned.id, statusOf('done'), 0);

    const [lateOne, unplannedOne] = await backend.listTaskActuals(project.id);

    expect(lateOne).toMatchObject({
      taskId: late.id,
      startedOn: '2026-11-11',
      finishedOn: null,
      startedLateDays: 1,
      runningLateDays: 2,
    });
    expect(unplannedOne).toMatchObject({
      startedOn: '2026-11-13',
      finishedOn: '2026-11-13',
      startedLateDays: null,
      finishedLateDays: null,
      runningLateDays: null,
    });
  });

  it('shows in the sample a late start, a late finish, work running late and a reopened task', async () => {
    const environment = testEnvironment();
    const backend = new DemoBackend(sampleWorkspace(environment.now()), environment);
    const [project] = await backend.listProjects();
    const tasks = await backend.listTasks(project?.id ?? '');
    const numberOf = (taskId: string) => tasks.find((task) => task.id === taskId)?.number;

    const actuals = await backend.listTaskActuals(project?.id ?? '');
    const numbers = (keep: (actual: (typeof actuals)[number]) => boolean) =>
      actuals.filter(keep).map((actual) => numberOf(actual.taskId));

    expect(numbers((actual) => actual.startedLateDays !== null && !actual.notStarted)).toEqual(
      expect.arrayContaining([2, 4]),
    );
    expect(numbers((actual) => actual.finishedLateDays !== null)).toEqual(
      expect.arrayContaining([2, 19, 25]),
    );
    expect(numbers((actual) => actual.runningLateDays !== null)).toContain(3);
    expect(numbers((actual) => actual.reopened)).toEqual([27]);
  });
});
