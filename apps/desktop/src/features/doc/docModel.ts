import type { Project, Task } from '@/shared/api/types';
import { taskReference } from '@/shared/lib/format';

/**
 * What a doc works out without drawing anything: the text a view keeps, the
 * headings of a page, its boxes and its words, the keys of tasks in it, and
 * what the tools of the bar do to the text.
 *
 * The page is drawn by the Markdown reader of `shared/ui/Prose`. The
 * functions here read the same text line by line, and agree with that
 * reader on everything a manager is likely to write. Where they do not, a
 * heading is missing from the list at the side and nothing else goes wrong.
 */

/* What the view keeps. */

/**
 * The text of the page a view keeps. Content that is missing, empty, not
 * JSON or of another shape is an empty page: nothing here ever throws.
 */
export function readPage(content: string | null): string {
  if (content === null || content.trim() === '') {
    return '';
  }
  let kept: unknown;
  try {
    kept = JSON.parse(content);
  } catch {
    return '';
  }
  if (typeof kept !== 'object' || kept === null || !('text' in kept)) {
    return '';
  }
  // A line ends in one character here, whatever wrote the text.
  return typeof kept.text === 'string' ? kept.text.replace(/\r\n?/g, '\n') : '';
}

/** A page as the view keeps it. A page without words is kept as nothing. */
export function writePage(text: string): string | null {
  return text.trim() === '' ? null : JSON.stringify({ text });
}

/* The lines of the text. */

interface SourceLine {
  text: string;
  /** The number of the line, from 1. */
  number: number;
  /** Whether the line is part of a block of code between fences. */
  isCode: boolean;
}

/** What may stand before the first sign of a line that is inside a quote or a list. */
const NESTING = String.raw`(?:[ \t]*>)*[ \t]*(?:(?:[-*+]|\d{1,9}[.)])[ \t]+)*`;

/** A fence of a block of code: three or more backticks or tildes. */
const FENCE = new RegExp(`^${NESTING}(\`{3,}|~{3,})(.*)$`);

/**
 * The lines of a text, each with whether it lies between the fences of a
 * block of code, where nothing is a heading or a box. A fence that is never
 * closed runs to the end of the text.
 */
function sourceLines(text: string): SourceLine[] {
  let fence: string | null = null;
  return text.split('\n').map((line, index) => {
    const found = FENCE.exec(line);
    const marks = found?.[1] ?? '';
    const rest = found?.[2] ?? '';
    let isCode = fence !== null;
    if (fence === null) {
      // The words after an opening fence of backticks have no backtick in them.
      if (found && !(marks.startsWith('`') && rest.includes('`'))) {
        fence = marks;
        isCode = true;
      }
    } else if (marks.startsWith(fence) && rest.trim() === '') {
      fence = null;
    }
    return { text: line, number: index + 1, isCode };
  });
}

/** The number of the line a place in the text is on, from 1. */
export function lineOf(text: string, place: number): number {
  let line = 1;
  for (let index = text.indexOf('\n'); index !== -1 && index < place;) {
    line += 1;
    index = text.indexOf('\n', index + 1);
  }
  return line;
}

/** The place in the text where a line begins. A line past the end begins at the end. */
export function startOfLine(text: string, line: number): number {
  let start = 0;
  for (let number = 1; number < line; number += 1) {
    const next = text.indexOf('\n', start);
    if (next === -1) {
      return text.length;
    }
    start = next + 1;
  }
  return start;
}

/* Headings. */

/** A heading of the page. */
export interface Heading {
  /** From 1, the largest, to 6. */
  level: number;
  /** Its words, without the signs of Markdown. */
  title: string;
  /** The line of the text it begins on, from 1. */
  line: number;
}

const HASH_HEADING = /^ {0,3}(#{1,6})(?:[ \t]+(.*))?$/;
const UNDERLINE = /^ {0,3}(=+|-+)[ \t]*$/;
/** A line that begins something other than a paragraph: a quote, a list, a rule, a table, HTML. */
const NOT_A_PARAGRAPH = /^(?: {0,3}(?:>|[-*+](?:[ \t]|$)|\d{1,9}[.)](?:[ \t]|$)|[|<])|[ \t])/;
const RULE = /^ {0,3}([-*_])(?:[ \t]*\1){2,}[ \t]*$/;

