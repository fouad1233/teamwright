import { beforeEach, describe, expect, it } from 'vitest';

import type { CustomField, Project, Task } from '../types';
import { DemoBackend } from './demoBackend';
import { sampleWorkspace } from './sampleWorkspace';
import { refusal, testEnvironment } from './testing';

describe('the custom fields of the sample workspace', () => {
  it('has one field of every kind and values on some tasks', async () => {
    const workspace = sampleWorkspace(new Date('2026-11-21T09:00:00.000Z'));
    const backend = new DemoBackend(workspace, testEnvironment());
    const [project] = workspace.projects;

    const fields = await backend.listFields(project?.id ?? '');
    const tasks = await backend.listTasks(project?.id ?? '');

    expect(fields.map((field) => field.name)).toEqual([
      'Supplier',
      'Unit cost',
      'Ordered',
      'Arrives',
      'Part number',
      'Datasheet',
    ]);
    expect(tasks.filter((task) => Object.keys(task.fieldValues).length > 0)).toHaveLength(4);
    for (const task of tasks) {
      for (const fieldId of Object.keys(task.fieldValues)) {
        expect(fields.map((field) => field.id)).toContain(fieldId);
      }
    }
  });
});

describe('custom fields in the DemoBackend', () => {
  let backend: DemoBackend;
  let project: Project;
  let task: Task;

  beforeEach(async () => {
    backend = new DemoBackend(undefined, testEnvironment());
    project = await backend.createProject({ name: 'Air quality node', workflow: 'standard' });
    task = await backend.createTask({ projectId: project.id, title: 'Order the sensors' });
  });

  async function stored(): Promise<Task | undefined> {
    return (await backend.listTasks(project.id)).find((candidate) => candidate.id === task.id);
  }

  async function supplier(): Promise<CustomField> {
    const field = await backend.addField(project.id, 'Supplier', 'choice');
    await backend.addFieldChoice(field.id, 'Mouser');
    return backend.addFieldChoice(field.id, 'LCSC', 'teal');
  }

  it('adds fields at the end, moves them, and refuses a name in use', async () => {
    await backend.addField(project.id, 'Supplier', 'choice');
    const cost = await backend.addField(project.id, 'Unit cost', 'number', '€');
    await backend.moveField(cost.id, 0);

    const fields = await backend.listFields(project.id);
    const taken = await refusal(backend.addField(project.id, ' supplier ', 'text'));
    const unit = await refusal(backend.addField(project.id, 'Datasheet', 'link', 'mm'));

    expect(fields.map((field) => field.name)).toEqual(['Unit cost', 'Supplier']);
    expect(fields[0]?.unit).toBe('€');
    expect(taken.code).toBe('field_name_taken');
    expect(unit.code).toBe('rule_broken');
  });

  it('gives a choice field choices in turn of color, never two with one name', async () => {
    const field = await supplier();

    expect(field.choices.map((choice) => [choice.name, choice.color])).toEqual([
      ['Mouser', 'blue'],
      ['LCSC', 'teal'],
    ]);
    expect((await refusal(backend.addFieldChoice(field.id, 'mouser'))).code).toBe(
      'choice_name_taken',
    );
    const ordered = await backend.addField(project.id, 'Ordered', 'yes_no');
    expect((await refusal(backend.addFieldChoice(ordered.id, 'Yes'))).code).toBe('rule_broken');
  });

  it('checks a value against its field and says it in the history', async () => {
    const field = await supplier();
    const cost = await backend.addField(project.id, 'Unit cost', 'number', '€');
    const link = await backend.addField(project.id, 'Datasheet', 'link');

    await backend.setFieldValue(task.id, cost.id, { kind: 'number', number: 2.5 });
    await backend.setFieldValue(task.id, cost.id, { kind: 'number', number: 3 });
    await backend.clearFieldValue(task.id, cost.id);

    const wrongKind = await refusal(
      backend.setFieldValue(task.id, cost.id, { kind: 'yes_no', yes: true }),
    );
    const noChoice = await refusal(
      backend.setFieldValue(task.id, field.id, { kind: 'choice', choiceId: 'nothing' }),
    );
    const badLink = await refusal(
      backend.setFieldValue(task.id, link.id, { kind: 'link', address: 'javascript:alert(1)' }),
    );
    expect([wrongKind.code, noChoice.code, badLink.code]).toEqual([
      'rule_broken',
      'not_found',
      'rule_broken',
    ]);
    const kinds = (await backend.listTaskHistory(task.id))
      .filter((change) => change.kind === 'field_value_changed')
      .map((change) =>
        change.kind === 'field_value_changed' ? [change.from?.kind, change.to?.kind] : [],
      );
    expect(kinds).toEqual([
      [undefined, 'number'],
      ['number', 'number'],
      ['number', undefined],
    ]);
  });

  it('clears the values of a removed choice and the filters by it', async () => {
    const field = await supplier();
    const [mouser, lcsc] = field.choices;
    const other = await backend.createTask({ projectId: project.id, title: 'Order the cables' });
    await backend.setFieldValue(task.id, field.id, { kind: 'choice', choiceId: mouser?.id ?? '' });
    await backend.setFieldValue(other.id, field.id, { kind: 'choice', choiceId: lcsc?.id ?? '' });
    const [board] = await backend.listViews(project.id);
    const everyTask = { assignee: { kind: 'anyone' }, hidesFinished: false } as const;
    await backend.setViewFilter(board?.id ?? '', {
      ...everyTask,
      fieldFilter: { kind: 'choice', fieldId: field.id, choiceId: mouser?.id ?? '' },
    });

    await backend.removeFieldChoice(field.id, mouser?.id ?? '');

    expect((await stored())?.fieldValues).toEqual({});
    const kept = (await backend.listTasks(project.id)).find((each) => each.id === other.id);
    expect(kept?.fieldValues[field.id]).toEqual({ kind: 'choice', choiceId: lcsc?.id });
    expect((await backend.listViews(project.id))[0]?.filter).toEqual(everyTask);
  });

  it('clears the values of a removed field, and filters a view only by a choice or a box', async () => {
    const ordered = await backend.addField(project.id, 'Ordered', 'yes_no');
    const notes = await backend.addField(project.id, 'Notes', 'text');
    await backend.setFieldValue(task.id, ordered.id, { kind: 'yes_no', yes: true });
    const [board] = await backend.listViews(project.id);
    const everyTask = { assignee: { kind: 'anyone' }, hidesFinished: false } as const;

    const notFilterable = await refusal(
      backend.setViewFilter(board?.id ?? '', {
        ...everyTask,
        fieldFilter: { kind: 'yes_no', fieldId: notes.id, yes: true },
      }),
    );
    await backend.setViewFilter(board?.id ?? '', {
      ...everyTask,
      fieldFilter: { kind: 'yes_no', fieldId: ordered.id, yes: false },
    });
    await backend.removeField(ordered.id);

    expect(notFilterable.code).toBe('rule_broken');
    expect((await stored())?.fieldValues).toEqual({});
    expect((await backend.listViews(project.id))[0]?.filter).toEqual(everyTask);
    expect((await backend.listFields(project.id)).map((field) => field.name)).toEqual(['Notes']);
  });
});
