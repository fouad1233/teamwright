import { beforeEach, describe, expect, it } from 'vitest';

import type { Id, Project, Sprint, StatusCategory, Task } from '../types';
import { DemoBackend } from './demoBackend';
import { sampleWorkspace } from './sampleWorkspace';
import { refusal, testEnvironment } from './testing';

/** A day of the clock of the tests, which starts on 11 November 2026 at nine. */
const DAY = 24 * 60 * 60 * 1000;

describe('the sprints of the sample workspace', () => {
  const workspace = sampleWorkspace(new Date('2026-11-11T09:00:00.000Z'));

  it('has a completed sprint, an active one with some work done, and a planned one', async () => {
    const backend = new DemoBackend(workspace, testEnvironment());
    const [project] = workspace.projects;

    const sprints = await backend.listSprints(project?.id ?? '');
    const [active] = sprints;
    const report = await backend.sprintReport(active?.id ?? '');

    expect(sprints.map((sprint) => sprint.state)).toEqual(['active', 'planned', 'completed']);
    expect(report.days.some((day) => day.remaining !== null && day.remaining < report.total)).toBe(
      true,
    );
  });

  it('puts every subtask into the sprint of its task, and leaves a backlog', () => {
    const byId = new Map(workspace.tasks.map((task) => [task.id, task]));

    for (const task of workspace.tasks.filter((candidate) => candidate.parentId !== null)) {
      expect(task.sprintId).toBe(byId.get(task.parentId ?? '')?.sprintId);
    }
    expect(workspace.tasks.some((task) => task.sprintId === null)).toBe(true);
  });
});

