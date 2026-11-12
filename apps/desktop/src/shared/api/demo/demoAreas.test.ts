import { beforeEach, describe, expect, it } from 'vitest';

import type { Area, Project, Task } from '../types';
import { DemoBackend } from './demoBackend';
import { sampleWorkspace } from './sampleWorkspace';
import { refusal, testEnvironment } from './testing';

describe('the areas of the sample workspace', () => {
  it('cuts the sample project into three areas and puts every task into one of them', async () => {
    const workspace = sampleWorkspace(new Date('2026-11-11T09:00:00.000Z'));
    const backend = new DemoBackend(workspace, testEnvironment());
    const [project] = workspace.projects;

    const areas = await backend.listAreas(project?.id ?? '');
    const tasks = await backend.listTasks(project?.id ?? '');

    expect(areas.map((area) => area.name)).toEqual(['Electronics', 'Firmware', 'Enclosure']);
    for (const area of areas) {
      expect(area.projectId).toBe(project?.id);
      expect(tasks.some((task) => task.areaId === area.id)).toBe(true);
    }
    for (const task of tasks) {
      const parent = tasks.find((other) => other.id === task.parentId);
      expect(task.areaId).toBe(parent?.areaId ?? task.areaId);
      expect(areas.map((area) => area.id)).toContain(task.areaId);
    }
  });
});