/**
 * The headings of a text, in order: a line that begins with one to six
 * hashes, and a paragraph that is underlined with `=` or `-`. Lines in a
 * block of code are not headings. A heading inside a quote or a list is
 * left out: it is no part of the frame of the page.
 */
export function headingsOf(text: string): Heading[] {
  const headings: Heading[] = [];
  // The lines of the paragraph that is being read, if one is.
  let paragraph: SourceLine[] = [];
  // Whether a quote or a list goes on: its lines can run on without a mark.
  let isNested = false;

  for (const line of sourceLines(text)) {
    const hashes = line.isCode ? null : HASH_HEADING.exec(line.text);
    const underline = line.isCode ? null : UNDERLINE.exec(line.text);
    const first = paragraph[0];

    if (hashes) {
      const title = plainWords((hashes[2] ?? '').replace(/(?:^|[ \t]+)#+[ \t]*$/, ''));
      headings.push({ level: hashes[1]?.length ?? 1, title, line: line.number });
      paragraph = [];
      isNested = false;
    } else if (underline && first) {
      const title = plainWords(paragraph.map((each) => each.text.trim()).join(' '));
      headings.push({ level: underline[1]?.startsWith('=') ? 1 : 2, title, line: first.number });
      paragraph = [];
    } else if (line.isCode || line.text.trim() === '' || RULE.test(line.text)) {
      paragraph = [];
      isNested = false;
    } else if (NOT_A_PARAGRAPH.test(line.text)) {
      // An indented line goes on with the paragraph it follows, and begins none.
      const goesOn = paragraph.length > 0 && /^[ \t]/.test(line.text);
      paragraph = goesOn ? [...paragraph, line] : [];
      isNested = isNested || !goesOn;
    } else if (!isNested) {
      paragraph = [...paragraph, line];
    }
  }
  return headings;
}

/** A character no text has, to set a piece of the line aside while the rest is read. */
const ASIDE = String.fromCharCode(0xe000);
const SET_ASIDE = new RegExp(`${ASIDE}(\\d+)${ASIDE}`, 'g');

/** The words of a line of Markdown without its signs: no stars, no brackets, no backticks. */
export function plainWords(inline: string): string {
  // Code and signs that are written out with a backslash are set aside
  // first: what looks like a sign in them is not one.
  const aside: string[] = [];
  const setAside = (piece: string) => {
    aside.push(piece);
    return `${ASIDE}${(aside.length - 1).toString()}${ASIDE}`;
  };
  return inline
    .replace(/(`+)(.+?)\1/g, (_, __, code: string) => setAside(code.trim()))
    .replace(/\\([\\`*_{}[\]()#+\-.!~|>])/g, (_, sign: string) => setAside(sign))
    .replace(/!?\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/!?\[([^\]]*)\]\[[^\]]*\]/g, '$1')
    .replace(/<(https?:\/\/[^>\s]+)>/g, '$1')
    .replace(/(\*\*|__|~~)(\S(?:.*?\S)?)\1/g, '$2')
    .replace(/\*(\S(?:.*?\S)?)\*/g, '$1')
    .replace(/(^|\W)_(\S(?:.*?\S)?)_(?!\w)/g, '$1$2')
    .replace(SET_ASIDE, (_, index: string) => aside[Number(index)] ?? '')
    .replace(/\s+/g, ' ')
    .trim();
}

/** One line of the list of headings at the side of the page. */
export interface ContentsEntry {
  title: string;
  /** The line of the text the heading begins on. */
  line: number;
  /** How far the entry is set in: 0 for the largest headings listed. */
  depth: number;
}

/** The smallest heading the list shows. The ones under it are drawn as labels, not as headings. */
const SMALLEST_LISTED = 3;

/**
 * The list of headings at the side of a page, which a page has when it has
 * more than two headings. Only the three largest sizes are listed, the ones
 * that are set as headings, and none without words. The title of the page,
 * a heading that stands first and is larger than every other, is left out:
 * it is where the page begins.
 */
export function contentsOf(headings: readonly Heading[]): ContentsEntry[] {
  const named = headings.filter(
    (heading) => heading.title !== '' && heading.level <= SMALLEST_LISTED,
  );
  if (named.length <= 2) {
    return [];
  }
  const first = named[0];
  const isTitle =
    first !== undefined && named.every((other) => other === first || other.level > first.level);
  const listed = isTitle ? named.slice(1) : named;
  const top = Math.min(...listed.map((heading) => heading.level));
  return listed.map((heading) => ({
    title: heading.title,
    line: heading.line,
    depth: heading.level - top,
  }));
}

/**
 * The entry a line of the text belongs to: the last one that begins on it
 * or before it. Nothing for a line above the first entry.
 */
export function entryAt(entries: readonly ContentsEntry[], line: number): ContentsEntry | null {
  let found: ContentsEntry | null = null;
  for (const entry of entries) {
    if (entry.line > line) {
      break;
    }
    found = entry;
  }
  return found;
}

/**
 * The entry the reader is at: the last one whose heading has come up to the
 * line where reading begins, a little under the top of what is seen. `tops`
 * is how far each heading is from that line, in the order of the entries:
 * nothing or less means it has been reached. Nothing above the first
 * heading. At the foot of the page the last entry is the one, however far
 * down it stands: a short last part never comes up to the line.
 */
export function entryInView(
  entries: readonly ContentsEntry[],
  tops: readonly (number | null)[],
  isAtFoot: boolean,
): ContentsEntry | null {
  if (isAtFoot) {
    return entries.at(-1) ?? null;
  }
  let found: ContentsEntry | null = null;
  entries.forEach((entry, index) => {
    const top = tops[index];
    if (top !== null && top !== undefined && top <= 0) {
      found = entry;
    }
  });
  return found;
}

/* Boxes and words. */

/** The beginning of an item of a list that has a box, up to the sign inside the box. */
const BOX_AT_PLACE = /^((?:[-*+]|\d{1,9}[.)])\s+\[)([ \txX])\]/;

/**
 * The text with one box ticked, or cleared if it was ticked. `place` is
 * where in the text the item of the box begins, as the Markdown reader
 * says it. A place without a box leaves the text as it was.
 */
export function withBoxTicked(text: string, place: number): string {
  const found = BOX_AT_PLACE.exec(text.slice(place, place + 64));
  const before = found?.[1];
  if (before === undefined) {
    return text;
  }
  const at = place + before.length;
  const isTicked = found?.[2] === 'x' || found?.[2] === 'X';
  return `${text.slice(0, at)}${isTicked ? ' ' : 'x'}${text.slice(at + 1)}`;
}

/** A line that is an item with a box: the box, a space, and something to do. */
const BOX_LINE = new RegExp(
  String.raw`^(?:[ \t]*>)*[ \t]*(?:[-*+]|\d{1,9}[.)])[ \t]+\[([ \txX])\][ \t]+\S`,
);

/** How many boxes a page has, and how many of them are ticked. */
export function boxTally(text: string): { boxes: number; ticked: number } {
  let boxes = 0;
  let ticked = 0;
  for (const line of sourceLines(text)) {
    const sign = line.isCode ? undefined : BOX_LINE.exec(line.text)?.[1];
    if (sign !== undefined) {
      boxes += 1;
      ticked += sign === 'x' || sign === 'X' ? 1 : 0;
    }
  }
  return { boxes, ticked };
}

/** The mark of an item of a list at the beginning of a line, with its number and its box. */
const ITEM_MARK = /^((?:[ \t]*>)*[ \t]*)(?:[-*+]|\d{1,9}[.)])[ \t]+(?:\[[ \txX]\][ \t]+)?/gm;

