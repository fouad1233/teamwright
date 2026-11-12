import { useEffect, useMemo, useRef, useState } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Moment, Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import { useTeamMembers } from '../team/TeamContext';
import { useConversationsOfProject } from '../work/WorkContext';
import {
  arrangeOffice,
  describeOffice,
  isAnybodyWorking,
  placeReference,
  rowLetter,
  rowsOf,
  summaryInWords,
  type OfficeMeasures,
  type Place,
  type PlaceTarget,
} from './officeModel';
import { OfficePlace } from './OfficePlace';
import styles from './OfficeView.module.css';

/** How often the time of work that goes on is counted again. */
const TICK_MILLISECONDS = 1000;

/** A length token of the style sheet, such as `--space-4`, in pixels. */
function lengthToken(element: Element, name: string): number {
  const value = Number.parseFloat(getComputedStyle(element).getPropertyValue(name));
  return Number.isNaN(value) ? 0 : value;
}

/**
 * The sizes the office is arranged by: the width it has, measured as the
 * window changes, and the widths of its parts, read from the tokens so
 * that the style sheet and the arithmetic agree.
 */
function useOfficeMeasures(): [React.RefObject<HTMLDivElement | null>, OfficeMeasures | null] {
  const sheet = useRef<HTMLDivElement>(null);
  const [measures, setMeasures] = useState<OfficeMeasures | null>(null);

  useEffect(() => {
    const element = sheet.current;
    if (!element) {
      return;
    }
    const observer = new ResizeObserver(([entry]) => {
      if (!entry) {
        return;
      }
      const width = entry.contentRect.width;
      setMeasures((current) =>
        current?.width === width
          ? current
          : {
              width,
              placeWidth: lengthToken(element, '--office-place-width'),
              asideWidth: lengthToken(element, '--office-aside-width'),
              gap: lengthToken(element, '--space-4'),
              // The room keeps a margin of --space-6 inside each wall.
              roomInset: 2 * lengthToken(element, '--space-6') + 2,
            },
      );
    });
    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  }, []);

  return [sheet, measures];
}

/**
 * The present moment, counted again every second while `isTicking`, so
 * that the time of work that goes on runs. Nothing is counted while nobody
 * works.
 */
function useNow(isTicking: boolean): Moment {
  const [now, setNow] = useState(() => new Date().toISOString());
  useEffect(() => {
    if (!isTicking) {
      return;
    }
    const timer = window.setInterval(() => {
      setNow(new Date().toISOString());
    }, TICK_MILLISECONDS);
    return () => {
      window.clearInterval(timer);
    };
  }, [isTicking]);
  return now;
}

interface OfficeViewProps {
  project: Project;
  /** Every task of the project, to say what each place works on and has to do. */
  tasks: Task[];
}

/**
 * The office of a project (decision 0018): a workshop drawn in plan, with
 * the manager and the Advisor at the front and a bench for every member of
 * the team on the floor. A lamp is lit over a bench where work goes on,
 * and a bench that waits for the manager is held by gold corners.
 */
export function OfficeView({ project, tasks }: OfficeViewProps) {
  const members = useTeamMembers();
  const conversations = useConversationsOfProject();
  const navigation = useNavigation();
  const [sheet, measures] = useOfficeMeasures();
  const now = useNow(isAnybodyWorking(conversations));

  const office = useMemo(
    () => describeOffice({ project, members, tasks, conversations, now }),
    [project, members, tasks, conversations, now],
  );

  const arrangement = measures
    ? arrangeOffice(members.length, measures)
    : { columns: 1, frontColumns: 1, isAsideBeside: false };
  const frontRows = rowsOf(office.front, arrangement.frontColumns);
  const floorRows = rowsOf(office.floor, arrangement.columns);

  function go(target: PlaceTarget) {
    switch (target.kind) {
      case 'task':
        navigation.openTask(target.taskId);
        return;
      case 'chat':
        navigation.openChat(target.memberId);
        return;
      case 'advisor':
        navigation.showView('advisor');
        return;
      case 'inbox':
        navigation.openInbox();
        return;
    }
  }

  function drawRow(row: Place[], index: number) {
    return (
      <li key={rowLetter(index)} className={styles.row}>
        <span className={classNames('label', styles.rowLetter)} aria-hidden>
          {rowLetter(index)}
        </span>
        <ul className={styles.places}>
          {row.map((place, column) => {
            const target = place.target;
            return (
              <li key={place.key} className={styles.place}>
                <OfficePlace
                  place={place}
                  reference={placeReference(index, column)}
                  onPress={
                    target
                      ? () => {
                          go(target);
                        }
                      : undefined
                  }
                />
              </li>
            );
          })}
        </ul>
      </li>
    );
  }

  const { summary } = office;

  return (
    <div className={styles.view}>
      <div
        ref={sheet}
        className={classNames(styles.sheet, arrangement.isAsideBeside && styles.besideAside)}
        // Hidden until it is measured, so that the room is never seen in
        // the wrong arrangement.
        data-measured={measures ? '' : undefined}
      >
        <section className={styles.room} aria-label="The office">
          <ol className={styles.rows} aria-label="The front: you and the Advisor">
            {frontRows.map((row, index) => drawRow(row, index))}
          </ol>
          <span className={styles.partition} aria-hidden />
          {floorRows.length > 0 ? (
            <ol className={styles.rows} aria-label="The floor: the team">
              {floorRows.map((row, index) => drawRow(row, frontRows.length + index))}
            </ol>
          ) : (
            <p className={styles.empty}>
              Nobody is on the team yet. Add members in the Team view, or plan the project with the
              Advisor.
            </p>
          )}
          <Door />
        </section>

        <aside className={styles.titleBlock} aria-label="The office in numbers">
          <div className={styles.titleRow}>
            <span className="label">Plan of the office</span>
            <span className="label">{project.key}</span>
          </div>
          <div className={styles.titleName}>
            <h2 className={styles.title}>The office</h2>
            <p className={styles.summary}>{summaryInWords(summary)}</p>
          </div>
          <dl className={styles.counts}>
            <div>
              <dt className="label">At work</dt>
              <dd>{summary.working}</dd>
            </div>
            <div>
              <dt className="label">Waits for you</dt>
              <dd>{summary.waiting}</dd>
            </div>
            <div>
              <dt className="label">Free</dt>
              <dd>{summary.free}</dd>
            </div>
            <div>
              <dt className="label">Used</dt>
              <dd>{summary.used}</dd>
            </div>
          </dl>
          <ul className={styles.legend} aria-label="Legend">
            <li>
              <span className={classNames(styles.key, styles.keyLit)} aria-hidden />
              Lamp lit: at work
            </li>
            <li>
              <span className={styles.keyCorners} aria-hidden>
                <span className={styles.joints}>
                  <span />
                </span>
              </span>
              Gold corners: waits for you
            </li>
            <li>
              <span className={styles.key} aria-hidden />
              Lamp out: free
            </li>
          </ul>
          <p className={styles.note}>
            Times say how long work has gone on. How long it will still take is not known, so it is
            not shown.
          </p>
        </aside>
      </div>
    </div>
  );
}

/**
 * The door of the room, drawn in plan the way an architect draws one: a
 * gap in the wall, the leaf standing open, and the arc it swings through.
 */
function Door() {
  return (
    <svg className={styles.door} viewBox="0 0 48 48" aria-hidden>
      <line x1="0.5" y1="48" x2="0.5" y2="0" className={styles.doorLeaf} />
      <path d="M0.5 0 A47.5 47.5 0 0 1 48 47.5" className={styles.doorSwing} />
    </svg>
  );
}
