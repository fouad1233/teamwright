import { describe, expect, it } from 'vitest';

import {
  EMPTY_VALUES,
  LABEL_MAX_LENGTH,
  addField,
  defaultForm,
  introduceForm,
  missingProperties,
  moveField,
  parseForm,
  problemsOf,
  relabelField,
  removeField,
  requestOf,
  requireField,
  retitleForm,
  serializeForm,
  type TaskForm,
} from './formModel';

const properties = (form: TaskForm) => form.fields.map((field) => field.property);

describe('defaultForm', () => {
  it('is named after the view and asks for a title, a description, a priority and a due date', () => {
    const form = defaultForm('Requests');
    expect(form.title).toBe('Requests');
    expect(form.introduction).toBe('');
    expect(properties(form)).toEqual(['title', 'description', 'priority', 'dueOn']);
    expect(form.fields.filter((field) => field.required).map((field) => field.property)).toEqual([
      'title',
    ]);
  });
});

describe('parseForm', () => {
  it('reads missing, empty and damaged content as the form a new view starts with', () => {
    for (const content of [null, '', '   ', '{', '[]', '42', '"text"', '{"fields":"no"}']) {
      expect(parseForm(content, 'Intake')).toEqual(defaultForm('Intake'));
    }
  });

  it('reads back what it wrote', () => {
    let form = defaultForm('Intake');
    form = addField(form, 'points');
    form = relabelField(form, 'title', 'What is needed?');
    form = requireField(form, 'dueOn', true);
    form = introduceForm(form, 'For the bench.');
    expect(parseForm(serializeForm(form), 'Other name')).toEqual(form);
  });

  it('drops fields it does not know and second fields of a property', () => {
    const content = JSON.stringify({
      title: 'Bugs',
      fields: [
        { property: 'title', label: 'Title', required: true },
        { property: 'colour', label: 'Colour' },
        'nonsense',
        { property: 'dueOn', label: 'When', required: true },
        { property: 'dueOn', label: 'Again' },
      ],
    });
    const form = parseForm(content, 'Intake');
    expect(properties(form)).toEqual(['title', 'dueOn']);
    expect(form.fields[1]).toEqual({ property: 'dueOn', label: 'When', required: true });
  });

  it('puts a required title at the top of a form that lost it', () => {
    const content = JSON.stringify({ title: 'Bugs', fields: [{ property: 'points' }] });
    const form = parseForm(content, 'Intake');
    expect(properties(form)).toEqual(['title', 'points']);
    expect(form.fields[0]?.required).toBe(true);
  });

  it('keeps the title required and gives empty labels and titles their usual words', () => {
    const content = JSON.stringify({
      title: '  ',
      introduction: 7,
      fields: [{ property: 'title', label: ' ', required: false }],
    });
    const form = parseForm(content, 'Intake');
    expect(form.title).toBe('Intake');
    expect(form.introduction).toBe('');
    expect(form.fields).toEqual([{ property: 'title', label: 'Title', required: true }]);
  });

  it('cuts a label that is too long', () => {
    const content = JSON.stringify({
      title: 'Bugs',
      fields: [{ property: 'title', label: 'x'.repeat(200) }],
    });
    expect(parseForm(content, 'Intake').fields[0]?.label).toHaveLength(LABEL_MAX_LENGTH);
  });
});