/**
 * How many words a page has. The words of a link are counted and its
 * address is not; neither is the number of an item or the sign in its box.
 */
export function wordCount(text: string): number {
  const read = text.replace(/\]\([^)\n]*\)/g, ']').replace(ITEM_MARK, '$1');
  return read.match(/[\p{L}\p{N}]+(?:['’-][\p{L}\p{N}]+)*/gu)?.length ?? 0;
}

/** What is counted on a page, in a few words: "412 words", "2 of 5 ticked". */
export function pageFacts(text: string): string[] {
  const words = wordCount(text);
  const { boxes, ticked } = boxTally(text);
  return [
    ...(words > 0 ? [`${words.toString()} ${words === 1 ? 'word' : 'words'}`] : []),
    ...(boxes > 0 ? [`${ticked.toString()} of ${boxes.toString()} ticked`] : []),
  ];
}

/* The keys of tasks. */

/**
 * Finds the keys of the tasks of a project in a text: the key of the
 * project, a hyphen and a number, as a word of its own. `XSSN-4` and
 * `SSN-4a` are no keys of the project `SSN`.
 */
export function taskKeyPattern(projectKey: string): RegExp {
  const key = projectKey.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  return new RegExp(`\\b${key}-\\d+\\b`);
}

/** The tasks of a project by the keys a manager knows them by. */
export function tasksByKey(
  project: Pick<Project, 'key'>,
  tasks: readonly Task[],
): Map<string, Task> {
  return new Map(tasks.map((task) => [taskReference(project, task), task]));
}

/* Writing: what the tools of the bar and the keys do to the text. */

/** What is selected in the text: from a place up to another. A caret is both at once. */
export interface Selection {
  start: number;
  end: number;
}

/**
 * A change to the text: a stretch of it is replaced, and something is
 * selected afterwards. It is small on purpose, so that the field can make
 * it the way typing is made and take it back the same way.
 */
export interface Edit {
  /** Where the replaced stretch begins. */
  from: number;
  /** Where it ends. The same place as `from` when nothing is replaced. */
  to: number;
  /** What takes its place. */
  insert: string;
  /** What is selected afterwards, counted in the changed text. */
  select: Selection;
}

/** The text after a change. */
export function applyEdit(text: string, edit: Edit): string {
  return `${text.slice(0, edit.from)}${edit.insert}${text.slice(edit.to)}`;
}

/**
 * Puts a mark on both sides of what is selected, as `**` does for bold, `_`
 * for italic and a backtick for code, or takes the marks away when they are
 * there. The spaces at the ends of the selection stay outside the marks.
 * With nothing selected the two marks are set down with the caret between
 * them, and taken away again if nothing was typed there.
 */
export function marked(text: string, selection: Selection, mark: string): Edit {
  const chosen = text.slice(selection.start, selection.end);
  const isBlank = chosen.trim() === '';
  const from = selection.start + (isBlank ? 0 : chosen.length - chosen.trimStart().length);
  const to = selection.end - (isBlank ? 0 : chosen.length - chosen.trimEnd().length);
  const inside = text.slice(from, to);
  const size = mark.length;

  // One underscore of two is half of another mark, and so is one backtick.
  const twice = size === 1 ? `${mark}${mark}` : null;
  const hasMarksAround =
    from >= size &&
    text.slice(from - size, from) === mark &&
    text.slice(to, to + size) === mark &&
    !(twice !== null && text.slice(from - 2, from) === twice && text.slice(to, to + 2) === twice);
  if (hasMarksAround) {
    return {
      from: from - size,
      to: to + size,
      insert: inside,
      select: { start: from - size, end: to - size },
    };
  }
  const hasMarksInside =
    inside.length > 2 * size &&
    inside.startsWith(mark) &&
    inside.endsWith(mark) &&
    !(twice !== null && inside.startsWith(twice) && inside.endsWith(twice));
  if (hasMarksInside) {
    const bare = inside.slice(size, inside.length - size);
    return { from, to, insert: bare, select: { start: from, end: from + bare.length } };
  }
  return {
    from,
    to,
    insert: `${mark}${inside}${mark}`,
    select: { start: from + size, end: to + size },
  };
}

/** An address of the web. */
const WEB_ADDRESS = /^https?:\/\/\S+$/i;

/**
 * Makes a link of what is selected. Words become the words of the link,
 * with the caret where its address goes; an address becomes the address,
 * with the caret where the words go. With nothing selected the caret
 * stands where the words go.
 */
export function linked(text: string, selection: Selection): Edit {
  const chosen = text.slice(selection.start, selection.end);
  const words = chosen.trim();
  const from = selection.start + (chosen.length - chosen.trimStart().length);
  const to = words === '' ? selection.end : from + words.length;
  if (words === '' || WEB_ADDRESS.test(words)) {
    const caret = from + 1;
    return { from, to, insert: `[](${words})`, select: { start: caret, end: caret } };
  }
  const caret = from + words.length + 3;
  return { from, to, insert: `[${words}]()`, select: { start: caret, end: caret } };
}

/** Where the line a place is on begins. */
function lineStartAt(text: string, place: number): number {
  return place <= 0 ? 0 : text.lastIndexOf('\n', place - 1) + 1;
}

/** Where the line a place is on ends: at its line break, or at the end of the text. */
function lineEndAt(text: string, place: number): number {
  const end = text.indexOf('\n', place);
  return end === -1 ? text.length : end;
}

/** The lines the selection touches, as one stretch of the text. */
function touchedLines(text: string, selection: Selection): { from: number; to: number } {
  // A selection that ends at the very beginning of a line does not touch that line.
  const reachesInto =
    selection.end > selection.start && text[selection.end - 1] === '\n'
      ? selection.end - 1
      : selection.end;
  return { from: lineStartAt(text, selection.start), to: lineEndAt(text, reachesInto) };
}

/**
 * Sets what is selected as code. Within one line it is marked with
 * backticks. Over several lines the lines are put between two fences, or
 * taken out from between them when they are there.
 */
export function coded(text: string, selection: Selection): Edit {
  if (!text.slice(selection.start, selection.end).includes('\n')) {
    return marked(text, selection, '`');
  }
  const { from, to } = touchedLines(text, selection);
  const block = text.slice(from, to);
  // The lines above and below, where there are any.
  const above = from > 0 ? text.slice(lineStartAt(text, from - 1), from - 1) : null;
  const below = to < text.length ? text.slice(to + 1, lineEndAt(text, to + 1)) : null;
  if (above !== null && below !== null && isFence(above) && isFence(below)) {
    const start = from - above.length - 1;
    return {
      from: start,
      to: to + 1 + below.length,
      insert: block,
      select: { start, end: start + block.length },
    };
  }
  const opening = '```\n';
  return {
    from,
    to,
    insert: `${opening}${block}\n\`\`\``,
    select: { start: from + opening.length, end: from + opening.length + block.length },
  };
}

function isFence(line: string): boolean {
  return /^[ \t]*(?:`{3,}|~{3,})[^`]*$/.test(line);
}

/** What the tools of the bar can make of whole lines. */
export type LineKind = 'heading' | 'list' | 'boxes' | 'quote';

/** The largest heading the tool of the bar steps down to before it lets go. */
const SMALLEST_HEADING_OF_THE_BAR = 3;

const HEADING_MARK = /^( {0,3})#{1,6}(?:[ \t]+|$)/;
const LIST_MARK = /^([ \t]*)(?:[-*+]|\d{1,9}[.)])[ \t]+(\[[ xX]\][ \t]+)?/;
const BULLET = /^[ \t]*[-*+][ \t]+(?!\[[ xX]\][ \t])/;
const BOX_MARK = /^[ \t]*(?:[-*+]|\d{1,9}[.)])[ \t]+\[[ xX]\][ \t]+/;
const QUOTE_MARK = /^( {0,3})>[ \t]?/;

