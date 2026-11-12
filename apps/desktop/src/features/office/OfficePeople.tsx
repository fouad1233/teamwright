import { useEffect, useRef } from 'react';

import { usePrefersReducedMotion } from '@/shared/ui/motion';

import { hashOf } from './characters/characterModel';
import type { Place } from './officeModel';
import styles from './OfficePeople.module.css';
import { activityOf, sitterOf } from './roomModel';
import { RoomPainter, type RoomSitter } from './roomPainter';

/** The people of the room, from its places. */
function sittersOf(places: readonly Place[]): RoomSitter[] {
  return places.map((place) => ({
    person: { key: place.key, activity: activityOf(place.state), seed: hashOf(place.key) },
    sitter: sitterOf(place.figure),
  }));
}

interface OfficePeopleProps {
  /** Every place of the room, front and floor. Each bench carries `data-place-key`. */
  places: readonly Place[];
}

/**
 * The people of the office (decision 0019), drawn on one canvas laid over
 * the room. It takes no room of its own and catches no pointer: the benches
 * below it stay what is pressed and read. Every person is also said in
 * words on their bench, so the canvas is hidden from screen readers.
 */
export function OfficePeople({ places }: OfficePeopleProps) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const painter = useRef<RoomPainter | null>(null);
  const isStill = usePrefersReducedMotion();

  useEffect(() => {
    const element = canvas.current;
    const room = element?.parentElement;
    if (!element || !room) {
      return;
    }
    const roomPainter = new RoomPainter(element, room);
    painter.current = roomPainter;

    const observer = new ResizeObserver(() => {
      roomPainter.measure();
    });
    observer.observe(room);

    const onVisibility = () => {
      roomPainter.setHidden(document.visibilityState === 'hidden');
    };
    document.addEventListener('visibilitychange', onVisibility);
    onVisibility();

    const scheme =
      typeof window.matchMedia === 'function'
        ? window.matchMedia('(prefers-color-scheme: light)')
        : undefined;
    const onScheme = () => {
      roomPainter.themeChanged();
    };
    scheme?.addEventListener('change', onScheme);

    return () => {
      observer.disconnect();
      document.removeEventListener('visibilitychange', onVisibility);
      scheme?.removeEventListener('change', onScheme);
      roomPainter.dispose();
      painter.current = null;
    };
  }, []);

  useEffect(() => {
    painter.current?.setSitters(sittersOf(places));
  }, [places]);

  useEffect(() => {
    painter.current?.setStill(isStill);
  }, [isStill]);

  return <canvas ref={canvas} className={styles.people} aria-hidden />;
}
