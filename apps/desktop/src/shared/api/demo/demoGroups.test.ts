import { beforeEach, describe, expect, it } from 'vitest';

import type { GroupKey, Project, TaskGroup, View } from '../types';
import { DemoBackend } from './demoBackend';
import { refusal, testEnvironment } from './testing';

/**
 * The groups of the tasks of a view, as `grouping.rs` and the use case
 * `groups.rs` describe them for the real application: the same cases.
 */
describe('grouping the tasks of a view in the DemoBackend', () => {
  let backend: DemoBackend;
  let project: Project;
  let board: View;
  let list: View;

  beforeEach(async () => {
    backend = new DemoBackend(undefined, testEnvironment());
    project = await backend.createProject({ name: 'Air quality node', workflow: 'standard' });
    const views = await backend.listViews(project.id);
    board = views.find((view) => view.kind === 'board') as View;
    list = views.find((view) => view.kind === 'list') as View;
  });

  async function groups(view: View): Promise<TaskGroup[]> {
    return backend.listTaskGroups(view.id);
  }

  function keys(found: TaskGroup[]): GroupKey[] {
    return found.map((group) => group.key);
  }

  it('groups a new view by status, with every status as a group', async () => {
    const task = await backend.createTask({ projectId: project.id, title: 'Draw the schematic' });

    const found = await groups(board);

    expect(board.grouping).toBe('status');
    expect(found).toHaveLength(project.statuses.length);
    expect(found[0]?.taskIds).toEqual([task.id]);
  });

  it('puts a subtask with its parent and not in a group of its own', async () => {
    const parent = await backend.createTask({ projectId: project.id, title: 'Parent' });
    await backend.createTask({ projectId: project.id, title: 'Child', parentId: parent.id });

    const found = await groups(board);

    expect(found[0]?.taskIds).toEqual([parent.id]);
  });

  it('groups by assignee: the manager, the team in its order, then nobody', async () => {
    const zoe = await backend.addMember({ projectId: project.id, name: 'Zoe', kind: 'agent' });
    const adam = await backend.addMember({ projectId: project.id, name: 'Adam', kind: 'agent' });
    const hers = await backend.createTask({ projectId: project.id, title: 'Hers' });
    const nobodys = await backend.createTask({ projectId: project.id, title: "Nobody's" });
    await backend.assignTask(hers.id, { kind: 'member', memberId: zoe.id });
    await backend.setViewGrouping(board.id, 'assignee');

    const found = await groups(board);

    expect(keys(found)).toEqual([
      { kind: 'manager' },
      { kind: 'member', id: zoe.id },
      { kind: 'member', id: adam.id },
      { kind: 'nobody' },
    ]);
    expect(found.map((group) => group.taskIds)).toEqual([[], [hers.id], [], [nobodys.id]]);
  });

  it('groups by priority from the highest, every priority being a group', async () => {
    const fire = await backend.createTask({ projectId: project.id, title: 'Fire' });
    const usual = await backend.createTask({ projectId: project.id, title: 'Usual' });
    await backend.prioritizeTask(fire.id, 'urgent');
    await backend.setViewGrouping(list.id, 'priority');

    const found = await groups(list);

    expect(keys(found)).toEqual([
      { kind: 'priority', priority: 'urgent' },
      { kind: 'priority', priority: 'high' },
      { kind: 'priority', priority: 'normal' },
      { kind: 'priority', priority: 'low' },
    ]);
    expect(found.map((group) => group.taskIds)).toEqual([[fire.id], [], [usual.id], []]);
  });

  it('groups by area in the order of the areas', async () => {
    const hardware = await backend.addArea(project.id, 'Hardware');
    const [general] = await backend.listAreas(project.id);
    const task = await backend.createTask({ projectId: project.id, title: 'Board layout' });
    await backend.moveTaskToArea(task.id, hardware.id);
    await backend.setViewGrouping(list.id, 'area');

    const found = await groups(list);

    expect(keys(found)).toEqual([
      { kind: 'area', id: general?.id },
      { kind: 'area', id: hardware.id },
    ]);
    expect(found[1]?.taskIds).toEqual([task.id]);
  });

  it('groups by tag by name, a task with several tags showing under each, and "no tag" last', async () => {
    const emc = await backend.addTag(project.id, 'emc');
    const battery = await backend.addTag(project.id, 'Battery');
    const both = await backend.createTask({ projectId: project.id, title: 'Both' });
    const plain = await backend.createTask({ projectId: project.id, title: 'Plain' });
    await backend.putTagOnTask(both.id, emc.id);
    await backend.putTagOnTask(both.id, battery.id);
    await backend.setViewGrouping(list.id, 'tag');

    const found = await groups(list);

    expect(keys(found)).toEqual([
      { kind: 'tag', id: battery.id },
      { kind: 'tag', id: emc.id },
      { kind: 'noTag' },
    ]);
    expect(found.map((group) => group.taskIds)).toEqual([[both.id], [both.id], [plain.id]]);
  });

  it('groups by sprint in the listed order, with the backlog last', async () => {
    const first = await backend.createSprint({ projectId: project.id });
    const second = await backend.createSprint({ projectId: project.id });
    const planned = await backend.createTask({ projectId: project.id, title: 'Planned' });
    const loose = await backend.createTask({ projectId: project.id, title: 'Loose' });
    await backend.planTask(planned.id, first.id);
    await backend.setViewGrouping(board.id, 'sprint');

    const found = await groups(board);

    expect(keys(found)).toEqual([
      { kind: 'sprint', id: first.id },
      { kind: 'sprint', id: second.id },
      { kind: 'backlog' },
    ]);
    expect(found[0]?.taskIds).toEqual([planned.id]);
    expect(found[2]?.taskIds).toEqual([loose.id]);
  });

  it('refuses to group a view that has no groups by anything but status', async () => {
    const calendar = (await backend.listViews(project.id)).find(
      (view) => view.kind === 'calendar',
    ) as View;

    const refused = await refusal(backend.setViewGrouping(calendar.id, 'assignee'));

    expect(refused.code).toBe('rule_broken');
    expect((await backend.setViewGrouping(calendar.id, 'status')).grouping).toBe('status');
  });

  it('keeps the grouping with the view', async () => {
    await backend.setViewGrouping(board.id, 'priority');

    const views = await backend.listViews(project.id);

    expect(views.find((view) => view.id === board.id)?.grouping).toBe('priority');
  });

  it('has no groups for a view that does not exist', async () => {
    const refused = await refusal(backend.listTaskGroups('nothing'));

    expect(refused.code).toBe('not_found');
  });
});
