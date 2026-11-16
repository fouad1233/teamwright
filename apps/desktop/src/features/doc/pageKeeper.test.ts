import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  LONGEST_WAIT_MILLISECONDS,
  PageKeeper,
  SAVE_AFTER_MILLISECONDS,
  savedInWords,
} from './pageKeeper';

describe('savedInWords', () => {
  it('says nothing of a page that was not written yet', () => {
    expect(savedInWords({ kind: 'not yet' })).toBeNull();
  });

  it('says when the page was kept, on a 24-hour clock', () => {
    expect(savedInWords({ kind: 'kept', at: new Date(2026, 10, 16, 14, 32) })).toBe('Saved 14:32');
    expect(savedInWords({ kind: 'kept', at: new Date(2026, 10, 16, 9, 5) })).toBe('Saved 09:05');
  });

  it('says that the page was refused', () => {
    expect(savedInWords({ kind: 'refused' })).toBe('Not saved');
  });
});

/** The content a view keeps for a text. */
function contentOf(text: string): string {
  return JSON.stringify({ text });
}

/**
 * A place to keep a page in that remembers what was written to it. The
 * backend answers when the test says so: `answer(true)` keeps the oldest
 * write that waits, `answer(false)` refuses it.
 */
function storeFor(content: string | null) {
  const waiting: ((wasKept: boolean) => void)[] = [];
  const store = {
    /** Every content handed to the backend, in order. */
    written: [] as (string | null)[],
    /** What the list of views has for the view. */
    listed: content as string | null | undefined,
    reloads: 0,
    write(written: string | null): Promise<boolean> {
      store.written.push(written);
      // The list shows a write at once, as `useSetViewContent` makes it.
      store.listed = written;
      return new Promise((resolve) => {
        waiting.push(resolve);
      });
    },
    stored: () => store.listed,
    reload: () => {
      store.reloads += 1;
    },
    async answer(wasKept: boolean): Promise<void> {
      waiting.shift()?.(wasKept);
      // What follows a write runs once its promise has settled.
      await Promise.resolve();
      await Promise.resolve();
    },
  };
  return store;
}

/** The moment every test begins at. */
const NOW = new Date(2026, 10, 16, 14, 32);

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(NOW);
});

afterEach(() => {
  vi.useRealTimers();
});

describe('a page when its view is opened', () => {
  it('shows the text the view keeps, to be read', () => {
    const page = new PageKeeper(contentOf('# Pin map'), storeFor(null));
    expect(page.snapshot()).toEqual({
      text: '# Pin map',
      isWriting: false,
      saved: { kind: 'not yet' },
    });
  });

  it('is empty for a view that keeps nothing, or something that cannot be read', () => {
    expect(new PageKeeper(null, storeFor(null)).snapshot().text).toBe('');
    expect(new PageKeeper('not json', storeFor(null)).snapshot().text).toBe('');
  });

  it('writes nothing by itself', () => {
    const store = storeFor(contentOf('# Pin map'));
    const page = new PageKeeper(store.listed ?? null, store);
    page.saveNow();
    vi.advanceTimersByTime(10 * SAVE_AFTER_MILLISECONDS);
    expect(store.written).toEqual([]);
  });
});

