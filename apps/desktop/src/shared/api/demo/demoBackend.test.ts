import { beforeEach, describe, expect, it } from 'vitest';

import { BackendError } from '../backend';
import type { Id, Project, Task } from '../types';
import { DemoBackend, type DemoEnvironment } from './demoBackend';
import { sampleWorkspace } from './sampleWorkspace';

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

function statusOf(project: Project, category: string): Id {
  const status = project.statuses.find((candidate) => candidate.category === category);
  if (!status) {
    throw new Error(`the project has no ${category} status`);
  }
  return status.id;
}

describe('DemoBackend', () => {
  let environment: ReturnType<typeof testEnvironment>;
  let backend: DemoBackend;
  let project: Project;

  beforeEach(async () => {
    environment = testEnvironment();
    backend = new DemoBackend(undefined, environment);
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'engineering' });
  });

  async function task(title: string): Promise<Task> {
    return backend.createTask({ projectId: project.id, title });
  }

  async function titlesIn(statusId: Id): Promise<string[]> {
    const tasks = await backend.listTasks(project.id);
    return tasks.filter((listed) => listed.statusId === statusId).map((listed) => listed.title);
  }

  describe('projects', () => {
    it('derives the key from the initials of the name', () => {
      expect(project.key).toBe('SSN');
    });

    it('numbers a derived key that is already taken', async () => {
      const second = await backend.createProject({
        name: 'Smart sensor node',
        workflow: 'standard',
      });

      expect(second.key).toBe('SSN2');
    });

    it('uses a requested key in capitals', async () => {
      const created = await backend.createProject({
        name: 'Display driver',
        key: 'drv',
        workflow: 'standard',
      });

      expect(created.key).toBe('DRV');
    });

    it('refuses a requested key that another project uses', async () => {
      const error = await refusal(
        backend.createProject({ name: 'Other', key: 'ssn', workflow: 'standard' }),
      );

      expect(error.code).toBe('project_key_taken');
    });

    it('refuses a project without a name', async () => {
      const error = await refusal(backend.createProject({ name: '   ', workflow: 'standard' }));

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe('the project name must not be empty');
    });

    it('starts a project with the statuses of the chosen workflow', () => {
      expect(project.statuses.map((status) => status.name)).toEqual([
        'To do',
        'In progress',
        'In revision',
        'In test',
        'Done',
      ]);
    });

    it('does not list an archived project', async () => {
      await backend.archiveProject(project.id);

      expect(await backend.listProjects()).toEqual([]);
    });
  });

  describe('tasks', () => {
    it('numbers tasks from one and puts each at the bottom of the first todo status', async () => {
      const first = await task('Choose the microcontroller');
      const second = await task('Draw the schematic');

      expect([first.number, second.number]).toEqual([1, 2]);
      expect(await titlesIn(statusOf(project, 'todo'))).toEqual([
        'Choose the microcontroller',
        'Draw the schematic',
      ]);
    });

    it('tidies the title and refuses an empty one', async () => {
      const tidy = await task('  Draw   the\nschematic ');
      const error = await refusal(task('  '));

      expect(tidy.title).toBe('Draw the schematic');
      expect(error.message).toBe('the task title must not be empty');
    });

    it('records when a task was changed', async () => {
      const created = await task('Draw the schematic');
      environment.advanceHours(2);

      const renamed = await backend.renameTask(created.id, 'Draw the full schematic');

      expect(renamed.createdAt).toBe('2026-11-11T09:00:00.000Z');
      expect(renamed.updatedAt).toBe('2026-11-11T11:00:00.000Z');
    });

    it('refuses a schedule that starts after it is due', async () => {
      const created = await task('Draw the schematic');

      const error = await refusal(backend.rescheduleTask(created.id, '2026-11-20', '2026-11-12'));

      expect(error.code).toBe('rule_broken');
    });

    it('hands out copies, so changing an answer does not change the backend', async () => {
      const created = await task('Draw the schematic');
      created.title = 'Changed by the caller';

      const [stored] = await backend.listTasks(project.id);

      expect(stored?.title).toBe('Draw the schematic');
    });
  });

  describe('moving tasks', () => {
    it('moves a task to a position in another status', async () => {
      const inProgress = statusOf(project, 'active');
      const first = await task('First');
      const second = await task('Second');
      const third = await task('Third');
      await backend.moveTask(first.id, inProgress, 0);
      await backend.moveTask(second.id, inProgress, 1);

      await backend.moveTask(third.id, inProgress, 1);

      expect(await titlesIn(inProgress)).toEqual(['First', 'Third', 'Second']);
    });

    it('treats a position past the end as the bottom', async () => {
      const todo = statusOf(project, 'todo');
      const first = await task('First');
      await task('Second');

      await backend.moveTask(first.id, todo, 99);

      expect(await titlesIn(todo)).toEqual(['Second', 'First']);
    });

    it('completes a task that reaches a done status and reopens one that leaves it', async () => {
      const created = await task('Order the samples');

      const completed = await backend.moveTask(created.id, statusOf(project, 'done'), 0);
      const reopened = await backend.moveTask(created.id, statusOf(project, 'active'), 0);

      expect(completed.completedAt).toBe('2026-11-11T09:00:00.000Z');
      expect(reopened.completedAt).toBeNull();
    });

    it('keeps the order after more insertions at one spot than ranks can hold', async () => {
      const todo = statusOf(project, 'todo');
      await task('First');
      await task('Second');
      await task('Third');
      const expected = ['First', 'Second'];
      const inserted: string[] = [];

      for (let number = 0; number < 80; number += 1) {
        const title = `Inserted ${number.toString()}`;
        const created = await task(title);
        await backend.moveTask(created.id, todo, 2);
        inserted.push(title);
      }

      expect(await titlesIn(todo)).toEqual([...expected, ...inserted.reverse(), 'Third']);
    });
  });

  describe('dependencies', () => {
    it('lets a task wait for another and stop waiting', async () => {
      const hardware = await task('Design the board');
      const firmware = await task('Write the firmware');

      const waiting = await backend.addTaskDependency(firmware.id, hardware.id);
      const released = await backend.removeTaskDependency(firmware.id, hardware.id);

      expect(waiting.blockedBy).toEqual([hardware.id]);
      expect(released.blockedBy).toEqual([]);
    });

    it('refuses a dependency that would close a loop and names both tasks', async () => {
      const hardware = await task('Design the board');
      const firmware = await task('Write the firmware');
      const enclosure = await task('Design the enclosure');
      await backend.addTaskDependency(firmware.id, hardware.id);
      await backend.addTaskDependency(enclosure.id, firmware.id);

      const error = await refusal(backend.addTaskDependency(hardware.id, enclosure.id));

      expect(error.code).toBe('dependency_cycle');
      expect(error.message).toBe(
        'SSN-1 cannot wait for SSN-3, because SSN-3 already waits for SSN-1',
      );
    });

    it('refuses a task that waits for itself', async () => {
      const hardware = await task('Design the board');

      const error = await refusal(backend.addTaskDependency(hardware.id, hardware.id));

      expect(error.message).toBe('a task cannot depend on itself');
    });
  });

  describe('deleting tasks', () => {
    it('removes the subtasks at every depth and releases waiting tasks', async () => {
      const board = await task('Design the board');
      const schematic = await backend.createTask({
        projectId: project.id,
        title: 'Draw the schematic',
        parentId: board.id,
      });
      await backend.createTask({
        projectId: project.id,
        title: 'Draw the power stage',
        parentId: schematic.id,
      });
      const firmware = await task('Write the firmware');
      await backend.addTaskDependency(firmware.id, schematic.id);

      await backend.deleteTask(board.id);

      const remaining = await backend.listTasks(project.id);
      expect(remaining.map((listed) => listed.title)).toEqual(['Write the firmware']);
      expect(remaining[0]?.blockedBy).toEqual([]);
    });
  });
});

describe('the sample workspace', () => {
  const workspace = sampleWorkspace(new Date('2026-11-11T09:00:00.000Z'));
  const [project] = workspace.projects;

  it('puts every task in a status of its project', () => {
    const statusIds = new Set(project?.statuses.map((status) => status.id));

    expect(workspace.tasks.every((task) => statusIds.has(task.statusId))).toBe(true);
  });

  it('only lets tasks wait for tasks that exist', () => {
    const taskIds = new Set(workspace.tasks.map((task) => task.id));

    expect(workspace.tasks.flatMap((task) => task.blockedBy).every((id) => taskIds.has(id))).toBe(
      true,
    );
  });

  it('continues the task numbers after the highest one in the sample', async () => {
    const backend = new DemoBackend(workspace, testEnvironment());

    const created = await backend.createTask({
      projectId: project?.id ?? '',
      title: 'Write the user manual',
    });

    expect(created.number).toBe(workspace.tasks.length + 1);
  });

  it('counts dates from the given day', () => {
    const first = workspace.tasks.find((task) => task.number === 1);

    expect(first?.startOn).toBe('2026-11-02');
    expect(first?.dueOn).toBe('2026-11-03');
  });
});
