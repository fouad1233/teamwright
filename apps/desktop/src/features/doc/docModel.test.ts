import { describe, expect, it } from 'vitest';

import type { Task } from '@/shared/api/types';

import {
  applyEdit,
  boxTally,
  coded,
  contentsOf,
  continued,
  entryAt,
  entryInView,
  headingsOf,
  lineOf,
  linked,
  marked,
  pageFacts,
  plainWords,
  readPage,
  startOfLine,
  stepFor,
  taskKeyPattern,
  tasksByKey,
  withBoxTicked,
  withLinesAs,
  wordCount,
  writePage,
  type Edit,
  type Selection,
} from './docModel';

/**
 * A text with its selection written into it: `|` is the caret, and two of
 * them stand at the ends of what is selected.
 */
function written(sketch: string): { text: string; selection: Selection } {
  const start = sketch.indexOf('|');
  const end = sketch.indexOf('|', start + 1);
  const text = sketch.replaceAll('|', '');
  return { text, selection: { start, end: end === -1 ? start : end - 1 } };
}

/** The text after a change, with what is selected written into it the same way. */
function after(text: string, edit: Edit): string {
  const changed = applyEdit(text, edit);
  const { start, end } = edit.select;
  return start === end
    ? `${changed.slice(0, start)}|${changed.slice(start)}`
    : `${changed.slice(0, start)}|${changed.slice(start, end)}|${changed.slice(end)}`;
}

describe('what the view keeps', () => {
  it('reads the text of a page', () => {
    expect(readPage('{"text":"# Pin map\\n\\nSee SSN-5."}')).toBe('# Pin map\n\nSee SSN-5.');
  });

  it('reads a page that also keeps something a newer version added', () => {
    expect(readPage('{"version":2,"text":"Notes","width":"wide"}')).toBe('Notes');
  });

  it('reads content that is missing, empty or of another shape as an empty page', () => {
    for (const content of [null, '', '   ', 'not json', '{', '[]', '7', 'null', '"text"']) {
      expect(readPage(content)).toBe('');
    }
    expect(readPage('{"notes":[]}')).toBe('');
    expect(readPage('{"text":12}')).toBe('');
    expect(readPage('{"text":null}')).toBe('');
  });

  it('ends every line in one character, whatever wrote the text', () => {
    expect(readPage(JSON.stringify({ text: 'one\r\ntwo\rthree\nfour' }))).toBe(
      'one\ntwo\nthree\nfour',
    );
  });

  it('keeps a page as its text, and a page without words as nothing', () => {
    expect(writePage('# Pin map')).toBe('{"text":"# Pin map"}');
    expect(writePage('')).toBeNull();
    expect(writePage(' \n\n ')).toBeNull();
    expect(readPage(writePage('Line one\n\n- [ ] "quoted" \\ and more'))).toBe(
      'Line one\n\n- [ ] "quoted" \\ and more',
    );
  });
});

describe('the lines of a text', () => {
  const text = 'one\ntwo\n\nfour';

  it('says which line a place is on', () => {
    expect(lineOf(text, 0)).toBe(1);
    expect(lineOf(text, 3)).toBe(1);
    expect(lineOf(text, 4)).toBe(2);
    expect(lineOf(text, 8)).toBe(3);
    expect(lineOf(text, text.length)).toBe(4);
  });

  it('says where a line begins', () => {
    expect(startOfLine(text, 1)).toBe(0);
    expect(startOfLine(text, 2)).toBe(4);
    expect(startOfLine(text, 3)).toBe(8);
    expect(startOfLine(text, 4)).toBe(9);
    expect(startOfLine(text, 9)).toBe(text.length);
  });
});

