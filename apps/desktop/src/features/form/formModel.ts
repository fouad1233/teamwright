/**
 * What the form view works out without drawing anything: the form it keeps
 * (its title, its introduction and its fields in order), how that is read
 * from and written to the view's content, the changes the manager makes to
 * it, and what a filled-in form asks of the task it becomes.
 */

import type { Assignee, CalendarDate, NewTask, Priority } from '@/shared/api/types';
import { parseEstimate } from '@/shared/lib/format';

import { parsePoints } from '../sprints/sprintModel';

/** A property of a task that a field of the form fills. */
export type FieldProperty =
  'title' | 'description' | 'assignee' | 'priority' | 'startOn' | 'dueOn' | 'points' | 'estimate';

/** One field of the form: the property it fills, what it is called, and whether it must be filled. */
export interface FormField {
  property: FieldProperty;
  label: string;
  required: boolean;
}

/** The form a view keeps. */
export interface TaskForm {
  title: string;
  /** A few words under the title for whoever fills the form in. May be empty. */
  introduction: string;
  /** The fields in order. The title of the task is always one of them, and required. */
  fields: FormField[];
}

/** Every property a field can fill, in the order "Add a field" offers them, with its usual name. */
export const PROPERTIES: readonly { property: FieldProperty; label: string }[] = [
  { property: 'title', label: 'Title' },
  { property: 'description', label: 'Description' },
  { property: 'assignee', label: 'Assignee' },
  { property: 'priority', label: 'Priority' },
  { property: 'startOn', label: 'Start date' },
  { property: 'dueOn', label: 'Due date' },
  { property: 'points', label: 'Points' },
  { property: 'estimate', label: 'Estimate' },
];

/** The longest a label may be, in characters. */
export const LABEL_MAX_LENGTH = 60;
/** The longest the title of the form may be, in characters. */
export const TITLE_MAX_LENGTH = 80;
/** The longest the introduction may be, in characters. */
export const INTRODUCTION_MAX_LENGTH = 400;

/** The usual name of a property, which a field is called until it is relabelled. */
export function usualLabel(property: FieldProperty): string {
  return PROPERTIES.find((known) => known.property === property)?.label ?? property;
}

function isProperty(value: unknown): value is FieldProperty {
  return PROPERTIES.some((known) => known.property === value);
}

/**
 * The form a new view starts with: named after the view, asking for a
 * title, a description, a priority and a due date.
 */
export function defaultForm(viewName: string): TaskForm {
  return {
    title: viewName.slice(0, TITLE_MAX_LENGTH),
    introduction: '',
    fields: (['title', 'description', 'priority', 'dueOn'] as const).map((property) => ({
      property,
      label: usualLabel(property),
      required: property === 'title',
    })),
  };
}

/** A label as it is kept: trimmed and not too long, or the usual name when it is empty. */
function tidyLabel(property: FieldProperty, label: string): string {
  const trimmed = label.trim().slice(0, LABEL_MAX_LENGTH);
  return trimmed === '' ? usualLabel(property) : trimmed;
}

/** Reads one field as it was written, or nothing if it is not a field. */
function readField(raw: unknown): FormField | undefined {
  if (typeof raw !== 'object' || raw === null) {
    return undefined;
  }
  const { property, label, required } = raw as Record<string, unknown>;
  if (!isProperty(property)) {
    return undefined;
  }
  return {
    property,
    label: tidyLabel(property, typeof label === 'string' ? label : ''),
    required: property === 'title' || required === true,
  };
}

/**
 * Reads the form a view keeps. Content that is missing, empty, damaged or
 * written by another version reads as the form a new view starts with;
 * what can be understood of it is kept. A field of a property it does not
 * know is dropped, a second field of the same property is dropped, and a
 * form without a title field gets one at the top.
 */
