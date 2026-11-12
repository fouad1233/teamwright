import { beforeEach, describe, expect, it } from 'vitest';

import type { Id, Member, Project, View, ViewKind } from '../types';
import { DemoBackend } from './demoBackend';
import { sampleWorkspace } from './sampleWorkspace';
import { refusal, testEnvironment } from './testing';

/** The kinds and the names of the views a new project starts with, in their order. */
const STARTING_VIEWS: { kind: ViewKind; name: string }[] = [
  { kind: 'board', name: 'Board' },
  { kind: 'backlog', name: 'Backlog' },
  { kind: 'list', name: 'List' },
  { kind: 'gantt', name: 'Gantt' },
  { kind: 'calendar', name: 'Calendar' },
  { kind: 'dashboard', name: 'Dashboard' },
];

const EVERY_TASK = { assignee: { kind: 'anyone' }, hidesFinished: false } as const;

describe('the views of the sample workspace', () => {
  it('gives the sample project the six views a new project starts with', async () => {
    const workspace = sampleWorkspace(new Date('2026-11-11T09:00:00.000Z'));
    const backend = new DemoBackend(workspace, testEnvironment());
    const [project] = workspace.projects;

    const views = await backend.listViews(project?.id ?? '');

    expect(views.map(({ kind, name }) => ({ kind, name }))).toEqual(STARTING_VIEWS);
    expect(views.every((view) => view.projectId === project?.id)).toBe(true);
  });
});