describe('a page that is changed', () => {
  it('shows the change at once and tells who listens', () => {
    const page = new PageKeeper(null, storeFor(null));
    let told = 0;
    const stop = page.subscribe(() => {
      told += 1;
    });
    const before = page.snapshot();
    page.change('One');
    expect(page.snapshot().text).toBe('One');
    expect(page.snapshot()).not.toBe(before);
    expect(told).toBe(1);

    // The same text again is no change.
    const same = page.snapshot();
    page.change('One');
    expect(page.snapshot()).toBe(same);
    expect(told).toBe(1);

    stop();
    page.change('Two');
    expect(told).toBe(1);
  });

  it('is written a second after the last change, once', () => {
    const store = storeFor(null);
    const page = new PageKeeper(null, store);
    page.change('O');
    vi.advanceTimersByTime(SAVE_AFTER_MILLISECONDS - 1);
    page.change('On');
    vi.advanceTimersByTime(SAVE_AFTER_MILLISECONDS - 1);
    page.change('One');
    vi.advanceTimersByTime(SAVE_AFTER_MILLISECONDS - 1);
    expect(store.written).toEqual([]);

    vi.advanceTimersByTime(1);
    expect(store.written).toEqual([contentOf('One')]);
    vi.advanceTimersByTime(10 * SAVE_AFTER_MILLISECONDS);
    expect(store.written).toEqual([contentOf('One')]);
  });

  it('is written at once when that is asked, and not a second time after the wait', () => {
    const store = storeFor(null);
    const page = new PageKeeper(null, store);
    page.change('One');
    page.saveNow();
    expect(store.written).toEqual([contentOf('One')]);
    page.saveNow();
    vi.advanceTimersByTime(10 * SAVE_AFTER_MILLISECONDS);
    expect(store.written).toEqual([contentOf('One')]);
  });

  it('is not written when the change was taken back before the wait was over', () => {
    const store = storeFor(contentOf('One'));
    const page = new PageKeeper(store.listed ?? null, store);
    page.change('One more');
    page.change('One');
    vi.advanceTimersByTime(10 * SAVE_AFTER_MILLISECONDS);
    expect(store.written).toEqual([]);
  });

  it('is kept as nothing once it has no words left', () => {
    const store = storeFor(contentOf('One'));
    const page = new PageKeeper(store.listed ?? null, store);
    page.change(' \n');
    page.saveNow();
    expect(store.written).toEqual([null]);
  });

  it('says when the backend kept it', async () => {
    const store = storeFor(null);
    const page = new PageKeeper(null, store);
    page.change('One');
    page.saveNow();
    expect(page.snapshot().saved).toEqual({ kind: 'not yet' });
    await store.answer(true);
    expect(page.snapshot().saved).toEqual({ kind: 'kept', at: NOW });
    expect(savedInWords(page.snapshot().saved)).toBe('Saved 14:32');
    expect(store.reloads).toBe(0);
  });

  it('is written every five seconds while the typing goes on without a pause', () => {
    const store = storeFor(null);
    const page = new PageKeeper(null, store);
    // A letter every half second, for eleven seconds.
    let typed = '';
    for (let letter = 0; letter < 22; letter += 1) {
      typed += 'a';
      page.change(typed);
      vi.advanceTimersByTime(500);
    }
    expect(store.written).toEqual([contentOf('a'.repeat(10)), contentOf('a'.repeat(20))]);

    // The pause after the last letter writes the rest.
    vi.advanceTimersByTime(SAVE_AFTER_MILLISECONDS);
    expect(store.written).toHaveLength(3);
    expect(store.written.at(-1)).toBe(contentOf(typed));
    expect(LONGEST_WAIT_MILLISECONDS).toBe(5000);
  });
});

describe('a page the backend refuses', () => {
  it('says so, keeps its text, and does not try again by itself', async () => {
    const store = storeFor(null);
    const page = new PageKeeper(null, store);
    page.change('One');
    page.saveNow();
    await store.answer(false);
    expect(page.snapshot()).toEqual({ text: 'One', isWriting: false, saved: { kind: 'refused' } });

    page.change('One, shorter');
    vi.advanceTimersByTime(10 * SAVE_AFTER_MILLISECONDS);
    expect(store.written).toEqual([contentOf('One')]);
  });

  it('tries again when it is to be written at once, also with the same text', async () => {
    const store = storeFor(null);
    const page = new PageKeeper(null, store);
    page.change('One');
    page.saveNow();
    await store.answer(false);

    page.saveNow();
    expect(store.written).toEqual([contentOf('One'), contentOf('One')]);
    await store.answer(true);
    expect(page.snapshot().saved.kind).toBe('kept');

    // Kept again, it is written by itself again.
    page.change('One and two');
    vi.advanceTimersByTime(SAVE_AFTER_MILLISECONDS);
    expect(store.written).toHaveLength(3);
  });

  it('counts a write that fails without an answer as refused', async () => {
    const store = storeFor(null);
    store.write = () => Promise.reject(new Error('the backend is gone'));
    const page = new PageKeeper(null, store);
    page.change('One');
    page.saveNow();
    await store.answer(true);
    expect(page.snapshot().saved).toEqual({ kind: 'refused' });
  });
});

describe('a page whose view is left', () => {
  it('is written at once, without the wait', () => {
    const store = storeFor(null);
    const page = new PageKeeper(null, store);
    page.change('One');
    page.leave();
    expect(store.written).toEqual([contentOf('One')]);
    vi.advanceTimersByTime(10 * SAVE_AFTER_MILLISECONDS);
    expect(store.written).toEqual([contentOf('One')]);
  });

  it('is not written when nothing waits to be', () => {
    const store = storeFor(contentOf('One'));
    const page = new PageKeeper(store.listed ?? null, store);
    page.leave();
    expect(store.written).toEqual([]);
  });

  it('is not written to a view that is gone from the list, now or after the wait', () => {
    const store = storeFor(contentOf('One'));
    const page = new PageKeeper(store.listed ?? null, store);
    page.change('One more');
    // The view was removed: the list has nothing for it.
    store.listed = undefined;
    page.leave();
    vi.advanceTimersByTime(10 * SAVE_AFTER_MILLISECONDS);
    expect(store.written).toEqual([]);
  });
});

