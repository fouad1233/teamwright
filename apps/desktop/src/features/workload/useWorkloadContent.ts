import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import type { View } from '@/shared/api/types';

import { useSetViewContent } from '../views/viewQueries';
import { parseContent, serializeContent, type WorkloadContent } from './workloadModel';

/** How long after the manager's last change the content is written, in milliseconds. */
const SAVE_AFTER = 1000;

/**
 * What the workload keeps of its own, and a way to change it. A change shows
 * at once; it is written about a second after the last one, and at once
 * when the view is left, so that a run of changes is one write.
 */
export function useWorkloadContent(view: View): [WorkloadContent, (next: WorkloadContent) => void] {
  const setViewContent = useSetViewContent();
  const stored = useMemo(() => parseContent(view.content), [view.content]);
  const [draft, setDraft] = useState<WorkloadContent | null>(null);

  // What is waiting to be written, and for which view, read by the timer and
  // by the farewell of the component.
  const pending = useRef<{ view: View; content: WorkloadContent } | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const flush = useCallback(() => {
    if (timer.current !== null) {
      clearTimeout(timer.current);
      timer.current = null;
    }
    const waiting = pending.current;
    pending.current = null;
    if (waiting) {
      // The cache of the views takes the content at once, so the draft can go.
      void setViewContent(waiting.view, serializeContent(waiting.content));
      setDraft(null);
    }
  }, [setViewContent]);

  const change = useCallback(
    (next: WorkloadContent) => {
      setDraft(next);
      pending.current = { view, content: next };
      if (timer.current !== null) {
        clearTimeout(timer.current);
      }
      timer.current = setTimeout(flush, SAVE_AFTER);
    },
    [view, flush],
  );

  // Leaving the view writes what is still waiting.
  useEffect(() => flush, [flush]);

  return [draft ?? stored, change];
}
