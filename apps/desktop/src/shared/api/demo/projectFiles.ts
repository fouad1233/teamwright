import { BackendError } from '../backend';
import type { FileEntry, FolderListing } from '../types';
import { isSameFolder } from './places';

/**
 * The files the browser demo pretends lie in the folders of its projects,
 * so that the files view shows well without a disk. The real application
 * reads the disk, and keeps the same rules: a path may not leave its
 * folder, the noise of the tools is left out, folders come first.
 */

/** A pretended file: its size, and how many hours before now it was changed. */
interface PretendedFile {
  size: number;
  hoursAgo: number;
}

/** A pretended folder: its entries by name. A string is a link that leads outside. */
interface PretendedFolder {
  [name: string]: PretendedFolder | PretendedFile | string;
}

const HOUR = 3_600_000;

function file(size: number, hoursAgo: number): PretendedFile {
  return { size, hoursAgo };
}

function isFile(entry: PretendedFolder | PretendedFile | string): entry is PretendedFile {
  return typeof entry === 'object' && typeof entry.size === 'number' && 'hoursAgo' in entry;
}

/** How many hours ago the newest file in a folder was changed, at any depth. */
function hoursSinceNewest(folder: PretendedFolder): number | null {
  let newest: number | null = null;
  for (const entry of Object.values(folder)) {
    const hours =
      typeof entry === 'string' ? null : isFile(entry) ? entry.hoursAgo : hoursSinceNewest(entry);
    if (hours !== null && (newest === null || hours < newest)) {
      newest = hours;
    }
  }
  return newest;
}

/** The work folder of the sample project: firmware, hardware and documents. */
const SENSOR_NODE: PretendedFolder = {
  '.git': {},
  '.gitignore': file(214, 300),
  'README.md': file(3_412, 5),
  firmware: {
    'CMakeLists.txt': file(1_208, 30),
    src: {
      'main.c': file(6_840, 2),
      'bme680.c': file(11_532, 3),
      'bme680.h': file(2_190, 3),
      'radio.c': file(8_744, 26),
      'power.c': file(4_302, 52),
    },
    include: { 'board.h': file(1_655, 52) },
    tests: { 'test_bme680.c': file(5_021, 4) },
    build: {},
    target: {},
  },
  hardware: {
    'sensor-node.kicad_pro': file(9_310, 74),
    'sensor-node.kicad_sch': file(486_112, 20),
    'sensor-node.kicad_pcb': file(1_904_377, 20),
    'bill-of-materials.csv': file(4_118, 19),
    'board-top.png': file(812_440, 19),
    'board-bottom.png': file(798_015, 19),
  },
  docs: {
    'requirements.md': file(7_904, 120),
    'power-budget.xlsx': file(28_772, 44),
    'test-plan.md': file(5_230, 8),
    'enclosure-sketch.jpg': file(1_240_558, 170),
  },
  references: {
    'BME680 datasheet.pdf': file(2_771_200, 96),
    'nRF52840 product specification.pdf': file(9_830_400, 96),
  },
  'vendor-libraries': 'C:\\Users\\you\\Libraries',
};

/** A folder of data sheets the demo hands out when the manager picks a folder. */
const DATASHEETS: PretendedFolder = {
  Bosch: {
    'BME680 datasheet.pdf': file(2_771_200, 400),
    'BME280 datasheet.pdf': file(1_966_080, 900),
  },
  Nordic: { 'nRF52840 product specification.pdf': file(9_830_400, 600) },
  'TPS62740 regulator.pdf': file(1_403_000, 700),
};

/** The pretended folders, by their full paths. */
const PRETENDED_FOLDERS: readonly (readonly [string, PretendedFolder])[] = [
  ['C:\\Users\\you\\Documents\\Teamwright\\SSN', SENSOR_NODE],
  ['C:\\Users\\you\\Projects\\sensor-node', SENSOR_NODE],
  ['C:\\Users\\you\\Documents\\Datasheets', DATASHEETS],
];

/** The names the tools keep for themselves, never shown. */
const NOISE = new Set(['.git', 'node_modules', 'target']);

/** The parts of a path inside a folder, refused if it leaves the folder by how it is written. */
function partsInside(path: string): string[] {
  const refused = () =>
    new BackendError('rule_broken', `the path ${path} does not stay inside the folder`);
  if (path.startsWith('/') || path.startsWith('\\')) {
    throw refused();
  }
  const parts = path.split(/[\\/]/).filter((part) => part !== '' && part !== '.');
  if (parts.some((part) => part.replace(/[ .]+$/, '') === '' || part.includes(':'))) {
    throw refused();
  }
  return parts;
}

/**
 * What lies in a pretended folder at a path inside it. A folder the demo
 * has no files for is empty.
 */
export function pretendedListing(
  folder: string,
  path: string,
  showsHidden: boolean,
  now: Date,
): FolderListing {
  const parts = partsInside(path);
  let current: PretendedFolder =
    PRETENDED_FOLDERS.find(([known]) => isSameFolder(known, folder))?.[1] ?? {};
  for (const part of parts) {
    const next = current[part];
    if (typeof next === 'string') {
      throw new BackendError('rule_broken', `the path ${path} does not stay inside the folder`);
    }
    if (next === undefined || isFile(next)) {
      throw new BackendError('not_found', `nothing that can be opened lies at ${path}`);
    }
    current = next;
  }
  const entries: FileEntry[] = Object.entries(current)
    .filter(([name]) => !NOISE.has(name) && (showsHidden || !name.startsWith('.')))
    .map(([name, entry]): FileEntry => {
      if (typeof entry === 'string') {
        return { name, kind: 'link', size: null, changedAt: null };
      }
      if (isFile(entry)) {
        return {
          name,
          kind: 'file',
          size: entry.size,
          changedAt: new Date(now.getTime() - entry.hoursAgo * HOUR).toISOString(),
        };
      }
      const hours = hoursSinceNewest(entry);
      return {
        name,
        kind: 'folder',
        size: null,
        changedAt: hours === null ? null : new Date(now.getTime() - hours * HOUR).toISOString(),
      };
    })
    .sort(
      (first, second) =>
        Number(second.kind === 'folder') - Number(first.kind === 'folder') ||
        first.name.toLowerCase().localeCompare(second.name.toLowerCase()),
    );
  return { entries, hasMore: false };
}
