import { useQueryClient } from '@tanstack/react-query';
import { Check, Code, Italic, Link, List, ListChecks, PenLine, TextQuote } from 'lucide-react';
import {
  Fragment,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
  type CSSProperties,
  type KeyboardEvent,
  type ReactNode,
} from 'react';

import { useNavigation } from '@/app/navigation';
import type { Project, Task, View } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { taskReference } from '@/shared/lib/format';
import { Button, IconButton } from '@/shared/ui/Button';
import { usePrefersReducedMotion } from '@/shared/ui/motion';
import { Prose, type KnownNames } from '@/shared/ui/Prose';
import { useNotifier, type Notifier } from '@/shared/ui/Toast';
import { Tooltip } from '@/shared/ui/Tooltip';

import { StatusMark } from '../tasks/TaskBadges';
import { statusOf } from '../tasks/taskModel';
import type { KindProps } from '../views/viewKinds';
import { useSetViewContent, viewsKey } from '../views/viewQueries';
import styles from './DocView.module.css';
import {
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
  startOfLine,
  taskKeyPattern,
  tasksByKey,
  withBoxTicked,
  withLinesAs,
  type ContentsEntry,
  type Edit,
  type Selection,
} from './docModel';
import { PageKeeper, savedInWords, type Saved } from './pageKeeper';

/**
 * How far under the top of what is seen reading begins, in pixels: a
 * heading that has come up to this line is the one the reader is at, and a
 * heading that is jumped to comes to rest just above it.
 */
const READING_LINE = 96;

/** How much of the page stays above a place that is jumped to in the field, in pixels. */
const ROOM_ABOVE = 48;

/** The code of a space that has no width. */
const ZERO_WIDTH_SPACE = 0x200b;

/**
 * A page of writing kept with the project. It is read as a set page, with
 * the list of its headings at the side, and written as its plain text in
 * Markdown. A box in it can be ticked while reading, and the key of a task
 * leads to that task.
 */
