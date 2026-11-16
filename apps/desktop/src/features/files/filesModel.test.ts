import { describe, expect, it } from 'vitest';

import {
  EMPTY_CONTENT,
  childPath,
  chosenFolder,
  crumbsOf,
  formatSize,
  fullPathOf,
  markOf,
  narrowed,
  parentPath,
  parseContent,
  serializeContent,
} from './filesModel';

describe('the content of a files view', () => {
  it('is read back as it was written', () => {
    const content = { folder: 'C:\\Work\\SSN', path: 'firmware/src', showsHidden: true };

    expect(parseContent(serializeContent(content))).toEqual(content);
  });

  it('falls back to the empty content when it is missing or damaged', () => {
    for (const written of [null, '', '   ', '{', '[]', '42', 'null']) {
      expect(parseContent(written)).toEqual(EMPTY_CONTENT);
    }
  });

  it('keeps the fields it can read and drops the others', () => {
    expect(parseContent('{"folder": 7, "path": "a//b/./c", "showsHidden": "yes"}')).toEqual({
      folder: null,
      path: 'a/b/c',
      showsHidden: false,
    });
  });
});

describe('walking a path inside a folder', () => {
  it('goes into a folder and up again', () => {
    expect(childPath('', 'firmware')).toBe('firmware');
    expect(childPath('firmware', 'src')).toBe('firmware/src');
    expect(parentPath('firmware/src')).toBe('firmware');
    expect(parentPath('firmware')).toBe('');
    expect(parentPath('')).toBe('');
  });

  it('gives crumbs from the folder itself down', () => {
    expect(crumbsOf('C:\\Users\\you\\SSN', 'firmware/src')).toEqual([
      { name: 'SSN', path: '' },
      { name: 'firmware', path: 'firmware' },
      { name: 'src', path: 'firmware/src' },
    ]);
  });

  it('writes a full path with the slashes of its folder', () => {
    expect(fullPathOf('C:\\Work\\SSN\\', 'firmware/src', 'main.c')).toBe(
      'C:\\Work\\SSN\\firmware\\src\\main.c',
    );
    expect(fullPathOf('/home/you/ssn', '', 'README.md')).toBe('/home/you/ssn/README.md');
    expect(fullPathOf('/home/you/ssn', '')).toBe('/home/you/ssn');
  });
});

describe('the mark of an entry', () => {
  it('follows the kind, and for a file the ending of its name', () => {
    expect(markOf({ name: 'firmware', kind: 'folder' })).toBe('folder');
    expect(markOf({ name: 'outside', kind: 'link' })).toBe('link');
    expect(markOf({ name: 'main.C', kind: 'file' })).toBe('text');
    expect(markOf({ name: 'Makefile', kind: 'file' })).toBe('text');
    expect(markOf({ name: 'board.PNG', kind: 'file' })).toBe('picture');
    expect(markOf({ name: 'bom.csv', kind: 'file' })).toBe('sheet');
    expect(markOf({ name: 'firmware.bin', kind: 'file' })).toBe('other');
    expect(markOf({ name: '.env', kind: 'file' })).toBe('other');
  });
});

describe('the size of a file', () => {
  it('is written in the unit that suits it', () => {
    expect(formatSize(null)).toBe('');
    expect(formatSize(0)).toBe('0 bytes');
    expect(formatSize(1)).toBe('1 byte');
    expect(formatSize(1023)).toBe('1023 bytes');
    expect(formatSize(1024)).toBe('1 KB');
    expect(formatSize(1536)).toBe('1.5 KB');
    expect(formatSize(48 * 1024)).toBe('48 KB');
    expect(formatSize(1.4 * 1024 * 1024)).toBe('1.4 MB');
    expect(formatSize(3 * 1024 ** 4)).toBe('3 TB');
  });
});

describe('narrowing the names', () => {
  const entries = [{ name: 'main.c' }, { name: 'Makefile' }, { name: 'sensor.h' }];

  it('keeps the names that hold the typed word, whatever the capitals', () => {
    expect(narrowed(entries, 'MA').map((entry) => entry.name)).toEqual(['main.c', 'Makefile']);
    expect(narrowed(entries, '  ')).toHaveLength(3);
    expect(narrowed(entries, 'xyz')).toEqual([]);
  });
});

describe('the folder that is shown', () => {
  it('is the one kept while the project has it, and otherwise the first', () => {
    const folders = ['C:\\Work', 'D:\\Sheets'];

    expect(chosenFolder(folders, 'D:\\Sheets')).toBe('D:\\Sheets');
    expect(chosenFolder(folders, 'E:\\Gone')).toBe('C:\\Work');
    expect(chosenFolder(folders, null)).toBe('C:\\Work');
    expect(chosenFolder([], null)).toBeNull();
  });
});
