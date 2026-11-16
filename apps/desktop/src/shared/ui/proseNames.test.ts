import { describe, expect, it } from 'vitest';

import { markNames, type TextNode } from './proseNames';

const KEYS = /\bSSN-\d+\b/;

function text(value: string): TextNode {
  return { type: 'text', value };
}

function name(value: string): TextNode {
  return { type: 'text', value, data: { hName: 'span', hProperties: { dataName: value } } };
}

/** The tree of one paragraph, marked. */
function marked(...children: TextNode[]): TextNode[] {
  const tree: TextNode = { type: 'root', children: [{ type: 'paragraph', children }] };
  markNames(KEYS)(tree);
  return tree.children?.[0]?.children ?? [];
}

describe('markNames', () => {
  it('cuts running text at every name and keeps what lies between', () => {
    expect(marked(text('See SSN-4 and SSN-12.'))).toEqual([
      text('See '),
      name('SSN-4'),
      text(' and '),
      name('SSN-12'),
      text('.'),
    ]);
  });

  it('marks a name that is the whole text, or stands at one of its ends', () => {
    expect(marked(text('SSN-4'))).toEqual([name('SSN-4')]);
    expect(marked(text('SSN-4 waits'))).toEqual([name('SSN-4'), text(' waits')]);
    expect(marked(text('after SSN-4'))).toEqual([text('after '), name('SSN-4')]);
  });

  it('leaves text without a name as the node it was', () => {
    const plain = text('Nothing to find here, not even XSSN-4 or SSN-4a.');
    expect(marked(plain)[0]).toBe(plain);
  });

  it('finds names inside emphasis, a list and a quote', () => {
    const tree: TextNode = {
      type: 'root',
      children: [
        {
          type: 'blockquote',
          children: [
            {
              type: 'list',
              children: [
                {
                  type: 'listItem',
                  children: [
                    {
                      type: 'paragraph',
                      children: [{ type: 'strong', children: [text('SSN-7 first')] }],
                    },
                  ],
                },
              ],
            },
          ],
        },
      ],
    };
    markNames(KEYS)(tree);
    const strong = tree.children?.[0]?.children?.[0]?.children?.[0]?.children?.[0]?.children?.[0];
    expect(strong?.children).toEqual([name('SSN-7'), text(' first')]);
  });

  it('never searches code or the text of a link', () => {
    const code: TextNode = { type: 'inlineCode', value: 'SSN-4' };
    const block: TextNode = { type: 'code', value: 'SSN-4' };
    const link: TextNode = { type: 'link', children: [text('SSN-4')] };
    expect(marked(code, block, link)).toEqual([
      { type: 'inlineCode', value: 'SSN-4' },
      { type: 'code', value: 'SSN-4' },
      { type: 'link', children: [text('SSN-4')] },
    ]);
  });

  it('finds every name whether or not the pattern is global, and marks none twice', () => {
    const tree: TextNode = { type: 'root', children: [text('SSN-1 SSN-2')] };
    const mark = markNames(/\bSSN-\d+\b/g);
    mark(tree);
    mark(tree);
    expect(tree.children).toEqual([name('SSN-1'), text(' '), name('SSN-2')]);
  });
});