export function DocView({ view, project, everyTask }: KindProps) {
  const navigation = useNavigation();
  const notifier = useNotifier();
  const isMotionReduced = usePrefersReducedMotion();
  const page = usePage(view);
  const { text, isWriting, saved } = useSyncExternalStore(page.subscribe, page.snapshot);

  const scroller = useRef<HTMLDivElement>(null);
  const field = useRef<HTMLTextAreaElement>(null);
  const ruler = useRef<HTMLDivElement>(null);
  const turn = useRef<HTMLButtonElement>(null);
  // Where the page was when it was turned from reading to writing or back,
  // so that the other face opens at the same part of it.
  const turnedAt = useRef<Place | null>(null);
  const frame = useRef<number | null>(null);

  const contents = useMemo(() => contentsOf(headingsOf(text)), [text]);
  // The line of the entry the reader, or the caret, is at.
  const [here, setHere] = useState<number | null>(null);

  const byKey = useMemo(() => tasksByKey(project, everyTask), [project, everyTask]);
  const { taskId: openTaskId, openTask } = navigation;
  const names = useMemo<KnownNames>(
    () => ({
      pattern: taskKeyPattern(project.key),
      draw: (key) => {
        const task = byKey.get(key);
        return (
          task && (
            <TaskKey
              project={project}
              task={task}
              isOpen={task.id === openTaskId}
              onOpen={() => {
                openTask(task.id);
              }}
            />
          )
        );
      },
    }),
    [project, byKey, openTaskId, openTask],
  );

  const tick = useCallback(
    (place: number) => {
      page.change(withBoxTicked(page.snapshot().text, place));
    },
    [page],
  );

  const follow = useCallback(
    (address: string) => {
      followLink(address, notifier);
    },
    [notifier],
  );

  /** Finds the entry the reader is at, from where the headings stand. */
  const findHere = useCallback(() => {
    const box = scroller.current;
    if (!box) {
      return;
    }
    const line = box.getBoundingClientRect().top + READING_LINE;
    const tops = contents.map((entry) => {
      const heading = headingOn(box, entry.line);
      return heading ? heading.getBoundingClientRect().top - line : null;
    });
    const isAtFoot = box.scrollTop > 0 && box.scrollTop + box.clientHeight >= box.scrollHeight - 2;
    setHere(entryInView(contents, tops, isAtFoot)?.line ?? null);
  }, [contents]);

  /** Looks where the reader is on the next frame, once for all the scrolling of that frame. */
  const lookSoon = useCallback(() => {
    frame.current ??= requestAnimationFrame(() => {
      frame.current = null;
      findHere();
    });
  }, [findHere]);

  // The page that is read was drawn again: its headings may stand elsewhere.
  useEffect(() => {
    if (!isWriting) {
      lookSoon();
    }
    return () => {
      if (frame.current !== null) {
        cancelAnimationFrame(frame.current);
        frame.current = null;
      }
    };
  }, [isWriting, lookSoon]);

  // The other face of the page opens at the part the first one was at.
  useLayoutEffect(() => {
    const place = turnedAt.current;
    const box = scroller.current;
    turnedAt.current = null;
    if (!place || !box) {
      return;
    }
    const area = field.current;
    if (isWriting && area) {
      const start = place.line === null ? 0 : startOfLine(area.value, place.line);
      area.focus({ preventScroll: true });
      area.setSelectionRange(start, start);
      box.scrollTop =
        place.line === null
          ? place.share * (box.scrollHeight - box.clientHeight)
          : topInField(box, area, ruler.current, start) - ROOM_ABOVE;
    } else {
      const heading = place.line === null ? null : headingOn(box, place.line);
      if (heading) {
        heading.scrollIntoView({ block: 'start' });
      } else {
        box.scrollTop = place.share * (box.scrollHeight - box.clientHeight);
      }
    }
  }, [isWriting]);

  /** Where the page is, as a share of its length. */
  function shareScrolled(): number {
    const box = scroller.current;
    const room = box ? box.scrollHeight - box.clientHeight : 0;
    return box && room > 0 ? box.scrollTop / room : 0;
  }

  function write() {
    turnedAt.current = { line: here, share: shareScrolled() };
    page.beginWriting();
  }

  function finish() {
    const area = field.current;
    const line = area ? lineOf(area.value, area.selectionStart) : 1;
    turnedAt.current = { line: entryAt(contents, line)?.line ?? null, share: shareScrolled() };
    // The focus goes to where the page is turned, before the field is gone.
    turn.current?.focus();
    page.endWriting();
  }

  /** Goes to a heading: the page scrolls to it, or the caret is put on its line. */
  function jump(line: number) {
    const box = scroller.current;
    const area = field.current;
    const behavior = isMotionReduced ? 'auto' : 'smooth';
    if (!box) {
      return;
    }
    if (isWriting && area) {
      const start = startOfLine(area.value, line);
      area.focus({ preventScroll: true });
      area.setSelectionRange(start, start);
      box.scrollTo({ top: topInField(box, area, ruler.current, start) - ROOM_ABOVE, behavior });
      setHere(line);
    } else {
      headingOn(box, line)?.scrollIntoView({ block: 'start', behavior });
    }
  }

  /** The caret moved: the entry it is under is the one the manager writes at. */
  function followCaret() {
    const area = field.current;
    if (area) {
      setHere(entryAt(contents, lineOf(area.value, area.selectionStart))?.line ?? null);
    }
  }

  /**
   * Changes the text of the field with a tool or a key. Says whether there
   * was a change to make.
   */
  function perform(change: (text: string, selection: Selection) => Edit | null): boolean {
    const area = field.current;
    const edit = area && change(area.value, { start: area.selectionStart, end: area.selectionEnd });
    if (!area || !edit) {
      return false;
    }
    makeEdit(area, edit);
    followCaret();
    return true;
  }

  /** The keys of the field: Escape ends the writing, Enter goes on with a list, Ctrl and a letter is a tool. */
  function handleFieldKey(event: KeyboardEvent<HTMLTextAreaElement>) {
    // Keys that put a letter together belong to the letter.
    if (event.nativeEvent.isComposing) {
      return;
    }
    if (event.key === 'Escape') {
      event.preventDefault();
      // Escape ends the writing. It must not also close the panel of a task.
      event.stopPropagation();
      finish();
    } else if (event.key === 'Enter' && !hasModifier(event)) {
      if (perform(continued)) {
        event.preventDefault();
      }
    } else if (isCommand(event) && !event.shiftKey) {
      const tool = TOOLS.find((each) => each.key === event.key.toLowerCase());
      if (tool) {
        event.preventDefault();
        perform(tool.change);
      }
    }
  }

  /** Ctrl and S, anywhere in the view, saves at once: the page saves itself, and a habit is a habit. */
  function handleKey(event: KeyboardEvent) {
    if (isCommand(event) && event.key.toLowerCase() === 's') {
      event.preventDefault();
      page.saveNow();
    }
  }

  const isEmpty = text.trim() === '';
  const facts = isWriting ? [] : pageFacts(text);

  return (
    <div className={styles.doc} onKeyDown={handleKey}>
      <div className={styles.bar}>
        {isWriting ? (
          <div
            className={styles.tools}
            role="toolbar"
            aria-label="Tools to write with"
            // A press on a tool leaves the caret in the field.
            onMouseDown={(event) => {
              event.preventDefault();
            }}
          >
            {TOOLS.map((tool) => (
              <Fragment key={tool.label}>
                <IconButton
                  label={
                    tool.key ? `${tool.label} (Ctrl and ${tool.key.toUpperCase()})` : tool.label
                  }
                  onClick={() => {
                    perform(tool.change);
                  }}
                >
                  {tool.face}
                </IconButton>
                {tool.endsGroup && <span className={styles.rule} aria-hidden />}
              </Fragment>
            ))}
          </div>
        ) : (
          facts.length > 0 && <p className={styles.facts}>{facts.join(' · ')}</p>
        )}
        <span className={styles.space} />
        <SavedNote saved={saved} />
        {/* One button in one place turns the page: it keeps the focus through the turn. */}
        <Button
          ref={turn}
          icon={isWriting ? <Check /> : <PenLine />}
          onClick={isWriting ? finish : write}
        >
          {isWriting ? 'Done' : 'Write'}
        </Button>
      </div>

      <div ref={scroller} className={styles.scroller} onScroll={isWriting ? undefined : lookSoon}>
        <div className={styles.sheet}>
          <div className={styles.column}>
            {isWriting ? (
              <div className={styles.desk}>
                <textarea
                  ref={field}
                  className={styles.source}
                  value={text}
                  aria-label="The text of the page"
                  placeholder={
                    'Write here. Markdown is understood: # begins a heading, - an item of a list, - [ ] a box to tick.'
                  }
                  onChange={(event) => {
                    page.change(event.target.value);
                  }}
                  onKeyDown={handleFieldKey}
                  onSelect={followCaret}
                />
                {/* The same text set the same way, to find how far down the field a line stands. */}
                <div ref={ruler} className={classNames(styles.source, styles.ruler)} aria-hidden />
              </div>
            ) : isEmpty ? (
              <div className={classNames(styles.empty, 'pattern-band')}>
                <p className={styles.emptyLead}>A page of writing kept with the project.</p>
                <p className={styles.emptyNote}>
                  What was agreed, a plan, notes for the team: press Write and begin. A box in the
                  text can be ticked while reading, and the key of a task, such as{' '}
                  <span className={styles.example}>{exampleKey(project, everyTask)}</span>, becomes
                  a link to that task.
                </p>
              </div>
            ) : (
              <Prose
                className={styles.page}
                text={text}
                onTick={tick}
                names={names}
                onFollow={follow}
              />
            )}
          </div>

          {contents.length > 0 && <Contents entries={contents} here={here} onJump={jump} />}
        </div>
      </div>
    </div>
  );
}