describe('the changes of the form', () => {
  const form = defaultForm('Intake');

  it('offers the properties that are not on the form yet, in their order', () => {
    expect(missingProperties(form)).toEqual(['assignee', 'startOn', 'points', 'estimate']);
  });

  it('adds a field at the foot, once', () => {
    const added = addField(form, 'assignee');
    expect(properties(added)).toEqual(['title', 'description', 'priority', 'dueOn', 'assignee']);
    expect(added.fields.at(-1)).toEqual({
      property: 'assignee',
      label: 'Assignee',
      required: false,
    });
    expect(addField(added, 'assignee')).toBe(added);
  });

  it('removes a field, but never the title', () => {
    expect(properties(removeField(form, 'description'))).toEqual(['title', 'priority', 'dueOn']);
    expect(removeField(form, 'title')).toBe(form);
  });

  it('moves a field to a place, kept within the fields', () => {
    expect(properties(moveField(form, 'dueOn', 0))).toEqual([
      'dueOn',
      'title',
      'description',
      'priority',
    ]);
    expect(properties(moveField(form, 'title', 99))).toEqual([
      'description',
      'priority',
      'dueOn',
      'title',
    ]);
    expect(moveField(form, 'priority', 2)).toBe(form);
    expect(moveField(form, 'points', 0)).toBe(form);
  });

  it('relabels a field, and an empty label brings back the usual name', () => {
    const relabelled = relabelField(form, 'dueOn', '  Needed by  ');
    expect(relabelled.fields[3]?.label).toBe('Needed by');
    expect(relabelField(relabelled, 'dueOn', ' ').fields[3]?.label).toBe('Due date');
  });

  it('requires a field or lets it go, but the title stays required', () => {
    expect(requireField(form, 'priority', true).fields[2]?.required).toBe(true);
    expect(requireField(form, 'title', false).fields[0]?.required).toBe(true);
  });

  it('retitles the form, keeping the title when the new one is empty', () => {
    expect(retitleForm(form, ' Bug reports ').title).toBe('Bug reports');
    expect(retitleForm(form, '  ')).toBe(form);
  });
});

describe('problemsOf', () => {
  const form = addField(addField(defaultForm('Intake'), 'points'), 'estimate');

  it('finds nothing wrong with a form whose required fields are filled', () => {
    expect(problemsOf(form, { ...EMPTY_VALUES, title: 'Fix the clock' }).size).toBe(0);
  });

  it('says at the field what is required and missing', () => {
    const required = requireField(requireField(form, 'dueOn', true), 'priority', true);
    const problems = problemsOf(required, { ...EMPTY_VALUES, title: '   ' });
    expect([...problems.keys()]).toEqual(['title', 'priority', 'dueOn']);
    expect(problems.get('title')).toBe('Write a title. A task cannot be made without one.');
  });

  it('says when the points or the estimate cannot be read', () => {
    const problems = problemsOf(form, {
      ...EMPTY_VALUES,
      title: 'Fix',
      points: 'three',
      estimate: 'soon',
    });
    expect(problems.get('points')).toBe('Write the points as a whole number, like 3.');
    expect(problems.get('estimate')).toBe('Write the estimate like 90m, 2h or 1h 30m.');
  });

  it('says when the due date comes before the start', () => {
    const dated = addField(form, 'startOn');
    const problems = problemsOf(dated, {
      ...EMPTY_VALUES,
      title: 'Fix',
      startOn: '2026-11-20',
      dueOn: '2026-11-18',
    });
    expect([...problems.keys()]).toEqual(['dueOn']);
  });

  it('looks only at the fields on the form', () => {
    expect(
      problemsOf(defaultForm('Intake'), { ...EMPTY_VALUES, title: 'Fix', points: 'x' }).size,
    ).toBe(0);
  });
});

describe('requestOf', () => {
  it('asks for what was filled in on the form, and nothing else', () => {
    let form = defaultForm('Intake');
    form = addField(addField(addField(form, 'assignee'), 'points'), 'estimate');
    const request = requestOf(form, {
      title: '  Fix the clock ',
      description: ' It drifts. ',
      assignee: { kind: 'manager' },
      priority: 'high',
      startOn: '2026-11-17',
      dueOn: '2026-11-20',
      points: '3',
      estimate: '1h 30m',
    });
    expect(request).toEqual({
      task: {
        title: 'Fix the clock',
        description: 'It drifts.',
        assignee: { kind: 'manager' },
        priority: 'high',
        dueOn: '2026-11-20',
        estimateMinutes: 90,
      },
      points: 3,
    });
  });

  it('leaves out what is empty', () => {
    const request = requestOf(defaultForm('Intake'), { ...EMPTY_VALUES, title: 'Fix' });
    expect(request).toEqual({ task: { title: 'Fix' }, points: null });
  });
});