describe('sprints and the backlog in the DemoBackend', () => {
  let environment: ReturnType<typeof testEnvironment>;
  let backend: DemoBackend;
  let project: Project;

  beforeEach(async () => {
    environment = testEnvironment();
    backend = new DemoBackend(undefined, environment);
    // To do, In progress, In revision, In test, Done.
    project = await backend.createProject({ name: 'Air quality node', workflow: 'engineering' });
  });

  /** The first status of a category in the workflow of the project. */
  function statusOf(category: StatusCategory): Id {
    const status = project.statuses.find((candidate) => candidate.category === category);
    if (!status) {
      throw new Error(`the workflow has no status of the category ${category}`);
    }
    return status.id;
  }

  async function task(title: string, more: { parentId?: Id } = {}): Promise<Task> {
    return backend.createTask({ projectId: project.id, title, ...more });
  }

  /** A task as it is now. The backend answers with copies. */
  async function now(taskId: Id): Promise<Task> {
    const found = (await backend.listTasks(project.id)).find((other) => other.id === taskId);
    if (!found) {
      throw new Error('the project has no such task');
    }
    return found;
  }

  async function sprintNow(sprintId: Id): Promise<Sprint | undefined> {
    return (await backend.listSprints(project.id)).find((sprint) => sprint.id === sprintId);
  }

  async function finish(taskId: Id): Promise<void> {
    await backend.moveTask(taskId, statusOf('done'), 0);
  }

  /** A planned sprint with one task in it, started from 10 to 23 November. */
  async function activeSprint(): Promise<{ sprint: Sprint; task: Task }> {
    const sprint = await backend.createSprint({ projectId: project.id });
    const planned = await task('Draw the schematic');
    await backend.planTask(planned.id, sprint.id);
    const started = await backend.startSprint(sprint.id, {
      name: sprint.name,
      goal: '',
      startOn: '2026-11-10',
      endOn: '2026-11-23',
    });
    return { sprint: started, task: await now(planned.id) };
  }

  describe('creating a sprint', () => {
    it('plans a sprint without dates, named after the number of sprints the project has had', async () => {
      await backend.createSprint({ projectId: project.id });

      const second = await backend.createSprint({ projectId: project.id, name: '   ' });

      expect(second).toMatchObject({
        projectId: project.id,
        name: 'Sprint 2',
        goal: '',
        startOn: null,
        endOn: null,
        state: 'planned',
        startedAt: null,
        completedAt: null,
        summary: null,
      });
    });

    it('trims the name and the goal, and keeps the dates', async () => {
      const sprint = await backend.createSprint({
        projectId: project.id,
        name: '  The   first board  ',
        goal: ' A schematic that can be routed ',
        startOn: '2026-11-16',
        endOn: '2026-11-29',
      });

      expect(sprint).toMatchObject({
        name: 'The first board',
        goal: 'A schematic that can be routed',
        startOn: '2026-11-16',
        endOn: '2026-11-29',
      });
    });

    it('counts a deleted sprint when it names a new one', async () => {
      const first = await backend.createSprint({ projectId: project.id });
      await backend.deleteSprint(first.id);

      const next = await backend.createSprint({ projectId: project.id });

      expect(next.name).toBe('Sprint 2');
    });

    it('refuses a name of more than 60 characters', async () => {
      const error = await refusal(
        backend.createSprint({ projectId: project.id, name: 'x'.repeat(61) }),
      );

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe(
        'the sprint name can have at most 60 characters, but this one has 61',
      );
    });

    it('refuses a start date after the end date', async () => {
      const error = await refusal(
        backend.createSprint({ projectId: project.id, startOn: '2026-11-20', endOn: '2026-11-19' }),
      );

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe('the start date 2026-11-20 is after the end date 2026-11-19');
    });

    it('refuses a project that does not exist', async () => {
      const error = await refusal(backend.createSprint({ projectId: 'nowhere' }));

      expect(error.code).toBe('not_found');
    });
  });

  describe('listing the sprints', () => {
    it('lists the active sprint, then the planned ones as created, then the completed ones, the latest first', async () => {
      const { sprint: first } = await activeSprint();
      await backend.completeSprint(first.id, { moveUnfinishedTo: null });
      environment.pass(DAY);
      const { sprint: second } = await activeSprint();
      await backend.completeSprint(second.id, { moveUnfinishedTo: null });
      const planned = await backend.createSprint({ projectId: project.id });
      const later = await backend.createSprint({ projectId: project.id });
      const { sprint: active } = await activeSprint();

      const listed = await backend.listSprints(project.id);

      expect(listed.map((sprint) => sprint.id)).toEqual([
        active.id,
        planned.id,
        later.id,
        second.id,
        first.id,
      ]);
    });
  });

  describe('changing a sprint', () => {
    it('replaces the name, the goal and the dates of a planned sprint', async () => {
      const sprint = await backend.createSprint({ projectId: project.id });

      const changed = await backend.changeSprint(sprint.id, {
        name: ' Board bring-up ',
        goal: ' Power on ',
        startOn: '2026-11-16',
        endOn: '2026-11-20',
      });

      expect(changed).toMatchObject({
        name: 'Board bring-up',
        goal: 'Power on',
        startOn: '2026-11-16',
        endOn: '2026-11-20',
        state: 'planned',
      });
    });

    it('changes a sprint that is active', async () => {
      const { sprint } = await activeSprint();

      const changed = await backend.changeSprint(sprint.id, {
        name: sprint.name,
        goal: 'Finish the schematic',
        startOn: '2026-11-10',
        endOn: '2026-11-30',
      });

      expect(changed).toMatchObject({ goal: 'Finish the schematic', endOn: '2026-11-30' });
    });

    it('refuses to take the dates away from a sprint that has started', async () => {
      const { sprint } = await activeSprint();

      const error = await refusal(
        backend.changeSprint(sprint.id, {
          name: sprint.name,
          goal: '',
          startOn: null,
          endOn: null,
        }),
      );

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe('a sprint that has started has a start and an end date');
    });

    it('refuses an empty name', async () => {
      const sprint = await backend.createSprint({ projectId: project.id });

      const error = await refusal(
        backend.changeSprint(sprint.id, { name: '  ', goal: '', startOn: null, endOn: null }),
      );

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe('the sprint name must not be empty');
    });

    it('refuses to change a completed sprint', async () => {
      const { sprint } = await activeSprint();
      await backend.completeSprint(sprint.id, { moveUnfinishedTo: null });

      const error = await refusal(
        backend.changeSprint(sprint.id, { name: 'Again', goal: '', startOn: null, endOn: null }),
      );

      expect(error.code).toBe('sprint_completed');
      expect(error.message).toBe('"Sprint 1" is completed and can no longer be changed');
    });
  });

  describe('deleting a sprint', () => {
    it('deletes a planned sprint and puts its tasks back into the backlog', async () => {
      const sprint = await backend.createSprint({ projectId: project.id });
      const planned = await task('Route the board');
      await backend.planTask(planned.id, sprint.id);

      await backend.deleteSprint(sprint.id);

      expect(await backend.listSprints(project.id)).toEqual([]);
      expect((await now(planned.id)).sprintId).toBeNull();
    });

    it('refuses to delete a sprint that is active', async () => {
      const { sprint } = await activeSprint();

      const error = await refusal(backend.deleteSprint(sprint.id));

      expect(error.code).toBe('sprint_active');
      expect(error.message).toBe(
        '"Sprint 1" has started and cannot be deleted; complete it instead',
      );
    });

    it('refuses to delete a completed sprint', async () => {
      const { sprint } = await activeSprint();
      await backend.completeSprint(sprint.id, { moveUnfinishedTo: null });

      const error = await refusal(backend.deleteSprint(sprint.id));

      expect(error.code).toBe('sprint_completed');
      expect(error.message).toBe('"Sprint 1" is completed and is kept as a record');
    });
  });

  describe('planning a task', () => {
    it('puts a task into a planned sprint, and its subtasks with it', async () => {
      const sprint = await backend.createSprint({ projectId: project.id });
      const parent = await task('Write the sensor drivers');
      const part = await task('Driver for the oxygen sensor', { parentId: parent.id });

      const planned = await backend.planTask(parent.id, sprint.id);

      expect(planned.sprintId).toBe(sprint.id);
      expect((await now(part.id)).sprintId).toBe(sprint.id);
    });

    it('puts a task into the sprint that is active', async () => {
      const { sprint } = await activeSprint();
      const late = await task('Order the parts');

      const planned = await backend.planTask(late.id, sprint.id);

      expect(planned.sprintId).toBe(sprint.id);
    });

    it('takes a task back into the backlog, and its subtasks with it', async () => {
      const sprint = await backend.createSprint({ projectId: project.id });
      const parent = await task('Write the sensor drivers');
      const part = await task('Driver for the oxygen sensor', { parentId: parent.id });
      await backend.planTask(parent.id, sprint.id);

      const back = await backend.planTask(parent.id, null);

      expect(back.sprintId).toBeNull();
      expect((await now(part.id)).sprintId).toBeNull();
    });

    it('refuses to plan into a completed sprint', async () => {
      const { sprint } = await activeSprint();
      await backend.completeSprint(sprint.id, { moveUnfinishedTo: null });
      const late = await task('Order the parts');

      const error = await refusal(backend.planTask(late.id, sprint.id));

      expect(error.code).toBe('sprint_completed');
      expect(error.message).toBe('"Sprint 1" is completed and takes no more tasks');
    });

    it('refuses to plan a subtask by itself', async () => {
      const sprint = await backend.createSprint({ projectId: project.id });
      const parent = await task('Write the sensor drivers');
      const part = await task('Driver for the oxygen sensor', { parentId: parent.id });

      const error = await refusal(backend.planTask(part.id, sprint.id));

      expect(error.code).toBe('subtask_planned_alone');
      expect(error.message).toBe('AQN-2 is a part of AQN-1 and goes where it goes');
    });

    it('refuses a sprint of another project', async () => {
      const other = await backend.createProject({ name: 'Weather station', workflow: 'standard' });
      const sprint = await backend.createSprint({ projectId: other.id });
      const planned = await task('Route the board');

      const error = await refusal(backend.planTask(planned.id, sprint.id));

      expect(error.code).toBe('different_projects');
    });

    it('refuses a sprint that does not exist', async () => {
      const planned = await task('Route the board');

      const error = await refusal(backend.planTask(planned.id, 'nowhere'));

      expect(error.code).toBe('not_found');
    });

    it('opens a new task in the backlog, without points', async () => {
      const created = await task('Route the board');

      expect(created).toMatchObject({ sprintId: null, points: null });
    });

    it('opens a new subtask in the sprint of its task', async () => {
      const sprint = await backend.createSprint({ projectId: project.id });
      const parent = await task('Write the sensor drivers');
      await backend.planTask(parent.id, sprint.id);

      const part = await task('Driver for the oxygen sensor', { parentId: parent.id });

      expect(part.sprintId).toBe(sprint.id);
    });
  });

  describe('starting a sprint', () => {
    it('starts a planned sprint with its name, its goal and its dates', async () => {
      const sprint = await backend.createSprint({ projectId: project.id });
      const planned = await task('Draw the schematic');
      await backend.planTask(planned.id, sprint.id);

      const started = await backend.startSprint(sprint.id, {
        name: ' Schematic ',
        goal: ' A schematic that can be routed ',
        startOn: '2026-11-11',
        endOn: '2026-11-24',
      });

      expect(started).toMatchObject({
        name: 'Schematic',
        goal: 'A schematic that can be routed',
        startOn: '2026-11-11',
        endOn: '2026-11-24',
        state: 'active',
        startedAt: '2026-11-11T09:00:00.000Z',
      });
    });

    it('refuses a sprint without tasks', async () => {
      const sprint = await backend.createSprint({ projectId: project.id });

      const error = await refusal(
        backend.startSprint(sprint.id, {
          name: sprint.name,
          goal: '',
          startOn: '2026-11-11',
          endOn: '2026-11-24',
        }),
      );

      expect(error.code).toBe('sprint_empty');
      expect(error.message).toBe('"Sprint 1" has no tasks; plan some into it before you start it');
    });

    it('refuses while another sprint of the project is active', async () => {
      await activeSprint();
      const next = await backend.createSprint({ projectId: project.id });
      const planned = await task('Route the board');
      await backend.planTask(planned.id, next.id);

      const error = await refusal(
        backend.startSprint(next.id, {
          name: next.name,
          goal: '',
          startOn: '2026-11-24',
          endOn: '2026-12-07',
        }),
      );

      expect(error.code).toBe('another_sprint_active');
      expect(error.message).toBe(
        '"Sprint 1" is still running; complete it before you start another sprint',
      );
    });

    it('refuses a start date after the end date', async () => {
      const sprint = await backend.createSprint({ projectId: project.id });
      const planned = await task('Draw the schematic');
      await backend.planTask(planned.id, sprint.id);

      const error = await refusal(
        backend.startSprint(sprint.id, {
          name: sprint.name,
          goal: '',
          startOn: '2026-11-24',
          endOn: '2026-11-11',
        }),
      );

      expect(error.code).toBe('rule_broken');
      expect((await sprintNow(sprint.id))?.state).toBe('planned');
    });

    it('refuses to start a completed sprint again', async () => {
      const { sprint } = await activeSprint();
      await backend.completeSprint(sprint.id, { moveUnfinishedTo: null });

      const error = await refusal(
        backend.startSprint(sprint.id, {
          name: sprint.name,
          goal: '',
          startOn: '2026-11-24',
          endOn: '2026-12-07',
        }),
      );

      expect(error.code).toBe('sprint_completed');
    });
  });

  describe('completing a sprint', () => {
    it('sends the unfinished tasks with their subtasks to the backlog and keeps the finished ones', async () => {
      const { sprint, task: unfinished } = await activeSprint();
      const part = await task('Power supply', { parentId: unfinished.id });
      const finished = await task('Choose the microcontroller');
      await backend.planTask(finished.id, sprint.id);
      await finish(finished.id);
      // A finished subtask of an unfinished task goes with its task.
      await finish(part.id);

      const completed = await backend.completeSprint(sprint.id, { moveUnfinishedTo: null });

      expect(completed).toMatchObject({
        state: 'completed',
        completedAt: '2026-11-11T09:00:00.000Z',
      });
      expect((await now(unfinished.id)).sprintId).toBeNull();
      expect((await now(part.id)).sprintId).toBeNull();
      expect((await now(finished.id)).sprintId).toBe(sprint.id);
    });

    it('sends the unfinished tasks to a planned sprint of the same project', async () => {
      const { sprint, task: unfinished } = await activeSprint();
      const next = await backend.createSprint({ projectId: project.id });

      await backend.completeSprint(sprint.id, { moveUnfinishedTo: next.id });

      expect((await now(unfinished.id)).sprintId).toBe(next.id);
    });

    it('counts as finished what is in a status of the category done, whatever its name', async () => {
      const { sprint, task: inReview } = await activeSprint();
      const accepted = await task('Choose the microcontroller');
      await backend.planTask(accepted.id, sprint.id);
      // A status of review that is called "Finished", and one of done that is not.
      project = await backend.changeStatus(project.id, statusOf('review'), {
        name: 'Finished',
        category: 'review',
        color: 'amber',
      });
      project = await backend.changeStatus(project.id, statusOf('done'), {
        name: 'Accepted',
        category: 'done',
        color: 'green',
      });
      await backend.moveTask(inReview.id, statusOf('review'), 0);
      await finish(accepted.id);

      const completed = await backend.completeSprint(sprint.id, { moveUnfinishedTo: null });

      expect(completed.summary).toEqual({ unit: 'tasks', committed: 2, completed: 1 });
      expect((await now(inReview.id)).sprintId).toBeNull();
    });

    it('sums up in points when any of its tasks has points', async () => {
      const { sprint, task: unfinished } = await activeSprint();
      await backend.setTaskPoints(unfinished.id, 5);
      const finished = await task('Choose the microcontroller');
      await backend.planTask(finished.id, sprint.id);
      await backend.setTaskPoints(finished.id, 3);
      await finish(finished.id);
      // A task without points counts as no points.
      const unpointed = await task('Order the parts');
      await backend.planTask(unpointed.id, sprint.id);

      const completed = await backend.completeSprint(sprint.id, { moveUnfinishedTo: null });

      expect(completed.summary).toEqual({ unit: 'points', committed: 8, completed: 3 });
    });

    it('sums up in tasks when none of its tasks has points, and leaves subtasks out', async () => {
      const { sprint, task: unfinished } = await activeSprint();
      await task('Power supply', { parentId: unfinished.id });
      const finished = await task('Choose the microcontroller');
      await backend.planTask(finished.id, sprint.id);
      await finish(finished.id);

      const completed = await backend.completeSprint(sprint.id, { moveUnfinishedTo: null });

      expect(completed.summary).toEqual({ unit: 'tasks', committed: 2, completed: 1 });
    });

    it('refuses a sprint that has not started', async () => {
      const sprint = await backend.createSprint({ projectId: project.id });

      const error = await refusal(backend.completeSprint(sprint.id, { moveUnfinishedTo: null }));

      expect(error.code).toBe('sprint_not_active');
      expect(error.message).toBe('"Sprint 1" has not started, so it cannot be completed');
    });

    it('refuses to send the unfinished tasks to a completed sprint', async () => {
      const { sprint: earlier } = await activeSprint();
      await backend.completeSprint(earlier.id, { moveUnfinishedTo: null });
      const { sprint } = await activeSprint();

      const error = await refusal(
        backend.completeSprint(sprint.id, { moveUnfinishedTo: earlier.id }),
      );

      expect(error.code).toBe('sprint_completed');
      expect((await sprintNow(sprint.id))?.state).toBe('active');
    });

    it('refuses to keep the unfinished tasks in the sprint that is completed', async () => {
      const { sprint } = await activeSprint();

      const error = await refusal(
        backend.completeSprint(sprint.id, { moveUnfinishedTo: sprint.id }),
      );

      expect(error.code).toBe('rule_broken');
    });

    it('refuses to send the unfinished tasks to a sprint of another project', async () => {
      const { sprint } = await activeSprint();
      const other = await backend.createProject({ name: 'Weather station', workflow: 'standard' });
      const elsewhere = await backend.createSprint({ projectId: other.id });

      const error = await refusal(
        backend.completeSprint(sprint.id, { moveUnfinishedTo: elsewhere.id }),
      );

      expect(error.code).toBe('different_projects');
    });
  });

  describe('story points', () => {
    it('gives a task points and takes them away again', async () => {
      const pointed = await task('Draw the schematic');

      expect((await backend.setTaskPoints(pointed.id, 8)).points).toBe(8);
      expect((await backend.setTaskPoints(pointed.id, null)).points).toBeNull();
    });

    it('refuses points that are not a whole number from 0 to 999', async () => {
      const pointed = await task('Draw the schematic');

      const errors = await Promise.all(
        [-1, 1000, 2.5].map((points) => refusal(backend.setTaskPoints(pointed.id, points))),
      );

      expect(errors.map((error) => error.code)).toEqual([
        'rule_broken',
        'rule_broken',
        'rule_broken',
      ]);
      expect(errors[0]?.message).toBe('story points are a whole number from 0 to 999');
    });
  });

  describe('the report of a sprint', () => {
    it('tells what remained at the end of each day, and nothing for the days to come', async () => {
      const sprint = await backend.createSprint({ projectId: project.id });
      const titles = ['Draw the schematic', 'Route the board', 'Order the parts'];
      const [first, second, third] = await Promise.all(titles.map((title) => task(title)));
      for (const [planned, points] of [
        [first, 3],
        [second, 5],
        [third, 2],
      ] as const) {
        await backend.planTask(planned?.id ?? '', sprint.id);
        await backend.setTaskPoints(planned?.id ?? '', points);
      }
      await backend.startSprint(sprint.id, {
        name: sprint.name,
        goal: '',
        startOn: '2026-11-10',
        endOn: '2026-11-14',
      });
      // Finished on 11 and on 12 November.
      await finish(first?.id ?? '');
      environment.pass(DAY);
      await finish(second?.id ?? '');

      const report = await backend.sprintReport(sprint.id);

      expect(report).toEqual({
        sprintId: sprint.id,
        unit: 'points',
        total: 10,
        days: [
          { date: '2026-11-10', remaining: 10 },
          { date: '2026-11-11', remaining: 7 },
          { date: '2026-11-12', remaining: 2 },
          { date: '2026-11-13', remaining: null },
          { date: '2026-11-14', remaining: null },
        ],
      });
    });

    it('counts a task that was finished before the sprint began on its first day', async () => {
      const sprint = await backend.createSprint({ projectId: project.id });
      const early = await task('Choose the microcontroller');
      await finish(early.id);
      await backend.planTask(early.id, sprint.id);
      environment.pass(2 * DAY);
      await backend.startSprint(sprint.id, {
        name: sprint.name,
        goal: '',
        startOn: '2026-11-13',
        endOn: '2026-11-14',
      });

      const report = await backend.sprintReport(sprint.id);

      expect(report.unit).toBe('tasks');
      expect(report.days.map((day) => day.remaining)).toEqual([0, null]);
    });

    it('tells nothing of a planned sprint but its total, and has no days without both dates', async () => {
      const sprint = await backend.createSprint({ projectId: project.id, startOn: '2026-11-10' });
      const planned = await task('Draw the schematic');
      await backend.planTask(planned.id, sprint.id);

      const report = await backend.sprintReport(sprint.id);

      expect(report).toEqual({ sprintId: sprint.id, unit: 'tasks', total: 1, days: [] });
    });

    it('stops on the day the sprint was completed', async () => {
      const { sprint, task: planned } = await activeSprint();
      await finish(planned.id);
      await backend.completeSprint(sprint.id, { moveUnfinishedTo: null });
      environment.pass(DAY);

      const report = await backend.sprintReport(sprint.id);

      expect(report.days.slice(0, 3)).toEqual([
        { date: '2026-11-10', remaining: 1 },
        { date: '2026-11-11', remaining: 0 },
        { date: '2026-11-12', remaining: null },
      ]);
    });
  });

  describe('removing what sprints are made of', () => {
    it('removes the sprints of a project with the project', async () => {
      const sprint = await backend.createSprint({ projectId: project.id });

      await backend.deleteProject(project.id);

      const error = await refusal(backend.sprintReport(sprint.id));
      expect(error.code).toBe('not_found');
    });

    it('takes a deleted task out of its sprint', async () => {
      const { sprint, task: planned } = await activeSprint();
      const other = await task('Route the board');
      await backend.planTask(other.id, sprint.id);

      await backend.deleteTask(planned.id);

      expect((await backend.sprintReport(sprint.id)).total).toBe(1);
    });
  });
});
