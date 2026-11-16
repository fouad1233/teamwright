import { describe, expect, it } from 'vitest';

import {
  EMPTY_GALLERY,
  MAX_CONTENT_BYTES,
  addressOfPicture,
  contentBytes,
  fitPicture,
  formatBytes,
  isPictureFile,
  parseContent,
  pictureOfAddress,
  reducedSize,
  roomNote,
  serializeContent,
  weightInWords,
  withPicture,
  withSize,
  withoutPicture,
  withoutPicturesOfGoneTasks,
  type GalleryContent,
} from './galleryModel';

/** A picture of a given weight in bytes of text. */
function pictureOf(bytes: number): string {
  return 'A'.repeat(bytes);
}

/** A gallery whose pictures weigh about so much together, in pictures of 100 kB. */
function galleryOf(count: number, each = 100 * 1024): GalleryContent {
  let content = EMPTY_GALLERY;
  for (let index = 0; index < count; index += 1) {
    content = withPicture(content, `task-${index.toString()}`, pictureOf(each));
  }
  return content;
}

describe('parseContent', () => {
  it('gives an empty gallery for nothing, empty text, damaged text or something else', () => {
    expect(parseContent(null)).toEqual(EMPTY_GALLERY);
    expect(parseContent('')).toEqual(EMPTY_GALLERY);
    expect(parseContent('{not json')).toEqual(EMPTY_GALLERY);
    expect(parseContent('[1, 2]')).toEqual(EMPTY_GALLERY);
    expect(parseContent('"words"')).toEqual(EMPTY_GALLERY);
  });

  it('reads back what it wrote', () => {
    const content = withSize(withPicture(EMPTY_GALLERY, 't1', 'QUJD'), 'large');
    expect(parseContent(serializeContent(content))).toEqual(content);
  });

  it('keeps what it can read of content from another version', () => {
    const read = parseContent(
      JSON.stringify({
        size: 'huge',
        pictures: { good: 'QUJD+/==', bad: '<script>', number: 4, empty: '' },
        other: true,
      }),
    );
    expect(read).toEqual({ size: 'medium', pictures: { good: 'QUJD+/==' } });
  });

  it('ignores pictures that are not a table', () => {
    expect(parseContent(JSON.stringify({ size: 'small', pictures: ['QUJD'] }))).toEqual({
      size: 'small',
      pictures: {},
    });
  });
});

describe('the pictures of the gallery', () => {
  it('puts a picture on a card, replaces it and takes it off', () => {
    const one = withPicture(EMPTY_GALLERY, 't1', 'QUJD');
    expect(one.pictures).toEqual({ t1: 'QUJD' });
    expect(withPicture(one, 't1', 'REVG').pictures).toEqual({ t1: 'REVG' });
    expect(withoutPicture(one, 't1').pictures).toEqual({});
    expect(withoutPicture(one, 'other')).toBe(one);
  });

  it('forgets the pictures of tasks that are gone, but not while no task is known', () => {
    const two = withPicture(withPicture(EMPTY_GALLERY, 't1', 'QUJD'), 't2', 'REVG');
    expect(withoutPicturesOfGoneTasks(two, new Set(['t1'])).pictures).toEqual({ t1: 'QUJD' });
    expect(withoutPicturesOfGoneTasks(two, new Set(['t1', 't2']))).toBe(two);
    expect(withoutPicturesOfGoneTasks(two, new Set())).toBe(two);
  });
});