export function parseForm(content: string | null, viewName: string): TaskForm {
  const fallback = defaultForm(viewName);
  if (content === null || content.trim() === '') {
    return fallback;
  }
  let raw: unknown;
  try {
    raw = JSON.parse(content);
  } catch {
    return fallback;
  }
  if (typeof raw !== 'object' || raw === null || Array.isArray(raw)) {
    return fallback;
  }
  const { title, introduction, fields } = raw as Record<string, unknown>;
  if (!Array.isArray(fields)) {
    return fallback;
  }
  const kept: FormField[] = [];
  for (const entry of fields) {
    const field = readField(entry);
    if (field && !kept.some((other) => other.property === field.property)) {
      kept.push(field);
    }
  }
  if (!kept.some((field) => field.property === 'title')) {
    kept.unshift({ property: 'title', label: usualLabel('title'), required: true });
  }
  return {
    title:
      typeof title === 'string' && title.trim() !== ''
        ? title.trim().slice(0, TITLE_MAX_LENGTH)
        : fallback.title,
    introduction:
      typeof introduction === 'string' ? introduction.slice(0, INTRODUCTION_MAX_LENGTH) : '',
    fields: kept,
  };
}

/** Writes the form as the view's content. */
export function serializeForm(form: TaskForm): string {
  return JSON.stringify({
    title: form.title,
    introduction: form.introduction,
    fields: form.fields.map(({ property, label, required }) => ({ property, label, required })),
  });
}

/** The properties that have no field on the form yet, in the order they are offered. */
export function missingProperties(form: TaskForm): FieldProperty[] {
  return PROPERTIES.map((known) => known.property).filter(
    (property) => !form.fields.some((field) => field.property === property),
  );
}

/** The form with a field for a property added at its foot. A property already on it changes nothing. */
export function addField(form: TaskForm, property: FieldProperty): TaskForm {
  if (form.fields.some((field) => field.property === property)) {
    return form;
  }
  return {
    ...form,
    fields: [
      ...form.fields,
      { property, label: usualLabel(property), required: property === 'title' },
    ],
  };
}

/** The form without the field of a property. The title stays: a task cannot be made without one. */
export function removeField(form: TaskForm, property: FieldProperty): TaskForm {
  if (property === 'title') {
    return form;
  }
  return { ...form, fields: form.fields.filter((field) => field.property !== property) };
}

/** The form with the field of a property moved to a place among the fields, kept within them. */
export function moveField(form: TaskForm, property: FieldProperty, place: number): TaskForm {
  const from = form.fields.findIndex((field) => field.property === property);
  if (from === -1) {
    return form;
  }
  const to = Math.max(0, Math.min(form.fields.length - 1, Math.round(place)));
  if (to === from) {
    return form;
  }
  const fields = [...form.fields];
  const [moved] = fields.splice(from, 1);
  if (moved === undefined) {
    return form;
  }
  fields.splice(to, 0, moved);
  return { ...form, fields };
}

/** The form with a field called something else. An empty label brings back the usual name. */
export function relabelField(form: TaskForm, property: FieldProperty, label: string): TaskForm {
  return {
    ...form,
    fields: form.fields.map((field) =>
      field.property === property ? { ...field, label: tidyLabel(property, label) } : field,
    ),
  };
}

/** The form with a field required or not. The title is always required. */
export function requireField(form: TaskForm, property: FieldProperty, required: boolean): TaskForm {
  return {
    ...form,
    fields: form.fields.map((field) =>
      field.property === property
        ? { ...field, required: property === 'title' || required }
        : field,
    ),
  };
}

/** The form with another title. An empty title keeps the one it had. */
export function retitleForm(form: TaskForm, title: string): TaskForm {
  const trimmed = title.trim().slice(0, TITLE_MAX_LENGTH);
  return trimmed === '' ? form : { ...form, title: trimmed };
}

/** The form with other words of introduction. */
export function introduceForm(form: TaskForm, introduction: string): TaskForm {
  return { ...form, introduction: introduction.trim().slice(0, INTRODUCTION_MAX_LENGTH) };
}

/**
 * What has been filled in. The points and the estimate are kept as they
 * were typed and read when the form is sent; a choice not made is nothing.
 */
