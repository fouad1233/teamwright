import type { Id } from '@/shared/api/types';

/** The sizes a card of the gallery can have, from the smallest on. */
export type CardSize = 'small' | 'medium' | 'large';

export const CARD_SIZES: readonly CardSize[] = ['small', 'medium', 'large'];

const DEFAULT_SIZE: CardSize = 'medium';

/**
 * What a gallery keeps of its own: the size of its cards, and one picture
 * for a task, by the task's id. A picture is a JPEG, kept as the base64 text
 * of its bytes.
 */
export interface GalleryContent {
  size: CardSize;
  pictures: Readonly<Record<Id, string>>;
}

export const EMPTY_GALLERY: GalleryContent = { size: DEFAULT_SIZE, pictures: {} };

/** The most bytes the content of a view may have, counted in UTF-8: 2 MiB. The backend refuses more. */
export const MAX_CONTENT_BYTES = 2 * 1024 * 1024;

/** The longest side of a kept picture, in pixels. */
export const LONGEST_SIDE = 640;

/**
 * The quality a picture is encoded with, from 0 to 1. At 640 pixels a
 * photograph then weighs about 50 to 70 kB, some 70 to 100 kB as text: a
 * dozen pictures take about half of what a view may hold.
 */
export const JPEG_QUALITY = 0.72;

/** What a picture weighs when the gallery has none yet to tell, in bytes of text. */
const TYPICAL_PICTURE_BYTES = 90 * 1024;

/** Below this much room the gallery says how much is left. */
const LITTLE_ROOM_BYTES = 512 * 1024;

/** The beginning of the address of a JPEG given as text. */
const JPEG_ADDRESS_START = 'data:image/jpeg;base64,';

/** The letters base64 is written in, padded at the end. */
const BASE64 = /^[A-Za-z0-9+/]+={0,2}$/;

function isCardSize(value: unknown): value is CardSize {
  return CARD_SIZES.includes(value as CardSize);
}

/** Whether text is a picture the gallery could have written. */
function isPicture(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && BASE64.test(value);
}

/**
 * Reads what a gallery keeps. Content that is missing, damaged or written by
 * another version gives what can be read of it, and never throws: a size
 * that is not known gives the middle size, a picture that is not base64 is
 * left out.
 */
export function parseContent(content: string | null): GalleryContent {
  if (content === null || content.trim() === '') {
    return EMPTY_GALLERY;
  }
  let read: unknown;
  try {
    read = JSON.parse(content);
  } catch {
    return EMPTY_GALLERY;
  }
  if (typeof read !== 'object' || read === null || Array.isArray(read)) {
    return EMPTY_GALLERY;
  }
  const { size, pictures } = read as { size?: unknown; pictures?: unknown };
  const kept: Record<Id, string> = {};
  if (typeof pictures === 'object' && pictures !== null && !Array.isArray(pictures)) {
    for (const [taskId, picture] of Object.entries(pictures)) {
      if (isPicture(picture)) {
        kept[taskId] = picture;
      }
    }
  }
  return { size: isCardSize(size) ? size : DEFAULT_SIZE, pictures: kept };
}

export function serializeContent(content: GalleryContent): string {
  return JSON.stringify({ size: content.size, pictures: content.pictures });
}

/** How many bytes the content weighs as the backend counts them. */
export function contentBytes(content: GalleryContent): number {
  return new TextEncoder().encode(serializeContent(content)).length;
}

export function withSize(content: GalleryContent, size: CardSize): GalleryContent {
  return { ...content, size };
}

export function withPicture(content: GalleryContent, taskId: Id, picture: string): GalleryContent {
  return { ...content, pictures: { ...content.pictures, [taskId]: picture } };
}

export function withoutPicture(content: GalleryContent, taskId: Id): GalleryContent {
  if (!(taskId in content.pictures)) {
    return content;
  }
  const pictures = { ...content.pictures };
  delete pictures[taskId];
  return { ...content, pictures };
}

/**
 * The content without the pictures of tasks that are gone, so that a
 * deleted task does not go on taking room. With no tasks known (while they
 * load) nothing is taken away.
 */
