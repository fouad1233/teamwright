import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
} from 'react';

import { useNavigation } from '@/app/navigation';
import type { Project } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatMoment, taskReference, today } from '@/shared/lib/format';
import { describeFailure } from '@/shared/ui/Toast';

import { useOpenArea } from '../areas/AreaContext';
import switchStyles from '../sprints/SprintBoard.module.css';
import { changeAfterTaskName, changeInWords, type HistoryNames } from '../tasks/taskHistoryModel';
import { useTeamMembers } from '../team/TeamContext';
import type { KindProps } from '../views/viewKinds';
import styles from './ActivityView.module.css';
import {
  changesToShow,
  dayHeading,
  logDays,
  logEntries,
  longestKey,
  nothingInWords,
  type LogEntry,
  type Narrowing,
} from './activityModel';
import { useActivityLog } from './useActivityLog';

/** What the log can be narrowed to, in the order of the control. */
const NARROWINGS: readonly { value: Narrowing; label: string }[] = [
  { value: 'everything', label: 'Everything' },
  { value: 'opened', label: 'Opened' },
  { value: 'moved', label: 'Moved' },
  { value: 'finished', label: 'Finished' },
  { value: 'planned', label: 'Planned' },
];

/**
 * Everything that happened to the tasks of a project, the newest first, as
 * a log book reads: a heading for every day, and under it a line for every
 * entry with its time, the key of the task and one plain sentence. The key
 * opens the task. Several changes made to one task in one go are one entry
 * with a part for each. A control narrows the log to what was opened,
 * moved, finished or planned, and "Earlier" at its foot reads on into the
 * past.
 */