export interface FormValues {
  title: string;
  description: string;
  assignee: Assignee | null;
  priority: Priority | null;
  startOn: CalendarDate | null;
  dueOn: CalendarDate | null;
  points: string;
  estimate: string;
}

/** A form with nothing filled in. */
export const EMPTY_VALUES: FormValues = {
  title: '',
  description: '',
  assignee: null,
  priority: null,
  startOn: null,
  dueOn: null,
  points: '',
  estimate: '',
};

/** What is said at a required field that was left empty. */
const MISSING: Record<FieldProperty, string> = {
  title: 'Write a title. A task cannot be made without one.',
  description: 'Write a few words about the task.',
  assignee: 'Choose who will do it.',
  priority: 'Choose a priority.',
  startOn: 'Choose a start date.',
  dueOn: 'Choose a due date.',
  points: 'Write the points.',
  estimate: 'Write the estimate.',
};

/** Whether a property has been filled in. */
function isFilled(values: FormValues, property: FieldProperty): boolean {
  switch (property) {
    case 'title':
    case 'description':
    case 'points':
    case 'estimate':
      return values[property].trim() !== '';
    case 'assignee':
    case 'priority':
    case 'startOn':
    case 'dueOn':
      return values[property] !== null;
  }
}

/**
 * What is wrong with what was filled in, field by field, in a plain
 * sentence each: a required field left empty, points or an estimate that
 * cannot be read, a start after the due date. Only the fields on the form
 * are looked at. Nothing wrong is an empty map.
 */
export function problemsOf(form: TaskForm, values: FormValues): Map<FieldProperty, string> {
  const problems = new Map<FieldProperty, string>();
  const has = (property: FieldProperty) => form.fields.some((field) => field.property === property);
  for (const field of form.fields) {
    if (field.required && !isFilled(values, field.property)) {
      problems.set(field.property, MISSING[field.property]);
    }
  }
  if (has('points') && !problems.has('points') && parsePoints(values.points) === undefined) {
    problems.set('points', 'Write the points as a whole number, like 3.');
  }
  if (
    has('estimate') &&
    !problems.has('estimate') &&
    parseEstimate(values.estimate) === undefined
  ) {
    problems.set('estimate', 'Write the estimate like 90m, 2h or 1h 30m.');
  }
  if (
    has('startOn') &&
    has('dueOn') &&
    values.startOn !== null &&
    values.dueOn !== null &&
    values.startOn > values.dueOn
  ) {
    problems.set('dueOn', 'The due date comes before the start. Choose a later one.');
  }
  return problems;
}

/** What a filled-in form asks for: the task to make, and the points to give it once it exists. */
export interface TaskRequest {
  task: Omit<NewTask, 'projectId'>;
  /** Making a task does not take points; they are set once the task exists. */
  points: number | null;
}

/**
 * The task a filled-in form becomes. Only the fields on the form count;
 * what is empty is left out, so that the task gets what a new task gets.
 * Call it once `problemsOf` has found nothing.
 */
export function requestOf(form: TaskForm, values: FormValues): TaskRequest {
  const has = (property: FieldProperty) => form.fields.some((field) => field.property === property);
  const task: Omit<NewTask, 'projectId'> = { title: values.title.trim() };
  const description = values.description.trim();
  if (has('description') && description !== '') {
    task.description = description;
  }
  if (has('assignee') && values.assignee !== null) {
    task.assignee = values.assignee;
  }
  if (has('priority') && values.priority !== null) {
    task.priority = values.priority;
  }
  if (has('startOn') && values.startOn !== null) {
    task.startOn = values.startOn;
  }
  if (has('dueOn') && values.dueOn !== null) {
    task.dueOn = values.dueOn;
  }
  const estimate = has('estimate') ? parseEstimate(values.estimate) : null;
  if (estimate !== null && estimate !== undefined) {
    task.estimateMinutes = estimate;
  }
  const points = has('points') ? (parsePoints(values.points) ?? null) : null;
  return { task, points };
}