export function withoutPicturesOfGoneTasks(
  content: GalleryContent,
  taskIds: ReadonlySet<Id>,
): GalleryContent {
  if (taskIds.size === 0) {
    return content;
  }
  const kept = Object.entries(content.pictures).filter(([taskId]) => taskIds.has(taskId));
  if (kept.length === Object.keys(content.pictures).length) {
    return content;
  }
  return { ...content, pictures: Object.fromEntries(kept) };
}

/** Whether a picture fits, and the content with it; or why it does not. */
export type Fitting = { fits: true; content: GalleryContent } | { fits: false; reason: string };

/**
 * Puts a picture on a task's card if the content still fits in what a view
 * may hold. A picture that replaces another gives back the room of the old
 * one. A picture that does not fit is refused with a sentence that says how
 * much it needs and how much is left.
 */
export function fitPicture(content: GalleryContent, taskId: Id, picture: string): Fitting {
  const without = withoutPicture(content, taskId);
  const next = withPicture(without, taskId, picture);
  const nextBytes = contentBytes(next);
  if (nextBytes <= MAX_CONTENT_BYTES) {
    return { fits: true, content: next };
  }
  const left = Math.max(0, MAX_CONTENT_BYTES - contentBytes(without));
  const needs = nextBytes - contentBytes(without);
  return {
    fits: false,
    reason:
      `This picture needs ${formatBytes(needs)}, and the gallery has ${formatBytes(left)} left. ` +
      'Take the picture off another card to make room.',
  };
}

/**
 * What the gallery says about its room, or nothing while there is plenty:
 * how many more pictures of the weight of the ones it has would fit.
 */
export function roomNote(content: GalleryContent): string | null {
  const left = Math.max(0, MAX_CONTENT_BYTES - contentBytes(content));
  if (left >= LITTLE_ROOM_BYTES) {
    return null;
  }
  const weights = Object.values(content.pictures).map((picture) => picture.length);
  const typical =
    weights.length === 0
      ? TYPICAL_PICTURE_BYTES
      : weights.reduce((sum, weight) => sum + weight, 0) / weights.length;
  const more = Math.floor(left / typical);
  if (more === 0) {
    return `The gallery is full: ${formatBytes(left)} left. Take a picture off a card to make room for another.`;
  }
  return `Room for about ${more.toString()} more ${more === 1 ? 'picture' : 'pictures'}: ${formatBytes(left)} left.`;
}

/** How many pictures the gallery keeps and what they weigh: "4 pictures, 412 kB of 2.0 MB". */
export function weightInWords(content: GalleryContent): string {
  const count = Object.keys(content.pictures).length;
  return (
    `${count.toString()} ${count === 1 ? 'picture' : 'pictures'}, ` +
    `${formatBytes(contentBytes(content))} of ${formatBytes(MAX_CONTENT_BYTES)}`
  );
}

/** A number of bytes in words: "740 bytes", "96 kB", "1.4 MB". Kilo is 1024, as the limit counts. */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) {
    return `${Math.round(bytes).toString()} bytes`;
  }
  if (bytes < 1024 * 1024) {
    return `${Math.round(bytes / 1024).toString()} kB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * The size a picture is drawn at to be kept: its own if its longest side is
 * no longer than the longest allowed, else made smaller in proportion, in
 * whole pixels and never below one. Nothing for a picture without pixels.
 */
export function reducedSize(
  width: number,
  height: number,
  longest: number = LONGEST_SIDE,
): { width: number; height: number } | null {
  if (!(width > 0 && height > 0 && Number.isFinite(width) && Number.isFinite(height))) {
    return null;
  }
  const scale = Math.min(1, longest / Math.max(width, height));
  return {
    width: Math.max(1, Math.round(width * scale)),
    height: Math.max(1, Math.round(height * scale)),
  };
}

/** The picture in the address a canvas gives for a JPEG, or nothing if it is not one. */
export function pictureOfAddress(address: string): string | null {
  if (!address.startsWith(JPEG_ADDRESS_START)) {
    return null;
  }
  const picture = address.slice(JPEG_ADDRESS_START.length);
  return isPicture(picture) ? picture : null;
}

/** The address a kept picture is shown from. */
export function addressOfPicture(picture: string): string {
  return JPEG_ADDRESS_START + picture;
}

/** Whether a file is a picture the browser can read, by its type. */
export function isPictureFile(file: { type: string }): boolean {
  return file.type.startsWith('image/') && file.type !== 'image/svg+xml';
}
