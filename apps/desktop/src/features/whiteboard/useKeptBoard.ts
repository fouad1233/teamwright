import { useEffect, useMemo, useRef, useState } from 'react';

import type { View } from '@/shared/api/types';
import { useNotifier } from '@/shared/ui/Toast';

import { useSetViewContent } from '../views/viewQueries';
import {
  MAX_CONTENT_BYTES,
  boardBytes,
  formatBytes,
  historyOf,
  parseBoard,
  recorded,
  redone,
  serializeBoard,
  undone,
  type Board,
  type History,
} from './whiteboardModel';

/** How long after the manager's last change the board is written, in milliseconds. */
const WRITE_DELAY = 1000;

/** The board on screen, and how it stands to the content of the view. */
interface Kept {
  history: History;
  /** The content of the view as it was when it was last looked at. */
  seen: string | null;
  /** What was last written from here. `undefined` while nothing was. */
  written: string | null | undefined;
  /** Whether a change made here still waits to be written. */
  isWaiting: boolean;
}

/** A write that waits for its moment. */
interface Waiting {
  timer: ReturnType<typeof setTimeout>;
  view: View;
  content: string | null;
}

/** A board kept in the content of its view, with undo and redo. */
export interface KeptBoard {
  board: Board;
  /** What the board weighs as the backend counts it, in bytes. */
  bytes: number;
  canUndo: boolean;
  canRedo: boolean;
  /**
   * Makes a change to the board as it is now. A change that would take the
   * board past what a view may hold is refused with a message. Resolves to
   * the board after the change.
   */
  change: (how: (board: Board) => Board) => Board;
  undo: () => void;
  redo: () => void;
}

/**
 * The board of a whiteboard, kept in the content of its view.
 *
 * A change shows at once and is a step that undo takes back. It is written
 * a second after the last one, so that a run of changes is one write, and
 * at once when the view is left. A board that arrives from elsewhere,
 * because another computer wrote the view, replaces the one on screen and
 * its history, unless a change made here still waits to be written: that
 * one is newer, and is written over it.
 */
export function useKeptBoard(view: View): KeptBoard {
  const setContent = useSetViewContent();
  const notifier = useNotifier();
  const [kept, setKept] = useState<Kept>(() => ({
    history: historyOf(parseBoard(view.content)),
    seen: view.content,
    written: undefined,
    isWaiting: false,
  }));
  const waiting = useRef<Waiting | null>(null);
  // The board as it is after the last change, for a change that comes
  // before the next render, such as a task that has just been made.
  const latest = useRef(kept.history);

  if (view.content !== kept.seen) {
    const isFromHere = view.content === kept.written || kept.isWaiting;
    setKept({
      ...kept,
      seen: view.content,
      history: isFromHere ? kept.history : historyOf(parseBoard(view.content)),
    });
  }

  useEffect(() => {
    latest.current = kept.history;
  }, [kept.history]);

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

  function keep(history: History) {
    latest.current = history;
    setKept((current) => ({ ...current, history, isWaiting: true }));
    if (waiting.current) {
      clearTimeout(waiting.current.timer);
    }
    waiting.current = {
      view,
      content: serializeBoard(history.present),
      timer: setTimeout(write, WRITE_DELAY),
    };
  }

  function change(how: (board: Board) => Board): Board {
    const before = latest.current;
    const next = how(before.present);
    if (next === before.present) {
      return next;
    }
    if (boardBytes(next) > MAX_CONTENT_BYTES) {
      notifier.showError(
        `The board is full: it may hold ${formatBytes(MAX_CONTENT_BYTES)}. Delete what is no longer needed, then try again.`,
      );
      return before.present;
    }
    keep(recorded(before, next));
    return next;
  }

  function undo() {
    const next = undone(latest.current);
    if (next !== latest.current) {
      keep(next);
    }
  }

  function redo() {
    const next = redone(latest.current);
    if (next !== latest.current) {
      keep(next);
    }
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

  const { history } = kept;
  const bytes = useMemo(() => boardBytes(history.present), [history.present]);

  return {
    board: history.present,
    bytes,
    canUndo: history.past.length > 0,
    canRedo: history.future.length > 0,
    change,
    undo,
    redo,
  };
}
