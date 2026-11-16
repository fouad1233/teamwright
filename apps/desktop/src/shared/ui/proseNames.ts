/**
 * Names in a Markdown text that the program knows, such as the key of a
 * task: finding them in the tree the Markdown reader builds, so that each
 * can be drawn as something of its own.
 */

/**
 * A node of the tree of a Markdown text, as far as it is read and changed
 * here: running text has a value, what holds other nodes has children, and
 * a node may say how it is to be drawn.
 */
export interface TextNode {
  type: string;
  value?: string;
  children?: TextNode[];
  data?: Record<string, unknown>;
}

/** The attribute a name carries once the text is drawn, with the name as its value. */
export const NAME_ATTRIBUTE = 'data-name';

/** What holds text that is not to be searched: a link already leads somewhere. */
const LEFT_ALONE = ['link', 'linkReference'];

/**
 * Marks every name a pattern finds in the running text of a tree. A name
 * becomes a piece of text of its own that is drawn as a `span` with the
 * name in `data-name`. Code is never searched, because it is not running
 * text, and neither is the text of a link.
 *
 * It is a plugin for the Markdown reader: `[markNames, pattern]`.
 */
export function markNames(pattern: RegExp) {
  const everywhere = new RegExp(
    pattern.source,
    pattern.flags.includes('g') ? pattern.flags : `${pattern.flags}g`,
  );
  return (tree: TextNode): void => {
    markIn(tree, everywhere);
  };
}

function markIn(parent: TextNode, pattern: RegExp): void {
  if (!parent.children) {
    return;
  }
  parent.children = parent.children.flatMap((child) => {
    if (child.type === 'text' && child.value !== undefined && child.data === undefined) {
      return pieces(child, child.value, pattern);
    }
    if (!LEFT_ALONE.includes(child.type)) {
      markIn(child, pattern);
    }
    return [child];
  });
}

/** A piece of running text cut at its names. Text without one stays the node it was. */
function pieces(node: TextNode, text: string, pattern: RegExp): TextNode[] {
  const cut: TextNode[] = [];
  let end = 0;
  for (const found of text.matchAll(pattern)) {
    const name = found[0];
    if (name === '') {
      continue;
    }
    if (found.index > end) {
      cut.push({ type: 'text', value: text.slice(end, found.index) });
    }
    cut.push({
      type: 'text',
      value: name,
      data: { hName: 'span', hProperties: { dataName: name } },
    });
    end = found.index + name.length;
  }
  if (cut.length === 0) {
    return [node];
  }
  if (end < text.length) {
    cut.push({ type: 'text', value: text.slice(end) });
  }
  return cut;
}
