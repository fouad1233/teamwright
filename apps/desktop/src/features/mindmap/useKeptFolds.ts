import { useEffect, useRef, useState } from 'react';

import type { View } from '@/shared/api/types';

import { useSetViewContent } from '../views/viewQueries';
import { contentOf, foldedIn } from './mindMapModel';

/** How long after the last fold the folds are written, in milliseconds. */
const WRITE_DELAY = 1000;

/** The folds on screen, and how they stand to the content of the view. */
interface Kept {
  folded: ReadonlySet<string>;
  /** The content of the view as it was when it was last looked at. */
  seen: string | null;
  /** What was last written from here. `undefined` while nothing was. */
  written: string | null | undefined;
  /** Whether a fold made here still waits to be written. */
  isWaiting: boolean;
}

/** A write that waits for its moment. */
interface Waiting {
  timer: ReturnType<typeof setTimeout>;
  view: View;
  content: string | null;
}

/** Which nodes of a mind map are folded, and the way to fold others. */
export interface KeptFolds {
  folded: ReadonlySet<string>;
  /** Shows these folds at once, and writes them a moment later. */
  fold: (folded: ReadonlySet<string>) => void;
}

/**
 * The folds of a mind map, kept in the content of its view.
 *
 * A fold shows at once. It is written a second after the last one, so that
 * folding several branches in a row is one write, and at once when the view
 * is left. Folds that arrive from elsewhere, because another computer wrote
 * the view, replace the ones on screen, unless a fold made here still waits
 * to be written: that one is newer, and is written over them.
 */
export function useKeptFolds(view: View): KeptFolds {
  const setContent = useSetViewContent();
  const [kept, setKept] = useState<Kept>(() => ({
    folded: foldedIn(view.content),
    seen: view.content,
    written: undefined,
    isWaiting: false,
  }));
  const waiting = useRef<Waiting | null>(null);

  if (view.content !== kept.seen) {
    // The content has changed since it was looked at. What was written from
    // here comes back as it is on screen already.
    const isFromHere = view.content === kept.written || kept.isWaiting;
    setKept({
      ...kept,
      seen: view.content,
      folded: isFromHere ? kept.folded : foldedIn(view.content),
    });
  }

  function write() {
    const due = waiting.current;
    if (!due) {
      return;
    }
    clearTimeout(due.timer);
    waiting.current = null;
    setKept((current) => ({ ...current, written: due.content, isWaiting: false }));
    void setContent(due.view, due.content);
  }

  function fold(folded: ReadonlySet<string>) {
    setKept((current) => ({ ...current, folded, isWaiting: true }));
    if (waiting.current) {
      clearTimeout(waiting.current.timer);
    }
    waiting.current = { view, content: contentOf(folded), timer: setTimeout(write, WRITE_DELAY) };
  }

  useEffect(
    () => () => {
      // The view is left: what waits is written at once.
      const due = waiting.current;
      if (due) {
        clearTimeout(due.timer);
        waiting.current = null;
        void setContent(due.view, due.content);
      }
    },
    [setContent],
  );

  return { folded: kept.folded, fold };
}
