import { describe, expect, it } from 'vitest';

import {
  ONLY_HTTPS,
  checkAddress,
  embedAddress,
  parseContent,
  serializeContent,
  shortenAddress,
} from './embedModel';

describe('checkAddress', () => {
  it('keeps an https address and drops the spaces round it', () => {
    expect(checkAddress('  https://example.com/a?b=1  ')).toEqual({
      ok: true,
      address: 'https://example.com/a?b=1',
    });
  });

  it('refuses http, other schemes and text that is no address', () => {
    for (const typed of [
      'http://example.com',
      'ftp://example.com',
      'javascript:alert(1)',
      'data:text/html,hi',
      'file:///c:/secret',
      'example.com',
      'not an address',
    ]) {
      expect(checkAddress(typed)).toEqual({ ok: false, reason: ONLY_HTTPS });
    }
  });

  it('asks for an address when the field is empty', () => {
    expect(checkAddress('   ')).toEqual({ ok: false, reason: 'Paste an address first.' });
  });

  it('refuses an address without a host', () => {
    expect(checkAddress('https://').ok).toBe(false);
  });

  it('refuses an address that carries a name or password', () => {
    expect(checkAddress('https://me:secret@example.com/').ok).toBe(false);
  });
});

describe('embedAddress', () => {
  it('turns a YouTube watch address into the no-cookie embed', () => {
    expect(embedAddress('https://www.youtube.com/watch?v=dQw4w9WgXcQ')).toBe(
      'https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ',
    );
    expect(embedAddress('https://m.youtube.com/watch?v=abc_DEF-12&list=PL1')).toBe(
      'https://www.youtube-nocookie.com/embed/abc_DEF-12',
    );
  });

  it('turns a youtu.be address and keeps where it starts', () => {
    expect(embedAddress('https://youtu.be/dQw4w9WgXcQ?t=90')).toBe(
      'https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ?start=90',
    );
    expect(embedAddress('https://youtu.be/dQw4w9WgXcQ?t=1m30s')).toBe(
      'https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ?start=90',
    );
  });

  it('turns a YouTube short and leaves a YouTube page that is no video', () => {
    expect(embedAddress('https://www.youtube.com/shorts/abc123')).toBe(
      'https://www.youtube-nocookie.com/embed/abc123',
    );
    expect(embedAddress('https://www.youtube.com/feed/subscriptions')).toBe(
      'https://www.youtube.com/feed/subscriptions',
    );
  });

  it('turns a Vimeo address, with the hash of an unlisted video', () => {
    expect(embedAddress('https://vimeo.com/76979871')).toBe(
      'https://player.vimeo.com/video/76979871',
    );
    expect(embedAddress('https://vimeo.com/76979871/abcdef0123')).toBe(
      'https://player.vimeo.com/video/76979871?h=abcdef0123',
    );
    expect(embedAddress('https://player.vimeo.com/video/76979871')).toBe(
      'https://player.vimeo.com/video/76979871',
    );
  });

  it('turns a Loom share', () => {
    expect(embedAddress('https://www.loom.com/share/0123456789abcdef?sid=x')).toBe(
      'https://www.loom.com/embed/0123456789abcdef',
    );
  });

  it('turns a Figma file or design address', () => {
    const file = 'https://www.figma.com/file/AbC123/My-board?node-id=1-2';
    expect(embedAddress(file)).toBe(
      `https://www.figma.com/embed?embed_host=teamwright&url=${encodeURIComponent(file)}`,
    );
    expect(embedAddress('https://www.figma.com/design/AbC123/Name')).toContain(
      'https://www.figma.com/embed?',
    );
    expect(embedAddress('https://www.figma.com/pricing')).toBe('https://www.figma.com/pricing');
  });

  it('turns Google Docs, Sheets and Slides that are published', () => {
    expect(embedAddress('https://docs.google.com/document/d/e/2PACX-1vK/pub')).toBe(
      'https://docs.google.com/document/d/e/2PACX-1vK/pub?embedded=true',
    );
    expect(embedAddress('https://docs.google.com/spreadsheets/d/e/2PACX-1vK/pubhtml')).toBe(
      'https://docs.google.com/spreadsheets/d/e/2PACX-1vK/pubhtml?widget=true&headers=false',
    );
    expect(embedAddress('https://docs.google.com/presentation/d/e/2PACX-1vK/pub?start=false')).toBe(
      'https://docs.google.com/presentation/d/e/2PACX-1vK/embed',
    );
  });

  it('turns Google Docs, Sheets and Slides that are shared', () => {
    expect(embedAddress('https://docs.google.com/document/d/1AbC-d_E/edit?usp=sharing')).toBe(
      'https://docs.google.com/document/d/1AbC-d_E/preview',
    );
    expect(embedAddress('https://docs.google.com/spreadsheets/d/1AbC/edit#gid=0')).toBe(
      'https://docs.google.com/spreadsheets/d/1AbC/preview',
    );
    expect(embedAddress('https://docs.google.com/presentation/d/1AbC/edit')).toBe(
      'https://docs.google.com/presentation/d/1AbC/embed',
    );
  });

  it('shows an address it does not know as it is', () => {
    expect(embedAddress('https://example.com/page?x=1')).toBe('https://example.com/page?x=1');
    expect(embedAddress('not an address')).toBe('not an address');
    expect(embedAddress('https://docs.google.com/forms/d/1AbC/edit')).toBe(
      'https://docs.google.com/forms/d/1AbC/edit',
    );
  });

  it('does not take a site that only ends like a known one', () => {
    expect(embedAddress('https://notyoutube.com/watch?v=abc')).toBe(
      'https://notyoutube.com/watch?v=abc',
    );
  });
});

describe('shortenAddress', () => {
  it('drops the scheme and keeps a short address whole', () => {
    expect(shortenAddress('https://example.com/')).toBe('example.com');
    expect(shortenAddress('https://example.com/a/b')).toBe('example.com/a/b');
  });

  it('shortens a long address in the middle, keeping both ends', () => {
    const long = `https://example.com/${'segment/'.repeat(20)}end.html`;
    const short = shortenAddress(long, 40);
    expect(short).toHaveLength(40);
    expect(short.startsWith('example.com/segment')).toBe(true);
    expect(short.endsWith('end.html')).toBe(true);
    expect(short).toContain('\u2026');
  });
});

describe('content', () => {
  it('reads what it wrote', () => {
    const content = { address: 'https://example.com/a' };
    expect(parseContent(serializeContent(content))).toEqual(content);
  });

  it('falls back to an empty view for content that is missing or damaged', () => {
    const empty = { address: null };
    for (const content of [
      null,
      '',
      '   ',
      'not json',
      '42',
      'null',
      '[]',
      '{}',
      '{"address":7}',
    ]) {
      expect(parseContent(content)).toEqual(empty);
    }
  });

  it('does not trust an address in the content that is not https', () => {
    expect(parseContent('{"address":"http://example.com"}')).toEqual({ address: null });
    expect(parseContent('{"address":"javascript:alert(1)"}')).toEqual({ address: null });
  });
});
