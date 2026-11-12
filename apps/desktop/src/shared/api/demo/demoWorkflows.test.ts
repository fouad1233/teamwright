import { beforeEach, describe, expect, it } from 'vitest';

import { BackendError } from '../backend';
import type { Id, Project, Status, Task } from '../types';
import { DemoBackend, type DemoEnvironment } from './demoBackend';

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
    advanceHours: (hours) => {
      now = new Date(now.getTime() + hours * 60 * 60 * 1000);
    },
  };
}

async function refusal(request: Promise<unknown>): Promise<BackendError> {
  try {
    await request;
  } catch (failure) {
    if (failure instanceof BackendError) {
      return failure;
    }
    throw failure;
  }
  throw new Error('the request was expected to be refused, but it succeeded');
}

function names(project: Project): string[] {
  return project.statuses.map((status) => status.name);
}

function statusNamed(project: Project, name: string): Status {
  const status = project.statuses.find((candidate) => candidate.name === name);
  if (!status) {
    throw new Error(`the project has no status named ${name}`);
  }
  return status;
}

describe('putting projects away and removing them in the DemoBackend', () => {
  let environment: ReturnType<typeof testEnvironment>;
  let backend: DemoBackend;
  let project: Project;

  beforeEach(async () => {
    environment = testEnvironment();
    backend = new DemoBackend(undefined, environment);
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'engineering' });
  });

  it('lists a project that was put away among the archived ones only', async () => {
    await backend.archiveProject(project.id);

    expect(await backend.listProjects()).toEqual([]);
    expect((await backend.listArchivedProjects()).map((listed) => listed.id)).toEqual([project.id]);
  });

  it('lists the project that was put away last first', async () => {
    const second = await backend.createProject({ name: 'Display driver', workflow: 'standard' });
    await backend.archiveProject(project.id);
    environment.advanceHours(1);
    await backend.archiveProject(second.id);

    const archived = await backend.listArchivedProjects();

    expect(archived.map((listed) => listed.name)).toEqual(['Display driver', 'Smart sensor node']);
  });

  it('brings a project back with everything it had', async () => {
    await backend.createTask({ projectId: project.id, title: 'Draw the schematic' });
    await backend.archiveProject(project.id);

    const restored = await backend.restoreProject(project.id);

    expect(restored.archivedAt).toBeNull();
    expect((await backend.listProjects()).map((listed) => listed.id)).toEqual([project.id]);
    expect(await backend.listTasks(project.id)).toHaveLength(1);
  });

  it('removes a project with its tasks and its team', async () => {
    const other = await backend.createProject({ name: 'Display driver', workflow: 'standard' });
    await backend.createTask({ projectId: project.id, title: 'Draw the schematic' });
    await backend.addMember({ projectId: project.id, name: 'Hardware engineer', kind: 'agent' });
    const kept = await backend.createTask({ projectId: other.id, title: 'Write the driver' });

    await backend.deleteProject(project.id);

    expect((await backend.listProjects()).map((listed) => listed.id)).toEqual([other.id]);
    expect((await refusal(backend.listTasks(project.id))).code).toBe('not_found');
    expect((await refusal(backend.listMembers(project.id))).code).toBe('not_found');
    expect((await backend.listTasks(other.id)).map((listed) => listed.id)).toEqual([kept.id]);
  });

  it('lets a new project take the key of one that was removed', async () => {
    await backend.deleteProject(project.id);

    const again = await backend.createProject({ name: 'Smart sensor node', workflow: 'standard' });

    expect(again.key).toBe('SSN');
  });

  it('reports a project that does not exist', async () => {
    expect((await refusal(backend.deleteProject('missing'))).code).toBe('not_found');
    expect((await refusal(backend.restoreProject('missing'))).code).toBe('not_found');
  });
});