describe('headingsOf', () => {
  it('finds the lines that begin with hashes, with their level and their line', () => {
    const text = '# Pin map\n\nWords.\n\n## Power\n\n### The charger\n\n###### Small';
    expect(headingsOf(text)).toEqual([
      { level: 1, title: 'Pin map', line: 1 },
      { level: 2, title: 'Power', line: 5 },
      { level: 3, title: 'The charger', line: 7 },
      { level: 6, title: 'Small', line: 9 },
    ]);
  });

  it('wants a space after the hashes, and at most six of them', () => {
    expect(
      headingsOf('#hashtag\n####### seven\n   # set in by three\n    # set in by four'),
    ).toEqual([{ level: 1, title: 'set in by three', line: 3 }]);
  });

  it('leaves the closing hashes out of the title', () => {
    expect(headingsOf('## Power ##\n# C# is not closed\n# Only hashes #\n#')).toEqual([
      { level: 2, title: 'Power', line: 1 },
      { level: 1, title: 'C# is not closed', line: 2 },
      { level: 1, title: 'Only hashes', line: 3 },
      { level: 1, title: '', line: 4 },
    ]);
  });

  it('finds a paragraph that is underlined, on the line where it begins', () => {
    const text = 'Pin map\n=======\n\nA plan of\nthe pins\n---\n\nWords.';
    expect(headingsOf(text)).toEqual([
      { level: 1, title: 'Pin map', line: 1 },
      { level: 2, title: 'A plan of the pins', line: 4 },
    ]);
  });

  it('takes a line of hyphens without a paragraph over it as a rule', () => {
    expect(headingsOf('Words.\n\n---\n\nMore words.\n\n- - -\n')).toEqual([]);
  });

  it('finds no heading in a block of code', () => {
    const text = [
      '```c',
      '# define PIN 4',
      '```',
      '',
      '~~~',
      '## not this',
      '```',
      '# nor this',
      '~~~',
      '# A real one',
    ].join('\n');
    expect(headingsOf(text)).toEqual([{ level: 1, title: 'A real one', line: 10 }]);
  });

  it('reads a fence that is never closed to the end of the text', () => {
    expect(headingsOf('# Before\n\n```\n# inside\n\n# still inside')).toEqual([
      { level: 1, title: 'Before', line: 1 },
    ]);
  });

  it('knows the fence of a block of code inside a list or a quote', () => {
    const text =
      '- Run this:\n  ```sh\n  # a comment\n  ```\n> ```\n> # quoted code\n> ```\n# After';
    expect(headingsOf(text)).toEqual([{ level: 1, title: 'After', line: 8 }]);
  });

  it('leaves out a heading inside a quote or a list', () => {
    expect(headingsOf('> # Quoted\n\n- # Listed\n\n# Plain')).toEqual([
      { level: 1, title: 'Plain', line: 5 },
    ]);
  });

  it('does not take the words of a list or a quote for an underlined heading', () => {
    const text = '- an item\n  that goes on\n---\n\n> a quote\nthat goes on\n===\n\n| a |\n---';
    expect(headingsOf(text)).toEqual([]);
  });

  it('tells the title without the signs of Markdown', () => {
    const text =
      '## The **pin** map, _drawn_ `twice` for [the board](https://example.com/b)\n' +
      '## A snake_case_name and 2 * 3 * 4\n' +
      '## Written \\*out\\* and `a*b*c`';
    expect(headingsOf(text).map((heading) => heading.title)).toEqual([
      'The pin map, drawn twice for the board',
      'A snake_case_name and 2 * 3 * 4',
      'Written *out* and a*b*c',
    ]);
  });
});

describe('plainWords', () => {
  it('keeps the words of links and pictures and drops their addresses', () => {
    expect(plainWords('See [the sheet](https://example.com/a_b) and ![a plan](plan.png)')).toBe(
      'See the sheet and a plan',
    );
    expect(plainWords('[the sheet][1] at <https://example.com>')).toBe(
      'the sheet at https://example.com',
    );
  });

  it('takes away bold, italic and struck marks', () => {
    expect(plainWords('**bold** __bold__ *it* _it_ ~~gone~~')).toBe('bold bold it it gone');
  });
});

