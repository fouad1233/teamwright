import { describe, expect, it } from 'vitest';

import { copiedInWords } from './placesModel';

describe('copiedInWords', () => {
  it('names one or two files and counts more', () => {
    expect(copiedInWords([])).toBe('Nothing was copied');
    expect(copiedInWords(['sheet.pdf'])).toBe('Copied sheet.pdf');
    expect(copiedInWords(['sheet.pdf', 'notes.txt'])).toBe('Copied sheet.pdf and notes.txt');
    expect(copiedInWords(['a', 'b', 'c'])).toBe('Copied 3 files');
  });
});
