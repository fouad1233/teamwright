import { describe, expect, it } from 'vitest';

import type { GoogleDrive } from '@/shared/api/types';

import {
  driveFolderInWords,
  googleDriveFactInWords,
  googleDriveStateInWords,
} from './googleDriveModel';

const MY_DRIVE = { path: 'G:\\My Drive', account: null };
const WORK = {
  path: '/Users/ada/Library/CloudStorage/GoogleDrive-ada@work.example/My Drive',
  account: 'ada@work.example',
};

describe('the words about Google Drive', () => {
  it('names a folder by its account when the path names one', () => {
    expect(driveFolderInWords(MY_DRIVE)).toBe('G:\\My Drive');
    expect(driveFolderInWords(WORK)).toBe('ada@work.example');
  });

  it('says whether this computer syncs through Drive', () => {
    const through: GoogleDrive = { finding: 'found', folders: [MY_DRIVE], syncsThrough: MY_DRIVE };
    expect(googleDriveStateInWords(through)).toBe('On: this computer syncs through it');
    expect(googleDriveFactInWords(through)).toBe('This computer syncs through G:\\My Drive.');
  });

  it('says what was found when syncing does not run through Drive', () => {
    const found: GoogleDrive = { finding: 'found', folders: [MY_DRIVE], syncsThrough: null };
    expect(googleDriveStateInWords(found)).toBe('Off');
    expect(googleDriveFactInWords(found)).toBe(
      'Google Drive for desktop keeps G:\\My Drive on this computer; syncing does not run through it.',
    );
    expect(googleDriveFactInWords({ ...found, folders: [MY_DRIVE, WORK] })).toContain('2 accounts');
    expect(
      googleDriveFactInWords({ finding: 'not_installed', folders: [], syncsThrough: null }),
    ).toBe('Google Drive for desktop is not on this computer.');
    expect(
      googleDriveFactInWords({ finding: 'not_for_this_system', folders: [], syncsThrough: null }),
    ).toBe('Google Drive for desktop is not made for this system.');
  });
});
