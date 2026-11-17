import { useRef, type KeyboardEvent } from 'react';

import type { ThemeName } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { THEMES, type ThemeEntry } from '@/shared/theme/themes';

import styles from './ThemeChooser.module.css';
import { useTheme } from './themeQueries';

const PALETTES = THEMES.filter((entry) => entry.kind === 'palette');
const SPECIALS = THEMES.filter((entry) => entry.kind === 'special');

/** The theme an arrow key leads to from another, round the end of the list. */
function themeAfter(name: ThemeName, key: string): ThemeName | null {
  const at = THEMES.findIndex((entry) => entry.name === name);
  const count = THEMES.length;
  const index: Record<string, number> = {
    ArrowRight: (at + 1) % count,
    ArrowDown: (at + 1) % count,
    ArrowLeft: (at - 1 + count) % count,
    ArrowUp: (at - 1 + count) % count,
    Home: 0,
    End: count - 1,
  };
  const next = key in index ? THEMES[index[key] ?? 0] : undefined;
  return next?.name ?? null;
}

/**
 * The theme of the window on this computer: ten sets of colours, five
 * plain and four with a pattern, besides Lamplight. Each is shown by a
 * small swatch drawn with its own tokens. One group of radio buttons: the
 * arrow keys go from one to the next and choose it.
 */
export function ThemeChooser() {
  const { theme, choose } = useTheme();
  const options = useRef(new Map<ThemeName, HTMLButtonElement>());

  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const next = themeAfter(theme, event.key);
    if (next === null) return;
    event.preventDefault();
    choose(next);
    options.current.get(next)?.focus();
  }

  const option = (entry: ThemeEntry) => (
    <ThemeOption
      key={entry.name}
      entry={entry}
      chosen={entry.name === theme}
      onChoose={choose}
      register={(element) => {
        if (element) options.current.set(entry.name, element);
        else options.current.delete(entry.name);
      }}
    />
  );

  return (
    <section className={styles.part} aria-label="Theme">
      <h2 className="label">Theme</h2>
      <div
        className={styles.groups}
        role="radiogroup"
        aria-label="Theme of this computer"
        onKeyDown={handleKeyDown}
      >
        <div role="group" aria-labelledby="themes-palettes" className={styles.group}>
          <h3 id="themes-palettes" className={styles.heading}>
            Palettes
          </h3>
          <div className={styles.options}>{PALETTES.map(option)}</div>
        </div>
        <div role="group" aria-labelledby="themes-special" className={styles.group}>
          <h3 id="themes-special" className={styles.heading}>
            Special palettes
          </h3>
          <div className={classNames(styles.options, styles.wide)}>{SPECIALS.map(option)}</div>
        </div>
      </div>
      <p className={styles.aside}>
        Kept on this computer only. Your other computers keep their own.
      </p>
    </section>
  );
}

interface ThemeOptionProps {
  entry: ThemeEntry;
  chosen: boolean;
  onChoose: (name: ThemeName) => void;
  register: (element: HTMLButtonElement | null) => void;
}

/** One theme: its swatch, its name and its line. */
function ThemeOption({ entry, chosen, onChoose, register }: ThemeOptionProps) {
  return (
    <button
      ref={register}
      type="button"
      role="radio"
      aria-checked={chosen}
      tabIndex={chosen ? 0 : -1}
      className={styles.option}
      data-chosen={chosen || undefined}
      onClick={() => {
        onChoose(entry.name);
      }}
    >
      <Swatch entry={entry} />
      <span className={styles.name}>
        {entry.label}
        {chosen && <span className={styles.inUse}>In use</span>}
      </span>
      <span className={styles.line}>{entry.line}</span>
    </button>
  );
}

/**
 * A small window drawn in the theme's own tokens: the ground, a sheet of
 * surface with lines of ink, the gold, and the tints of three statuses.
 * A special palette shows a strip of its pattern beside the sheet.
 */
function Swatch({ entry }: { entry: ThemeEntry }) {
  return (
    <span className={styles.frame} aria-hidden="true">
      <span className={styles.swatch} data-theme-preview={entry.name}>
        {entry.kind === 'special' && <span className={styles.strip} />}
        <span className={styles.sheet}>
          <span className={styles.ink} />
          <span className={styles.inkSoft} />
          <span className={styles.tints}>
            <span className={styles.tint} data-tone="blue" />
            <span className={styles.tint} data-tone="amber" />
            <span className={styles.tint} data-tone="green" />
          </span>
        </span>
        <span className={styles.gold} />
      </span>
    </span>
  );
}
