import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useSyncExternalStore } from 'react';

import { useBackend, useIsDemo } from '@/shared/api/BackendProvider';
import type { ThemeName } from '@/shared/api/types';
import { THEME_CHANGED, applyTheme, shownTheme } from '@/shared/theme/applyTheme';
import { isThemeName, rememberTheme, themeOrDefault } from '@/shared/theme/themes';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

/** The name under which the theme of this computer is cached. */
const THEME_KEY = ['appearance', 'theme'] as const;

function subscribeToTheme(onChange: () => void): () => void {
  window.addEventListener(THEME_CHANGED, onChange);
  return () => {
    window.removeEventListener(THEME_CHANGED, onChange);
  };
}

/** The theme the window shows now, and a way to choose another. */
export function useTheme(): { theme: ThemeName; choose: (name: ThemeName) => void } {
  const backend = useBackend();
  const notifier = useNotifier();
  const queryClient = useQueryClient();
  const theme = useSyncExternalStore(subscribeToTheme, shownTheme, () => shownTheme());

  const choose = useCallback(
    (name: ThemeName) => {
      const before = shownTheme();
      if (name === before) return;
      // The window crosses over at once; the backend keeps the choice behind it.
      applyTheme(name, true);
      rememberTheme(name);
      backend
        .chooseTheme(name)
        .then((kept) => {
          queryClient.setQueryData<ThemeName>(THEME_KEY, kept);
        })
        .catch((failure: unknown) => {
          applyTheme(before, true);
          rememberTheme(before);
          notifier.showError(describeFailure(failure));
        });
    },
    [backend, notifier, queryClient],
  );

  return { theme, choose };
}

declare global {
  interface Window {
    /**
     * In the demo only: shows a theme by its name, as the chooser does, so
     * that a script that takes pictures can turn the window.
     */
    teamwrightTheme?: (name: string) => void;
  }
}

/**
 * Keeps the window in the theme of this computer. The window opened in the
 * theme it remembered; when the backend answers with another one, the
 * backend is right, and the window turns to it and remembers it.
 */
export function useKeepTheme(): void {
  const backend = useBackend();
  const isDemo = useIsDemo();
  const { choose } = useTheme();
  const kept = useQuery({ queryKey: THEME_KEY, queryFn: () => backend.theme() });

  useEffect(() => {
    if (kept.data === undefined) return;
    const name = themeOrDefault(kept.data);
    applyTheme(name);
    rememberTheme(name);
  }, [kept.data]);

  useEffect(() => {
    if (!isDemo) return;
    window.teamwrightTheme = (name: string) => {
      if (isThemeName(name)) choose(name);
    };
    return () => {
      delete window.teamwrightTheme;
    };
  }, [isDemo, choose]);
}
