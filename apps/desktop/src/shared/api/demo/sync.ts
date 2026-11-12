import { BackendError } from '../backend';
import type { Computer, Id, Moment, SyncNotice, SyncPhase, SyncState } from '../types';

/**
 * The identity of the computer the demo pretends to run on. It is fixed,
 * not drawn, so that the identifiers the demo hands out stay the same in
 * every test.
 */
export const THIS_COMPUTER: Id = '0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d10';

/** The identity of the second computer the demo pretends, "Laptop". */
export const LAPTOP: Id = '0196f3a2-7c1e-7d53-9b6f-6b5d0e1f2a3c';

/** The name the demo pretends the operating system gives this computer. */
export const SUGGESTED_NAME = 'Desk';

/** How long after the folder is chosen "Laptop" shows up in it, in milliseconds. */
export const LAPTOP_SHOWS_UP_AFTER = 1500;

/** The version of Teamwright the pretended "Laptop" runs. */
const LAPTOP_VERSION = '0.1.0';

/** The longest name a computer can have, as in the domain of the real application. */
const COMPUTER_NAME_MAX_LENGTH = 60;

/** What the real application says when another computer uses this one's identity. */
export const IDENTITY_IN_USE =
  'another computer uses the identity of this computer; give this computer a new identity and start Teamwright again';

/** What the real application says once this computer has a new identity. */
export const NEW_IDENTITY_TAKEN =
  'This computer has a new identity. Start Teamwright again to go on.';

/**
 * Something a scenario can make the pretended folder do, to show every
 * state in a browser:
 *
 * - `writing`: a round is under way, until the next round.
 * - `unreachable`: the folder cannot be reached, until it is cleared.
 * - `identity_in_use`: another computer writes under this one's identity.
 */
export type SyncTrouble = 'writing' | 'unreachable' | 'identity_in_use';

/** What the pretended syncing needs from the demo backend around it. */
export interface SyncSurroundings {
  now(): Date;
  later(work: () => void, milliseconds: number): void;
  /**
   * Takes in what an accepted computer wrote, the first time it is read.
   * Answers with the projects in which something changed.
   */
  takeIn(computerId: Id): Id[];
  /** Tells whoever listens that the state changed. */
  stateChanged(state: SyncState): void;
  /** Tells whoever listens that syncing changed projects. */
  dataChanged(projectIds: Id[]): void;
}

/**
 * Syncing through a folder, pretended: there is no folder and no second
 * computer, but every state the real application can be in can be reached
 * and looked at. The refusals are those of the real application, with the
 * same codes and messages.
 */
export class PretendedSync {
  private folder: string | null = null;
  private computerName: string | null = null;
  private phase: SyncPhase = 'off';
  private lastRound: Moment | null = null;
  private problem: string | null = null;
  private readonly computers: Computer[] = [];
  private kept: SyncNotice[] = [];
  private nextNoticeId = 1;
  /** Set once this computer got a new identity: syncing waits for a start of the program. */
  private restartNeeded = false;
  private trouble: SyncTrouble | null = null;
  /** The computers whose writing has been taken in at least once. */
  private readonly takenIn = new Set<Id>();

  constructor(private readonly surroundings: SyncSurroundings) {}

  state(): SyncState {
    return {
      folder: this.folder,
      thisComputer: THIS_COMPUTER,
      computerName: this.computerName,
      suggestedName: SUGGESTED_NAME,
      phase: this.restartNeeded ? 'stopped' : this.folder === null ? 'off' : this.phase,
      lastRound: this.lastRound,
      problem: this.problem,
      computers: structuredClone(this.computers),
    };
  }

  chooseFolder(folder: string, computerName: string, knownFolders?: readonly string[]): SyncState {
    const name = tidyComputerName(computerName);
    if (folder === '') {
      throw new BackendError('rule_broken', 'the folder must be named');
    }
    if (knownFolders && !knownFolders.includes(folder)) {
      throw new BackendError('folder_not_found', `the folder ${folder} does not exist`);
    }
    this.computerName = name;
    this.folder = folder;
    this.phase = 'in_step';
    // The first round writes the whole log. Whatever it finds goes into
    // the state; the folder is chosen all the same.
    this.tryRound();
    if (!this.computers.some((computer) => computer.id === LAPTOP)) {
      this.surroundings.later(() => {
        this.laptopShowsUp();
      }, LAPTOP_SHOWS_UP_AFTER);
    }
    return this.state();
  }

  stop(): SyncState {
    this.folder = null;
    this.phase = 'off';
    this.lastRound = null;
    this.problem = null;
    this.restartNeeded = false;
    return this.state();
  }

  syncNow(): SyncState {
    this.round();
    return this.state();
  }

  rename(computerName: string): SyncState {
    this.computerName = tidyComputerName(computerName);
    return this.state();
  }

  accept(computerId: Id): SyncState {
    this.requireComputer(computerId).acceptance = 'accepted';
    this.tryRound();
    return this.state();
  }

  ignore(computerId: Id): SyncState {
    this.requireComputer(computerId).acceptance = 'ignored';
    return this.state();
  }

  notices(): SyncNotice[] {
    return structuredClone(this.kept).sort(
      (first, second) => second.at.localeCompare(first.at) || second.id - first.id,
    );
  }

  /** Keeps a notice for the manager. */
  addNotice(notice: Omit<SyncNotice, 'id' | 'at'>): void {
    this.kept.push({ ...notice, id: this.nextNoticeId, at: this.nowAsText() });
    this.nextNoticeId += 1;
  }

