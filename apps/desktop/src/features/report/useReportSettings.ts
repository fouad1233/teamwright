import { useCallback, useEffect, useRef, useState } from 'react';

import type { View } from '@/shared/api/types';

import { useSetViewContent } from '../views/viewQueries';
import { parseReport, serializeReport, type ReportSettings } from './reportModel';

/** How long after the manager's last change the report is written, in milliseconds. */
const SAVE_DELAY = 1000;

/**
 * The introduction and the parts a report view keeps. A change shows at
 * once and is written about a second after the last one, so that a
 * paragraph typed is one write, not one a key; what still waits is
 * written at once when the view is left.
 */
export function useReportSettings(
  view: View,
): [ReportSettings, (how: (settings: ReportSettings) => ReportSettings) => void] {
  const setContent = useSetViewContent();
  const [settings, setSettings] = useState(() => parseReport(view.content));
  // The content the report was last read from, so that content that
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
    // Content this view wrote comes back the same, in the moment it is
    // written; other content replaces the report.
    if (view.content !== serializeReport(settings)) {
      setSettings(parseReport(view.content));
    }
  }

  const change = useCallback(
    (how: (settings: ReportSettings) => ReportSettings) => {
      const next = how(settings);
      if (next === settings) {
        return;
      }
      setSettings(next);
      waiting.current = { view, content: serializeReport(next) };
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(write, SAVE_DELAY);
    },
    [settings, view, write],
  );

  return [settings, change];
}
