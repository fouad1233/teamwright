import { BackendError } from '../backend';
import type { InstallProgress, UpdateInstall } from '../types';

/** How long the pretended look at the address of updates takes, in milliseconds. */
const LOOK_MILLISECONDS = 700;

/** How long the pretended download takes, in milliseconds. */
const DOWNLOAD_MILLISECONDS = 6000;

/** The size of the pretended download, in bytes: about what a Mac update weighs. */
const DOWNLOAD_BYTES = 14_800_000;

/**
 * Installing an update in the demo (decision 0045). Nothing leaves the
 * browser: pressing the button pretends a look, a download whose progress
 * grows with the time, and an update that waits for a restart. A restart
 * pretends to happen and leaves the demo as it was.
 */
export class PretendedInstall {
  #startedAt: number | null = null;
  #version: string | null = null;
  #restarted = false;

  constructor(private readonly now: () => number = () => Date.now()) {}

  /** Where the pretended install stands. */
  state(specialistsAtWork: number): UpdateInstall {
    return {
      updatesItself: true,
      byHandReason: null,
      installsOnRestart: false,
      source: 'github.com/fouad1233/teamwright',
      specialistsAtWork,
      progress: this.#progress(),
    };
  }

  /** Starts the pretended install of a version, unless one is under way or ready. */
  start(version: string, specialistsAtWork: number): UpdateInstall {
    const phase = this.#progress().phase;
    if (phase === 'idle' || phase === 'failed') {
      this.#startedAt = this.now();
      this.#version = version;
      this.#restarted = false;
    }
    return this.state(specialistsAtWork);
  }

  /** Pretends a restart, refusing as the program does while specialists work. */
  restart(specialistsAtWork: number, evenWhileWorking: boolean): UpdateInstall {
    if (this.#progress().phase !== 'ready') {
      throw new BackendError('update_not_ready', 'no update is downloaded and ready yet');
    }
    if (specialistsAtWork > 0 && !evenWhileWorking) {
      const who =
        specialistsAtWork === 1 ? '1 specialist is' : `${specialistsAtWork} specialists are`;
      const them = specialistsAtWork === 1 ? 'it' : 'them';
      throw new BackendError(
        'specialists_at_work',
        `${who} at work; restarting now would stop ${them}`,
      );
    }
    // The demo cannot restart; it goes back to having nothing to install.
    this.#restarted = true;
    return this.state(specialistsAtWork);
  }

  #progress(): InstallProgress {
    if (this.#startedAt === null || this.#version === null || this.#restarted) {
      return { phase: 'idle' };
    }
    const elapsed = this.now() - this.#startedAt;
    if (elapsed < LOOK_MILLISECONDS) {
      return { phase: 'looking' };
    }
    const share = (elapsed - LOOK_MILLISECONDS) / DOWNLOAD_MILLISECONDS;
    if (share < 1) {
      return {
        phase: 'downloading',
        version: this.#version,
        received: Math.round(DOWNLOAD_BYTES * share),
        total: DOWNLOAD_BYTES,
      };
    }
    return { phase: 'ready', version: this.#version };
  }
}
