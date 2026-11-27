import { describe, expect, it } from 'vitest';

import type {
  InstallFailureKind,
  InstallProgress,
  UpdateInstall,
  UpdateStatus,
} from '@/shared/api/types';

import {
  byHandInWords,
  downloadShare,
  lastLookInWords,
  laterInWords,
  markInWords,
  newerInWords,
  progressInWords,
  restartWarningInWords,
  sourceInWords,
} from './updateModel';

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

function install(overrides: Partial<UpdateInstall> = {}): UpdateInstall {
  return {
    updatesItself: true,
    byHandReason: null,
    installsOnRestart: false,
    source: 'github.com/fouad1233/teamwright',
    specialistsAtWork: 0,
    progress: { phase: 'idle' },
    ...overrides,
  };
}

function at(progress: InstallProgress, overrides: Partial<UpdateInstall> = {}): UpdateInstall {
  return install({ progress, ...overrides });
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

  it('name the version, call a pre-release one, and say how it is installed', () => {
    const words = newerInWords(status({ newer: NEWER }), install());
    expect(words).toContain('Version 0.2.0 is out');
    expect(words).toContain('pre-release');
    expect(words).toContain('only when you press the button');
    expect(newerInWords(status({ newer: NEWER }), install({ updatesItself: false }))).toContain(
      'does not install it itself',
    );
    expect(markInWords(status({ newer: NEWER }))).toBe('0.2.0 is out');
  });

  it('call a full release a version, not a pre-release', () => {
    const words = newerInWords(status({ newer: { ...NEWER, isPreRelease: false } }));
    expect(words).toContain('a newer version');
    expect(words).not.toContain('pre-release');
  });
});

describe('installing an update', () => {
  it('says where the update comes from and that its signature is checked', () => {
    const words = sourceInWords(install());
    expect(words).toContain('github.com/fouad1233/teamwright');
    expect(words).toContain('signed with the key built into this program');
  });

  it('says nothing while nothing was asked', () => {
    expect(progressInWords(install())).toBeNull();
  });

  it('shows the download as it arrives', () => {
    const downloading: InstallProgress = {
      phase: 'downloading',
      version: '0.3.0',
      received: 3_700_000,
      total: 14_800_000,
    };
    expect(downloadShare(downloading)).toBe(0.25);
    expect(progressInWords(at(downloading))).toBe('Downloading 0.3.0: 3.7 MB of 14.8 MB (25%)');
    const sizeUnknown: InstallProgress = { ...downloading, total: null };
    expect(downloadShare(sizeUnknown)).toBeNull();
    expect(progressInWords(at(sizeUnknown))).toBe('Downloading 0.3.0: 3.7 MB');
  });

  it('says that a ready version starts at the restart, or is installed then on Windows', () => {
    const ready: InstallProgress = { phase: 'ready', version: '0.3.0' };
    expect(progressInWords(at(ready))).toContain('It starts when Teamwright restarts');
    expect(progressInWords(at(ready, { installsOnRestart: true }))).toContain(
      'It is installed when Teamwright restarts',
    );
    expect(laterInWords(at(ready))).toContain('keep working');
  });

  it('says in words that a download with the wrong signature was refused', () => {
    const refused: InstallProgress = {
      phase: 'failed',
      failure: 'signatureRefused',
      detail: null,
    };
    expect(progressInWords(at(refused))).toContain('not signed with the key built into Teamwright');
    expect(progressInWords(at(refused))).toContain('Nothing was installed');
  });

  it('says every other failure without alarm, with what to do', () => {
    const failed = (failure: InstallFailureKind, detail: string | null = null) =>
      progressInWords(at({ phase: 'failed', failure, detail }));
    expect(failed('nothingNewer')).toContain('nothing newer for this computer yet');
    expect(failed('noAnswer')).toContain('could not be reached');
    expect(failed('notForThisComputer')).toContain('no files for this kind of computer');
    expect(failed('notInstalled', 'disk full')).toContain('(disk full)');
  });

  it('says why a copy updates by hand', () => {
    expect(byHandInWords(install())).toBeNull();
    expect(
      byHandInWords(install({ updatesItself: false, byHandReason: 'systemPackage' })),
    ).toContain('.deb or .rpm');
    expect(
      byHandInWords(install({ updatesItself: false, byHandReason: 'notInstalled' })),
    ).toContain('built from the source');
  });

  it('warns before a restart would stop specialists', () => {
    expect(restartWarningInWords(install())).toBeNull();
    expect(restartWarningInWords(install({ specialistsAtWork: 1 }))).toBe(
      '1 specialist is at work. Restarting now stops it; you can continue the conversation afterwards.',
    );
    expect(restartWarningInWords(install({ specialistsAtWork: 3 }))).toContain(
      '3 specialists are at work',
    );
  });
});