describe('the statuses of a project in the DemoBackend', () => {
  let environment: ReturnType<typeof testEnvironment>;
  let backend: DemoBackend;
  let project: Project;

  beforeEach(async () => {
    environment = testEnvironment();
    backend = new DemoBackend(undefined, environment);
    // To do, In progress, In revision, In test, Done.
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'engineering' });
  });

  async function taskIn(title: string, status: Status): Promise<Task> {
    return backend.createTask({ projectId: project.id, title, statusId: status.id });
  }

  async function titlesIn(statusId: Id): Promise<string[]> {
    const tasks = await backend.listTasks(project.id);
    return tasks.filter((listed) => listed.statusId === statusId).map((listed) => listed.title);
  }

  describe('adding a status', () => {
    it('puts a new step of the work behind the other steps, in front of review and done', async () => {
      const changed = await backend.addStatus(project.id, {
        name: 'Waiting for parts',
        category: 'active',
        color: 'pink',
      });

      expect(names(changed)).toEqual([
        'To do',
        'In progress',
        'In revision',
        'Waiting for parts',
        'In test',
        'Done',
      ]);
    });

    it('puts a status in which work has not started behind the other ones of its kind', async () => {
      const changed = await backend.addStatus(project.id, {
        name: 'Backlog',
        category: 'todo',
        color: 'gray',
      });

      expect(names(changed).slice(0, 3)).toEqual(['To do', 'Backlog', 'In progress']);
    });

    it('puts a second done status at the end', async () => {
      const changed = await backend.addStatus(project.id, {
        name: 'Cancelled',
        category: 'done',
        color: 'red',
      });

      expect(names(changed).at(-1)).toBe('Cancelled');
    });

    it('tidies the name and keeps the color and the category', async () => {
      const changed = await backend.addStatus(project.id, {
        name: '  Waiting   for parts ',
        category: 'active',
        color: 'pink',
      });

      expect(statusNamed(changed, 'Waiting for parts')).toMatchObject({
        category: 'active',
        color: 'pink',
      });
    });

    it('refuses a name another status has, whatever the capitals', async () => {
      const error = await refusal(
        backend.addStatus(project.id, { name: 'in TEST', category: 'review', color: 'amber' }),
      );

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe('the workflow already has a status named "in TEST"');
    });

    it('refuses an empty name', async () => {
      const error = await refusal(
        backend.addStatus(project.id, { name: '  ', category: 'active', color: 'blue' }),
      );

      expect(error.message).toBe('the status name must not be empty');
    });
  });

  describe('changing a status', () => {
    it('gives a status another name and color', async () => {
      const inTest = statusNamed(project, 'In test');

      const changed = await backend.changeStatus(project.id, inTest.id, {
        name: 'On the bench',
        category: 'review',
        color: 'teal',
      });

      expect(changed.statuses.find((status) => status.id === inTest.id)).toEqual({
        id: inTest.id,
        name: 'On the bench',
        category: 'review',
        color: 'teal',
      });
    });

    it('lets a status keep its own name', async () => {
      const inTest = statusNamed(project, 'In test');

      const changed = await backend.changeStatus(project.id, inTest.id, {
        name: 'In test',
        category: 'review',
        color: 'red',
      });

      expect(statusNamed(changed, 'In test').color).toBe('red');
    });

    it('refuses the name of another status', async () => {
      const inTest = statusNamed(project, 'In test');

      const error = await refusal(
        backend.changeStatus(project.id, inTest.id, {
          name: 'Done',
          category: 'review',
          color: 'amber',
        }),
      );

      expect(error.code).toBe('rule_broken');
    });

    it('completes the tasks of a status that becomes a done status', async () => {
      const inTest = statusNamed(project, 'In test');
      const tested = await taskIn('Select the sensors', inTest);
      environment.advanceHours(2);

      await backend.changeStatus(project.id, inTest.id, {
        name: 'Accepted',
        category: 'done',
        color: 'green',
      });

      const [stored] = (await backend.listTasks(project.id)).filter(
        (listed) => listed.id === tested.id,
      );
      expect(stored?.completedAt).toBe('2026-11-11T11:00:00.000Z');
    });

    it('reopens the tasks of a done status that becomes something else', async () => {
      const cancelled = statusNamed(
        await backend.addStatus(project.id, { name: 'Cancelled', category: 'done', color: 'red' }),
        'Cancelled',
      );
      const dropped = await taskIn('Add a second display', cancelled);

      await backend.changeStatus(project.id, cancelled.id, {
        name: 'On hold',
        category: 'active',
        color: 'red',
      });

      const [stored] = (await backend.listTasks(project.id)).filter(
        (listed) => listed.id === dropped.id,
      );
      expect(dropped.completedAt).not.toBeNull();
      expect(stored?.completedAt).toBeNull();
    });

    it('refuses to change the only status in which work has not started', async () => {
      const todo = statusNamed(project, 'To do');

      const error = await refusal(
        backend.changeStatus(project.id, todo.id, {
          name: 'To do',
          category: 'active',
          color: 'gray',
        }),
      );

      expect(error.message).toBe('a workflow needs at least one status in the "todo" category');
    });

    it('refuses to change the only status in which work is done', async () => {
      const done = statusNamed(project, 'Done');

      const error = await refusal(
        backend.changeStatus(project.id, done.id, {
          name: 'Done',
          category: 'review',
          color: 'green',
        }),
      );

      expect(error.message).toBe('a workflow needs at least one status in the "done" category');
    });
  });

  describe('moving a status', () => {
    it('moves a status to another place', async () => {
      const inTest = statusNamed(project, 'In test');

      const changed = await backend.moveStatus(project.id, inTest.id, 1);

      expect(names(changed)).toEqual(['To do', 'In test', 'In progress', 'In revision', 'Done']);
    });

    it('treats a place past the end as the last place', async () => {
      const todo = statusNamed(project, 'To do');

      const changed = await backend.moveStatus(project.id, todo.id, 99);

      expect(names(changed).at(-1)).toBe('To do');
    });

    it('reports a status the project does not have', async () => {
      expect((await refusal(backend.moveStatus(project.id, 'missing', 0))).code).toBe(
        'rule_broken',
      );
    });
  });

  describe('removing a status', () => {
    it('removes a status that has no tasks', async () => {
      const inRevision = statusNamed(project, 'In revision');

      const changed = await backend.removeStatus(project.id, inRevision.id);

      expect(names(changed)).toEqual(['To do', 'In progress', 'In test', 'Done']);
    });

    it('refuses to remove a status with tasks while they have nowhere to go', async () => {
      const inRevision = statusNamed(project, 'In revision');
      await taskIn('Define the pin map', inRevision);

      const error = await refusal(backend.removeStatus(project.id, inRevision.id));

      expect(error.code).toBe('status_in_use');
      expect(error.message).toBe(
        'the status "In revision" still has 1 task, which need another status to go to',
      );
      expect(names((await backend.listProjects())[0] ?? project)).toContain('In revision');
    });

    it('puts the tasks of a removed status at the bottom of another, in their order', async () => {
      const inProgress = statusNamed(project, 'In progress');
      const inRevision = statusNamed(project, 'In revision');
      await taskIn('Draw the schematic', inProgress);
      await taskIn('Define the pin map', inRevision);
      await taskIn('Check the footprints', inRevision);

      await backend.removeStatus(project.id, inRevision.id, inProgress.id);

      expect(await titlesIn(inProgress.id)).toEqual([
        'Draw the schematic',
        'Define the pin map',
        'Check the footprints',
      ]);
    });

    it('completes tasks that go to a done status', async () => {
      const inTest = statusNamed(project, 'In test');
      const tested = await taskIn('Select the sensors', inTest);

      await backend.removeStatus(project.id, inTest.id, statusNamed(project, 'Done').id);

      const [stored] = (await backend.listTasks(project.id)).filter(
        (listed) => listed.id === tested.id,
      );
      expect(stored?.completedAt).toBe('2026-11-11T09:00:00.000Z');
    });

    it('refuses to remove the only status in which work has not started', async () => {
      const error = await refusal(
        backend.removeStatus(project.id, statusNamed(project, 'To do').id),
      );

      expect(error.message).toBe('a workflow needs at least one status in the "todo" category');
    });

    it('refuses to send the tasks to a status the project does not have', async () => {
      const inRevision = statusNamed(project, 'In revision');
      await taskIn('Define the pin map', inRevision);

      const error = await refusal(backend.removeStatus(project.id, inRevision.id, 'missing'));

      expect(error.code).toBe('rule_broken');
    });
  });
});