describe('contentsOf', () => {
  const heading = (level: number, title: string, line: number) => ({ level, title, line });

  it('lists nothing for a page with two headings or fewer', () => {
    expect(contentsOf([])).toEqual([]);
    expect(contentsOf([heading(1, 'Title', 1), heading(2, 'Part', 4)])).toEqual([]);
  });

  it('leaves out the title of the page, and sets the rest in by their level', () => {
    const headings = [
      heading(1, 'Pin map', 1),
      heading(2, 'Power', 5),
      heading(3, 'The charger', 9),
      heading(2, 'Sensors', 14),
    ];
    expect(contentsOf(headings)).toEqual([
      { title: 'Power', line: 5, depth: 0 },
      { title: 'The charger', line: 9, depth: 1 },
      { title: 'Sensors', line: 14, depth: 0 },
    ]);
  });

  it('keeps a first heading that is not the only one of its size', () => {
    const headings = [heading(2, 'Power', 1), heading(2, 'Sensors', 5), heading(2, 'Display', 9)];
    expect(contentsOf(headings).map((entry) => entry.title)).toEqual([
      'Power',
      'Sensors',
      'Display',
    ]);
    const chapters = [heading(1, 'One', 1), heading(2, 'Part', 3), heading(1, 'Two', 9)];
    expect(contentsOf(chapters)).toEqual([
      { title: 'One', line: 1, depth: 0 },
      { title: 'Part', line: 3, depth: 1 },
      { title: 'Two', line: 9, depth: 0 },
    ]);
  });

  it('leaves out headings without words and those that are set as labels', () => {
    const headings = [
      heading(2, 'Power', 1),
      heading(2, '', 3),
      heading(3, 'Charger', 5),
      heading(4, 'A label', 7),
      heading(6, 'A smaller label', 9),
      heading(2, 'Sensors', 11),
    ];
    expect(contentsOf(headings)).toEqual([
      { title: 'Power', line: 1, depth: 0 },
      { title: 'Charger', line: 5, depth: 1 },
      { title: 'Sensors', line: 11, depth: 0 },
    ]);
  });

  it('does not count labels and headings without words among the headings of a page', () => {
    const headings = [heading(1, 'Title', 1), heading(2, 'Part', 3), heading(4, 'A label', 5)];
    expect(contentsOf(headings)).toEqual([]);
    expect(contentsOf([heading(2, 'One', 1), heading(2, '', 3), heading(2, 'Two', 5)])).toEqual([]);
  });

  it('takes a lone first heading for the title whatever its size', () => {
    const headings = [heading(2, 'Pin map', 1), heading(3, 'Power', 3), heading(3, 'Sensors', 7)];
    expect(contentsOf(headings).map((entry) => entry.title)).toEqual(['Power', 'Sensors']);
  });
});

describe('where the reader is', () => {
  const entries = [
    { title: 'Power', line: 5, depth: 0 },
    { title: 'Sensors', line: 14, depth: 0 },
    { title: 'Display', line: 30, depth: 0 },
  ];

  it('finds the entry a line of the text belongs to', () => {
    expect(entryAt(entries, 1)).toBeNull();
    expect(entryAt(entries, 5)?.title).toBe('Power');
    expect(entryAt(entries, 13)?.title).toBe('Power');
    expect(entryAt(entries, 14)?.title).toBe('Sensors');
    expect(entryAt(entries, 99)?.title).toBe('Display');
    expect(entryAt([], 3)).toBeNull();
  });

  it('finds the entry whose heading has come up to the line where reading begins', () => {
    expect(entryInView(entries, [120, 400, 900], false)).toBeNull();
    expect(entryInView(entries, [0, 280, 780], false)?.title).toBe('Power');
    expect(entryInView(entries, [-300, -20, 480], false)?.title).toBe('Sensors');
  });

  it('passes over a heading that is not drawn', () => {
    expect(entryInView(entries, [-300, null, 480], false)?.title).toBe('Power');
  });

  it('is at the last entry at the foot of the page, however far down it stands', () => {
    expect(entryInView(entries, [-300, -20, 480], true)?.title).toBe('Display');
    expect(entryInView([], [], true)).toBeNull();
  });
});

describe('boxes', () => {
  const text =
    'Before\n\n- [ ] Order the parts\n- [x] Choose the chip\n  1. [X] Nested\n> * [ ] Quoted';

  it('ticks the box of the item that begins at a place, and clears a ticked one', () => {
    const first = text.indexOf('- [ ] Order');
    const second = text.indexOf('- [x] Choose');
    const nested = text.indexOf('1. [X]');
    const quoted = text.indexOf('* [ ]');
    expect(withBoxTicked(text, first)).toBe(text.replace('- [ ] Order', '- [x] Order'));
    expect(withBoxTicked(text, second)).toBe(text.replace('- [x] Choose', '- [ ] Choose'));
    expect(withBoxTicked(text, nested)).toBe(text.replace('1. [X]', '1. [ ]'));
    expect(withBoxTicked(text, quoted)).toBe(text.replace('* [ ] Quoted', '* [x] Quoted'));
  });

  it('leaves the text as it was at a place without a box', () => {
    expect(withBoxTicked(text, 0)).toBe(text);
    expect(withBoxTicked(text, text.indexOf('[ ] Order'))).toBe(text);
    expect(withBoxTicked(text, 9999)).toBe(text);
    expect(withBoxTicked('- [y] not a box', 0)).toBe('- [y] not a box');
  });

  it('counts the boxes of a page and the ticked ones', () => {
    expect(boxTally(text)).toEqual({ boxes: 4, ticked: 2 });
    expect(boxTally('No boxes here.\n- a plain item')).toEqual({ boxes: 0, ticked: 0 });
  });

  it('counts no box in a block of code, and none without something to do', () => {
    expect(boxTally('```\n- [ ] in code\n```\n- [ ]\n- [ ] \n- [x] real')).toEqual({
      boxes: 1,
      ticked: 1,
    });
  });
});

