import { memo } from 'react';
import Markdown, { type Components } from 'react-markdown';
import remarkGfm from 'remark-gfm';

import styles from './Prose.module.css';

interface ProseProps {
  /** The text, written the way a specialist writes: in Markdown. */
  text: string;
}

/**
 * Text that somebody else wrote, shown the way it was meant: with its
 * paragraphs, lists, emphasis, code and tables.
 *
 * The text comes from a model, so nothing in it is trusted. It cannot bring
 * HTML of its own, a link in it cannot be followed, and a picture in it is
 * not fetched: following either would take the window, or a request, to a
 * place the manager did not choose.
 */
export const Prose = memo(function Prose({ text }: ProseProps) {
  return (
    <div className={styles.prose} data-selectable>
      <Markdown remarkPlugins={PLUGINS} components={COMPONENTS}>
        {text}
      </Markdown>
    </div>
  );
});

/** Tables, struck-through text and lists of things to do, as most models write them. */
const PLUGINS = [remarkGfm];

const COMPONENTS: Components = {
  // A link is shown as its words and, where that says more, its address.
  a({ href, children }) {
    const words = typeof children === 'string' ? children : undefined;
    return (
      <span>
        {children}
        {href !== undefined && href !== words && (
          <>
            {' '}
            <span className={styles.address}>({href})</span>
          </>
        )}
      </span>
    );
  },
  img({ alt }) {
    return <span className={styles.left}>[picture{alt ? `: ${alt}` : ''}]</span>;
  },
  // A table can be wider than the panel it is read in.
  table({ children }) {
    return (
      <div className={styles.wide}>
        <table>{children}</table>
      </div>
    );
  },
  // The box of a thing to do, drawn in the hand of the interface instead of
  // the browser's.
  input({ checked }) {
    return (
      <span
        className={styles.box}
        data-checked={checked ? '' : undefined}
        role="img"
        aria-label={checked ? 'Done' : 'To do'}
      />
    );
  },
};
