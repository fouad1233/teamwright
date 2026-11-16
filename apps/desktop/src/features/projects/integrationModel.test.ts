import { describe, expect, it } from 'vitest';

import type { ProjectIntegrations } from '@/shared/api/types';

import { EVENT_LABELS, chosenInWords, slackInWords, webAddressesInWords } from './integrationModel';

const none: ProjectIntegrations = { repository: null, slack: null, webAddresses: [] };

describe('the words of the integrations', () => {
  it('counts the chosen kinds of events', () => {
    expect(chosenInWords([])).toBe('No events');
    expect(chosenInWords(['task.finished'])).toBe('1 kind of event');
    expect(chosenInWords(['task.finished', 'agent.waiting'])).toBe('2 kinds of events');
    expect(chosenInWords(EVENT_LABELS.map(({ kind }) => kind))).toBe('Every event');
  });

  it('says whether each integration is on', () => {
    expect(slackInWords(none)).toBe('Off');
    expect(
      slackInWords({ ...none, slack: { address: 'https://hooks.slack.com/…', events: [] } }),
    ).toBe('On: https://hooks.slack.com/…');
    expect(webAddressesInWords(none)).toBe('Off');
    const listening = { address: 'https://example.com/…', isSigned: false, events: [] };
    expect(webAddressesInWords({ ...none, webAddresses: [listening] })).toBe('On: 1 address');
    expect(webAddressesInWords({ ...none, webAddresses: [listening, listening] })).toBe(
      'On: 2 addresses',
    );
  });
});