describe('what is counted on a page', () => {
  it('counts the words, and not the address of a link', () => {
    expect(wordCount('')).toBe(0);
    expect(wordCount("# The pin map\n\nIt isn't final: 40 by 60 mm.")).toBe(10);
    expect(wordCount('See [the data sheet](https://example.com/a/b/c.pdf) first.')).toBe(5);
    expect(wordCount('Größe, naïve café')).toBe(3);
  });

  it('counts neither the number of an item nor the sign in its box', () => {
    expect(wordCount('1. one\n2. two\n10) ten')).toBe(3);
    expect(wordCount('- [x] done\n  * [ ] open\n> 3. [X] quoted')).toBe(3);
    expect(wordCount('It was 3. Then 4.')).toBe(5);
  });

  it('says it in a few words', () => {
    expect(pageFacts('')).toEqual([]);
    expect(pageFacts('One')).toEqual(['1 word']);
    expect(pageFacts('Two words\n\n- [x] done\n- [ ] open')).toEqual(['4 words', '1 of 2 ticked']);
  });
});

describe('the keys of tasks', () => {
  it('finds the keys of the project as words of their own', () => {
    const pattern = new RegExp(taskKeyPattern('SSN').source, 'g');
    const found = 'SSN-4, (SSN-12) and SSN-7. Not XSSN-4, SSN-4a, ssn-4, SSN- 4 or IOT-3.'.match(
      pattern,
    );
    expect(found).toEqual(['SSN-4', 'SSN-12', 'SSN-7']);
  });

  it('takes the key of the project as it is written', () => {
    expect(taskKeyPattern('A.B').test('AxB-1')).toBe(false);
    expect(taskKeyPattern('A.B').test('A.B-1')).toBe(true);
  });

  it('looks tasks up by their keys', () => {
    const tasks = [{ number: 4 }, { number: 12 }] as Task[];
    const byKey = tasksByKey({ key: 'SSN' }, tasks);
    expect(byKey.get('SSN-4')).toBe(tasks[0]);
    expect(byKey.get('SSN-12')).toBe(tasks[1]);
    expect(byKey.get('SSN-04')).toBeUndefined();
  });
});

describe('marked', () => {
  const bold = (sketch: string) => {
    const { text, selection } = written(sketch);
    return after(text, marked(text, selection, '**'));
  };
  const italic = (sketch: string) => {
    const { text, selection } = written(sketch);
    return after(text, marked(text, selection, '_'));
  };

  it('puts the mark on both sides of what is selected and keeps it selected', () => {
    expect(bold('the |pin map| is')).toBe('the **|pin map|** is');
    expect(italic('the |pin map| is')).toBe('the _|pin map|_ is');
  });

  it('leaves the spaces at the ends of the selection outside the marks', () => {
    expect(bold('the| pin map |is')).toBe('the **|pin map|** is');
  });

  it('takes the marks away when they stand around what is selected', () => {
    expect(bold('the **|pin map|** is')).toBe('the |pin map| is');
    expect(italic('the _|pin map|_ is')).toBe('the |pin map| is');
  });

  it('takes the marks away when they are part of what is selected', () => {
    expect(bold('the |**pin map**| is')).toBe('the |pin map| is');
  });

  it('sets the marks down with the caret between them, and takes them away again', () => {
    expect(bold('a | b')).toBe('a **|** b');
    expect(bold('a **|** b')).toBe('a | b');
    expect(bold('|')).toBe('**|**');
  });

  it('does not take half of a double mark for a single one', () => {
    expect(italic('__|bold|__')).toBe('___|bold|___');
    expect(italic('|__bold__|')).toBe('_|__bold__|_');
  });

  it('takes bold out of bold and italic, and leaves the italic', () => {
    expect(bold('***|both|***')).toBe('*|both|*');
  });
});

