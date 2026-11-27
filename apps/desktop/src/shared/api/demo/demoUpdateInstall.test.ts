import { describe, expect, it } from 'vitest';

import { BackendError } from '../backend';
import { DemoBackend } from './demoBackend';
import { PretendedInstall } from './updateInstall';

/** A pretended install on a clock the test moves. */
function onAClock() {
  let now = 1_000_000;
  const install = new PretendedInstall(() => now);
  return {
    install,
    advance: (milliseconds: number) => {
      now += milliseconds;
    },
  };
}

describe('the pretended install of the demo', () => {
  it('does nothing until the button is pressed', () => {
    const { install } = onAClock();
    expect(install.state(0).progress).toEqual({ phase: 'idle' });
    expect(install.state(0).updatesItself).toBe(true);
  });

  it('looks, downloads with a growing progress, and waits for a restart', () => {
    const { install, advance } = onAClock();
    expect(install.start('0.2.0', 0).progress).toEqual({ phase: 'looking' });
    advance(700 + 3000);
    const halfway = install.state(0).progress;
    expect(halfway.phase).toBe('downloading');
    if (halfway.phase === 'downloading') {
      expect(halfway.received / (halfway.total ?? 1)).toBeCloseTo(0.5);
    }
    advance(3000);
    expect(install.state(0).progress).toEqual({ phase: 'ready', version: '0.2.0' });
  });

  it('refuses to restart under specialists unless the manager says yes', () => {
    const { install, advance } = onAClock();
    install.start('0.2.0', 0);
    advance(10_000);
    expect(() => install.restart(2, false)).toThrow(
      new BackendError(
        'specialists_at_work',
        '2 specialists are at work; restarting now would stop them',
      ),
    );
    expect(install.restart(2, true).progress).toEqual({ phase: 'idle' });
  });

  it('refuses a restart while nothing is ready', () => {
    const { install } = onAClock();
    expect(() => install.restart(0, true)).toThrow(BackendError);
  });

  it('is reached through the demo backend', async () => {
    const backend = new DemoBackend();
    const started = await backend.installUpdate();
    expect(started.progress.phase).toBe('looking');
    expect(started.source).toBe('github.com/fouad1233/teamwright');
    expect((await backend.updateInstall()).progress.phase).not.toBe('idle');
  });
});