/** Where a page is: the line of the heading it is at, if any, and how far it is scrolled. */
interface Place {
  line: number | null;
  /** How far the page is scrolled, as a share of its length. */
  share: number;
}

/**
 * The page of the view while the view is open, kept in step with the view:
 * it is handed the content whenever the list of views is loaded again, and
 * writes at once when the view is left, hidden or closed.
 */
function usePage(view: View): PageKeeper {
  const setContent = useSetViewContent();
  const queryClient = useQueryClient();
  const [page] = useState(() => {
    const key = viewsKey(view.projectId);
    return new PageKeeper(view.content, {
      write: (content) => setContent(view, content),
      stored: () =>
        queryClient.getQueryData<View[]>(key)?.find((listed) => listed.id === view.id)?.content,
      reload: () => {
        void queryClient.invalidateQueries({ queryKey: key });
      },
    });
  });

  useEffect(() => {
    page.take(view.content);
  }, [page, view.content]);

  useEffect(() => {
    const save = () => {
      page.saveNow();
    };
    const saveWhenHidden = () => {
      if (document.visibilityState === 'hidden') {
        save();
      }
    };
    document.addEventListener('visibilitychange', saveWhenHidden);
    window.addEventListener('pagehide', save);
    return () => {
      document.removeEventListener('visibilitychange', saveWhenHidden);
      window.removeEventListener('pagehide', save);
      page.leave();
    };
  }, [page]);

  return page;
}