export function ActivityView({ project, tasks, everyTask }: KindProps) {
  const navigation = useNavigation();
  const members = useTeamMembers();
  const openArea = useOpenArea();
  const log = useActivityLog(project.id);
  const [narrowing, setNarrowing] = useState<Narrowing>('everything');
  const scroller = useRef<HTMLDivElement>(null);
  const sheet = useRef<HTMLDivElement>(null);
  const earlier = useRef<HTMLButtonElement>(null);
  const end = useRef<HTMLParagraphElement>(null);
  /** How many entries the log showed when the reader asked for earlier ones. */
  const entriesBeforeEarlier = useRef<number | null>(null);
  /** The key that was pressed to open the task the panel at the side shows. */
  const pressedKey = useRef<HTMLButtonElement | null>(null);
  const openTaskId = navigation.taskId;
  const todayDate = today();

  const names = useMemo<HistoryNames>(
    () => ({ project, tasks: everyTask, members }),
    [project, everyTask, members],
  );
  // While one area is open, the log tells only what happened to its tasks.
  const taskIds = useMemo(
    () => (openArea === undefined ? null : new Set(tasks.map((task) => task.id))),
    [openArea, tasks],
  );
  const loaded = log.changes;
  const entries = useMemo(
    () => logEntries(changesToShow(loaded ?? [], narrowing, project, taskIds)),
    [loaded, narrowing, project, taskIds],
  );
  const days = useMemo(() => logDays(entries), [entries]);
  // The column of keys is as wide for every narrowing, so that the
  // sentences do not move sideways when the control is pressed.
  const keyLength = useMemo(
    () =>
      longestKey(project, [
        ...everyTask.map((task) => task.number),
        ...(loaded ?? []).map((change) => change.task.number),
      ]),
    [project, everyTask, loaded],
  );

  const entryCount = entries.length;
  const { hasEarlier, isReadingEarlier, readEarlier: readOn } = log;
  const hasFailed = log.failure !== undefined;

  // The reader asks for earlier entries, not for earlier pages of the
  // history. A page that holds nothing the narrowed log shows is followed
  // by the next one, and a log that shows nothing yet reads back until it
  // does, or until the history is at its beginning.
  //
  // When entries have arrived, the reader's place is kept: the page stays
  // where it is, and the keyboard goes on from the first of them, which
  // stands where the button stood.
  useLayoutEffect(() => {
    if (isReadingEarlier) {
      return;
    }
    const before = entriesBeforeEarlier.current;
    const hasNothingNew = entryCount === (before ?? 0);
    if (hasNothingNew && hasEarlier && !hasFailed) {
      readOn();
      return;
    }
    if (before === null) {
      return;
    }
    entriesBeforeEarlier.current = null;
    // Somebody who has gone elsewhere in the meantime is left there.
    const focused = document.activeElement;
    if (focused !== earlier.current && focused !== document.body) {
      return;
    }
    const arrived = [...(sheet.current?.querySelectorAll('[data-entry]') ?? [])].slice(before);
    const next =
      arrived.flatMap((entry) => entry.querySelector('button') ?? [])[0] ??
      // Nothing that arrived can be opened. Where the log has reached its
      // beginning the button is gone, and its last line takes its place.
      (earlier.current === null ? end.current : null);
    if (next) {
      next.focus({ preventScroll: true });
      next.scrollIntoView({ block: 'nearest' });
    }
  }, [entryCount, isReadingEarlier, hasEarlier, hasFailed, readOn]);

  // When the panel of a task closes, the keyboard is nowhere. It goes back
  // to the key that opened the task, so that the reader goes on from there.
  useEffect(() => {
    if (openTaskId !== null) {
      return;
    }
    const key = pressedKey.current;
    pressedKey.current = null;
    if (key?.isConnected && document.activeElement === document.body) {
      key.focus();
    }
  }, [openTaskId]);

  if (loaded === undefined) {
    return hasFailed ? (
      <p className={styles.note} role="alert">
        The history could not be loaded. {describeFailure(log.failure)}
      </p>
    ) : (
      <p className={styles.note}>Loading the history…</p>
    );
  }

  if (loaded.length === 0 && log.isSettled && !hasEarlier) {
    return (
      <div className={classNames(styles.empty, 'pattern-band')}>
        <p className={styles.emptyLead}>Everything that happened, newest first.</p>
        <p className={styles.emptyNote}>
          Whenever a task is opened, given to somebody, moved or planned, a line is written here,
          under its day. Open the first task with New task, and the log begins.
        </p>
      </div>
    );
  }

  /** Whether the log is at its beginning, with nothing more to come. */
  const isAtBeginning = log.isSettled && !hasEarlier && !isReadingEarlier;

  function readEarlier() {
    if (isReadingEarlier) {
      return;
    }
    entriesBeforeEarlier.current = entryCount;
    readOn();
  }

  return (
    <div className={styles.activity}>
      <header className={styles.head}>
        <NarrowingSwitch
          narrowing={narrowing}
          onNarrow={(chosen) => {
            setNarrowing(chosen);
            // Another log begins at its newest entry, and what was being
            // read for the log before no longer says where to go on.
            scroller.current?.scrollTo({ top: 0 });
            entriesBeforeEarlier.current = null;
          }}
        />
      </header>

      <div ref={scroller} className={styles.scroller}>
        <div
          ref={sheet}
          className={styles.sheet}
          style={{ '--key-length': keyLength } as CSSProperties}
        >
          {days.map((day) => {
            const heading = dayHeading(day.date, todayDate);
            return (
              <section key={day.date} className={styles.day}>
                <h2 className={styles.dayHead}>
                  <span className={styles.dayName}>{heading.name}</span>
                  {heading.date !== null && <span className="label">{heading.date}</span>}
                </h2>
                <ol className={styles.entries}>
                  {day.entries.map((entry) => (
                    <Entry
                      key={entry.key}
                      project={project}
                      entry={entry}
                      names={names}
                      isOpenInPanel={openTaskId === entry.task.id}
                      onOpen={(key) => {
                        pressedKey.current = key;
                        navigation.openTask(entry.task.id);
                      }}
                    />
                  ))}
                </ol>
              </section>
            );
          })}

          {entryCount === 0 &&
            !hasFailed &&
            (isAtBeginning ? (
              <p ref={end} className={styles.nothing} tabIndex={-1}>
                {nothingInWords(narrowing)}
              </p>
            ) : (
              <p className={styles.nothing}>Looking further back…</p>
            ))}

          {(hasFailed || entryCount > 0) && (
            <footer className={styles.foot}>
              {hasFailed && (
                <p className={styles.failure} role="alert">
                  The history could not be loaded. {describeFailure(log.failure)}
                </p>
              )}
              {hasEarlier && (
                <button
                  ref={earlier}
                  type="button"
                  className={styles.earlier}
                  aria-disabled={isReadingEarlier}
                  onClick={readEarlier}
                >
                  {hasFailed ? 'Try again' : 'Earlier'}
                </button>
              )}
              {isAtBeginning && entryCount > 0 && (
                <p ref={end} className={styles.end} tabIndex={-1}>
                  The log begins here.
                </p>
              )}
            </footer>
          )}
        </div>
      </div>
    </div>
  );
}

