import type { DriveFolder, GoogleDrive } from '@/shared/api/types';

/** Google's page for Google Drive for desktop, opened in the manager's own browser. */
export const DRIVE_FOR_DESKTOP_ADDRESS = 'https://www.google.com/drive/download/';

/** A folder of Drive as the manager knows it: by its account when the path names one. */
export function driveFolderInWords(folder: DriveFolder): string {
  return folder.account ?? folder.path;
}

/** The one line in the head of the part "Google Drive" of the integrations. */
export function googleDriveStateInWords(drive: GoogleDrive): string {
  return drive.syncsThrough === null ? 'Off' : 'On: this computer syncs through it';
}

/** What there is of Google Drive on this computer, in one sentence. */
export function googleDriveFactInWords(drive: GoogleDrive): string {
  if (drive.syncsThrough !== null) {
    return `This computer syncs through ${drive.syncsThrough.path}.`;
  }
  switch (drive.finding) {
    case 'found':
      return drive.folders.length === 1
        ? `Google Drive for desktop keeps ${drive.folders[0]?.path ?? ''} on this computer; syncing does not run through it.`
        : `Google Drive for desktop keeps a folder for each of ${drive.folders.length.toString()} accounts on this computer; syncing does not run through any.`;
    case 'not_installed':
      return 'Google Drive for desktop is not on this computer.';
    case 'not_for_this_system':
      return 'Google Drive for desktop is not made for this system.';
  }
}