/**
 * Sets the lines the selection touches as a heading, a list, a list of
 * boxes or a quote, or lets them go when they all are one already.
 *
 * A heading steps down with every use: the largest, the second, the third,
 * and then plain text again. A list and a list of boxes take each other's
 * place. Empty lines are passed over, except in a quote, which they would
 * otherwise cut in two.
 */
export function withLinesAs(text: string, selection: Selection, kind: LineKind): Edit {
  const { from, to } = touchedLines(text, selection);
  const lines = text.slice(from, to).split('\n');
  const written = lines.filter((line) => line.trim() !== '');
  const isOneEmptyLine = written.length === 0;
  const changed = lines.map(changeFor(kind, isOneEmptyLine ? lines : written, isOneEmptyLine));
  const insert = changed.join('\n');

  if (selection.start !== selection.end) {
    return { from, to, insert, select: { start: from, end: from + insert.length } };
  }
  // A caret keeps its place among the words of its line.
  const line = lines[0] ?? '';
  const grown = (changed[0] ?? '').length - line.length;
  const beginning = (changed[0] ?? '').length - bodyOf(changed[0] ?? '').length;
  const caret = Math.max(selection.start + grown, from + beginning);
  return { from, to, insert, select: { start: caret, end: caret } };
}

/** A line without the marks that stand before its words. */
function bodyOf(line: string): string {
  return line.replace(QUOTE_MARK, '').replace(HEADING_MARK, '').replace(LIST_MARK, '');
}