describe('linked', () => {
  const link = (sketch: string) => {
    const { text, selection } = written(sketch);
    return after(text, linked(text, selection));
  };

  it('makes the selected words the words of a link, with the caret where the address goes', () => {
    expect(link('see |the data sheet| first')).toBe('see [the data sheet](|) first');
    expect(link('see| the data sheet |first')).toBe('see [the data sheet](|) first');
  });

  it('makes a selected address the address, with the caret where the words go', () => {
    expect(link('see |https://example.com/sheet.pdf| first')).toBe(
      'see [|](https://example.com/sheet.pdf) first',
    );
  });

  it('sets an empty link down where the caret is, ready for its words', () => {
    expect(link('see | first')).toBe('see [|]() first');
  });
});

describe('coded', () => {
  const code = (sketch: string) => {
    const { text, selection } = written(sketch);
    return after(text, coded(text, selection));
  };

  it('marks what is selected within a line with backticks, and takes them away again', () => {
    expect(code('set |PIN_SDA| to 4')).toBe('set `|PIN_SDA|` to 4');
    expect(code('set `|PIN_SDA|` to 4')).toBe('set |PIN_SDA| to 4');
    expect(code('set | to 4')).toBe('set `|` to 4');
  });

  it('puts several lines between two fences, whole lines only', () => {
    expect(code('Before\n#define |SDA 4\n#define SCL| 5\nAfter')).toBe(
      'Before\n```\n|#define SDA 4\n#define SCL 5|\n```\nAfter',
    );
  });

  it('does not take in the line a selection only ends at the beginning of', () => {
    expect(code('|one\ntwo\n|three')).toBe('```\n|one\ntwo|\n```\nthree');
  });

  it('takes several lines out from between their fences', () => {
    expect(code('Before\n```c\n|one\ntwo|\n```\nAfter')).toBe('Before\n|one\ntwo|\nAfter');
    expect(code('```\n|one\ntwo|\n```')).toBe('|one\ntwo|');
  });
});

describe('withLinesAs', () => {
  const lines = (kind: Parameters<typeof withLinesAs>[2]) => (sketch: string) => {
    const { text, selection } = written(sketch);
    return after(text, withLinesAs(text, selection, kind));
  };
  const heading = lines('heading');
  const list = lines('list');
  const boxes = lines('boxes');
  const quote = lines('quote');

  it('steps a heading down with every use, and then lets it go', () => {
    expect(heading('Before\nPo|wer\nAfter')).toBe('Before\n# Po|wer\nAfter');
    expect(heading('Before\n# Po|wer\nAfter')).toBe('Before\n## Po|wer\nAfter');
    expect(heading('Before\n## Po|wer\nAfter')).toBe('Before\n### Po|wer\nAfter');
    expect(heading('Before\n### Po|wer\nAfter')).toBe('Before\nPo|wer\nAfter');
  });

  it('keeps the caret behind the marks when it stood among them', () => {
    expect(heading('|Power')).toBe('# |Power');
    expect(heading('#|## Power')).toBe('|Power');
    expect(heading('|')).toBe('# |');
  });

  it('makes a heading of an item of a list without its mark', () => {
    expect(heading('  - [ ] Order |parts')).toBe('# Order |parts');
  });

  it('makes headings of several lines, by the size of the first, and passes over empty ones', () => {
    expect(heading('|one\n\n## two\n- three|')).toBe('|# one\n\n# two\n# three|');
    expect(heading('|## one\ntwo|')).toBe('|### one\n### two|');
  });

  it('takes only the hashes away when it lets a heading go', () => {
    expect(heading('|### one\n- two\n\n  1. three|')).toBe('|one\n- two\n\n  1. three|');
  });

  it('makes a list of the lines that are touched, and passes over empty ones', () => {
    expect(list('Before\n|one\n\ntwo|\nAfter')).toBe('Before\n|- one\n\n- two|\nAfter');
    expect(list('o|ne')).toBe('- o|ne');
    expect(list('|')).toBe('- |');
  });

  it('lets a list go when every line is an item already', () => {
    expect(list('|- one\n* two|')).toBe('|one\ntwo|');
    expect(list('  - o|ne')).toBe('  o|ne');
  });

  it('makes a plain list of numbers, boxes and headings, and keeps how far they are set in', () => {
    expect(list('|1. one\n  - [x] two\n## three|')).toBe('|- one\n  - two\n- three|');
  });

  it('makes boxes of lines and of items, and leaves the boxes that are there', () => {
    expect(boxes('|one\n- two\n- [x] three\n3. four|')).toBe(
      '|- [ ] one\n- [ ] two\n- [x] three\n- [ ] four|',
    );
    expect(boxes('|')).toBe('- [ ] |');
  });

  it('lets the boxes go when every line has one', () => {
    expect(boxes('|- [ ] one\n- [x] two|')).toBe('|one\ntwo|');
  });

  it('quotes every line that is touched, the empty ones too', () => {
    expect(quote('Before\n|one\n\ntwo|\nAfter')).toBe('Before\n|> one\n>\n> two|\nAfter');
    expect(quote('o|ne')).toBe('> o|ne');
    expect(quote('|')).toBe('> |');
  });

  it('lets a quote go when every line is quoted', () => {
    expect(quote('|> one\n>\n> two|')).toBe('|one\n\ntwo|');
    expect(quote('> > o|ne')).toBe('> o|ne');
  });

  it('does not touch the line a selection only ends at the beginning of', () => {
    expect(list('|one\n|two')).toBe('|- one|\ntwo');
  });
});

