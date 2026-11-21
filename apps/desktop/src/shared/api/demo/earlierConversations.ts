import { BackendError } from '../backend';
import type { ConversationsOfFolder, Digest, DigestDraft, EarlierConversation, Id } from '../types';
import { isInside, isSameFolder } from './places';

/**
 * Earlier conversations with Claude Code, as the demo pretends to find them
 * on the computer, and the digests the manager keeps of them (decision
 * 0041).
 *
 * The real application reads them where Claude Code keeps them, and makes
 * a digest by the rules in `crates/domain/src/digest.rs`, which are tested
 * there. The demo makes its digests in the same form, with simpler rules,
 * so that the whole flow shows in a browser. Every conversation here is
 * made up.
 */

/** A made-up earlier conversation, read through. */
export interface PretendedConversation {
  source: string;
  /**
   * The folder it was held in. `null` for the work folder of the project it
   * is shown with, so that it is found among the folders of the project.
   */
  heldIn: string | null;
  title: string | null;
  requests: string[];
  changedFiles: string[];
  lastAnswer: string | null;
  startedAt: string;
  endedAt: string;
  sizeBytes: number;
}

const FIRST_REQUEST_LENGTH = 600;
const REQUEST_LENGTH = 200;
const MOST_REQUESTS = 16;
const MOST_FILES = 30;
const LAST_ANSWER_LENGTH = 600;
const FIRST_REQUEST_IN_LIST = 240;
const TITLE_LENGTH = 120;
const TITLE_FROM_REQUEST_LENGTH = 80;
const DIGEST_LENGTH = 8000;

/** The earlier conversations and the digests kept of them. */
export class PretendedEarlierConversations {
  private readonly conversations: PretendedConversation[];
  private readonly digests: Digest[] = [];

  constructor(conversations: readonly PretendedConversation[]) {
    this.conversations = structuredClone([...conversations]);
  }

  /**
   * The conversations grouped by folder, the folders of the project first,
   * then the others with the newest conversation first.
   */
  find(projectId: Id, projectFolders: readonly string[]): ConversationsOfFolder[] {
    const workFolder = projectFolders[0] ?? '';
    const kept = new Set(
      this.digests
        .filter((digest) => digest.projectId === projectId)
        .map((digest) => digest.source),
    );
    const folders: ConversationsOfFolder[] = [];
    for (const conversation of this.conversations) {
      const heldIn = conversation.heldIn ?? workFolder;
      const firstRequest = conversation.requests[0];
      const card: EarlierConversation = {
        source: conversation.source,
        heldIn,
        title: conversation.title,
        firstRequest:
          firstRequest === undefined
            ? null
            : cut(oneLine(hideSecrets(firstRequest)), FIRST_REQUEST_IN_LIST),
        startedAt: conversation.startedAt,
        endedAt: conversation.endedAt,
        sizeBytes: conversation.sizeBytes,
        isKept: kept.has(conversation.source),
      };
      const folder = folders.find((known) => known.folder === heldIn);
      if (folder) {
        folder.conversations.push(card);
      } else {
        folders.push({
          folder: heldIn,
          isOfProject: projectFolders.some(
            (known) => isSameFolder(known, heldIn) || isInside(heldIn, known),
          ),
          conversations: [card],
        });
      }
    }
    for (const folder of folders) {
      folder.conversations.sort((one, other) => newest(other).localeCompare(newest(one)));
    }
    return folders.sort(
      (one, other) =>
        Number(other.isOfProject) - Number(one.isOfProject) ||
        newestOf(other).localeCompare(newestOf(one)) ||
        one.folder.localeCompare(other.folder),
    );
  }

  /** The digest of a conversation, made by the rules. */
  draft(source: string, workFolder: string): DigestDraft {
    const conversation = this.conversations.find((known) => known.source === source);
    if (!conversation) {
      throw new BackendError(
        'not_found',
        `there is no earlier conversation with the identifier ${source}`,
      );
    }
    return draftOf({ ...conversation, heldIn: conversation.heldIn ?? workFolder });
  }

