import { useEffect, useState } from 'react';

/** How often words such as "2 min ago" are written again, in milliseconds. */
const EVERY_HALF_MINUTE = 30_000;

/**
 * The time now, renewed every half minute, so that words that say how long
 * ago something was stay true while they are on screen.
 */
export function useNow(): Date {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const timer = setInterval(() => {
      setNow(new Date());
    }, EVERY_HALF_MINUTE);
    return () => {
      clearInterval(timer);
    };
  }, []);
  return now;
}
