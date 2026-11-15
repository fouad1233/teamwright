import { describe, expect, it } from 'vitest';

import { workbookFileName } from './workbookExport';

describe('the name suggested for the workbook of a project', () => {
  it('is the name of the project with .xlsx', () => {
    expect(workbookFileName('Smart home hub')).toBe('Smart home hub.xlsx');
  });

  it('leaves out the characters a file name may not hold', () => {
    expect(workbookFileName('Hub: v2 / "lights"?')).toBe('Hub v2 lights.xlsx');
  });

  it('falls back to a plain name when nothing is left', () => {
    expect(workbookFileName(' / ')).toBe('Project.xlsx');
  });
});