  /** Keeps a digest with a project, as the manager corrected it. */
  keep(projectId: Id, draft: DigestDraft, id: Id, now: string): Digest {
    const title = oneLine(draft.title);
    if (title === '') {
      throw new BackendError('rule_broken', 'the title of the digest must not be empty');
    }
    if (title.length > TITLE_LENGTH) {
      throw new BackendError(
        'rule_broken',
        `the title of the digest can have at most ${TITLE_LENGTH.toString()} characters, but this one has ${title.length.toString()}`,
      );
    }
    const text = draft.text.replaceAll('\r\n', '\n').replaceAll('\r', '\n').trim();
    if (text === '') {
      throw new BackendError('rule_broken', 'the digest must not be empty');
    }
    if (text.length > DIGEST_LENGTH) {
      throw new BackendError(
        'rule_broken',
        `the digest can have at most ${DIGEST_LENGTH.toString()} characters, but this one has ${text.length.toString()}`,
      );
    }
    const earlier = this.digests.find(
      (digest) => digest.projectId === projectId && digest.source === draft.source,
    );
    if (earlier) {
      throw new BackendError(
        'digest_already_kept',
        `a digest of this conversation is kept with the project already: "${earlier.title}"`,
      );
    }
    const digest: Digest = {
      id,
      projectId,
      source: draft.source,
      heldIn: draft.heldIn.trim(),
      title,
      text,
      startedAt: draft.startedAt,
      endedAt: draft.endedAt,
      keptAt: now,
    };
    this.digests.push(digest);
    return structuredClone(digest);
  }

  /** The digests of a project, the one kept last first. */
  digestsOf(projectId: Id): Digest[] {
    return structuredClone(
      this.digests
        .filter((digest) => digest.projectId === projectId)
        .sort(
          (one, other) => other.keptAt.localeCompare(one.keptAt) || other.id.localeCompare(one.id),
        ),
    );
  }

  /** Removes a digest. */
  remove(digestId: Id): void {
    const place = this.digests.findIndex((digest) => digest.id === digestId);
    if (place < 0) {
      throw new BackendError('not_found', `there is no digest with the identifier ${digestId}`);
    }
    this.digests.splice(place, 1);
  }

  /** Forgets the digests of a project that is gone. */
  forgetProject(projectId: Id): void {
    for (let place = this.digests.length - 1; place >= 0; place -= 1) {
      if (this.digests[place]?.projectId === projectId) {
        this.digests.splice(place, 1);
      }
    }
  }
}

function newest(card: EarlierConversation): string {
  return card.endedAt ?? card.startedAt ?? '';
}

function newestOf(folder: ConversationsOfFolder): string {
  return folder.conversations.map(newest).sort().at(-1) ?? '';
}

/** The digest of a conversation, in the form the real application gives it. */
function draftOf(conversation: PretendedConversation & { heldIn: string }): DigestDraft {
  const requests = conversation.requests
    .map((request) => oneLine(hideSecrets(request)))
    .filter((request) => request !== '');
  const [first, ...later] = requests;
  if (first === undefined) {
    throw new BackendError(
      'rule_broken',
      'the conversation holds no request of yours, so there is nothing to digest',
    );
  }
  const lines = [heldLine(conversation, requests.length), '', 'First request:'];
  lines.push(cut(first, FIRST_REQUEST_LENGTH));
  if (later.length > 0) {
    lines.push('', 'Then, in order:');
    const numbered = (request: string, place: number) =>
      `${(place + 2).toString()}. ${cut(request, REQUEST_LENGTH)}`;
    if (later.length <= MOST_REQUESTS) {
      lines.push(...later.map(numbered));
    } else {
      const head = MOST_REQUESTS / 2;
      const tail = MOST_REQUESTS - head;
      lines.push(...later.slice(0, head).map(numbered));
      lines.push(
        `(the ${(later.length - head - tail).toString()} requests in between are left out)`,
      );
      lines.push(
        ...later
          .slice(later.length - tail)
          .map((request, place) => numbered(request, later.length - tail + place)),
      );
    }
  }
  const files = [
    ...new Set(conversation.changedFiles.map((file) => shownFile(file, conversation.heldIn))),
  ];
  if (files.length > 0) {
    lines.push('', `Files written or changed (${files.length.toString()}):`);
    lines.push(...files.slice(0, MOST_FILES).map((file) => `- ${file}`));
    if (files.length > MOST_FILES) {
      lines.push(`- and ${(files.length - MOST_FILES).toString()} more`);
    }
  }
  const answer =
    conversation.lastAnswer === null ? '' : oneLine(hideSecrets(conversation.lastAnswer));
  if (answer !== '') {
    lines.push('', 'Last answer:', cut(answer, LAST_ANSWER_LENGTH));
  }
  const title = conversation.title === null ? '' : oneLine(hideSecrets(conversation.title));
  return {
    source: conversation.source,
    heldIn: conversation.heldIn,
    title: cut(title === '' ? cut(first, TITLE_FROM_REQUEST_LENGTH) : title, TITLE_LENGTH),
    text: lines.join('\n').slice(0, DIGEST_LENGTH),
    startedAt: conversation.startedAt,
    endedAt: conversation.endedAt,
  };
}

