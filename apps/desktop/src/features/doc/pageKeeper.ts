import { readPage, stepFor, writePage } from './docModel';

/** How long after the manager's last change the page is written, in milliseconds. */
export const SAVE_AFTER_MILLISECONDS = 1000;

/**
 * How long a change waits at most, in milliseconds: somebody who types
 * without a pause is still written every few seconds, so that a computer
 * that stops never takes more than that.
 */
export const LONGEST_WAIT_MILLISECONDS = 5000;

/** How many of the texts a page held before are remembered. */
const TEXTS_REMEMBERED = 12;

/** What became of the page when it was last written. */
export type Saved =
  /** It has not been written since the view was opened. */
  | { kind: 'not yet' }
  /** The backend kept it, at this moment. */
  | { kind: 'kept'; at: Date }
  /** The backend refused it. The text is still on the page, and not stored. */
  | { kind: 'refused' };

/** The time of day of the interface: a 24-hour clock, whatever the language of the computer. */
const CLOCK = new Intl.DateTimeFormat('en-GB', { hour: '2-digit', minute: '2-digit' });

/**
 * What became of the page, in two words for the corner of the view: "Saved
 * 14:32", or "Not saved". Nothing for a page that was not written yet.
 */
export function savedInWords(saved: Saved): string | null {
  switch (saved.kind) {
    case 'not yet':
      return null;
    case 'kept':
      return `Saved ${CLOCK.format(saved.at)}`;
    case 'refused':
      return 'Not saved';
  }
}

/** A page as it is shown. */
export interface PageState {
  /** Its text, in Markdown: what is read, and what is written in the field. */
  text: string;
  /** Whether the manager writes it or reads it. */
  isWriting: boolean;
  saved: Saved;
}

/** What a page needs from the place it is kept in. */
export interface PageStore {
  /** Writes the content of the view. Resolves to whether the backend kept it. */
  write(content: string | null): Promise<boolean>;
  /** The content the list of views has for the view now. Nothing known is `undefined`. */
  stored(): string | null | undefined;
  /** Asks for the list of views again. */
  reload(): void;
}

/**
 * The page of a doc while its view is open: the text the manager reads and
 * writes, and when it is written to the view.
 *
 * - A change shows at once and is written a second after the last one, so
 *   that typing does not write with every key, and five seconds after the
 *   first one at the latest. Leaving the view, ending the writing and
 *   asking for it write at once.
 * - A write the backend refuses is not tried again by itself: a message
 *   with every pause in the typing would only be in the way. The text stays
 *   on the page, and the next time it is written at once it is tried again.
 * - What the manager typed is never given up for what the list of views
 *   says (`stepFor`): the list is loaded again for many reasons while a page
 *   is open, and it can be behind what the page wrote a moment ago.
 *
 * It knows nothing of React. A component reads it with
 * `useSyncExternalStore(page.subscribe, page.snapshot)`.
 */
export class PageKeeper {
  private readonly store: PageStore;
  private readonly listeners = new Set<() => void>();
  private state: PageState;
  /** The text of the last write that was handed to the backend, or the text read at first. */
  private sent: string;
  /** The texts this page held before, the last ones only. */
  private known: string[];
  private timer: ReturnType<typeof setTimeout> | undefined;
  /** When the first change that is not written yet was made. */
  private waitingSince: number | undefined;
  private writesUnderWay = 0;

  constructor(content: string | null, store: PageStore) {
    const text = readPage(content);
    this.store = store;
    this.state = { text, isWriting: false, saved: { kind: 'not yet' } };
    this.sent = text;
    this.known = [text];
  }

  /** Calls a listener whenever the page changes. Returns how to stop that. */
  readonly subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  /** The page as it is now. The same object until the page changes. */
  readonly snapshot = (): PageState => this.state;

  /** The manager changed the text: by typing, with a tool of the bar, or by ticking a box. */
  change(text: string): void {
    if (text === this.state.text) {
      return;
    }
    this.show({ text });
    clearTimeout(this.timer);
    this.timer = undefined;
    if (this.state.saved.kind === 'refused') {
      return;
    }
    const now = Date.now();
    this.waitingSince ??= now;
    const latest = this.waitingSince + LONGEST_WAIT_MILLISECONDS;
    this.timer = setTimeout(
      () => {
        this.saveNow();
      },
      Math.max(Math.min(SAVE_AFTER_MILLISECONDS, latest - now), 0),
    );
  }

  beginWriting(): void {
    this.show({ isWriting: true });
  }

  /** Back to reading: the page is written at once, and what arrived meanwhile is looked at. */
  endWriting(): void {
    this.show({ isWriting: false });
    this.saveNow();
    this.take(this.store.stored());
  }

  /** Writes what is not written yet, without waiting. */
  saveNow(): void {
    this.stopWaiting();
    if (this.hasUnsaved()) {
      void this.write();
    }
  }

  /**
   * The view is left: what is not written yet is written at once. A view
   * that is gone from the list was removed, and is not written to: the
   * backend could only refuse.
   */
  leave(): void {
    if (this.store.stored() === undefined) {
      this.stopWaiting();
    } else {
      this.saveNow();
    }
  }

  /** Looks at the content the list of views has for the view, whenever it is handed in again. */
  take(content: string | null | undefined): void {
    if (content === undefined) {
      return;
    }
    const stored = readPage(content);
    const page = { text: this.state.text, isBusy: this.isBusy(), known: this.known };
    switch (stepFor(stored, page)) {
      case 'keep':
        return;
      case 'reload':
        this.store.reload();
        return;
      case 'adopt':
        this.sent = stored;
        this.remember(stored);
        this.show({ text: stored });
    }
  }

  private hasUnsaved(): boolean {
    return this.state.text !== this.sent || this.state.saved.kind === 'refused';
  }

  private isBusy(): boolean {
    return this.state.isWriting || this.hasUnsaved() || this.writesUnderWay > 0;
  }

  private stopWaiting(): void {
    clearTimeout(this.timer);
    this.timer = undefined;
    this.waitingSince = undefined;
  }

  private async write(): Promise<void> {
    const text = this.state.text;
    const content = writePage(text);
    this.sent = text;
    this.remember(text);
    this.writesUnderWay += 1;
    let wasKept = false;
    try {
      wasKept = await this.store.write(content);
    } catch {
      // A write that fails without an answer is not kept either.
    } finally {
      this.writesUnderWay -= 1;
    }
    this.show({ saved: wasKept ? { kind: 'kept', at: new Date() } : { kind: 'refused' } });

    // The list of views can be behind what was just written: when it was
    // asked for before this write and answered after it. Left like that, the
    // page would open on the older text the next time.
    const stored = this.store.stored();
    const isLast = this.writesUnderWay === 0 && this.state.text === text;
    if (wasKept && isLast && stored !== undefined && stored !== content) {
      this.store.reload();
    }
  }

  private remember(text: string): void {
    this.known = [...this.known.filter((known) => known !== text), text].slice(-TEXTS_REMEMBERED);
  }

  private show(change: Partial<PageState>): void {
    this.state = { ...this.state, ...change };
    for (const listener of this.listeners) {
      listener();
    }
  }
}
