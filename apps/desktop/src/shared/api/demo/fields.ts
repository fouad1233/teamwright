import { BackendError } from '../backend';
import type { CustomField, FieldValue } from '../types';

/**
 * The rules about custom fields that concern one field, one name or one
 * value, for the demo backend. The real application has them in its domain
 * crate (`field.rs`).
 */

/** The most fields a project may have. */
export const MOST_FIELDS_IN_A_PROJECT = 30;
/** The most choices a field may have. */
export const MOST_CHOICES = 50;
/** The largest number a field holds, either side of zero. */
const LARGEST_NUMBER = 1e12;

/** Tidies a text: trimmed and on one line, not empty and not too long. */
function tidy(raw: string, field: string, maxLength: number): string {
  const text = raw.trim().split(/\s+/).join(' ');
  if (text === '') {
    throw new BackendError('rule_broken', `the ${field} must not be empty`);
  }
  const length = [...text].length;
  if (length > maxLength) {
    throw new BackendError(
      'rule_broken',
      `the ${field} can have at most ${maxLength.toString()} characters, but this one has ${length.toString()}`,
    );
  }
  return text;
}

export function tidyFieldName(raw: string): string {
  return tidy(raw, 'field name', 40);
}

export function tidyChoiceName(raw: string): string {
  return tidy(raw, 'choice name', 30);
}

/** Tidies a unit; nothing or an empty text is no unit. */
export function tidyUnit(raw: string | null | undefined): string | null {
  return raw === null || raw === undefined || raw.trim() === '' ? null : tidy(raw, 'unit', 8);
}

/** Refuses a name another field of the project has, whatever the capitals. */
export function refuseFieldNameInUse(others: readonly CustomField[], name: string): void {
  const wanted = name.toLowerCase();
  if (others.some((field) => field.name.toLowerCase() === wanted)) {
    throw new BackendError('field_name_taken', `the project already has a field called "${name}"`);
  }
}

/** Refuses a name another choice of the field has, whatever the capitals. */
export function refuseChoiceNameInUse(field: CustomField, name: string, except?: string): void {
  const wanted = name.toLowerCase();
  if (
    field.choices.some((choice) => choice.id !== except && choice.name.toLowerCase() === wanted)
  ) {
    throw new BackendError('choice_name_taken', `the field already has a choice called "${name}"`);
  }
}

/** Refuses a field that has no choices. */
export function requireChoices(field: CustomField): void {
  if (field.kind !== 'choice') {
    throw new BackendError('rule_broken', `the field "${field.name}" has no choices`);
  }
}

/** The words a field of a kind holds, as the backend says them. */
const KIND_IN_WORDS: Record<CustomField['kind'], string> = {
  text: 'text',
  number: 'number',
  date: 'date',
  yes_no: 'yes or no',
  choice: 'choice',
  link: 'web address',
};

/**
 * Checks a value against its field and tidies it: of the kind of the field,
 * a text on one line, an ordinary number, a real date, a choice of the
 * field, a link that starts with https:// and names a host.
 */
export function checkedValue(field: CustomField, value: FieldValue): FieldValue {
  if (value.kind !== field.kind) {
    throw new BackendError(
      'rule_broken',
      `the field "${field.name}" holds a ${KIND_IN_WORDS[field.kind]}`,
    );
  }
  switch (value.kind) {
    case 'text':
      return { kind: 'text', text: tidy(value.text, 'text of a field', 200) };
    case 'number':
      if (!Number.isFinite(value.number) || Math.abs(value.number) > LARGEST_NUMBER) {
        throw new BackendError(
          'rule_broken',
          'a number in a field lies between minus and plus a thousand billion',
        );
      }
      return { kind: 'number', number: value.number + 0 };
    case 'date':
      if (!/^\d{4}-\d{2}-\d{2}$/.test(value.date) || Number.isNaN(Date.parse(value.date))) {
        throw new BackendError(
          'invalid_request',
          `'${value.date}' is not a date in the form year-month-day`,
        );
      }
      return value;
    case 'yes_no':
      return value;
    case 'choice':
      if (!field.choices.some((choice) => choice.id === value.choiceId)) {
        throw new BackendError('not_found', 'the field has no such choice');
      }
      return value;
    case 'link': {
      const address = value.address.trim();
      const host = address.startsWith('https://') ? address.slice(8).split(/[/?#]/)[0] : '';
      if (
        !host ||
        host.startsWith('.') ||
        host.startsWith(':') ||
        /\s/.test(address) ||
        address.length > 2000
      ) {
        throw new BackendError(
          'rule_broken',
          'a link starts with https:// and names a host, with no spaces',
        );
      }
      return { kind: 'link', address };
    }
  }
}

/** The values of a task without the one of a field. */
export function withoutValue(
  values: Readonly<Record<string, FieldValue>>,
  fieldId: string,
): Record<string, FieldValue> {
  return Object.fromEntries(Object.entries(values).filter(([id]) => id !== fieldId));
}

/** The fields of a project in their order. */
export function inOrder(fields: readonly CustomField[]): CustomField[] {
  return [...fields].sort(
    (first, second) => first.position - second.position || first.id.localeCompare(second.id),
  );
}