/** The first line of a digest: where, when, for how long, and how many requests. */
function heldLine(
  conversation: PretendedConversation & { heldIn: string },
  requests: number,
): string {
  const start = conversation.startedAt.slice(0, 10);
  const end = conversation.endedAt.slice(0, 10);
  const days =
    Math.round((Date.parse(`${end}T00:00:00Z`) - Date.parse(`${start}T00:00:00Z`)) / 86_400_000) +
    1;
  const when =
    start === end ? ` on ${start}` : ` from ${start} to ${end} (${days.toString()} days)`;
  const count = requests === 1 ? '1 request' : `${requests.toString()} requests`;
  return `Held in ${conversation.heldIn}${when}, ${count} of the manager.`;
}

/** A file by its path from the folder of the conversation, or by its name alone. */
function shownFile(file: string, folder: string): string {
  const path = file.replaceAll('\\', '/');
  const root = folder.replaceAll('\\', '/').replace(/\/+$/, '');
  if (path.toLowerCase().startsWith(`${root.toLowerCase()}/`)) {
    return path.slice(root.length + 1);
  }
  return path.split('/').at(-1) ?? path;
}

function oneLine(text: string): string {
  return text.split(/\s+/).filter(Boolean).join(' ');
}

/** Cuts a line to at most `limit` characters, at a word if that keeps half, and marks the cut. */
function cut(text: string, limit: number): string {
  if (text.length <= limit) {
    return text;
  }
  const kept = text.slice(0, limit - 1);
  const space = kept.lastIndexOf(' ');
  return `${(space * 2 >= limit ? kept.slice(0, space) : kept).trimEnd()}…`;
}

/** The demo's short form of the rules that hide what looks like a secret. */
export function hideSecrets(text: string): string {
  return text
    .replace(/\b(?:sk-|ghp_|github_pat_|xoxb-|AKIA|AIza|glpat-)[A-Za-z0-9_.-]{12,}/g, '[hidden]')
    .replace(
      /\b([A-Za-z_-]*(?:password|passwd|secret|token|api[_-]?key))([=:]\s*)\S+/gi,
      '$1$2[hidden]',
    );
}

/** Moments a number of days and hours before `today`. */
function daysBefore(today: Date, days: number, hour: number): string {
  const moment = new Date(today);
  moment.setUTCDate(moment.getUTCDate() - days);
  moment.setUTCHours(hour, 0, 0, 0);
  return moment.toISOString();
}

/**
 * The earlier conversations of the sample: three about the device of the
 * sample project, held in its folder, and others held elsewhere on the
 * pretended computer. Made up, every one.
 */
