import { beforeEach, describe, expect, it } from 'vitest';

import type { Project, ViewFilter } from '../types';
import { DemoBackend } from './demoBackend';
import { refusal, testEnvironment } from './testing';

/**
 * A task made inside a view gets what the view's filter asks of it, so that
 * it stays in the view: the tests of tags and fields in the application crate say the
 * same, with the same codes.
 */
describe('a task made inside a filtered view, in the DemoBackend', () => {
  let backend: DemoBackend;
  let project: Project;
  let boardId: string;

  beforeEach(async () => {
    backend = new DemoBackend(undefined, testEnvironment());
    project = await backend.createProject({ name: 'Air quality node', workflow: 'standard' });
    const [board] = await backend.listViews(project.id);
    boardId = board?.id ?? '';
  });

  const everyTask: ViewFilter = { assignee: { kind: 'anyone' }, hidesFinished: false };

  it('carries the tag the view is filtered by', async () => {
    const tag = await backend.addTag(project.id, 'EMC');
    await backend.setViewFilter(boardId, { ...everyTask, tagId: tag.id });

    const made = await backend.createTask({
      projectId: project.id,
      title: 'Shield the radio',
      viewId: boardId,
    });
    const subtask = await backend.createTask({
      projectId: project.id,
      title: 'Solder the shield',
      parentId: made.id,
      viewId: boardId,
    });
    const outside = await backend.createTask({ projectId: project.id, title: 'Plain' });

    expect(made.tagIds).toEqual([tag.id]);
    expect(subtask.tagIds).toEqual([]);
    expect(outside.tagIds).toEqual([]);
  });

  it('holds the choice, or the yes, the view is filtered by, and nothing for no', async () => {
    const supplier = await backend.addField(project.id, 'Supplier', 'choice');
    const withChoice = await backend.addFieldChoice(supplier.id, 'Mouser');
    const choiceId = withChoice.choices[0]?.id ?? '';
    const ordered = await backend.addField(project.id, 'Ordered', 'yes_no');
    const make = async (filter: ViewFilter) => {
      await backend.setViewFilter(boardId, filter);
      return backend.createTask({ projectId: project.id, title: 'Order', viewId: boardId });
    };

    const byChoice = await make({
      ...everyTask,
      fieldFilter: { kind: 'choice', fieldId: supplier.id, choiceId },
    });
    const byYes = await make({
      ...everyTask,
      fieldFilter: { kind: 'yes_no', fieldId: ordered.id, yes: true },
    });
    const byNo = await make({
      ...everyTask,
      fieldFilter: { kind: 'yes_no', fieldId: ordered.id, yes: false },
    });

    expect(byChoice.fieldValues).toEqual({ [supplier.id]: { kind: 'choice', choiceId } });
    expect(byYes.fieldValues).toEqual({ [ordered.id]: { kind: 'yes_no', yes: true } });
    expect(byNo.fieldValues).toEqual({});
  });

  it('refuses a view of another project, and one that does not exist', async () => {
    const other = await backend.createProject({ name: 'Weather station', workflow: 'standard' });

    const foreign = await refusal(
      backend.createTask({ projectId: other.id, title: 'Shield', viewId: boardId }),
    );
    const missing = await refusal(
      backend.createTask({ projectId: project.id, title: 'Shield', viewId: 'nothing' }),
    );

    expect(foreign.code).toBe('different_projects');
    expect(missing.code).toBe('not_found');
  });
});
