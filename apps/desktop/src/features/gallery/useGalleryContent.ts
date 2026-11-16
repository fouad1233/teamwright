import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import type { View } from '@/shared/api/types';

import { useSetViewContent } from '../views/viewQueries';
import { parseContent, serializeContent, type GalleryContent } from './galleryModel';

/** How long after the manager's last change the content is written, in milliseconds. */
const SAVE_AFTER = 1000;

/**
 * What the gallery keeps of its own, and a way to change it. A change shows
 * at once; it is written about a second after the last one, and at once
 * when the view is left, so that several pictures dropped one after the
 * other are one write.
 */
export function useGalleryContent(
  view: View,
): [GalleryContent, (change: (content: GalleryContent) => GalleryContent) => void] {
  const setViewContent = useSetViewContent();
  const stored = useMemo(() => parseContent(view.content), [view.content]);
  const [draft, setDraft] = useState<GalleryContent | null>(null);
  const current = draft ?? stored;

  // What is waiting to be written, and for which view, read by the timer and
  // by the farewell of the component.
  const pending = useRef<{ view: View; content: GalleryContent } | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  // The content as last drawn, so that a change made before the next render
  // (a second picture that finished reducing) builds on the one before.
  const drawn = useRef(current);
  useEffect(() => {
    drawn.current = current;
  }, [current]);

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
    (how: (content: GalleryContent) => GalleryContent) => {
      const before = pending.current?.content ?? drawn.current;
      const next = how(before);
      // A change that changes nothing, such as a refused picture, writes nothing.
      if (next === before) {
        return;
      }
      drawn.current = next;
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

  return [current, change];
}