describe('views in the DemoBackend', () => {
  let backend: DemoBackend;
  let project: Project;

  beforeEach(async () => {
    backend = new DemoBackend(undefined, testEnvironment());
    project = await backend.createProject({ name: 'Air quality node', workflow: 'standard' });
  });

  async function views(): Promise<View[]> {
    return backend.listViews(project.id);
  }

  async function names(): Promise<string[]> {
    return (await views()).map((view) => view.name);
  }

  /** The view of the project that has a name. */
  async function viewNamed(name: string): Promise<View> {
    const found = (await views()).find((view) => view.name === name);
    if (!found) {
      throw new Error(`the project has no view called ${name}`);
    }
    return found;
  }

  async function member(name: string, projectId: Id = project.id): Promise<Member> {
    return backend.addMember({ projectId, name, kind: 'agent' });
  }

  describe('the views of a project', () => {
    it('starts a new project with one view of each kind, named after it, showing every task', async () => {
      const listed = await views();

      expect(listed.map(({ kind, name }) => ({ kind, name }))).toEqual(STARTING_VIEWS);
      for (const view of listed) {
        expect(view.projectId).toBe(project.id);
        expect(view.filter).toEqual(EVERY_TASK);
      }
      expect(new Set(listed.map((view) => view.id)).size).toBe(6);
    });

    it('gives the six views to a project that was kept before it had views', async () => {
      const kept = new DemoBackend(
        { projects: [project], tasks: [], members: [] },
        testEnvironment(),
      );

      const listed = await kept.listViews(project.id);

      expect(listed.map(({ kind, name }) => ({ kind, name }))).toEqual(STARTING_VIEWS);
    });

    it('refuses to list the views of a project that does not exist', async () => {
      const error = await refusal(backend.listViews('nowhere'));

      expect(error.code).toBe('not_found');
      expect(error.message).toBe('there is no project with the identifier nowhere');
    });

    it('removes the views with their project', async () => {
      const board = await viewNamed('Board');

      await backend.deleteProject(project.id);

      expect((await refusal(backend.listViews(project.id))).code).toBe('not_found');
      const error = await refusal(backend.renameView(board.id, 'Plan'));
      expect(error.code).toBe('not_found');
      expect(error.message).toBe(`there is no view with the identifier ${board.id}`);
    });

    it('keeps the views of a project apart from those of another', async () => {
      const other = await backend.createProject({ name: 'Weather station', workflow: 'standard' });

      await backend.addView({ projectId: other.id, kind: 'board' });

      expect(await names()).toEqual(STARTING_VIEWS.map((view) => view.name));
      expect((await backend.listViews(other.id)).map((view) => view.name)).toContain('Board 2');
    });
  });

  describe('adding a view', () => {
    it('adds it at the end, named after its kind and numbered when the name is taken', async () => {
      const added = await backend.addView({ projectId: project.id, kind: 'board' });

      expect(added).toMatchObject({
        projectId: project.id,
        kind: 'board',
        name: 'Board 2',
        filter: EVERY_TASK,
      });
      expect((await views()).at(-1)).toEqual(added);
    });

    it('takes the name of the kind as it is when no view has it', async () => {
      await backend.renameView((await viewNamed('List')).id, 'Everything');

      const added = await backend.addView({ projectId: project.id, kind: 'list' });

      expect(added.name).toBe('List');
    });

    it('numbers the name with the first number that is free, whatever the capitals', async () => {
      await backend.addView({ projectId: project.id, kind: 'gantt', name: 'gantt 2' });
      await backend.addView({ projectId: project.id, kind: 'gantt', name: 'Gantt 4' });

      const added = await backend.addView({ projectId: project.id, kind: 'gantt' });

      expect(added.name).toBe('Gantt 3');
    });

    it('names it after its kind when the name is empty', async () => {
      const added = await backend.addView({ projectId: project.id, kind: 'calendar', name: '  ' });

      expect(added.name).toBe('Calendar 2');
    });

    it('trims the name it is given and puts it on one line', async () => {
      const added = await backend.addView({
        projectId: project.id,
        kind: 'board',
        name: '  Firmware \n  only ',
      });

      expect(added.name).toBe('Firmware only');
    });

    it('accepts a name of 40 characters', async () => {
      const added = await backend.addView({
        projectId: project.id,
        kind: 'list',
        name: 'x'.repeat(40),
      });

      expect(added.name).toHaveLength(40);
    });

    it('refuses a name of more than 40 characters', async () => {
      const error = await refusal(
        backend.addView({ projectId: project.id, kind: 'list', name: 'x'.repeat(41) }),
      );

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe(
        'the view name can have at most 40 characters, but this one has 41',
      );
    });

    it('refuses a name another view of the project has, whatever the capitals', async () => {
      const error = await refusal(
        backend.addView({ projectId: project.id, kind: 'list', name: ' BOARD ' }),
      );

      expect(error.code).toBe('view_name_taken');
      expect(error.message).toBe('the project already has a view called "BOARD"');
    });

    it('refuses a second backlog', async () => {
      const error = await refusal(backend.addView({ projectId: project.id, kind: 'backlog' }));

      expect(error.code).toBe('view_kind_taken');
      expect(error.message).toBe(
        'the project already has a backlog, "Backlog", and can have only one',
      );
    });

    it('refuses a second dashboard, whatever the first one is called', async () => {
      await backend.renameView((await viewNamed('Dashboard')).id, 'Health');

      const error = await refusal(
        backend.addView({ projectId: project.id, kind: 'dashboard', name: 'Another' }),
      );

      expect(error.code).toBe('view_kind_taken');
      expect(error.message).toBe(
        'the project already has a dashboard, "Health", and can have only one',
      );
    });

    it('adds a backlog again once it has been removed', async () => {
      await backend.removeView((await viewNamed('Backlog')).id);

      const added = await backend.addView({ projectId: project.id, kind: 'backlog' });

      expect(added.name).toBe('Backlog');
      expect((await views()).at(-1)?.kind).toBe('backlog');
    });

    it('refuses a view for a project that does not exist', async () => {
      const error = await refusal(backend.addView({ projectId: 'nowhere', kind: 'board' }));

      expect(error.code).toBe('not_found');
      expect(error.message).toBe('there is no project with the identifier nowhere');
    });
  });

  describe('renaming a view', () => {
    it('trims the new name and keeps the place and the filter of the view', async () => {
      const list = await viewNamed('List');

      const renamed = await backend.renameView(list.id, '  Everything  ');

      expect(renamed).toEqual({ ...list, name: 'Everything' });
      expect((await names())[2]).toBe('Everything');
    });

    it('lets a view change the capitals of its own name', async () => {
      const renamed = await backend.renameView((await viewNamed('Board')).id, 'BOARD');

      expect(renamed.name).toBe('BOARD');
    });

    it('refuses an empty name', async () => {
      const error = await refusal(backend.renameView((await viewNamed('Board')).id, '   '));

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe('the view name must not be empty');
    });

    it('refuses a name of more than 40 characters', async () => {
      const error = await refusal(
        backend.renameView((await viewNamed('Board')).id, 'y'.repeat(45)),
      );

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe(
        'the view name can have at most 40 characters, but this one has 45',
      );
    });

    it('refuses the name of another view of the project, whatever the capitals', async () => {
      const error = await refusal(backend.renameView((await viewNamed('Board')).id, 'list'));

      expect(error.code).toBe('view_name_taken');
      expect(error.message).toBe('the project already has a view called "list"');
    });

    it('refuses a view that does not exist', async () => {
      const error = await refusal(backend.renameView('nowhere', 'Plan'));

      expect(error.code).toBe('not_found');
      expect(error.message).toBe('there is no view with the identifier nowhere');
    });
  });

  describe('filtering a view', () => {
    it('shows the tasks of one member of the team, and can hide the finished ones', async () => {
      const engineer = await member('Firmware engineer');
      const board = await viewNamed('Board');

      const filtered = await backend.setViewFilter(board.id, {
        assignee: { kind: 'member', memberId: engineer.id },
        hidesFinished: true,
      });

      expect(filtered).toEqual({
        ...board,
        filter: { assignee: { kind: 'member', memberId: engineer.id }, hidesFinished: true },
      });
      expect(await viewNamed('Board')).toEqual(filtered);
    });

    it('shows the tasks of the manager, of nobody, or of anyone again', async () => {
      const gantt = await viewNamed('Gantt');
      const calendar = await viewNamed('Calendar');
      const list = await viewNamed('List');

      await backend.setViewFilter(gantt.id, {
        assignee: { kind: 'manager' },
        hidesFinished: false,
      });
      await backend.setViewFilter(calendar.id, {
        assignee: { kind: 'nobody' },
        hidesFinished: true,
      });
      await backend.setViewFilter(list.id, { assignee: { kind: 'nobody' }, hidesFinished: false });
      const cleared = await backend.setViewFilter(list.id, EVERY_TASK);

      expect((await viewNamed('Gantt')).filter.assignee).toEqual({ kind: 'manager' });
      expect((await viewNamed('Calendar')).filter).toEqual({
        assignee: { kind: 'nobody' },
        hidesFinished: true,
      });
      expect(cleared.filter).toEqual(EVERY_TASK);
    });

    it('refuses to filter a backlog', async () => {
      const error = await refusal(
        backend.setViewFilter((await viewNamed('Backlog')).id, {
          assignee: { kind: 'manager' },
          hidesFinished: false,
        }),
      );

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe(
        '"Backlog" is a backlog, which shows every task and cannot be filtered',
      );
    });

    it('refuses to filter a dashboard', async () => {
      const error = await refusal(
        backend.setViewFilter((await viewNamed('Dashboard')).id, {
          assignee: { kind: 'anyone' },
          hidesFinished: true,
        }),
      );

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe(
        '"Dashboard" is a dashboard, which shows every task and cannot be filtered',
      );
    });

    it('refuses a member of the team of another project', async () => {
      const other = await backend.createProject({ name: 'Weather station', workflow: 'standard' });
      const stranger = await member('Firmware engineer', other.id);

      const error = await refusal(
        backend.setViewFilter((await viewNamed('Board')).id, {
          assignee: { kind: 'member', memberId: stranger.id },
          hidesFinished: false,
        }),
      );

      expect(error.code).toBe('member_not_on_team');
      expect(error.message).toBe('the filter names a member who is not on the team of the project');
    });

    it('refuses a member who does not exist', async () => {
      const error = await refusal(
        backend.setViewFilter((await viewNamed('Board')).id, {
          assignee: { kind: 'member', memberId: 'nobody-known' },
          hidesFinished: false,
        }),
      );

      expect(error.code).toBe('member_not_on_team');
      expect(error.message).toBe('the filter names a member who is not on the team of the project');
    });

    it('refuses a view that does not exist', async () => {
      const error = await refusal(backend.setViewFilter('nowhere', EVERY_TASK));

      expect(error.code).toBe('not_found');
      expect(error.message).toBe('there is no view with the identifier nowhere');
    });

    it('shows everybody again in a view that showed only a member who left the team', async () => {
      const engineer = await member('Firmware engineer');
      const designer = await member('Industrial designer');
      const board = await viewNamed('Board');
      const list = await viewNamed('List');
      await backend.setViewFilter(board.id, {
        assignee: { kind: 'member', memberId: engineer.id },
        hidesFinished: true,
      });
      await backend.setViewFilter(list.id, {
        assignee: { kind: 'member', memberId: designer.id },
        hidesFinished: false,
      });

      await backend.removeMember(engineer.id);

      expect((await viewNamed('Board')).filter).toEqual({
        assignee: { kind: 'anyone' },
        hidesFinished: true,
      });
      expect((await viewNamed('List')).filter.assignee).toEqual({
        kind: 'member',
        memberId: designer.id,
      });
    });
  });

  describe('moving a view', () => {
    it('puts it at the place it is given and answers with the views in their new order', async () => {
      const calendar = await viewNamed('Calendar');

      const moved = await backend.moveView(calendar.id, 0);

      const order = ['Calendar', 'Board', 'Backlog', 'List', 'Gantt', 'Dashboard'];
      expect(moved.map((view) => view.name)).toEqual(order);
      expect(await names()).toEqual(order);
    });

    it('moves a view one place to the right', async () => {
      await backend.moveView((await viewNamed('Board')).id, 1);

      expect(await names()).toEqual(['Backlog', 'Board', 'List', 'Gantt', 'Calendar', 'Dashboard']);
    });

    it('puts it last for a place past the end, and first for a place before the start', async () => {
      await backend.moveView((await viewNamed('Board')).id, 99);
      await backend.moveView((await viewNamed('Dashboard')).id, -3);

      expect(await names()).toEqual(['Dashboard', 'Backlog', 'List', 'Gantt', 'Calendar', 'Board']);
    });

    it('refuses a view that does not exist', async () => {
      const error = await refusal(backend.moveView('nowhere', 0));

      expect(error.code).toBe('not_found');
      expect(error.message).toBe('there is no view with the identifier nowhere');
    });
  });

  describe('removing a view', () => {
    it('removes it and keeps the others in their order', async () => {
      await backend.removeView((await viewNamed('Gantt')).id);

      expect(await names()).toEqual(['Board', 'Backlog', 'List', 'Calendar', 'Dashboard']);
    });

    it('refuses to remove the last view of a project', async () => {
      for (const name of ['Backlog', 'List', 'Gantt', 'Calendar', 'Dashboard']) {
        await backend.removeView((await viewNamed(name)).id);
      }

      const error = await refusal(backend.removeView((await viewNamed('Board')).id));

      expect(error.code).toBe('last_view');
      expect(error.message).toBe('"Board" is the last view of the project and cannot be removed');
      expect(await names()).toEqual(['Board']);
    });

    it('refuses a view that does not exist', async () => {
      const error = await refusal(backend.removeView('nowhere'));

      expect(error.code).toBe('not_found');
      expect(error.message).toBe('there is no view with the identifier nowhere');
    });
  });
});
