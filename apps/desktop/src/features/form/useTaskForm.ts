import { useCallback, useEffect, useRef, useState } from 'react';

import type { View } from '@/shared/api/types';

import { useSetViewContent } from '../views/viewQueries';
import { parseForm, serializeForm, type TaskForm } from './formModel';

/** How long after the manager's last change the form is written, in milliseconds. */
const SAVE_DELAY = 1000;

/**
 * The form a view keeps. A view that has never been changed keeps nothing
 * and shows the form a new view starts with, named after the view; the
 * form is written into the content only once the manager changes
 * something. A change shows at once and is written about a second after
 * the last one; what still waits is written at once when the view is left.
 */
export function useTaskForm(view: View): [TaskForm, (how: (form: TaskForm) => TaskForm) => void] {
  const setContent = useSetViewContent();
  const [form, setForm] = useState(() => parseForm(view.content, view.name));
  // What the form was last read from, so that content that arrives from
  // another computer, or a view renamed before its form was kept, is taken in.
  const [readFrom, setReadFrom] = useState({ content: view.content, name: view.name });
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

  if (view.content !== readFrom.content || view.name !== readFrom.name) {
    setReadFrom({ content: view.content, name: view.name });
    // Content this view wrote comes back the same; other content replaces the form.
    if (view.content === null || view.content !== serializeForm(form)) {
      setForm(parseForm(view.content, view.name));
    }
  }

  const change = useCallback(
    (how: (form: TaskForm) => TaskForm) => {
      const next = how(form);
      if (next === form) {
        return;
      }
      setForm(next);
      waiting.current = { view, content: serializeForm(next) };
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(write, SAVE_DELAY);
    },
    [form, view, write],
  );

  return [form, change];
}