describe('areas in the DemoBackend', () => {
  let backend: DemoBackend;
  let project: Project;

  beforeEach(async () => {
    backend = new DemoBackend(undefined, testEnvironment());
    project = await backend.createProject({ name: 'Air quality node', workflow: 'standard' });
  });

  async function areas(): Promise<Area[]> {
    return backend.listAreas(project.id);
  }

  async function names(): Promise<string[]> {
    return (await areas()).map((area) => area.name);
  }

  /** The area of the project that has a name. */
  async function areaNamed(name: string): Promise<Area> {
    const found = (await areas()).find((area) => area.name === name);
    if (!found) {
      throw new Error(`the project has no area called ${name}`);
    }
    return found;
  }

  /** Gives the project the areas named, after "General". */
  async function addAreas(...added: string[]): Promise<void> {
    for (const name of added) {
      await backend.addArea(project.id, name);
    }
  }

  async function task(title: string, details: Partial<Task> = {}): Promise<Task> {
    return backend.createTask({
      projectId: project.id,
      title,
      ...(details.parentId ? { parentId: details.parentId } : {}),
      ...(details.areaId ? { areaId: details.areaId } : {}),
    });
  }

  async function stored(taskId: string): Promise<Task | undefined> {
    return (await backend.listTasks(project.id)).find((candidate) => candidate.id === taskId);
  }

  describe('the areas of a project', () => {
    it('starts a new project with one area called "General"', async () => {
      const listed = await areas();

      expect(listed).toHaveLength(1);
      expect(listed[0]).toMatchObject({ projectId: project.id, name: 'General' });
    });

    it('gives "General" to a project that was kept before it had areas, with all its tasks in it', async () => {
      const first = await task('Choose the sensors');
      const second = await task('Draw the schematic');
      // A task as it was kept before tasks had areas.
      const withoutArea = (kept: Task) => {
        const old: Partial<Task> = { ...kept };
        delete old.areaId;
        return old as Task;
      };
      const kept = new DemoBackend(
        { projects: [project], tasks: [withoutArea(first), withoutArea(second)], members: [] },
        testEnvironment(),
      );

      const listed = await kept.listAreas(project.id);
      const tasks = await kept.listTasks(project.id);

      expect(listed.map((area) => area.name)).toEqual(['General']);
      expect(tasks.map((kept) => kept.areaId)).toEqual([listed[0]?.id, listed[0]?.id]);
    });

    it('puts a kept task whose area the project does not have into the first area', async () => {
      await addAreas('Firmware');
      const [general, firmware] = await areas();
      const lost = { ...(await task('Write the drivers')), areaId: 'gone' };
      const kept = new DemoBackend(
        { projects: [project], tasks: [lost], members: [], areas: [firmware, general] as Area[] },
        testEnvironment(),
      );

      expect((await kept.listTasks(project.id))[0]?.areaId).toBe(firmware?.id);
    });

    it('refuses to list the areas of a project that does not exist', async () => {
      const error = await refusal(backend.listAreas('nowhere'));

      expect(error.code).toBe('not_found');
      expect(error.message).toBe('there is no project with the identifier nowhere');
    });

    it('removes the areas with their project', async () => {
      const general = await areaNamed('General');

      await backend.deleteProject(project.id);

      expect((await refusal(backend.listAreas(project.id))).code).toBe('not_found');
      const error = await refusal(backend.renameArea(general.id, 'Board'));
      expect(error.code).toBe('not_found');
      expect(error.message).toBe(`there is no area with the identifier ${general.id}`);
    });

    it('keeps the areas of a project apart from those of another', async () => {
      const other = await backend.createProject({ name: 'Weather station', workflow: 'standard' });

      await backend.addArea(other.id, 'Firmware');

      expect(await names()).toEqual(['General']);
      expect((await backend.listAreas(other.id)).map((area) => area.name)).toEqual([
        'General',
        'Firmware',
      ]);
    });
  });

  describe('adding an area', () => {
    it('adds it at the end', async () => {
      const added = await backend.addArea(project.id, 'Firmware');

      expect(added).toMatchObject({ projectId: project.id, name: 'Firmware' });
      expect((await areas()).at(-1)).toEqual(added);
    });

    it('trims the name it is given and puts it on one line', async () => {
      const added = await backend.addArea(project.id, '  Power \n  supply ');

      expect(added.name).toBe('Power supply');
    });

    it('accepts a name of 40 characters', async () => {
      const added = await backend.addArea(project.id, 'x'.repeat(40));

      expect(added.name).toHaveLength(40);
    });

    it('refuses a name of more than 40 characters', async () => {
      const error = await refusal(backend.addArea(project.id, 'x'.repeat(41)));

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe(
        'the area name can have at most 40 characters, but this one has 41',
      );
    });

    it('refuses an empty name', async () => {
      const error = await refusal(backend.addArea(project.id, ' \n '));

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe('the area name must not be empty');
    });

    it('refuses a name another area of the project has, whatever the capitals', async () => {
      const error = await refusal(backend.addArea(project.id, ' GENERAL '));

      expect(error.code).toBe('area_name_taken');
      expect(error.message).toBe('the project already has an area called "GENERAL"');
    });

    it('accepts a name an area of another project has', async () => {
      const other = await backend.createProject({ name: 'Weather station', workflow: 'standard' });
      await backend.addArea(other.id, 'Firmware');

      const added = await backend.addArea(project.id, 'Firmware');

      expect(added.name).toBe('Firmware');
    });

    it('refuses an area for a project that does not exist', async () => {
      const error = await refusal(backend.addArea('nowhere', 'Firmware'));

      expect(error.code).toBe('not_found');
      expect(error.message).toBe('there is no project with the identifier nowhere');
    });
  });

  describe('renaming an area', () => {
    it('trims the new name and keeps the place of the area', async () => {
      await addAreas('Firmware', 'Enclosure');
      const firmware = await areaNamed('Firmware');

      const renamed = await backend.renameArea(firmware.id, '  Software  ');

      expect(renamed).toEqual({ ...firmware, name: 'Software' });
      expect(await names()).toEqual(['General', 'Software', 'Enclosure']);
    });

    it('lets an area change the capitals of its own name', async () => {
      const renamed = await backend.renameArea((await areaNamed('General')).id, 'GENERAL');

      expect(renamed.name).toBe('GENERAL');
    });

    it('refuses an empty name', async () => {
      const error = await refusal(backend.renameArea((await areaNamed('General')).id, '   '));

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe('the area name must not be empty');
    });

    it('refuses a name of more than 40 characters', async () => {
      const error = await refusal(
        backend.renameArea((await areaNamed('General')).id, 'y'.repeat(45)),
      );

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe(
        'the area name can have at most 40 characters, but this one has 45',
      );
    });

    it('refuses the name of another area of the project, whatever the capitals', async () => {
      await addAreas('Firmware');

      const error = await refusal(backend.renameArea((await areaNamed('General')).id, 'firmware'));

      expect(error.code).toBe('area_name_taken');
      expect(error.message).toBe('the project already has an area called "firmware"');
    });

    it('refuses an area that does not exist', async () => {
      const error = await refusal(backend.renameArea('nowhere', 'Board'));

      expect(error.code).toBe('not_found');
      expect(error.message).toBe('there is no area with the identifier nowhere');
    });
  });

  describe('moving an area', () => {
    beforeEach(async () => {
      await addAreas('Board', 'Firmware', 'Enclosure');
    });

    it('puts it at the place it is given and answers with the areas in their new order', async () => {
      const moved = await backend.moveArea((await areaNamed('Enclosure')).id, 0);

      const order = ['Enclosure', 'General', 'Board', 'Firmware'];
      expect(moved.map((area) => area.name)).toEqual(order);
      expect(await names()).toEqual(order);
    });

    it('moves an area one place down', async () => {
      await backend.moveArea((await areaNamed('General')).id, 1);

      expect(await names()).toEqual(['Board', 'General', 'Firmware', 'Enclosure']);
    });

    it('puts it last for a place past the end, and first for a place before the start', async () => {
      await backend.moveArea((await areaNamed('General')).id, 99);
      await backend.moveArea((await areaNamed('Firmware')).id, -3);

      expect(await names()).toEqual(['Firmware', 'Board', 'Enclosure', 'General']);
    });

    it('refuses an area that does not exist', async () => {
      const error = await refusal(backend.moveArea('nowhere', 0));

      expect(error.code).toBe('not_found');
      expect(error.message).toBe('there is no area with the identifier nowhere');
    });
  });

  describe('removing an area', () => {
    it('removes an area without tasks and keeps the others in their order', async () => {
      await addAreas('Board', 'Firmware');

      await backend.removeArea((await areaNamed('Board')).id);

      expect(await names()).toEqual(['General', 'Firmware']);
    });

    it('does not look at where the tasks go when the area has none', async () => {
      await addAreas('Board');

      await backend.removeArea((await areaNamed('Board')).id, 'nowhere');

      expect(await names()).toEqual(['General']);
    });

    it('refuses to remove the last area of a project', async () => {
      const error = await refusal(backend.removeArea((await areaNamed('General')).id));

      expect(error.code).toBe('last_area');
      expect(error.message).toBe('"General" is the last area of the project and cannot be removed');
      expect(await names()).toEqual(['General']);
    });

    it('refuses to remove an area with a task without saying where it goes', async () => {
      await addAreas('Firmware');
      await task('Write the drivers', { areaId: (await areaNamed('Firmware')).id });

      const error = await refusal(backend.removeArea((await areaNamed('Firmware')).id));

      expect(error.code).toBe('area_in_use');
      expect(error.message).toBe(
        'the area "Firmware" still has 1 task, which need another area to go to',
      );
    });

    it('counts the subtasks among the tasks that need another area', async () => {
      await addAreas('Firmware');
      const firmware = await areaNamed('Firmware');
      const drivers = await task('Write the drivers', { areaId: firmware.id });
      await task('Driver for the oxygen sensor', { parentId: drivers.id });
      await task('Driver for the humidity sensor', { parentId: drivers.id });

      const error = await refusal(backend.removeArea(firmware.id, firmware.id));

      expect(error.code).toBe('area_in_use');
      expect(error.message).toBe(
        'the area "Firmware" still has 3 tasks, which need another area to go to',
      );
      expect(await names()).toEqual(['General', 'Firmware']);
    });

    it('moves its tasks to the area it is given, in their status and at their place', async () => {
      await addAreas('Firmware');
      const general = await areaNamed('General');
      const firmware = await areaNamed('Firmware');
      const drivers = await task('Write the drivers', { areaId: firmware.id });
      const part = await task('Driver for the oxygen sensor', { parentId: drivers.id });

      await backend.removeArea(firmware.id, general.id);

      expect(await names()).toEqual(['General']);
      expect(await stored(drivers.id)).toMatchObject({
        areaId: general.id,
        statusId: drivers.statusId,
        rank: drivers.rank,
      });
      expect((await stored(part.id))?.areaId).toBe(general.id);
    });

    it('refuses to move its tasks to an area of another project', async () => {
      await addAreas('Firmware');
      const firmware = await areaNamed('Firmware');
      await task('Write the drivers', { areaId: firmware.id });
      const other = await backend.createProject({ name: 'Weather station', workflow: 'standard' });
      const [elsewhere] = await backend.listAreas(other.id);

      const error = await refusal(backend.removeArea(firmware.id, elsewhere?.id));

      expect(error.code).toBe('different_projects');
      expect(error.message).toBe(
        'the tasks of an area can only go to another area of the same project',
      );
      expect(await names()).toEqual(['General', 'Firmware']);
    });

    it('refuses to move its tasks to an area that does not exist', async () => {
      await addAreas('Firmware');
      const firmware = await areaNamed('Firmware');
      await task('Write the drivers', { areaId: firmware.id });

      const error = await refusal(backend.removeArea(firmware.id, 'nowhere'));

      expect(error.code).toBe('not_found');
      expect(error.message).toBe('there is no area with the identifier nowhere');
    });

    it('refuses an area that does not exist', async () => {
      const error = await refusal(backend.removeArea('nowhere'));

      expect(error.code).toBe('not_found');
      expect(error.message).toBe('there is no area with the identifier nowhere');
    });
  });

  describe('the area of a new task', () => {
    it('puts a task into the first area when none is named', async () => {
      await addAreas('Firmware');
      await backend.moveArea((await areaNamed('Firmware')).id, 0);

      const created = await task('Write the drivers');

      expect(created.areaId).toBe((await areaNamed('Firmware')).id);
    });

    it('puts a task into the area it names', async () => {
      await addAreas('Firmware', 'Enclosure');
      const enclosure = await areaNamed('Enclosure');

      const created = await task('Design the enclosure', { areaId: enclosure.id });

      expect(created.areaId).toBe(enclosure.id);
    });

    it('puts a subtask into the area of its task, named or not', async () => {
      await addAreas('Firmware');
      const firmware = await areaNamed('Firmware');
      const drivers = await task('Write the drivers', { areaId: firmware.id });

      const first = await task('Driver for the oxygen sensor', { parentId: drivers.id });
      const second = await task('Driver for the humidity sensor', {
        parentId: drivers.id,
        areaId: firmware.id,
      });

      expect(first.areaId).toBe(firmware.id);
      expect(second.areaId).toBe(firmware.id);
    });

    it('refuses a subtask in another area than its task', async () => {
      await addAreas('Firmware');
      const drivers = await task('Write the drivers', {
        areaId: (await areaNamed('Firmware')).id,
      });

      const error = await refusal(
        task('Driver for the oxygen sensor', {
          parentId: drivers.id,
          areaId: (await areaNamed('General')).id,
        }),
      );

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe('a subtask is in the area of the task it is a part of, AQN-1');
    });

    it('refuses an area of another project', async () => {
      const other = await backend.createProject({ name: 'Weather station', workflow: 'standard' });
      const [elsewhere] = await backend.listAreas(other.id);

      const error = await refusal(task('Write the drivers', { areaId: elsewhere?.id ?? '' }));

      expect(error.code).toBe('different_projects');
      expect(error.message).toBe('a task can only be put into an area of its own project');
    });

    it('refuses an area that does not exist', async () => {
      const error = await refusal(task('Write the drivers', { areaId: 'nowhere' }));

      expect(error.code).toBe('not_found');
      expect(error.message).toBe('there is no area with the identifier nowhere');
    });
  });

  describe('moving a task to another area', () => {
    it('moves the task with its subtasks and answers with the task', async () => {
      await addAreas('Firmware');
      const firmware = await areaNamed('Firmware');
      const drivers = await task('Write the drivers');
      const part = await task('Driver for the oxygen sensor', { parentId: drivers.id });
      const deeper = await task('Read the sensor in sleep', { parentId: part.id });

      const moved = await backend.moveTaskToArea(drivers.id, firmware.id);

      expect(moved).toMatchObject({ id: drivers.id, areaId: firmware.id });
      expect((await stored(part.id))?.areaId).toBe(firmware.id);
      expect((await stored(deeper.id))?.areaId).toBe(firmware.id);
    });

    it('keeps the status, the place and the sprint of the task', async () => {
      await addAreas('Firmware');
      const sprint = await backend.createSprint({ projectId: project.id });
      const drivers = await task('Write the drivers');
      await backend.planTask(drivers.id, sprint.id);

      const moved = await backend.moveTaskToArea(drivers.id, (await areaNamed('Firmware')).id);

      expect(moved).toMatchObject({
        statusId: drivers.statusId,
        rank: drivers.rank,
        sprintId: sprint.id,
      });
    });

    it('changes nothing when the task is in the area already', async () => {
      const drivers = await task('Write the drivers');

      const moved = await backend.moveTaskToArea(drivers.id, drivers.areaId);

      expect(moved).toEqual(drivers);
    });

    it('refuses to move a subtask by itself', async () => {
      await addAreas('Firmware');
      const drivers = await task('Write the drivers');
      const part = await task('Driver for the oxygen sensor', { parentId: drivers.id });

      const error = await refusal(
        backend.moveTaskToArea(part.id, (await areaNamed('Firmware')).id),
      );

      expect(error.code).toBe('subtask_moved_alone');
      expect(error.message).toBe('AQN-2 is a part of AQN-1 and goes where it goes');
    });

    it('refuses an area of another project', async () => {
      const drivers = await task('Write the drivers');
      const other = await backend.createProject({ name: 'Weather station', workflow: 'standard' });
      const [elsewhere] = await backend.listAreas(other.id);

      const error = await refusal(backend.moveTaskToArea(drivers.id, elsewhere?.id ?? ''));

      expect(error.code).toBe('different_projects');
      expect(error.message).toBe('a task can only be put into an area of its own project');
      expect((await stored(drivers.id))?.areaId).toBe(drivers.areaId);
    });

    it('refuses a task that does not exist', async () => {
      const error = await refusal(
        backend.moveTaskToArea('nowhere', (await areaNamed('General')).id),
      );

      expect(error.code).toBe('not_found');
      expect(error.message).toBe('there is no task with the identifier nowhere');
    });

    it('refuses an area that does not exist', async () => {
      const drivers = await task('Write the drivers');

      const error = await refusal(backend.moveTaskToArea(drivers.id, 'nowhere'));

      expect(error.code).toBe('not_found');
      expect(error.message).toBe('there is no area with the identifier nowhere');
    });
  });
});
