/**
 * What the files view works out without drawing anything: how its content
 * is read and written, how a path inside a folder is walked, what mark a
 * name gets, how a size is written, and which entries a typed word leaves.
 *
 * The rules about what may be read live in the backend; a path here is only
 * what the manager clicked through.
 */

import type { FileEntry } from '@/shared/api/types';

/** What the view keeps of its own. */
export interface FilesContent {
  /** The folder of the project last opened, as the project names it. Nothing for the first. */
  folder: string | null;
  /** The path inside that folder, with `/` between its parts; empty for the folder itself. */
  path: string;
  /** Whether names that begin with a dot are shown. */
  showsHidden: boolean;
}

/** The content of a view that has kept nothing yet. */
export const EMPTY_CONTENT: FilesContent = { folder: null, path: '', showsHidden: false };

/**
 * Reads what the view keeps. Content that is missing, damaged or written by
 * another version falls back, field by field, to the empty content.
 */
export function parseContent(content: string | null): FilesContent {
  if (content === null || content.trim() === '') {
    return EMPTY_CONTENT;
  }
  let raw: unknown;
  try {
    raw = JSON.parse(content);
  } catch {
    return EMPTY_CONTENT;
  }
  if (typeof raw !== 'object' || raw === null) {
    return EMPTY_CONTENT;
  }
  const fields = raw as Record<string, unknown>;
  return {
    folder: typeof fields.folder === 'string' && fields.folder !== '' ? fields.folder : null,
    path: typeof fields.path === 'string' ? partsOf(fields.path).join('/') : '',
    showsHidden: fields.showsHidden === true,
  };
}

/** Writes what the view keeps. */
export function serializeContent(content: FilesContent): string {
  return JSON.stringify(content);
}

/** The parts of a path inside a folder, without empty ones. */
export function partsOf(path: string): string[] {
  return path.split(/[\\/]/).filter((part) => part !== '' && part !== '.');
}

/** The path of an entry inside the folder at `path`. */
export function childPath(path: string, name: string): string {
  return [...partsOf(path), name].join('/');
}

/** The path one folder up; the folder itself stays where it is. */
export function parentPath(path: string): string {
  return partsOf(path).slice(0, -1).join('/');
}

/** One step of the crumbs above the list: a name and where pressing it leads. */
export interface Crumb {
  name: string;
  path: string;
}

/** The crumbs of a path: the folder itself first, then each part. */
export function crumbsOf(folder: string, path: string): Crumb[] {
  const parts = partsOf(path);
  return [
    { name: lastPartOf(folder), path: '' },
    ...parts.map((name, index) => ({ name, path: parts.slice(0, index + 1).join('/') })),
  ];
}

/** The last part of a full path: the name of a folder of the project. */
export function lastPartOf(fullPath: string): string {
  const parts = partsOf(fullPath);
  return parts.at(-1) ?? fullPath;
}

/** The full path of an entry, written the way the folder is written. */
export function fullPathOf(folder: string, path: string, name?: string): string {
  const separator = folder.includes('\\') ? '\\' : '/';
  const parts = name === undefined ? partsOf(path) : [...partsOf(path), name];
  const base = folder.replace(/[\\/]+$/, '');
  return parts.length === 0 ? base : [base, ...parts].join(separator);
}

/** What an entry is, for the small mark beside its name. */
export type Mark = 'folder' | 'text' | 'picture' | 'sheet' | 'link' | 'other';

const TEXT_ENDINGS = new Set([
  'txt',
  'md',
  'markdown',
  'rst',
  'pdf',
  'doc',
  'docx',
  'odt',
  'rtf',
  'c',
  'h',
  'cpp',
  'hpp',
  'cc',
  'rs',
  'py',
  'ts',
  'tsx',
  'js',
  'jsx',
  'json',
  'toml',
  'yaml',
  'yml',
  'ini',
  'cfg',
  'xml',
  'html',
  'css',
  'sh',
  'ld',
  'mk',
  'cmake',
  'log',
  'kicad_sch',
  'kicad_pcb',
  'kicad_pro',
]);
const PICTURE_ENDINGS = new Set([
  'png',
  'jpg',
  'jpeg',
  'gif',
  'svg',
  'webp',
  'bmp',
  'tif',
  'tiff',
  'ico',
  'heic',
]);
const SHEET_ENDINGS = new Set(['csv', 'tsv', 'xls', 'xlsx', 'xlsm', 'ods', 'numbers']);
/** Names without an ending that are text all the same. */
const TEXT_NAMES = new Set(['makefile', 'readme', 'license', 'changelog', 'dockerfile']);

/** The mark of an entry: by its kind, and for a file by the ending of its name. */
export function markOf(entry: Pick<FileEntry, 'name' | 'kind'>): Mark {
  if (entry.kind === 'folder') {
    return 'folder';
  }
  if (entry.kind === 'link') {
    return 'link';
  }
  const name = entry.name.toLowerCase();
  const dot = name.lastIndexOf('.');
  if (dot <= 0) {
    return TEXT_NAMES.has(name) ? 'text' : 'other';
  }
  const ending = name.slice(dot + 1);
  if (TEXT_ENDINGS.has(ending)) {
    return 'text';
  }
  if (PICTURE_ENDINGS.has(ending)) {
    return 'picture';
  }
  if (SHEET_ENDINGS.has(ending)) {
    return 'sheet';
  }
  return 'other';
}

const KIBIBYTE = 1024;
const UNITS = ['KB', 'MB', 'GB', 'TB'];

/** Writes a size for people: "512 bytes", "2 KB", "1.4 MB". Nothing for a folder. */
export function formatSize(bytes: number | null): string {
  if (bytes === null) {
    return '';
  }
  if (bytes < KIBIBYTE) {
    return bytes === 1 ? '1 byte' : `${bytes.toString()} bytes`;
  }
  let size = bytes / KIBIBYTE;
  let unit = 0;
  while (size >= KIBIBYTE && unit < UNITS.length - 1) {
    size /= KIBIBYTE;
    unit += 1;
  }
  // One decimal while the number is small, so that 1.4 MB and 1.9 MB differ.
  const written = size < 10 ? size.toFixed(1).replace(/\.0$/, '') : Math.round(size).toString();
  return `${written} ${UNITS[unit] ?? ''}`;
}

/** The entries whose names hold the typed word, without regard to capitals. */
export function narrowed<Entry extends Pick<FileEntry, 'name'>>(
  entries: readonly Entry[],
  typed: string,
): Entry[] {
  const word = typed.trim().toLowerCase();
  if (word === '') {
    return [...entries];
  }
  return entries.filter((entry) => entry.name.toLowerCase().includes(word));
}

/**
 * The folder to show: the one kept if the project still has it, and
 * otherwise the first. Nothing when the project has no folder.
 */
export function chosenFolder(folders: readonly string[], kept: string | null): string | null {
  if (kept !== null && folders.includes(kept)) {
    return kept;
  }
  return folders[0] ?? null;
}