/** What one kind of line does to a line, given the lines that count. */
function changeFor(
  kind: LineKind,
  counted: readonly string[],
  takesEmptyLines: boolean,
): (line: string) => string {
  const skip = (line: string) => !takesEmptyLines && line.trim() === '';
  switch (kind) {
    case 'heading': {
      const level = /^ {0,3}(#{1,6})(?:[ \t]|$)/.exec(counted[0] ?? '')?.[1]?.length ?? 0;
      const next = level >= SMALLEST_HEADING_OF_THE_BAR ? 0 : level + 1;
      if (next === 0) {
        // Letting go takes the hashes away and nothing else.
        return (line) => line.replace(HEADING_MARK, '$1');
      }
      // A heading is no item of a list and is not set in.
      const hashes = `${'#'.repeat(next)} `;
      return (line) =>
        skip(line)
          ? line
          : `${hashes}${withListMark(line.replace(HEADING_MARK, ''), '').trimStart()}`;
    }
    case 'list': {
      const areAll = counted.every((line) => BULLET.test(line));
      return (line) =>
        skip(line) ? line : withListMark(line.replace(HEADING_MARK, '$1'), areAll ? '' : '- ');
    }
    case 'boxes': {
      const areAll = counted.every((line) => BOX_MARK.test(line));
      return (line) => {
        if (skip(line) || (!areAll && BOX_MARK.test(line))) {
          return line;
        }
        return withListMark(line.replace(HEADING_MARK, '$1'), areAll ? '' : '- [ ] ');
      };
    }
    case 'quote': {
      const areAll = counted.every((line) => QUOTE_MARK.test(line));
      return (line) => {
        if (areAll) {
          return line.replace(QUOTE_MARK, '$1');
        }
        return line.trim() === '' && !takesEmptyLines ? '>' : `> ${line}`;
      };
    }
  }
}

/** A line with the mark of a list put in the place of the one it has, if it has one. */
function withListMark(line: string, mark: string): string {
  if (LIST_MARK.test(line)) {
    return line.replace(LIST_MARK, `$1${mark}`);
  }
  const indent = /^[ \t]*/.exec(line)?.[0] ?? '';
  return `${indent}${mark}${line.slice(indent.length)}`;
}

/** The beginning of an item of a list: what it is set in by, its mark, and its box if it has one. */
const ITEM_START =
  /^((?:[ \t]{0,3}>[ \t]?)*[ \t]*)(?:([-*+])|(\d{1,9})([.)]))([ \t]+)(\[[ xX]\][ \t]+)?/;
