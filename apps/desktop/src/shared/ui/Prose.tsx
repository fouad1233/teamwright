import {
  createContext,
  createElement,
  memo,
  useContext,
  useMemo,
  type ComponentProps,
  type ReactNode,
} from 'react';
import Markdown, { type Components, type ExtraProps, type Options } from 'react-markdown';
import remarkGfm from 'remark-gfm';

import { classNames } from '@/shared/lib/classNames';

import styles from './Prose.module.css';
import { NAME_ATTRIBUTE, markNames } from './proseNames';

/** Names in the running text that the program knows, such as the key of a task. */
export interface KnownNames {
  /** Finds a name. It is tried on running text only: never on code, never inside a link. */
  pattern: RegExp;
  /** What stands in the place of a name. Nothing leaves the name as it was written. */
  draw: (name: string) => ReactNode;
}

interface ProseProps {
  /** The text, written the way a specialist writes: in Markdown. */
  text: string;
  /**
   * The look of the text, for a place that sets it in its own way, such as a
   * page. Left out, the text is set as a note in the margin of a drawing.
   */
  className?: string;
  /**
   * Lets the boxes of a list of things to do be pressed. It is called with
   * the place in the text where the item of the pressed box begins.
   */
  onTick?: (place: number) => void;
  /** Names to draw as something of their own. */
  names?: KnownNames;
  /**
   * Lets the links of the text be pressed, for text the manager wrote. It is
   * called with the address, and decides what following it means. Only an
   * address of the web can be followed.
   */
  onFollow?: (address: string) => void;
}

/**
 * Text that somebody else wrote, shown the way it was meant: with its
 * paragraphs, lists, emphasis, code and tables.
 *
 * The text comes from a model, so nothing in it is trusted. It cannot bring
 * HTML of its own, a link in it cannot be followed, and a picture in it is
 * not fetched: following either would take the window, or a request, to a
 * place the manager did not choose.
 *
 * A place that shows text the manager wrote can ask for more: boxes that
 * can be ticked, names drawn as links into the program, and links that can
 * be pressed. Every heading says on which line of the text it stands
 * (`data-line`), so that a page can be jumped through by its headings.
 */
export const Prose = memo(function Prose({ text, className, onTick, names, onFollow }: ProseProps) {
  const draw = names?.draw;
  const source = names?.pattern.source;
  const flags = names?.pattern.flags;
  const plugins = useMemo(
    (): Plugins =>
      source === undefined ? PLUGINS : [...PLUGINS, [markNames, new RegExp(source, flags)]],
    [source, flags],
  );
  const abilities = useMemo(() => ({ onTick, draw, onFollow }), [onTick, draw, onFollow]);

  return (
    <div className={className ?? styles.prose} data-selectable>
      <AbilitiesContext value={abilities}>
        <Markdown remarkPlugins={plugins} components={COMPONENTS}>
          {text}
        </Markdown>
      </AbilitiesContext>
    </div>
  );
});

/** What the Markdown reader is extended with. */
type Plugins = NonNullable<Options['remarkPlugins']>;

/** Tables, struck-through text and lists of things to do, as most models write them. */
const PLUGINS: Plugins = [remarkGfm];

/** What the place that shows the text lets its reader do with it. */
interface Abilities {
  onTick?: (place: number) => void;
  draw?: (name: string) => ReactNode;
  onFollow?: (address: string) => void;
}

const AbilitiesContext = createContext<Abilities>({});

/** Where in the text the item of a list begins, for the box that stands in it. */
const ItemPlaceContext = createContext<number | undefined>(undefined);

/** An address that leads to the web. Nothing else is followed. */
const WEB_ADDRESS = /^https?:\/\//i;

/**
 * A link. It is shown as its words and, where that says more, its address.
 * Where links can be followed and it leads to the web, it is pressed
 * instead. The window never goes there by itself: the place that shows the
 * text is told the address and nothing else happens.
 */
function Link({ href, children }: ComponentProps<'a'>) {
  const { onFollow } = useContext(AbilitiesContext);
  if (onFollow !== undefined && href !== undefined && WEB_ADDRESS.test(href)) {
    return (
      <a
        className={styles.link}
        href={href}
        title={href}
        draggable={false}
        onClick={(event) => {
          event.preventDefault();
          onFollow(href);
        }}
        onAuxClick={(event) => {
          event.preventDefault();
        }}
      >
        {children}
      </a>
    );
  }
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
}

/** An item of a list. It tells the box in it where in the text it begins. */
function Item({ node, children, ...rest }: ComponentProps<'li'> & ExtraProps) {
  return (
    <ItemPlaceContext value={node?.position?.start.offset}>
      <li {...rest}>{children}</li>
    </ItemPlaceContext>
  );
}

/**
 * The box of a thing to do, drawn in the hand of the interface instead of
 * the browser's. It can be pressed where the place that shows the text
 * lets its boxes be ticked.
 */
function Box({ checked = false }: ComponentProps<'input'>) {
  const { onTick } = useContext(AbilitiesContext);
  const place = useContext(ItemPlaceContext);
  if (onTick === undefined || place === undefined) {
    return (
      <span
        className={styles.box}
        data-checked={checked ? '' : undefined}
        role="img"
        aria-label={checked ? 'Done' : 'To do'}
      />
    );
  }
  return (
    <button
      type="button"
      className={classNames(styles.box, styles.tickable)}
      data-checked={checked ? '' : undefined}
      role="checkbox"
      aria-checked={checked}
      aria-label="Done"
      onClick={() => {
        onTick(place);
      }}
    />
  );
}

/** A name the text was searched for, drawn by the place that shows the text. */
function Name({ node, children, ...rest }: ComponentProps<'span'> & ExtraProps) {
  const { draw } = useContext(AbilitiesContext);
  const name = (rest as Record<string, unknown>)[NAME_ATTRIBUTE];
  if (node === undefined || typeof name !== 'string') {
    return <span {...rest}>{children}</span>;
  }
  return draw?.(name) ?? name;
}

/** A heading, which says on which line of the text it stands. */
function Heading({ node, children, ...rest }: ComponentProps<'h1'> & ExtraProps) {
  return createElement(
    node?.tagName ?? 'h1',
    { ...rest, 'data-line': node?.position?.start.line },
    children,
  );
}

const COMPONENTS: Components = {
  a: Link,
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
  li: Item,
  input: Box,
  span: Name,
  h1: Heading,
  h2: Heading,
  h3: Heading,
  h4: Heading,
  h5: Heading,
  h6: Heading,
};
