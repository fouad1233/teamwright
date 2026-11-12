import { describe, expect, it } from 'vitest';

import { hashOf, lookOf, OUTFITS, roleOfMember, type Role } from './characterModel';
import { HAIRS, SKINS } from './palette';
import { HAIR_STYLES } from './sprites';

const role = (name: string, responsibility = '') => roleOfMember({ name, responsibility });

describe('roleOfMember', () => {
  it.each<[string, Role]>([
    ['Firmware engineer', 'software'],
    ['Embedded developer', 'software'],
    ['Software engineer', 'software'],
    ['Web app programmer', 'software'],
    ['Hardware engineer', 'hardware'],
    ['Electronics engineer', 'hardware'],
    ['Electrical designer', 'hardware'],
    ['PCB layout', 'hardware'],
    ['Mechanical engineer', 'mechanical'],
    ['Mechanical designer', 'mechanical'],
    ['CAD specialist', 'mechanical'],
    ['Industrial designer', 'designer'],
    ['Interface designer', 'designer'],
    ['UX lead', 'designer'],
    ['Test engineer', 'test'],
    ['Software test engineer', 'test'],
    ['QA', 'test'],
    ['Technical writer', 'writer'],
    ['Documentation', 'writer'],
    ['Researcher', 'researcher'],
    ['Research analyst', 'researcher'],
    ['Data scientist', 'researcher'],
  ])('sees in "%s" the %s character', (name, expected) => {
    expect(role(name)).toBe(expected);
  });

  it('reads the responsibility when the name says nothing', () => {
    expect(role('Sam Rivera', 'Writes the firmware of the sensor.')).toBe('software');
    expect(role('Lighting engineer', 'The enclosure, and how the board sits in it.')).toBe(
      'mechanical',
    );
  });

  it('prefers what the name says over the responsibility', () => {
    expect(role('Test engineer', 'Tests the firmware.')).toBe('test');
  });

  it('matches short words only whole', () => {
    expect(role('Builder')).toBe('default');
    expect(role('Squad lead')).toBe('default');
  });

  it('gives the default character to a role it does not know', () => {
    expect(role('Lighting engineer')).toBe('default');
    expect(role('')).toBe('default');
  });
});

describe('hashOf', () => {
  it('is the same for the same text and differs for another', () => {
    expect(hashOf('member-1')).toBe(hashOf('member-1'));
    expect(hashOf('member-1')).not.toBe(hashOf('member-2'));
  });
});

describe('lookOf', () => {
  const member = (id: string, name = 'Firmware engineer') =>
    lookOf({ kind: 'member', member: { id, name, responsibility: '', color: 'blue' } });

  it('stays the same for the same member between visits', () => {
    expect(member('a1b2')).toEqual(member('a1b2'));
  });

  it('gives two members of the same role different looks', () => {
    const looks = ['m-1', 'm-2', 'm-3', 'm-4', 'm-5', 'm-6'].map((id) => member(id));
    const distinct = new Set(looks.map((look) => `${look.skin}.${look.hair}.${look.hairStyle}`));
    expect(distinct.size).toBeGreaterThan(3);
  });

  it('keeps every choice within what is drawn', () => {
    for (let index = 0; index < 200; index += 1) {
      const look = member(`member-${index.toString()}`);
      expect(look.skin).toBeLessThan(SKINS.length);
      expect(look.hair).toBeLessThan(HAIRS.length);
      expect(HAIR_STYLES).toContain(look.hairStyle);
    }
  });

  it('dresses a member in its own color', () => {
    const look = lookOf({
      kind: 'member',
      member: { id: 'x', name: 'Test engineer', responsibility: '', color: 'teal' },
    });
    expect(look.shirt).toBe('teal');
    expect(look.role).toBe('test');
  });

  it('always draws the manager and the Advisor as the same people', () => {
    expect(lookOf({ kind: 'manager' })).toEqual(lookOf({ kind: 'manager' }));
    expect(lookOf({ kind: 'manager' }).shirt).toBe('brass');
    expect(lookOf({ kind: 'advisor' }).role).toBe('advisor');
  });

  it('tells every role apart by what it wears or what stands on its bench', () => {
    const outfits = Object.values(OUTFITS).map(
      (outfit) => `${outfit.garment}.${String(outfit.headwear)}.${outfit.prop}`,
    );
    expect(new Set(outfits).size).toBe(outfits.length);
    const props = Object.values(OUTFITS).map((outfit) => outfit.prop);
    expect(new Set(props).size).toBe(props.length);
  });
});