  dismiss(noticeId: number): void {
    this.kept = this.kept.filter((notice) => notice.id !== noticeId);
  }

  newIdentity(): SyncState {
    this.restartNeeded = true;
    this.phase = 'stopped';
    this.problem = NEW_IDENTITY_TAKEN;
    return this.state();
  }

  /**
   * Makes the pretended folder show a trouble from now on, or with nothing
   * be well again, and runs a round to show it. A round under way lasts
   * until the next round.
   */
  pretend(trouble: SyncTrouble | null): SyncState {
    this.trouble = trouble;
    if (trouble === 'writing') {
      this.phase = 'writing';
      this.surroundings.stateChanged(this.state());
    } else if (this.folder !== null) {
      this.tryRound();
    }
    return this.state();
  }

  /**
   * Makes another computer show up in the pretended folder, or changes one
   * that is there, by its name. A new one is undecided.
   */
  pretendComputer(
    name: string,
    details: Partial<Pick<Computer, 'version' | 'clockAhead' | 'newerFormat'>> = {},
  ): SyncState {
    let computer = this.computers.find((known) => known.name === name);
    if (!computer) {
      computer = {
        id: `0196f3a2-7c1e-7d53-9b6f-${(this.computers.length + 1).toString().padStart(12, '0')}`,
        name,
        version: LAPTOP_VERSION,
        acceptance: 'undecided',
        lastHeard: this.nowAsText(),
        clockAhead: false,
        newerFormat: false,
      };
      this.computers.push(computer);
    }
    Object.assign(computer, details);
    this.surroundings.stateChanged(this.state());
    return this.state();
  }

  /** Runs a round and keeps what went wrong in the state instead of refusing. */
  private tryRound(): void {
    try {
      this.round();
    } catch (failure) {
      if (!(failure instanceof BackendError)) {
        throw failure;
      }
    }
  }

  /**
   * One round: writes, and takes in what the accepted computers wrote.
   * Tells the listeners how it went, also when it went wrong.
   */
  private round(): void {
    if (this.restartNeeded) {
      throw new BackendError('identity_in_use', IDENTITY_IN_USE);
    }
    try {
      const changed = this.runRound();
      this.phase = 'in_step';
      this.problem = null;
      this.lastRound = this.nowAsText();
      this.surroundings.stateChanged(this.state());
      if (changed.length > 0) {
        this.surroundings.dataChanged(changed);
      }
    } catch (failure) {
      if (failure instanceof BackendError) {
        this.phase = failure.code === 'identity_in_use' ? 'stopped' : 'unreachable';
        this.problem = failure.message;
        this.lastRound = this.nowAsText();
        this.surroundings.stateChanged(this.state());
      }
      throw failure;
    }
  }

  /** The work of a round. Answers with the projects in which something changed. */
  private runRound(): Id[] {
    const folder = this.folder;
    if (folder === null) {
      throw new BackendError('sync_off', 'no sync folder is chosen');
    }
    if (this.trouble === 'unreachable') {
      throw new BackendError(
        'sync_folder_unreachable',
        `the sync folder ${folder} cannot be reached`,
      );
    }
    if (this.trouble === 'identity_in_use') {
      throw new BackendError('identity_in_use', IDENTITY_IN_USE);
    }
    if (this.trouble === 'writing') {
      this.trouble = null;
    }
    const changed = new Set<Id>();
    for (const computer of this.computers) {
      if (computer.acceptance !== 'accepted') {
        continue;
      }
      computer.lastHeard = this.nowAsText();
      if (!this.takenIn.has(computer.id)) {
        this.takenIn.add(computer.id);
        for (const projectId of this.surroundings.takeIn(computer.id)) {
          changed.add(projectId);
        }
      }
    }
    return [...changed];
  }

  /** The second computer writes into the folder for the first time. */
  private laptopShowsUp(): void {
    if (this.folder === null || this.computers.some((computer) => computer.id === LAPTOP)) {
      return;
    }
    this.computers.push({
      id: LAPTOP,
      name: 'Laptop',
      version: LAPTOP_VERSION,
      acceptance: 'undecided',
      lastHeard: this.nowAsText(),
      clockAhead: false,
      newerFormat: false,
    });
    this.surroundings.stateChanged(this.state());
  }

  private requireComputer(computerId: Id): Computer {
    const computer = this.computers.find((known) => known.id === computerId);
    if (!computer) {
      throw new BackendError('not_found', `there is no computer with the identifier ${computerId}`);
    }
    return computer;
  }

  private nowAsText(): Moment {
    return this.surroundings.now().toISOString();
  }
}

/**
 * Checks and tidies the name of a computer as the real application does:
 * spaces at the ends go, spaces and line breaks inside become one space,
 * and the name is neither empty nor longer than 60 characters.
 */
function tidyComputerName(raw: string): string {
  const name = raw.trim().split(/\s+/).join(' ');
  if (name === '') {
    throw new BackendError('rule_broken', 'the name of the computer must not be empty');
  }
  const length = [...name].length;
  if (length > COMPUTER_NAME_MAX_LENGTH) {
    throw new BackendError(
      'rule_broken',
      `the name of the computer can have at most ${COMPUTER_NAME_MAX_LENGTH.toString()} characters, but this one has ${length.toString()}`,
    );
  }
  return name;
}
