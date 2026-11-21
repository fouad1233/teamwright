import { describe, expect, it } from 'vitest';

import type { UpdateStatus } from '@/shared/api/types';

import { lastLookInWords, markInWords, newerInWords } from './updateModel';

const NOW = new Date(2026, 10, 21, 15, 0, 0);

function status(overrides: Partial<UpdateStatus> = {}): UpdateStatus {
  return {
    thisVersion: '0.1.0',
    isOn: false,
    lastLook: null,
    lastLookFailed: false,
    newer: null,
    ...overrides,
  };
}

const NEWER = {
  version: '0.2.0',
  isPreRelease: true,
  address: 'https://github.com/o/r/releases/tag/v0.2.0',
};

describe('lastLookInWords', () => {
  it('says that no look was made', () => {
    expect(lastLookInWords(status(), NOW)).toBe('Not looked yet.');
  });

  it('says when it looked, and that nothing newer was found', () => {
    const lastLook = new Date(NOW.getTime() - 4 * 60_000).toISOString();
    expect(lastLookInWords(status({ lastLook }), NOW)).toBe(
      'Looked 4 min ago. This is the newest version.',
    );
  });

  it('does not repeat itself when something newer was found', () => {
    const lastLook = new Date(NOW.getTime() - 4 * 60_000).toISOString();
    expect(lastLookInWords(status({ lastLook, newer: NEWER }), NOW)).toBe('Looked 4 min ago.');
  });

  it('says plainly, without alarm, that a look could not be made', () => {
    const lastLook = new Date(NOW.getTime() - 2 * 3_600_000).toISOString();
    expect(lastLookInWords(status({ lastLook, lastLookFailed: true }), NOW)).toBe(
      'Could not look 2 h ago. Nothing was lost; try again later.',
    );
  });
});

describe('newerInWords and markInWords', () => {
  it('say nothing while no newer version is known', () => {
    expect(newerInWords(status())).toBeNull();
    expect(markInWords(status())).toBeNull();
  });

  it('name the version, call a pre-release one, and say it is not installed from inside', () => {
    const words = newerInWords(status({ newer: NEWER }));
    expect(words).toContain('Version 0.2.0 is out');
    expect(words).toContain('pre-release');
    expect(words).toContain('does not install');
    expect(markInWords(status({ newer: NEWER }))).toBe('0.2.0 is out');
  });

  it('call a full release a version, not a pre-release', () => {
    const words = newerInWords(status({ newer: { ...NEWER, isPreRelease: false } }));
    expect(words).toContain('a newer version');
    expect(words).not.toContain('pre-release');
  });
});
