import { useEffect, useRef, useSyncExternalStore } from 'react';

import { classNames } from '@/shared/lib/classNames';

import { lookOf, type Sitter } from './characterModel';
import { colorsOf, currentTheme, deviceUnit, paintSprite, readThemeColors } from './paint';
import styles from './Portrait.module.css';
import { portrait } from './spriteModel';

const LIGHT_SCHEME = '(prefers-color-scheme: light)';

function subscribeToScheme(onChange: () => void): () => void {
  const query =
    typeof window.matchMedia === 'function' ? window.matchMedia(LIGHT_SCHEME) : undefined;
  query?.addEventListener('change', onChange);
  return () => {
    query?.removeEventListener('change', onChange);
  };
}

/** The size of a pixel of a portrait, in pixels of the page. */
const PORTRAIT_SCALE = 2;

interface PortraitProps {
  sitter: Sitter;
  /** The name of who is drawn, for screen readers and as a tooltip. */
  name: string;
  /** A person is round, like their avatar; an AI specialist and the Advisor are a block. */
  isPerson?: boolean;
}

/**
 * The head and shoulders of a character of the office (decision 0019), in
 * the frame an avatar has: the member's own color as a wash, a block for
 * an AI specialist, a circle for a person, gold for the manager.
 */
export function Portrait({ sitter, name, isPerson = false }: PortraitProps) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const theme = useSyncExternalStore(subscribeToScheme, currentTheme, () => 'night' as const);
  const look = lookOf(sitter);
  const lookKey = JSON.stringify(look);

  useEffect(() => {
    const element = canvas.current;
    const context = element?.getContext('2d');
    if (!element || !context) {
      return;
    }
    const ratio = window.devicePixelRatio || 1;
    const size = element.clientWidth;
    element.width = Math.round(size * ratio);
    element.height = Math.round(size * ratio);
    const unit = deviceUnit(PORTRAIT_SCALE, ratio);
    const drawn: ReturnType<typeof lookOf> = JSON.parse(lookKey) as ReturnType<typeof lookOf>;
    const picture = paintSprite(portrait(drawn), colorsOf(drawn, readThemeColors(theme)), unit);
    context.clearRect(0, 0, element.width, element.height);
    context.imageSmoothingEnabled = false;
    // Set on the bottom edge, so the shoulders meet the frame.
    context.drawImage(
      picture,
      Math.floor((element.width - picture.width) / 2),
      element.height - picture.height,
    );
  }, [lookKey, theme]);

  return (
    <span
      className={classNames(
        styles.portrait,
        isPerson && styles.person,
        sitter.kind === 'manager' && styles.manager,
        sitter.kind === 'advisor' && styles.advisor,
      )}
      data-tone={sitter.kind === 'member' ? sitter.member.color : undefined}
      role="img"
      aria-label={name}
      title={name}
    >
      <canvas ref={canvas} className={styles.picture} />
    </span>
  );
}
