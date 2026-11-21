import type { CustomField, FieldChoice, FieldKind, FieldValue, Id, Task } from '@/shared/api/types';
import { formatDate } from '@/shared/lib/format';

/**
 * What the custom fields work out without drawing anything: the words for
 * a kind and a value, how a typed number is read, and what a value sorts
 * by. Pure functions, so that they can be tested alone. The rules about a
 * value (a number is a number, a link starts with https://) live in the
 * domain; these only prepare what is sent and show what comes back.
 */

/** The kinds, in the order the dialog offers them, with their names. */
export const FIELD_KINDS: readonly { kind: FieldKind; label: string; hint: string }[] = [
  { kind: 'text', label: 'Text', hint: 'One line, such as a part number' },
  { kind: 'number', label: 'Number', hint: 'With a unit after it if you like, such as € or mm' },
  { kind: 'date', label: 'Date', hint: 'A day on the calendar' },
  { kind: 'yes_no', label: 'Yes or no', hint: 'A box to tick' },
  { kind: 'choice', label: 'Choice', hint: 'One of a list you write, each with a color' },
  { kind: 'link', label: 'Link', hint: 'A web address that starts with https://' },
];

/** The name of a kind: "Yes or no". */
export function kindLabel(kind: FieldKind): string {
  return FIELD_KINDS.find((entry) => entry.kind === kind)?.label ?? kind;
}

/** The fields of a project in their order. */
export function inOrder(fields: readonly CustomField[]): CustomField[] {
  return [...fields].sort(
    (first, second) => first.position - second.position || first.id.localeCompare(second.id),
  );
}

/** The choice a value of a choice field names, if the field still has it. */
export function choiceOf(field: CustomField, value: FieldValue | undefined): FieldChoice | null {
  if (value?.kind !== 'choice') {
    return null;
  }
  return field.choices.find((choice) => choice.id === value.choiceId) ?? null;
}

const numberFormat = new Intl.NumberFormat('en', { maximumFractionDigits: 6 });

/** A number with the unit of its field after it: "18.4 €", "1,200 mm". */
export function numberInWords(number: number, unit: string | null): string {
  const written = numberFormat.format(number);
  return unit ? `${written} ${unit}` : written;
}

/**
 * A value in a few words, as a cell or a line of history shows it. A choice
 * removed since says so; a link shows its address without `https://`.
 */
export function valueInWords(field: CustomField | undefined, value: FieldValue): string {
  switch (value.kind) {
    case 'text':
      return value.text;
    case 'number':
      return numberInWords(value.number, field?.unit ?? null);
    case 'date':
      return formatDate(value.date);
    case 'yes_no':
      return value.yes ? 'Yes' : 'No';
    case 'choice': {
      const choice = field?.choices.find((candidate) => candidate.id === value.choiceId);
      return choice ? choice.name : 'a choice that was removed';
    }
    case 'link':
      return shortAddress(value.address);
  }
}

/** A web address without its `https://` and a slash at the end. */
export function shortAddress(address: string): string {
  return address.replace(/^https:\/\//, '').replace(/\/$/, '');
}

/**
 * Reads a number as the manager types it: "18.4", "1,200", "-3", with a
 * comma or a point for the decimals when only one of them is used once.
 * Nothing for text that is not a number; the backend checks the rest.
 */
export function parseNumber(typed: string): number | null {
  let text = typed.trim().replace(/\s/g, '');
  if (text === '') {
    return null;
  }
  const commas = text.split(',').length - 1;
  const points = text.split('.').length - 1;
  if (commas === 1 && points === 0 && !/^-?\d{1,3},\d{3}$/.test(text)) {
    // "12,5" is twelve and a half; "1,200" is twelve hundred.
    text = text.replace(',', '.');
  } else {
    text = text.replace(/,/g, '');
  }
  if (!/^-?(\d+\.?\d*|\.\d+)$/.test(text)) {
    return null;
  }
  const number = Number(text);
  return Number.isFinite(number) ? number : null;
}

/** A number written back for editing, without grouping: "1200.5". */
export function numberForEditing(number: number): string {
  return String(number);
}

/**
 * What a task sorts by in the column of a field: the text, the number, the
 * date, no before yes, a choice by its place in the field. Nothing for an
 * empty cell, which goes last either way.
 */
export function sortValueOfField(field: CustomField, task: Task): string | number | null {
  const value = task.fieldValues[field.id];
  if (value === undefined) {
    return null;
  }
  switch (value.kind) {
    case 'text':
      return value.text;
    case 'number':
      return value.number;
    case 'date':
      return value.date;
    case 'yes_no':
      return value.yes ? 1 : 0;
    case 'choice': {
      const place = field.choices.findIndex((choice) => choice.id === value.choiceId);
      return place === -1 ? null : place;
    }
    case 'link':
      return shortAddress(value.address);
  }
}

/** Whether a task holds the value a filter by a field asks for. A box nobody ticked reads as no. */
export function holdsFiltered(
  task: Task,
  filter:
    { kind: 'choice'; fieldId: Id; choiceId: Id } | { kind: 'yes_no'; fieldId: Id; yes: boolean },
): boolean {
  const value = task.fieldValues[filter.fieldId];
  if (filter.kind === 'choice') {
    return value?.kind === 'choice' && value.choiceId === filter.choiceId;
  }
  const yes = value?.kind === 'yes_no' && value.yes;
  return filter.yes ? yes : !yes;
}
