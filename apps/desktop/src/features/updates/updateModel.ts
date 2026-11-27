import type {
  InstallFailureKind,
  InstallProgress,
  UpdateInstall,
  UpdateStatus,
} from '@/shared/api/types';

import { sinceInWords } from '../sync/syncModel';

/**
 * What the last look came to, in a line: when it was, or that it could not
 * be made, or that none was. A failed look is only said, never shown as an
 * error: the program may simply be offline.
 */
export function lastLookInWords(status: UpdateStatus, now: Date = new Date()): string {
  if (status.lastLook === null) {
    return 'Not looked yet.';
  }
  const when = sinceInWords(status.lastLook, now);
  if (status.lastLookFailed) {
    return `Could not look ${when}. Nothing was lost; try again later.`;
  }
  return status.newer === null ? `Looked ${when}. This is the newest version.` : `Looked ${when}.`;
}

/**
 * The sentence about a newer version. A pre-release is called one, so that
 * the manager is not led to think it is the settled one. It says whether
 * this copy installs it when asked (decision 0045) or only the release page
 * can help; without knowing yet, it says neither.
 */
export function newerInWords(
  status: UpdateStatus,
  install: UpdateInstall | null = null,
): string | null {
  const { newer } = status;
  if (newer === null) {
    return null;
  }
  const what = newer.isPreRelease ? 'a newer pre-release' : 'a newer version';
  const out = `Version ${newer.version} is out, ${what} than the ${status.thisVersion} that runs here.`;
  if (install === null) {
    return out;
  }
  return install.updatesItself
    ? `${out} Teamwright installs it only when you press the button, and restarts only when you say so.`
    : `${out} Teamwright does not install it itself: the release page tells how to get it.`;
}

/** The few words at the foot of the navigation. */
export function markInWords(status: UpdateStatus): string | null {
  return status.newer === null ? null : `${status.newer.version} is out`;
}

/**
 * Why this copy cannot install its own update, in a sentence, or null when
 * it can (decision 0045).
 */
export function byHandInWords(install: UpdateInstall): string | null {
  switch (install.byHandReason) {
    case 'systemPackage':
      return 'This copy was installed from a .deb or .rpm package, whose files belong to the package manager of the system, so Teamwright does not replace them itself. Install the new package the same way.';
    case 'notInstalled':
      return 'This copy was built from the source rather than installed, so there is no installed program to replace.';
    case null:
      return null;
  }
}

/** Where the update comes from and how it is checked, in a line. */
export function sourceInWords(install: UpdateInstall): string {
  return `From the releases of ${install.source}. Teamwright checks that it is signed with the key built into this program, and refuses it otherwise.`;
}

/** A number of bytes in megabytes, with one decimal. */
function megabytes(bytes: number): string {
  return `${(bytes / 1_000_000).toFixed(1)} MB`;
}

/** How much of the download has arrived, from 0 to 1, or null if the size is not known. */
export function downloadShare(progress: InstallProgress): number | null {
  if (progress.phase !== 'downloading' || progress.total === null || progress.total <= 0) {
    return null;
  }
  return Math.min(1, progress.received / progress.total);
}

/** Why the last try failed, in words that say what to do. */
function failureInWords(
  install: UpdateInstall,
  failure: InstallFailureKind,
  detail: string | null,
): string {
  switch (failure) {
    case 'nothingNewer':
      return 'The address of updates offers nothing newer for this computer yet. The update may still be on its way: try again later, or get it from the release page.';
    case 'noAnswer':
      return 'The address of updates could not be reached. Nothing was changed; try again later.';
    case 'notForThisComputer':
      return 'The update has no files for this kind of computer. Get it from the release page.';
    case 'signatureRefused':
      return 'The download was refused: it is not signed with the key built into Teamwright, so it may not come from Teamwright. Nothing was installed.';
    case 'notInstalled':
      return `The new version could not be put in place${detail ? ` (${detail})` : ''}. The version you have still runs.`;
    case 'byHand':
      return byHandInWords(install) ?? 'This copy cannot install its own updates.';
  }
}

/** Where installing stands, in a sentence; null while nothing was asked. */
export function progressInWords(install: UpdateInstall): string | null {
  const { progress } = install;
  switch (progress.phase) {
    case 'idle':
      return null;
    case 'looking':
      return 'Asking for the update…';
    case 'downloading': {
      const share = downloadShare(progress);
      return share === null
        ? `Downloading ${progress.version}: ${megabytes(progress.received)}`
        : `Downloading ${progress.version}: ${megabytes(progress.received)} of ${megabytes(progress.total ?? 0)} (${Math.floor(share * 100)}%)`;
    }
    case 'ready':
      return install.installsOnRestart
        ? `${progress.version} is downloaded and its signature checked. It is installed when Teamwright restarts.`
        : `${progress.version} is installed and its signature checked. It starts when Teamwright restarts.`;
    case 'failed':
      return failureInWords(install, progress.failure, progress.detail);
  }
}

/**
 * What a restart would do to the specialists at work, or null when none
 * works: the manager says yes to it before the program restarts.
 */
export function restartWarningInWords(install: UpdateInstall): string | null {
  const count = install.specialistsAtWork;
  if (count === 0) {
    return null;
  }
  const who = count === 1 ? '1 specialist is' : `${count} specialists are`;
  const them = count === 1 ? 'it' : 'them';
  const conversations = count === 1 ? 'conversation' : 'conversations';
  return `${who} at work. Restarting now stops ${them}; you can continue the ${conversations} afterwards.`;
}

/** What waits after "Later". */
export function laterInWords(install: UpdateInstall): string {
  return install.installsOnRestart
    ? 'It is installed when you choose Restart now. If you close Teamwright first, it is downloaded again next time.'
    : 'Nothing more is needed now: keep working, and the new version starts the next time you open Teamwright.';
}
