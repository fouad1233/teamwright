import { beforeEach, describe, expect, it } from 'vitest';

import type { Project, Status, WorkflowTemplate } from '../types';
import { DemoBackend } from './demoBackend';
import { refusal, testEnvironment } from './testing';

/*
 * The templates of a new project and the work-in-progress limits of its
 * statuses (decision 0033), in the demo backend. The cases follow the tests
 * of the application in `crates/application/src/projects.rs`.
 */

function statusNamed(project: Project, name: string): Status {
  const status = project.statuses.find((candidate) => candidate.name === name);
  if (!status) {
    throw new Error(`the project has no status named ${name}`);
  }
  return status;
}

function limits(project: Project): (number | null)[] {
  return project.statuses.map((status) => status.limit ?? null);
}

describe('the templates of a new project in the DemoBackend', () => {
  let backend: DemoBackend;

  beforeEach(() => {
    backend = new DemoBackend(undefined, testEnvironment());
  });

  async function made(workflow: WorkflowTemplate) {
    const project = await backend.createProject({ name: 'Smart sensor node', workflow });
    return {
      project,
      views: (await backend.listViews(project.id)).map((view) => view.kind),
      areas: (await backend.listAreas(project.id)).map((area) => area.name),
      sprints: await backend.listSprints(project.id),
      members: await backend.listMembers(project.id),
    };
  }

  it('lists the templates of the program in their order', async () => {
    const templates = await backend.listProjectTemplates();

    expect(templates.map((template) => template.key)).toEqual([
      'standard',
      'kanban',
      'scrum',
      'hardware',
      'engineering',
    ]);
    for (const template of templates) {
      expect(template.description.split('. ')).toHaveLength(2);
    }
  });

  it('makes a standard project as projects were made before templates', async () => {
    const { project, views, areas, sprints } = await made('standard');

    expect(project.statuses.map((status) => status.name)).toEqual([
      'To do',
      'In progress',
      'In review',
      'Done',
    ]);
    expect(limits(project)).toEqual([null, null, null, null]);
    expect(views).toEqual(['board', 'backlog', 'list', 'gantt', 'calendar', 'dashboard']);
    expect(areas).toEqual(['General']);
    expect(sprints).toEqual([]);
  });

  it('makes a Kanban project with limits and without a backlog or a sprint', async () => {
    const { project, views, sprints } = await made('kanban');

    expect(project.statuses.map((status) => status.name)).toEqual([
      'Backlog',
      'Ready',
      'In progress',
      'Review',
      'Done',
    ]);
    expect(limits(project)).toEqual([null, 5, 3, 2, null]);
    expect(views).toEqual(['board', 'list', 'calendar', 'dashboard']);
    expect(sprints).toEqual([]);
  });

  it('makes a Scrum project with the backlog first and a first sprint planned', async () => {
    const { project, views, sprints } = await made('scrum');

    expect(views.slice(0, 2)).toEqual(['backlog', 'board']);
    expect(sprints.map((sprint) => [sprint.name, sprint.state, sprint.startOn])).toEqual([
      ['Sprint 1', 'planned', null],
    ]);
    const second = await backend.createSprint({ projectId: project.id });
    expect(second.name).toBe('Sprint 2');
  });

  it('makes a hardware product with the steps of a device and an area for each part', async () => {
    const { project, areas } = await made('hardware');

    expect(project.statuses.map((status) => status.name)).toEqual([
      'To do',
      'Design',
      'Design review',
      'Ordered',
      'Build',
      'Test',
      'Done',
    ]);
    expect(areas).toEqual(['Electronics', 'Firmware', 'Enclosure']);
    const task = await backend.createTask({ projectId: project.id, title: 'Draw the schematic' });
    const [electronics] = await backend.listAreas(project.id);
    expect(task.areaId).toBe(electronics?.id);
  });

  it('gives no project a team', async () => {
    for (const template of await backend.listProjectTemplates()) {
      const { members } = await made(template.key);
      expect(members).toEqual([]);
    }
  });
});

describe('work-in-progress limits in the DemoBackend', () => {
  let backend: DemoBackend;
  let project: Project;

  beforeEach(async () => {
    backend = new DemoBackend(undefined, testEnvironment());
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'engineering' });
  });

  it('gives a status a limit, another one and none', async () => {
    const inTest = statusNamed(project, 'In test');

    const limited = await backend.setStatusLimit(project.id, inTest.id, 2);
    expect(statusNamed(limited, 'In test').limit).toBe(2);
    const changed = await backend.setStatusLimit(project.id, inTest.id, 4);
    expect(statusNamed(changed, 'In test').limit).toBe(4);
    const cleared = await backend.setStatusLimit(project.id, inTest.id, null);
    expect(statusNamed(cleared, 'In test').limit).toBeNull();
  });

  it('refuses a limit out of range, not whole, or on a done status', async () => {
    const inTest = statusNamed(project, 'In test');
    const done = statusNamed(project, 'Done');

    for (const wrong of [0, 1000, 2.5]) {
      const error = await refusal(backend.setStatusLimit(project.id, inTest.id, wrong));
      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe('a limit is a whole number of tasks from 1 to 999');
    }
    const onDone = await refusal(backend.setStatusLimit(project.id, done.id, 3));
    expect(onDone.code).toBe('rule_broken');
  });

  it('lets a task into a full status, and counts it over the limit', async () => {
    const todo = statusNamed(project, 'To do');
    const inProgress = statusNamed(project, 'In progress');
    await backend.setStatusLimit(project.id, inProgress.id, 1);
    await backend.createTask({
      projectId: project.id,
      title: 'Draw the schematic',
      statusId: inProgress.id,
    });
    const waiting = await backend.createTask({
      projectId: project.id,
      title: 'Define the pin map',
      statusId: todo.id,
    });

    const moved = await backend.moveTask(waiting.id, inProgress.id, 1);

    expect(moved.statusId).toBe(inProgress.id);
    const loads = await backend.listStatusLoads(project.id);
    expect(loads.find((load) => load.statusId === inProgress.id)).toEqual({
      statusId: inProgress.id,
      count: 2,
      limit: 1,
      fill: 'over',
    });
  });

  it('counts every task of the project but not its subtasks', async () => {
    const inProgress = statusNamed(project, 'In progress');
    await backend.setStatusLimit(project.id, inProgress.id, 2);
    const parent = await backend.createTask({
      projectId: project.id,
      title: 'Draw the schematic',
      statusId: inProgress.id,
    });
    await backend.createTask({
      projectId: project.id,
      title: 'Draw the power supply',
      statusId: inProgress.id,
      parentId: parent.id,
    });
    await backend.createTask({
      projectId: project.id,
      title: 'Define the pin map',
      statusId: inProgress.id,
    });

    const loads = await backend.listStatusLoads(project.id);

    expect(loads.map((load) => load.count)).toEqual([0, 2, 0, 0, 0]);
    expect(loads.map((load) => load.fill)).toEqual([null, 'at', null, null, null]);
  });

  it('drops the limit of a status that becomes a done status', async () => {
    const inTest = statusNamed(project, 'In test');
    await backend.setStatusLimit(project.id, inTest.id, 2);

    const changed = await backend.changeStatus(project.id, inTest.id, {
      name: 'Accepted',
      category: 'done',
      color: 'green',
    });

    expect(statusNamed(changed, 'Accepted').limit).toBeNull();
  });
});