describe('the weight of the content', () => {
  it('counts the bytes the backend counts', () => {
    const content = withPicture(EMPTY_GALLERY, 'é', 'QUJD');
    // The id is two bytes in UTF-8 though it is one letter.
    expect(contentBytes(content)).toBe(serializeContent(content).length + 1);
  });

  it('lets a dozen pictures of a typical weight in with room to spare', () => {
    const dozen = galleryOf(12);
    expect(contentBytes(dozen)).toBeLessThan(MAX_CONTENT_BYTES * 0.65);
    expect(fitPicture(dozen, 'one-more', pictureOf(100 * 1024)).fits).toBe(true);
  });

  it('refuses a picture that does not fit, with how much it needs and how much is left', () => {
    const nearlyFull = galleryOf(20);
    const fitting = fitPicture(nearlyFull, 'too-much', pictureOf(100 * 1024));
    expect(fitting.fits).toBe(false);
    if (!fitting.fits) {
      expect(fitting.reason).toMatch(
        /^This picture needs 100 kB, and the gallery has \d+ kB left\./,
      );
      expect(fitting.reason).toContain('Take the picture off another card');
    }
  });

  it('gives back the room of the picture a new one replaces', () => {
    const nearlyFull = galleryOf(20);
    const fitting = fitPicture(nearlyFull, 'task-3', pictureOf(100 * 1024));
    expect(fitting.fits).toBe(true);
  });

  it('accepts a picture that fills the content to the last byte', () => {
    const empty = contentBytes(withPicture(EMPTY_GALLERY, 't', ''));
    const exact = pictureOf(MAX_CONTENT_BYTES - empty);
    expect(fitPicture(EMPTY_GALLERY, 't', exact).fits).toBe(true);
    expect(fitPicture(EMPTY_GALLERY, 't', exact + 'A').fits).toBe(false);
  });
});

describe('roomNote', () => {
  it('says nothing while there is plenty of room', () => {
    expect(roomNote(EMPTY_GALLERY)).toBeNull();
    expect(roomNote(galleryOf(12))).toBeNull();
  });

  it('says how many more pictures of the same weight fit when little room is left', () => {
    expect(roomNote(galleryOf(17))).toMatch(/^Room for about 3 more pictures: \d+ kB left\.$/);
    expect(roomNote(galleryOf(19))).toMatch(/^Room for about 1 more picture: /);
  });

  it('says that the gallery is full when not one more fits', () => {
    expect(roomNote(galleryOf(20))).toMatch(/^The gallery is full: \d+ kB left\./);
  });
});

describe('weightInWords', () => {
  it('says how many pictures there are and what they weigh of what the view may hold', () => {
    expect(weightInWords(galleryOf(4))).toBe('4 pictures, 400 kB of 2.0 MB');
    expect(weightInWords(galleryOf(1))).toBe('1 picture, 100 kB of 2.0 MB');
  });
});

describe('formatBytes', () => {
  it('says bytes, kilobytes and megabytes', () => {
    expect(formatBytes(740)).toBe('740 bytes');
    expect(formatBytes(96 * 1024)).toBe('96 kB');
    expect(formatBytes(1.4 * 1024 * 1024)).toBe('1.4 MB');
  });
});

describe('reducedSize', () => {
  it('makes a large picture as long as allowed on its longest side, in proportion', () => {
    expect(reducedSize(4032, 3024)).toEqual({ width: 640, height: 480 });
    expect(reducedSize(1080, 1920)).toEqual({ width: 360, height: 640 });
    expect(reducedSize(1000, 1000, 100)).toEqual({ width: 100, height: 100 });
  });

  it('never makes a picture larger', () => {
    expect(reducedSize(320, 200)).toEqual({ width: 320, height: 200 });
  });

  it('keeps at least one pixel of a very thin picture', () => {
    expect(reducedSize(10000, 2)).toEqual({ width: 640, height: 1 });
  });

  it('gives nothing for a picture without pixels', () => {
    expect(reducedSize(0, 100)).toBeNull();
    expect(reducedSize(Number.NaN, 100)).toBeNull();
  });
});

describe('the address of a picture', () => {
  it('takes the picture out of the address of a JPEG and puts it back', () => {
    expect(pictureOfAddress('data:image/jpeg;base64,QUJD')).toBe('QUJD');
    expect(addressOfPicture('QUJD')).toBe('data:image/jpeg;base64,QUJD');
  });

  it('refuses an address that is not a JPEG', () => {
    expect(pictureOfAddress('data:image/png;base64,QUJD')).toBeNull();
    expect(pictureOfAddress('data:image/jpeg;base64,')).toBeNull();
    expect(pictureOfAddress('data:,')).toBeNull();
  });
});

describe('isPictureFile', () => {
  it('takes the pictures a browser can draw and nothing else', () => {
    expect(isPictureFile({ type: 'image/jpeg' })).toBe(true);
    expect(isPictureFile({ type: 'image/png' })).toBe(true);
    expect(isPictureFile({ type: 'image/svg+xml' })).toBe(false);
    expect(isPictureFile({ type: 'application/pdf' })).toBe(false);
    expect(isPictureFile({ type: '' })).toBe(false);
  });
});