/** One tool of the bar: what it is called, what it shows, its key, and what it does to the text. */
interface Tool {
  label: string;
  face: ReactNode;
  /** The letter that does the same when pressed with Ctrl. */
  key?: string;
  /** Whether a hairline follows it, where one group of tools ends. */
  endsGroup?: boolean;
  change: (text: string, selection: Selection) => Edit;
}

/**
 * A letter as the mark of a tool, drawn with the stroke of the other marks
 * of the bar. The set of marks the interface uses has none for a heading
 * and none for bold.
 */
function letter(strokes: string): ReactNode {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      <path d={strokes} />
    </svg>
  );
}

const TOOLS: readonly Tool[] = [
  {
    label: 'Heading, smaller with every press',
    face: letter('M6 4v16M18 4v16M6 12h12'),
    change: (text, selection) => withLinesAs(text, selection, 'heading'),
  },
  {
    label: 'Bold',
    face: letter('M7 4h6a4 4 0 0 1 0 8H7zM7 12h7a4 4 0 0 1 0 8H7z'),
    key: 'b',
    change: (text, selection) => marked(text, selection, '**'),
  },
  {
    label: 'Italic',
    face: <Italic />,
    key: 'i',
    endsGroup: true,
    change: (text, selection) => marked(text, selection, '_'),
  },
  {
    label: 'List',
    face: <List />,
    change: (text, selection) => withLinesAs(text, selection, 'list'),
  },
  {
    label: 'Boxes to tick',
    face: <ListChecks />,
    endsGroup: true,
    change: (text, selection) => withLinesAs(text, selection, 'boxes'),
  },
  { label: 'Link', face: <Link />, key: 'k', change: linked },
  { label: 'Code', face: <Code />, key: 'e', change: coded },
  {
    label: 'Quote',
    face: <TextQuote />,
    change: (text, selection) => withLinesAs(text, selection, 'quote'),
  },
];

/** When the page was last saved, said quietly at the end of the bar. */
function SavedNote({ saved }: { saved: Saved }) {
  const words = savedInWords(saved);
  if (words === null) {
    return null;
  }
  return saved.kind === 'refused' ? (
    <span
      className={classNames(styles.saved, styles.refused)}
      title="The page could not be saved. It is still here: press Done, or Ctrl and S, to try again."
    >
      {words}
    </span>
  ) : (
    <span
      className={styles.saved}
      title="The page is saved by itself, a moment after every change."
    >
      {words}
    </span>
  );
}

interface ContentsProps {
  entries: readonly ContentsEntry[];
  /** The line of the entry the reader or the caret is at, if it is at one. */
  here: number | null;
  onJump: (line: number) => void;
}

/** The headings of the page in a narrow list at its side. A press goes to the heading. */
function Contents({ entries, here, onJump }: ContentsProps) {
  return (
    <nav className={styles.contents} aria-label="Contents">
      <h2 className="label">Contents</h2>
      <ul className={styles.entries}>
        {entries.map((entry) => (
          <li key={entry.line}>
            <button
              type="button"
              className={styles.entry}
              style={{ '--depth': entry.depth } as CSSProperties}
              aria-current={entry.line === here ? 'location' : undefined}
              onClick={() => {
                onJump(entry.line);
              }}
            >
              {entry.title}
            </button>
          </li>
        ))}
      </ul>
    </nav>
  );
}

interface TaskKeyProps {
  project: Project;
  task: Task;
  /** Whether the task is the one the panel at the side shows. */
  isOpen: boolean;
  onOpen: () => void;
}