describe('writing and reading', () => {
  it('goes from reading to writing and back', () => {
    const page = new PageKeeper(null, storeFor(null));
    page.beginWriting();
    expect(page.snapshot().isWriting).toBe(true);
    page.endWriting();
    expect(page.snapshot().isWriting).toBe(false);
  });

  it('writes the page at once when the writing ends', () => {
    const store = storeFor(null);
    const page = new PageKeeper(null, store);
    page.beginWriting();
    page.change('One');
    page.endWriting();
    expect(store.written).toEqual([contentOf('One')]);
  });
});

describe('a page whose stored content is handed in again', () => {
  it('stays as it is when the content is its own', () => {
    const store = storeFor(contentOf('One'));
    const page = new PageKeeper(store.listed ?? null, store);
    const before = page.snapshot();
    page.take(contentOf('One'));
    page.take(undefined);
    expect(page.snapshot()).toBe(before);
    expect(store.reloads).toBe(0);
  });

  it('takes what another computer wrote while it is read and at rest', () => {
    const store = storeFor(contentOf('One'));
    const page = new PageKeeper(store.listed ?? null, store);
    page.take(contentOf('From the laptop'));
    expect(page.snapshot().text).toBe('From the laptop');
    // What it took is not written back.
    page.saveNow();
    vi.advanceTimersByTime(10 * SAVE_AFTER_MILLISECONDS);
    expect(store.written).toEqual([]);
  });

  it('takes a view that was emptied elsewhere', () => {
    const store = storeFor(contentOf('One'));
    const page = new PageKeeper(store.listed ?? null, store);
    page.take(null);
    expect(page.snapshot().text).toBe('');
  });

  it('keeps what the manager types while the list of views is loaded again', async () => {
    const store = storeFor(contentOf('One'));
    const page = new PageKeeper(store.listed ?? null, store);
    page.beginWriting();
    page.change('One and a half');
    // The list answers with what it had before the typing.
    page.take(contentOf('One'));
    expect(page.snapshot().text).toBe('One and a half');

    vi.advanceTimersByTime(SAVE_AFTER_MILLISECONDS);
    page.change('One and a half, and more');
    // It answers again, while the first write is under way.
    page.take(contentOf('One'));
    await store.answer(true);
    page.take(contentOf('One and a half'));
    expect(page.snapshot().text).toBe('One and a half, and more');

    page.endWriting();
    expect(store.written).toEqual([
      contentOf('One and a half'),
      contentOf('One and a half, and more'),
    ]);
  });

  it('keeps its text while it is written, even with nothing unsaved', () => {
    const store = storeFor(contentOf('One'));
    const page = new PageKeeper(store.listed ?? null, store);
    page.beginWriting();
    page.take(contentOf('From the laptop'));
    expect(page.snapshot().text).toBe('One');
  });

  it('takes what arrived while it was written once the writing ends untouched', () => {
    const store = storeFor(contentOf('One'));
    const page = new PageKeeper(store.listed ?? null, store);
    page.beginWriting();
    store.listed = contentOf('From the laptop');
    page.take(store.listed);
    page.endWriting();
    expect(page.snapshot().text).toBe('From the laptop');
    expect(store.written).toEqual([]);
  });

  it('keeps a text that was refused, whatever the list says', async () => {
    const store = storeFor(contentOf('One'));
    const page = new PageKeeper(store.listed ?? null, store);
    page.change('One, too long');
    page.saveNow();
    await store.answer(false);
    // After a refusal the views are loaded again, with what is stored.
    page.take(contentOf('One'));
    expect(page.snapshot().text).toBe('One, too long');
  });

  it('asks for the list again when it is handed a text it wrote itself earlier', async () => {
    const store = storeFor(contentOf('One'));
    const page = new PageKeeper(store.listed ?? null, store);
    page.change('Two');
    page.saveNow();
    await store.answer(true);
    page.change('Three');
    page.saveNow();
    await store.answer(true);
    expect(store.reloads).toBe(0);

    // A list that was asked for before the last write answers after it.
    page.take(contentOf('Two'));
    expect(page.snapshot().text).toBe('Three');
    expect(store.reloads).toBe(1);
    page.take(contentOf('One'));
    expect(page.snapshot().text).toBe('Three');
    expect(store.reloads).toBe(2);
  });

  it('asks for the list again when it is behind what was just written', async () => {
    const store = storeFor(contentOf('One'));
    const page = new PageKeeper(store.listed ?? null, store);
    page.change('Two');
    page.saveNow();
    // The list answers with the older text while the write is under way.
    store.listed = contentOf('One');
    page.take(store.listed);
    expect(store.reloads).toBe(0);
    await store.answer(true);
    expect(store.reloads).toBe(1);
    expect(page.snapshot().text).toBe('Two');
  });

  it('does not ask while a newer write is still to come', async () => {
    const store = storeFor(contentOf('One'));
    const page = new PageKeeper(store.listed ?? null, store);
    page.change('Two');
    page.saveNow();
    page.change('Three');
    store.listed = contentOf('One');
    await store.answer(true);
    expect(store.reloads).toBe(0);
  });
});