describe('continued', () => {
  const enter = (sketch: string) => {
    const { text, selection } = written(sketch);
    const edit = continued(text, selection);
    return edit === null ? null : after(text, edit);
  };

  it('begins the next line the way the item it leaves begins', () => {
    expect(enter('- one|')).toBe('- one\n- |');
    expect(enter('* one|\nafter')).toBe('* one\n* |\nafter');
    expect(enter('  + one|')).toBe('  + one\n  + |');
  });

  it('goes on with the next number in a numbered list', () => {
    expect(enter('1. one|')).toBe('1. one\n2. |');
    expect(enter('9) nine|')).toBe('9) nine\n10) |');
  });

  it('gives the next item an empty box', () => {
    expect(enter('- [x] done|')).toBe('- [x] done\n- [ ] |');
    expect(enter('2. [ ] open|')).toBe('2. [ ] open\n3. [ ] |');
  });

  it('takes the rest of the line along to the new item', () => {
    expect(enter('- one| two')).toBe('- one\n- | two');
  });

  it('goes on with a quote, and with a list inside a quote', () => {
    expect(enter('> said|')).toBe('> said\n> |');
    expect(enter('> - one|')).toBe('> - one\n> - |');
  });

  it('takes the mark away from an item without words, which ends the list', () => {
    expect(enter('- one\n- |')).toBe('- one\n|');
    expect(enter('- one\n- [ ] |\nafter')).toBe('- one\n|\nafter');
    expect(enter('> |')).toBe('|');
  });

  it('leaves Enter alone on a plain line, among the marks, and with a selection', () => {
    expect(enter('plain|')).toBeNull();
    expect(enter('|- one')).toBeNull();
    expect(enter('-| one')).toBeNull();
    expect(enter('- |one| two')).toBeNull();
    expect(enter('|')).toBeNull();
  });
});

describe('stepFor', () => {
  const page = { text: 'now', isBusy: false, known: ['first', 'before', 'now'] };

  it('keeps the page when the stored text is the text it shows', () => {
    expect(stepFor('now', page)).toBe('keep');
  });

  it('takes a text it has never held when it is at rest', () => {
    expect(stepFor('from another computer', page)).toBe('adopt');
    expect(stepFor('', page)).toBe('adopt');
  });

  it('never gives up its text while it is written or holds something unsaved', () => {
    expect(stepFor('from another computer', { ...page, isBusy: true })).toBe('keep');
    expect(stepFor('before', { ...page, isBusy: true })).toBe('keep');
  });

  it('asks again when it is handed a text it wrote itself earlier', () => {
    expect(stepFor('before', page)).toBe('reload');
    expect(stepFor('first', page)).toBe('reload');
  });
});
