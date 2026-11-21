import { describe, expect, it } from 'vitest';

import type { CustomField, Task } from '@/shared/api/types';

import { holdsFiltered, parseNumber, sortValueOfField, valueInWords } from './fieldModel';

const supplier: CustomField = {
  id: 'supplier',
  projectId: 'p',
  name: 'Supplier',
  kind: 'choice',
  unit: null,
  position: 0,
  choices: [
    { id: 'mouser', name: 'Mouser', color: 'blue' },
    { id: 'lcsc', name: 'LCSC', color: 'teal' },
  ],
};
const cost: CustomField = {
  ...supplier,
  id: 'cost',
  name: 'Unit cost',
  kind: 'number',
  unit: '€',
  choices: [],
};

function task(fieldValues: Task['fieldValues']): Task {
  return { fieldValues } as Task;
}

describe('a value in words', () => {
  it('says a number with its unit, a box as yes or no, and a removed choice so', () => {
    expect(valueInWords(cost, { kind: 'number', number: 1200.5 })).toBe('1,200.5 €');
    expect(valueInWords(undefined, { kind: 'yes_no', yes: false })).toBe('No');
    expect(valueInWords(supplier, { kind: 'choice', choiceId: 'lcsc' })).toBe('LCSC');
    expect(valueInWords(supplier, { kind: 'choice', choiceId: 'gone' })).toBe(
      'a choice that was removed',
    );
    expect(valueInWords(undefined, { kind: 'link', address: 'https://ti.com/ds/' })).toBe(
      'ti.com/ds',
    );
  });
});

describe('a typed number', () => {
  it('reads points, commas and groups, and refuses what is not a number', () => {
    expect(parseNumber(' 18.4 ')).toBe(18.4);
    expect(parseNumber('12,5')).toBe(12.5);
    expect(parseNumber('1,200')).toBe(1200);
    expect(parseNumber('1,200.75')).toBe(1200.75);
    expect(parseNumber('-3')).toBe(-3);
    expect(parseNumber('')).toBeNull();
    expect(parseNumber('12 mm')).toBeNull();
    expect(parseNumber('1e3')).toBeNull();
  });
});

describe('sorting and filtering by a field', () => {
  it('sorts a choice by its place in the field and leaves an empty cell out', () => {
    expect(
      sortValueOfField(supplier, task({ supplier: { kind: 'choice', choiceId: 'lcsc' } })),
    ).toBe(1);
    expect(sortValueOfField(supplier, task({}))).toBeNull();
  });

  it('counts a box nobody ticked as no', () => {
    const filter = { kind: 'yes_no', fieldId: 'ordered', yes: false } as const;
    expect(holdsFiltered(task({}), filter)).toBe(true);
    expect(holdsFiltered(task({ ordered: { kind: 'yes_no', yes: true } }), filter)).toBe(false);
    expect(
      holdsFiltered(task({ ordered: { kind: 'yes_no', yes: true } }), { ...filter, yes: true }),
    ).toBe(true);
  });
});