export function sampleEarlierConversations(today: Date = new Date()): PretendedConversation[] {
  const at = (days: number, hour: number) => daysBefore(today, days, hour);
  return [
    {
      source: '-Users-you-Documents-Teamwright-SSN/7c1e0a52-sensor-choice',
      heldIn: null,
      title: 'Choosing the sensors for the air node',
      requests: [
        'I want to build a small device that measures the air in a room: CO2, temperature and humidity, with a little screen. Help me choose the sensors. It should cost less than 40 euros in parts.',
        'Compare the SCD41 with the MH-Z19C for CO2.',
        'The SCD41 it is. What about temperature and humidity, is the one inside the SCD41 good enough?',
        'Add a BME680 then, I want air quality too.',
        'Write the list of parts into parts.md with prices.',
      ],
      changedFiles: ['C:\\Users\\you\\Documents\\Teamwright\\SSN\\parts.md'],
      lastAnswer:
        'The list of parts is in parts.md: an ESP32-C3, the SCD41 for CO2, a BME680 for air quality, a 1.54 inch e-paper screen. The parts come to about 36 euros.',
      startedAt: at(24, 9),
      endedAt: at(23, 17),
      sizeBytes: 182_400,
    },
    {
      source: '-Users-you-Documents-Teamwright-SSN/9d4b11f0-power',
      heldIn: null,
      title: null,
      requests: [
        'Can the node run on a battery for a month? It measures every five minutes.',
        'No battery then. USB-C power only for the first version.',
      ],
      changedFiles: [],
      lastAnswer:
        'With the SCD41 measuring every five minutes and Wi-Fi sending each value, a month on a small battery is not realistic. USB-C power keeps the first version simple.',
      startedAt: at(20, 14),
      endedAt: at(20, 15),
      sizeBytes: 41_900,
    },
    {
      source: '-Users-you-Documents-Teamwright-SSN-firmware/2a7f3c88-firmware-start',
      heldIn: 'C:\\Users\\you\\Documents\\Teamwright\\SSN\\firmware',
      title: 'First firmware for the ESP32-C3',
      requests: [
        'Set up a PlatformIO project for the ESP32-C3 that reads the SCD41 over I2C and prints the CO2 value.',
        'Use pins 4 and 5 for I2C.',
        'It works. The Wi-Fi password is WIFI_PASSWORD=hunter2-living-room, put it in a config file that git ignores.',
      ],
      changedFiles: [
        'C:\\Users\\you\\Documents\\Teamwright\\SSN\\firmware\\platformio.ini',
        'C:\\Users\\you\\Documents\\Teamwright\\SSN\\firmware\\src\\main.cpp',
        'C:\\Users\\you\\Documents\\Teamwright\\SSN\\firmware\\src\\config.h',
        'C:\\Users\\you\\Documents\\Teamwright\\SSN\\firmware\\.gitignore',
      ],
      lastAnswer:
        'The Wi-Fi settings are read from src/config.h, which .gitignore now leaves out of git.',
      startedAt: at(12, 10),
      endedAt: at(11, 18),
      sizeBytes: 356_700,
    },
    {
      source: '-Users-you-Projects-weather-station/41aa90e2-station',
      heldIn: 'C:\\Users\\you\\Projects\\weather-station',
      title: 'Weather station on the balcony',
      requests: [
        'Plan a weather station for my balcony with wind speed and rain.',
        'Make it solar powered.',
      ],
      changedFiles: ['C:\\Users\\you\\Projects\\weather-station\\plan.md'],
      lastAnswer: 'The plan is in plan.md.',
      startedAt: at(60, 9),
      endedAt: at(58, 12),
      sizeBytes: 98_000,
    },
    {
      source: '-Users-you-Projects-weather-station/5b02c7d1-rain',
      heldIn: 'C:\\Users\\you\\Projects\\weather-station',
      title: null,
      requests: ['How does a tipping bucket rain gauge count?'],
      changedFiles: [],
      lastAnswer: 'Each tip of the bucket closes a reed switch once.',
      startedAt: at(55, 20),
      endedAt: at(55, 20),
      sizeBytes: 12_300,
    },
    {
      source: '-Users-you-Projects-website/0e66d2b9-site',
      heldIn: 'C:\\Users\\you\\Projects\\website',
      title: 'A page for the workshop',
      requests: ['Make a one-page website for my workshop, plain HTML.'],
      changedFiles: ['C:\\Users\\you\\Projects\\website\\index.html'],
      lastAnswer: 'index.html is ready.',
      startedAt: at(5, 16),
      endedAt: at(5, 17),
      sizeBytes: 64_100,
    },
    {
      source: '-Users-you/d3f4e5a6-question',
      heldIn: 'C:\\Users\\you',
      title: null,
      requests: [
        'What is the difference between I2C and SPI, in short?',
        'And which one is better for a screen?',
      ],
      changedFiles: [],
      lastAnswer: 'For a screen SPI is usually better: it is faster.',
      startedAt: at(2, 21),
      endedAt: at(2, 21),
      sizeBytes: 8_700,
    },
  ];
}
