import { describe, expect, it } from 'vitest';

import { FULL_ACCESS_WORDS, coordinatorWithFullAccess, copiedInWords } from './placesModel';

describe('copiedInWords', () => {
  it('names one or two files and counts more', () => {
    expect(copiedInWords([])).toBe('Nothing was copied');
    expect(copiedInWords(['sheet.pdf'])).toBe('Copied sheet.pdf');
    expect(copiedInWords(['sheet.pdf', 'notes.txt'])).toBe('Copied sheet.pdf and notes.txt');
    expect(copiedInWords(['a', 'b', 'c'])).toBe('Copied 3 files');
  });
});

describe('the words about full access', () => {
  it('say what it gives, that a file can be deleted, what stays closed and where it holds', () => {
    const words = FULL_ACCESS_WORDS.join(' ');
    expect(words).toContain('anywhere your account can');
    expect(words).toContain('change or delete any of your files');
    expect(words).toContain('SSH and GnuPG keys stay closed');
    expect(words).toContain('for this computer only');
  });

  it('name the coordinator only when it starts work here while full access is on', () => {
    expect(coordinatorWithFullAccess(false, true)).toBeNull();
    expect(coordinatorWithFullAccess(true, false)).toBeNull();
    expect(coordinatorWithFullAccess(true, true)).toContain('has full access too');
  });
});