interface NarrowingSwitchProps {
  narrowing: Narrowing;
  onNarrow: (narrowing: Narrowing) => void;
}

/**
 * The control that narrows the log: the keys of the switch of the board,
 * of which one is down. With the keyboard it is one stop, and the arrow
 * keys go from key to key.
 */
function NarrowingSwitch({ narrowing, onNarrow }: NarrowingSwitchProps) {
  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const place = NARROWINGS.findIndex((choice) => choice.value === narrowing);
    const last = NARROWINGS.length - 1;
    let next: number;
    switch (event.key) {
      case 'ArrowRight':
      case 'ArrowDown':
        next = place === last ? 0 : place + 1;
        break;
      case 'ArrowLeft':
      case 'ArrowUp':
        next = place === 0 ? last : place - 1;
        break;
      case 'Home':
        next = 0;
        break;
      case 'End':
        next = last;
        break;
      default:
        return;
    }
    event.preventDefault();
    const choice = NARROWINGS[next];
    const key = event.currentTarget.children[next];
    if (choice && key instanceof HTMLElement) {
      onNarrow(choice.value);
      key.focus();
    }
  }

  return (
    <div
      className={switchStyles.switch}
      role="radiogroup"
      aria-label="What the log shows"
      onKeyDown={handleKeyDown}
    >
      {NARROWINGS.map((choice) => {
        const isChosen = choice.value === narrowing;
        return (
          <button
            key={choice.value}
            type="button"
            role="radio"
            aria-checked={isChosen}
            tabIndex={isChosen ? 0 : -1}
            className={classNames(switchStyles.choice, isChosen && switchStyles.chosen)}
            onClick={() => {
              onNarrow(choice.value);
            }}
          >
            {choice.label}
          </button>
        );
      })}
    </div>
  );
}

interface EntryProps {
  project: Project;
  entry: LogEntry;
  names: HistoryNames;
  /** Whether the panel at the side shows the task of the entry. */
  isOpenInPanel: boolean;
  /** Opens the task of the entry. It is told which key was pressed for it. */
  onOpen: (key: HTMLButtonElement) => void;
}

/**
 * One entry of the log: its time, the key of its task, and what happened.
 * One change is a sentence that begins with the title of the task. Several
 * are the title and a part for each, in the words of the history of a task.
 */
function Entry({ project, entry, names, isOpenInPanel, onOpen }: EntryProps) {
  const { task, changes } = entry;
  const reference = taskReference(project, task);
  const [only] = changes;
  const notes = [
    // Only a change made on another computer says where, as in the history
    // of a task.
    entry.computer !== undefined && `on ${entry.computer}`,
    task.gone && 'since deleted',
  ].filter(Boolean);
  // What is to be said beside the entry, quietly, after its first line.
  const aside = notes.length > 0 && <span className={styles.aside}>{notes.join(', ')}</span>;

  return (
    <li
      className={classNames(
        styles.entry,
        entry.repeatsTime && styles.follows,
        task.gone && styles.gone,
      )}
      data-entry
    >
      <time className={styles.time} dateTime={entry.at} title={formatMoment(entry.at)}>
        {entry.time}
      </time>

      {task.gone ? (
        // A task that was deleted since is named, and leads nowhere.
        <span className={styles.key}>{reference}</span>
      ) : (
        <button
          type="button"
          className={classNames(styles.key, styles.link, isOpenInPanel && styles.isOpen)}
          onClick={(event) => {
            onOpen(event.currentTarget);
          }}
        >
          {reference}
        </button>
      )}

      {changes.length === 1 && only ? (
        <p className={styles.what} data-selectable>
          <span className={styles.title}>{task.title}</span> {changeAfterTaskName(only, names)}
          {aside}
        </p>
      ) : (
        <div className={styles.what} data-selectable>
          <span className={styles.title}>{task.title}</span>
          {aside}
          <ul className={styles.parts}>
            {changes.map((change, place) => (
              // The parts of an entry only ever grow at one end, so the
              // place of a part identifies it.
              <li key={place}>{changeInWords(change, names)}</li>
            ))}
          </ul>
        </div>
      )}
    </li>
  );
}
