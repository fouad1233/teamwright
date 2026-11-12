import type { Color, Member } from '@/shared/api/types';

import { HAIRS, SKINS } from './palette';
import { HAIR_STYLES, type Garment, type HairStyle, type Headwear, type PropName } from './sprites';

/*
 * Which character stands for whom (decision 0019). A member's role is
 * recognised from the words of its name and its responsibility; what tells
 * two members of the same role apart comes from its identifier, so that it
 * never changes between visits.
 */

/** The kinds of character the office knows. */
export type Role =
  | 'manager'
  | 'advisor'
  | 'software'
  | 'hardware'
  | 'mechanical'
  | 'designer'
  | 'test'
  | 'writer'
  | 'researcher'
  | 'default';

/** What a role wears and what stands on its bench. */
export interface Outfit {
  garment: Garment;
  headwear: Headwear | null;
  /** What stands on the bench in front of it. */
  prop: PropName;
}

/** What tells each role apart at a glance. Never a label: what it wears or works with. */
export const OUTFITS: Readonly<Record<Role, Outfit>> = {
  manager: { garment: 'jacket', headwear: null, prop: 'tray' },
  advisor: { garment: 'robe', headwear: 'glasses', prop: 'plan' },
  software: { garment: 'shirt', headwear: 'headphones', prop: 'laptop' },
  hardware: { garment: 'shirt', headwear: 'loupe', prop: 'solder' },
  mechanical: { garment: 'apron', headwear: null, prop: 'caliper' },
  designer: { garment: 'shirt', headwear: 'beret', prop: 'tablet' },
  test: { garment: 'labCoat', headwear: null, prop: 'scope' },
  writer: { garment: 'cardigan', headwear: null, prop: 'notebook' },
  researcher: { garment: 'vest', headwear: 'glasses', prop: 'papers' },
  default: { garment: 'shirt', headwear: null, prop: 'mug' },
};

/**
 * The words that tell a role, in the order the roles are looked for; the
 * first that matches wins. A stem matches every word it begins ("electr"
 * finds "electronics" and "electrical"); a short word matches only itself,
 * so that "ui" does not find "build".
 */
const ROLE_WORDS: readonly { role: Role; stems: readonly string[]; words: readonly string[] }[] = [
  // Before software and hardware: a "software test engineer" tests.
  { role: 'test', stems: ['test', 'quality', 'verif', 'validat'], words: ['qa'] },
  {
    role: 'writer',
    stems: ['writer', 'writing', 'document', 'manual', 'editor', 'author'],
    words: ['docs'],
  },
  {
    role: 'researcher',
    stems: ['research', 'analyst', 'analys', 'scientist', 'statistic'],
    words: ['data'],
  },
  // Before design: a "mechanical designer" is a mechanical engineer.
  {
    role: 'mechanical',
    stems: ['mechanic', 'enclosure', 'manufactur', 'housing'],
    words: ['cad', 'structural'],
  },
  {
    role: 'hardware',
    stems: ['hardware', 'electr', 'circuit', 'schematic', 'analog'],
    words: ['pcb', 'rf', 'power'],
  },
  {
    role: 'designer',
    stems: ['design', 'interface', 'industrial', 'visual', 'graphic'],
    words: ['ux', 'ui'],
  },
  {
    role: 'software',
    stems: [
      'software',
      'firmware',
      'embedded',
      'develop',
      'program',
      'coder',
      'backend',
      'frontend',
    ],
    words: ['code', 'web', 'app', 'apps', 'devops', 'stack'],
  },
];

/** The words of a text, in lower case, without punctuation. */
function wordsOf(text: string): string[] {
  return text
    .toLowerCase()
    .split(/[^\p{L}\p{N}]+/u)
    .filter((word) => word !== '');
}

/** The role a text speaks of, if any. */
function roleInWords(text: string): Role | null {
  const found = wordsOf(text);
  for (const { role, stems, words } of ROLE_WORDS) {
    const matches = (word: string) =>
      words.includes(word) || stems.some((stem) => word.startsWith(stem));
    if (found.some(matches)) {
      return role;
    }
  }
  return null;
}

/**
 * The role of a member of the team: what its name says, or else what its
 * responsibility says, or else the default. A person is recognised the same
 * way as an AI specialist.
 */
export function roleOfMember(member: Pick<Member, 'name' | 'responsibility'>): Role {
  return roleInWords(member.name) ?? roleInWords(member.responsibility) ?? 'default';
}

/** A number from a text that is always the same for the same text (FNV-1a). */
export function hashOf(text: string): number {
  let hash = 0x811c9dc5;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= text.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return hash >>> 0;
}

/** Everything that decides how one character is drawn. */
export interface Look {
  role: Role;
  /** An index into SKINS. */
  skin: number;
  /** An index into HAIRS. */
  hair: number;
  hairStyle: HairStyle;
  /** The color of the shirt: the member's own, or the gold of the manager. */
  shirt: Color | 'brass';
  /** A number that sets this character's own rhythm, so that a room does not move in step. */
  seed: number;
}

/** Who a character stands for. */
export type Sitter =
  | { kind: 'manager' }
  | { kind: 'advisor' }
  | { kind: 'member'; member: Pick<Member, 'id' | 'name' | 'responsibility' | 'color'> };

/** The gray of HAIRS, which the Advisor always has. */
const GRAY_HAIR = 4;

/**
 * The look of a character. The manager and the Advisor are always the same
 * people. A member's skin, hair and the way it wears it come from its
 * identifier, each from a different part of the number, so they vary
 * independently; its shirt is its own color.
 */
export function lookOf(sitter: Sitter): Look {
  switch (sitter.kind) {
    case 'manager':
      return { role: 'manager', skin: 1, hair: 1, hairStyle: 'parted', shirt: 'brass', seed: 7 };
    case 'advisor':
      return {
        role: 'advisor',
        skin: 2,
        hair: GRAY_HAIR,
        hairStyle: 'short',
        shirt: 'gray',
        seed: 13,
      };
    case 'member': {
      const { member } = sitter;
      const hash = hashOf(member.id);
      return {
        role: roleOfMember(member),
        skin: hash % SKINS.length,
        hair: Math.floor(hash / 7) % HAIRS.length,
        hairStyle: HAIR_STYLES[Math.floor(hash / 61) % HAIR_STYLES.length] ?? 'short',
        shirt: member.color,
        seed: hash,
      };
    }
  }
}
