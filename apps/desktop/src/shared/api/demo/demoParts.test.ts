import { beforeEach, describe, expect, it } from 'vitest';

import type { Project, Task } from '../types';
import { DemoBackend } from './demoBackend';
import { refusal, testEnvironment } from './testing';

/**
 * Moving a task that exists under another task, or out from under one, as
 * the tests of `make_part_of` in the application describe it: the same
 * cases, with the same codes and messages.
 */
describe('moving a task under another task', () => {
  let backend: DemoBackend;
  let project: Project;
  let board: Task;
  let schematic: Task;
  let symbols: Task;
  let manual: Task;

  /** A board with a part that has a part, and a task of its own after it. */
  beforeEach(async () => {
    backend = new DemoBackend(undefined, testEnvironment());
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'standard' });
    const create = (title: string, parentId?: string) =>
      backend.createTask({ projectId: project.id, title, ...(parentId ? { parentId } : {}) });
    board = await create('Design the board');
    schematic = await create('Draw the schematic', board.id);
    symbols = await create('Draw the symbols', schematic.id);
    manual = await create('Write the manual');
  });

  async function now(task: Task): Promise<Task> {
    const tasks = await backend.listTasks(project.id);
    const found = tasks.find((listed) => listed.id === task.id);
    if (!found) {
      throw new Error(`${task.title} is gone`);
    }
    return found;
  }

  it('refuses a part that waits for the task it is a part of', async () => {
    const error = await refusal(backend.addTaskDependency(schematic.id, board.id));

    expect(error.code).toBe('dependency_cycle');
    expect(error.message).toBe(
      'SSN-2 cannot wait for SSN-1, because SSN-1 already waits for SSN-2',
    );
  });

  it('takes its parts into the area and the sprint of its new whole', async () => {
    const documents = await backend.addArea(project.id, 'Documents');
    await backend.moveTaskToArea(manual.id, documents.id);
    const sprint = await backend.createSprint({ projectId: project.id });
    await backend.planTask(manual.id, sprint.id);

    const moved = await backend.makeTaskPartOf(schematic.id, manual.id);

    expect(moved.parentId).toBe(manual.id);
    for (const task of [moved, await now(symbols)]) {
      expect(task.areaId).toBe(documents.id);
      expect(task.sprintId).toBe(sprint.id);
    }
    expect((await now(symbols)).parentId).toBe(schematic.id);
    const history = await backend.listTaskHistory(schematic.id);
    expect(history).toContainEqual(
      expect.objectContaining({
        kind: 'made_part_of',
        fromTaskId: board.id,
        toTaskId: manual.id,
      }),
    );
  });

  it('keeps the area and the sprint of a part made a task of its own', async () => {
    const sprint = await backend.createSprint({ projectId: project.id });
    await backend.planTask(board.id, sprint.id);

    const alone = await backend.makeTaskPartOf(schematic.id, null);

    expect(alone.parentId).toBeNull();
    expect(alone.sprintId).toBe(sprint.id);
    expect(alone.areaId).toBe(board.areaId);
    expect((await now(symbols)).parentId).toBe(schematic.id);
    await backend.planTask(schematic.id, null);
  });

  it('is not made a part of itself or of its own parts', async () => {
    const itself = await refusal(backend.makeTaskPartOf(board.id, board.id));
    expect(itself.code).toBe('part_in_circle');
    expect(itself.message).toBe('SSN-1 cannot be a part of itself');

    const below = await refusal(backend.makeTaskPartOf(board.id, symbols.id));
    expect(below.message).toBe('SSN-1 cannot be a part of SSN-3, which is a part of SSN-1 already');
    expect((await now(board)).parentId).toBeNull();
  });

  it('is not made a part of a task it waits for', async () => {
    await backend.addTaskDependency(manual.id, board.id);

    const error = await refusal(backend.makeTaskPartOf(manual.id, schematic.id));

    expect(error.code).toBe('part_in_circle');
    expect(error.message).toBe(
      'SSN-4 cannot be a part of SSN-2, because SSN-4 or one of its parts waits for SSN-2, and a task with parts starts only once its parts are finished',
    );
  });

  it('is not made a part of a task of another project', async () => {
    const other = await backend.createProject({ name: 'Display driver', workflow: 'standard' });
    const driver = await backend.createTask({ projectId: other.id, title: 'Write the driver' });

    const error = await refusal(backend.makeTaskPartOf(board.id, driver.id));

    expect(error.code).toBe('different_projects');
  });

  it('changes nothing when moved where it is', async () => {
    const same = await backend.makeTaskPartOf(schematic.id, board.id);

    expect(same).toEqual(schematic);
  });
});
