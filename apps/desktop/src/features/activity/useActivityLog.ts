import { useCallback, useEffect, useMemo, useState } from 'react';

import type { Id, ProjectChange } from '@/shared/api/types';

import { PROJECT_HISTORY_PAGE, useProjectHistory } from '../tasks/taskQueries';
import { isReadFarEnough, oldestMomentOf, type HistoryPages } from './activityModel';

/** The history of a project as the log reads it. */
export interface ActivityLog {
  /** The changes that are loaded, the newest first. Nothing before the first of them arrive. */
  changes: readonly ProjectChange[] | undefined;
  /**
   * Whether the changes have been read to a place to stop at: a full page,
   * or the beginning of the history. Until then more of them are on their
   * way, and it cannot be said whether earlier ones are left.
   */
  isSettled: boolean;
  /** Whether changes earlier than the loaded ones are still to be read. */
  hasEarlier: boolean;
  /** Whether earlier changes have been asked for and have not arrived yet. */
  isReadingEarlier: boolean;
  /** Reads the changes before the loaded ones. After a failure it tries again. */
  readEarlier: () => void;
  /** What went wrong the last time the history was asked for, if something did. */
  failure: unknown;
}

/** The pages the reader was last shown in full. */
interface Shown {
  pages: HistoryPages;
  hasEarlier: boolean;
}

/**
 * Reads the history of a project for the log, and keeps the reader's place
 * in it.
 *
 * The history is asked for again, from its first page, whenever a task of
 * the project changes. Until it is read back as far as the reader had
 * come, the log keeps what it showed, so that it neither goes blank nor
 * loses its end. A page that came back short is asked on from at once (see
 * `isReadFarEnough`).
 */
export function useActivityLog(projectId: Id): ActivityLog {
  const { data, hasNextPage, isFetching, isError, error, fetchNextPage } =
    useProjectHistory(projectId);
  const pages = data?.pages;
  const [shown, setShown] = useState<Shown | null>(null);
  const [wantsEarlier, setWantsEarlier] = useState(false);

  const isFarEnough =
    pages !== undefined &&
    isReadFarEnough(pages, hasNextPage, PROJECT_HISTORY_PAGE, {
      to: shown === null ? null : oldestMomentOf(shown.pages),
      andEarlier: wantsEarlier,
    });

  // What has been read far enough is what the log shows from now on. This
  // is noted while rendering, not after it, so that the log never draws a
  // list that is about to be replaced.
  if (pages !== undefined && isFarEnough && shown?.pages !== pages) {
    setShown({ pages, hasEarlier: hasNextPage });
    setWantsEarlier(false);
  }

  useEffect(() => {
    // A reading that failed is not tried again by itself: it would fail
    // again and again. The reader asks.
    if (pages !== undefined && !isFarEnough && !isFetching && !isError) {
      void fetchNextPage();
    }
  }, [pages, isFarEnough, isFetching, isError, fetchNextPage]);

  const readEarlier = useCallback(() => {
    if (isError) {
      void fetchNextPage();
    } else {
      setWantsEarlier(true);
    }
  }, [isError, fetchNextPage]);

  // The first reading shows what it has at once, even while it asks on
  // after a short page.
  const pagesToShow = isFarEnough ? pages : (shown?.pages ?? pages);
  const changes = useMemo(() => pagesToShow?.flat(), [pagesToShow]);

  return {
    changes,
    isSettled: isFarEnough || shown !== null,
    hasEarlier: isFarEnough ? hasNextPage : (shown?.hasEarlier ?? false),
    isReadingEarlier: wantsEarlier && (isFetching || !isError),
    readEarlier,
    failure: isError ? error : undefined,
  };
}