const QUOTE_START = /^((?:[ \t]{0,3}>[ \t]?)+)/;

/**
 * What Enter does at the end of an item of a list or a line of a quote: the
 * next line begins the same way, with the next number in a numbered list
 * and an empty box in a list of boxes. On an item that has no words yet,
 * Enter takes its mark away instead, which ends the list. Nothing when
 * Enter is to do what it always does.
 */
export function continued(text: string, selection: Selection): Edit | null {
  if (selection.start !== selection.end) {
    return null;
  }
  const caret = selection.start;
  const from = lineStartAt(text, caret);
  const line = text.slice(from, lineEndAt(text, caret));
  const item = ITEM_START.exec(line);
  const start = item?.[0] ?? QUOTE_START.exec(line)?.[0];
  if (start === undefined || caret - from < start.length) {
    return null;
  }
  if (line.trim() === start.trim()) {
    return { from, to: from + line.length, insert: '', select: { start: from, end: from } };
  }
  const next = item ? nextItemStart(item) : start;
  const place = caret + 1 + next.length;
  return { from: caret, to: caret, insert: `\n${next}`, select: { start: place, end: place } };
}

/** How the item after one begins. */
function nextItemStart(item: RegExpExecArray): string {
  const [, indent = '', bullet, number, stop = '', space = ' ', box] = item;
  const mark = bullet ?? `${(Number(number) + 1).toString()}${stop}`;
  return `${indent}${mark}${space}${box === undefined ? '' : '[ ] '}`;
}

/* Keeping in step with what is stored. */

/** What a page does when the stored text is not the text it shows. */
export type Step = 'keep' | 'adopt' | 'reload';

/**
 * What to do with a stored text that is handed to a page.
 *
 * A page that is being written, or that holds something not yet kept by
 * the backend, keeps its own text: nothing the manager typed is ever given
 * up for what the list of views says. A page at rest takes the stored text,
 * which another computer may have written. But a stored text this page
 * wrote itself earlier is the list being behind, not news: the page keeps
 * its text and asks for the list again.
 */
export function stepFor(
  stored: string,
  page: { text: string; isBusy: boolean; known: readonly string[] },
): Step {
  if (stored === page.text || page.isBusy) {
    return 'keep';
  }
  return page.known.includes(stored) ? 'reload' : 'adopt';
}
