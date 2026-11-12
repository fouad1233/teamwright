import { BackendError } from '../backend';
import type { DemoEnvironment } from './demoBackend';

/**
 * An environment whose time, identifiers and pauses the test controls.
 * Nothing that was put off happens until the test lets the time pass.
 */
export function testEnvironment(): DemoEnvironment & {
  /** Lets so many milliseconds pass and does what was put off until then. */
  pass(milliseconds: number): void;
  /** Lets enough time pass for everything that was put off. */
  passAll(): void;
} {
  let now = new Date('2026-11-11T09:00:00.000Z').getTime();
  let identifiers = 0;
  let putOff: { due: number; work: () => void }[] = [];

  function pass(milliseconds: number) {
    const until = now + milliseconds;
    for (;;) {
      const next = putOff.filter((entry) => entry.due <= until).sort((a, b) => a.due - b.due)[0];
      if (!next) {
        break;
      }
      putOff = putOff.filter((entry) => entry !== next);
      now = next.due;
      next.work();
    }
    now = until;
  }

  return {
    now: () => new Date(now),
    newId: () => {
      identifiers += 1;
      return `id-${identifiers.toString()}`;
    },
    later: (work, milliseconds) => {
      putOff.push({ due: now + milliseconds, work });
    },
    pass,
    passAll: () => {
      pass(60_000);
    },
  };
}

/** The refusal a request ends in. Fails the test if the request is not refused. */
export async function refusal(request: Promise<unknown>): Promise<BackendError> {
  try {
    await request;
  } catch (failure) {
    if (failure instanceof BackendError) {
      return failure;
    }
    throw failure;
  }
  throw new Error('the request was expected to be refused, but it succeeded');
}
