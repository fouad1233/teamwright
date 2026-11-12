import { describe, expect, it } from 'vitest';

import { waitingInWords } from './inboxModel';

describe('waitingInWords', () => {
  it('says how many things wait, and that none does', () => {
    expect(waitingInWords(0)).toBe('Nothing waits for you');
    expect(waitingInWords(1)).toBe('1 thing waits for you');
    expect(waitingInWords(3)).toBe('3 things wait for you');
  });
});