/**
 * The key of a task in the running text: the mark of its status and the
 * key, which opens the task. Its title shows when it is pointed at.
 */
function TaskKey({ project, task, isOpen, onOpen }: TaskKeyProps) {
  const status = statusOf(project, task);
  const reference = taskReference(project, task);
  return (
    <Tooltip label={status ? `${task.title} · ${status.name}` : task.title} side="top">
      <button
        type="button"
        className={classNames(styles.taskKey, isOpen && styles.isOpen)}
        data-progress={status?.category}
        aria-label={`${reference} ${task.title}`}
        onClick={onOpen}
      >
        {status && <StatusMark status={status} />}
        {reference}
      </button>
    </Tooltip>
  );
}

/** A key to show what a key looks like: that of the first task, or the one it would have. */
function exampleKey(project: Project, tasks: readonly Task[]): string {
  const first = Math.min(...tasks.map((task) => task.number), Number.MAX_SAFE_INTEGER);
  return taskReference(project, { number: tasks.length > 0 ? first : 1 });
}

/** Whether a key is pressed with Ctrl, or with the command key of a Mac. AltGr is neither. */
function isCommand(event: KeyboardEvent): boolean {
  return (event.ctrlKey || event.metaKey) && !event.altKey;
}

/** Whether a key is pressed together with any of the keys that change what it means. */
function hasModifier(event: KeyboardEvent): boolean {
  return event.ctrlKey || event.metaKey || event.altKey || event.shiftKey;
}

/**
 * Makes a change to the text of a field the way typing makes it, so that
 * the keys that take typing back take it back too.
 */
function makeEdit(area: HTMLTextAreaElement, edit: Edit): void {
  area.focus({ preventScroll: true });
  area.setSelectionRange(edit.from, edit.to);
  const isMade =
    edit.insert === ''
      ? edit.from === edit.to || document.execCommand('delete')
      : document.execCommand('insertText', false, edit.insert);
  if (!isMade) {
    // A web view that does not take the command of an editor: the text is
    // set, and the field is told, so that the change is still kept.
    area.setRangeText(edit.insert, edit.from, edit.to, 'end');
    area.dispatchEvent(new Event('input', { bubbles: true }));
  }
  area.setSelectionRange(edit.select.start, edit.select.end);
}

/** The heading of the page that is read which begins on a line of the text. */
function headingOn(box: HTMLElement, line: number): HTMLElement | null {
  return box.querySelector<HTMLElement>(`[data-line="${line.toString()}"]`);
}

/**
 * How far down what scrolls a place in the text of the field stands, in
 * pixels. The ruler lies over the field and sets the same text the same
 * way, so the text up to the place ends in it where the place is.
 */
function topInField(
  box: HTMLElement,
  area: HTMLTextAreaElement,
  ruler: HTMLElement | null,
  place: number,
): number {
  const top = area.getBoundingClientRect().top - box.getBoundingClientRect().top + box.scrollTop;
  if (!ruler) {
    return top;
  }
  const mark = document.createElement('span');
  // A sign without width, so that the mark has a line to stand on.
  mark.textContent = String.fromCharCode(ZERO_WIDTH_SPACE);
  // What the ruler holds is put there by hand; nothing else draws into it.
  ruler.textContent = area.value.slice(0, place);
  ruler.append(mark);
  const down = mark.offsetTop;
  ruler.textContent = '';
  return top + down;
}

/**
 * Follows a link of the page. The address gets a window of its own, cut
 * loose from this one, so that the page there cannot reach back. Where no
 * window opens, as in the desktop program, whose shell has no way yet to
 * open an address in the browser, the address is copied and the manager is
 * told to paste it there.
 */
function followLink(address: string, notifier: Notifier): void {
  const opened = window.open('', '_blank');
  if (opened) {
    opened.opener = null;
    opened.location.href = address;
    return;
  }
  const tell = () => {
    notifier.showNotice(`Open this address in your browser: ${address}`);
  };
  try {
    navigator.clipboard.writeText(address).then(() => {
      notifier.showNotice('The address is copied. Paste it into your browser to open it.');
    }, tell);
  } catch {
    // A web view that lets nothing be copied.
    tell();
  }
}
