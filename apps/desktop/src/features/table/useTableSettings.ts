import { useCallback, useEffect, useRef, useState } from 'react';

import type { View } from '@/shared/api/types';

import { useSetViewContent } from '../views/viewQueries';
import { parseSettings, serializeSettings, type TableSettings } from './tableModel';

/** How long after the manager's last change the settings are written, in milliseconds. */
const SAVE_DELAY = 1000;

/**
 * The columns and the order a table view keeps. A change shows at once and
 * is written about a second after the last one, so that turning several
 * columns on and off is one write; what is still waiting is written at once
 * when the view is left.
 */
export function useTableSettings(
  view: View,
): [TableSettings, (change: (settings: TableSettings) => TableSettings) => void] {
  const setContent = useSetViewContent();
  const [settings, setSettings] = useState(() => parseSettings(view.content));
  // The content the settings were last read from, so that content that
  // arrives from another computer is taken in.
  const [readFrom, setReadFrom] = useState(view.content);
  const waiting = useRef<{ view: View; content: string } | null>(null);
  const timer = useRef<number | undefined>(undefined);

  const write = useCallback(() => {
    window.clearTimeout(timer.current);
    timer.current = undefined;
    const pending = waiting.current;
    waiting.current = null;
    if (pending !== null) {
      void setContent(pending.view, pending.content);
    }
  }, [setContent]);

  // Leaving the view writes what still waits.
  useEffect(() => write, [write]);

  if (view.content !== readFrom) {
    setReadFrom(view.content);
    // Content this view wrote comes back the same; other content replaces
    // the settings.
    if (view.content !== serializeSettings(settings)) {
      setSettings(parseSettings(view.content));
    }
  }

  const change = useCallback(
    (how: (settings: TableSettings) => TableSettings) => {
      const next = how(settings);
      setSettings(next);
      waiting.current = { view, content: serializeSettings(next) };
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(write, SAVE_DELAY);
    },
    [settings, view, write],
  );

  return [settings, change];
}
