/**
 * The integrations of the projects of the demo: what the manager set for
 * GitHub, a Slack channel and the web addresses that listen. The demo
 * sends nothing anywhere; it keeps what was set, and checks it the way the
 * real application does.
 */

import { BackendError } from '../backend';
import type {
  Deliveries,
  Delivery,
  EventKind,
  ListeningAddress,
  ProjectIntegrations,
} from '../types';

/** Every kind of event, in the order the interface lists them. */
export const EVENT_KINDS: readonly EventKind[] = [
  'task.created',
  'task.status_changed',
  'task.finished',
  'agent.waiting',
  'agent.stopped',
  'sprint.started',
  'sprint.completed',
];

/** The kinds a new Slack channel is told. */
const TOLD_IN_A_CHANNEL: readonly EventKind[] = [
  'task.finished',
  'agent.waiting',
  'agent.stopped',
  'sprint.started',
  'sprint.completed',
];

/** The most web addresses a project sends to. */
export const MOST_WEB_ADDRESSES = 5;

interface Kept {
  repository: string | null;
  slack: { address: string; events: Set<EventKind> } | null;
  webAddresses: { address: string; secret: string | null; events: Set<EventKind> }[];
}

/** An address that the real application would refuse, refused the same way. */
function requireAddress(text: string, field: string): string {
  const address = text.trim();
  const host = address.startsWith('https://') ? (address.slice(8).split(/[/?#]/)[0] ?? '') : '';
  if (host === '' || host.startsWith('.') || host.startsWith(':') || /\s/.test(address)) {
    throw new BackendError(
      'rule_broken',
      `${field} starts with https:// and names a host, with no spaces`,
    );
  }
  return address;
}

/** The host of an address with the rest hidden, as the real application sends it back. */
export function shownAddress(address: string): string {
  const rest = address.slice('https://'.length);
  const host = rest.split(/[/?#]/)[0] ?? '';
  return host.length === rest.length ? `https://${host}` : `https://${host}/…`;
}

function requireRepository(text: string): string {
  const name = text.trim();
  const part = /^[A-Za-z0-9._-]{1,100}$/;
  const [owner, repository, more] = name.split('/');
  const isPart = (value: string | undefined) =>
    value !== undefined && part.test(value) && value !== '.' && value !== '..';
  if (more !== undefined || !isPart(owner) || !isPart(repository)) {
    throw new BackendError(
      'rule_broken',
      'a repository is named as owner/name, such as octo-org/sensor-node',
    );
  }
  return name;
}

function sorted(events: Set<EventKind>): EventKind[] {
  return EVENT_KINDS.filter((kind) => events.has(kind));
}

/** What the demo keeps of the integrations of each project. */
export class PretendedIntegrations {
  private readonly kept = new Map<string, Kept>();
  private readonly delivered = new Map<string, Delivery[]>();

  /** The integrations of a project as the interface sees them. */
  of(projectId: string): ProjectIntegrations {
    const kept = this.keptOf(projectId);
    return {
      repository: kept.repository,
      slack:
        kept.slack === null
          ? null
          : { address: shownAddress(kept.slack.address), events: sorted(kept.slack.events) },
      webAddresses: kept.webAddresses.map((listening): ListeningAddress => ({
        address: shownAddress(listening.address),
        isSigned: listening.secret !== null,
        events: sorted(listening.events),
      })),
    };
  }

  /** The address of the Slack channel of a project, in full, if one is set. */
  slackAddressOf(projectId: string): string | null {
    return this.keptOf(projectId).slack?.address ?? null;
  }

  setRepository(projectId: string, name: string | null): ProjectIntegrations {
    const kept = this.keptOf(projectId);
    kept.repository = name === null || name.trim() === '' ? null : requireRepository(name);
    return this.of(projectId);
  }

  setSlackAddress(projectId: string, address: string | null): ProjectIntegrations {
    const kept = this.keptOf(projectId);
    if (address === null || address.trim() === '') {
      kept.slack = null;
    } else {
      kept.slack = {
        address: requireAddress(address, 'the address of a channel'),
        events: kept.slack?.events ?? new Set(TOLD_IN_A_CHANNEL),
      };
    }
    return this.of(projectId);
  }

  setSlackEvent(projectId: string, kind: EventKind, on: boolean): ProjectIntegrations {
    const slack = this.keptOf(projectId).slack;
    if (slack === null) {
      throw new BackendError('no_channel', 'no Slack channel is set for this project');
    }
    choose(slack.events, kind, on);
    return this.of(projectId);
  }

  addWebAddress(projectId: string, address: string, secret: string | null): ProjectIntegrations {
    const kept = this.keptOf(projectId);
    const checked = requireAddress(address, 'a web address');
    if (kept.webAddresses.length >= MOST_WEB_ADDRESSES) {
      throw new BackendError(
        'rule_broken',
        `a project sends to at most ${MOST_WEB_ADDRESSES.toString()} web addresses`,
      );
    }
    kept.webAddresses.push({
      address: checked,
      secret: secret === null || secret.trim() === '' ? null : secret.trim(),
      events: new Set(EVENT_KINDS),
    });
    return this.of(projectId);
  }

  removeWebAddress(projectId: string, index: number): ProjectIntegrations {
    const kept = this.keptOf(projectId);
    this.requireAddressAt(kept, index);
    kept.webAddresses.splice(index, 1);
    return this.of(projectId);
  }

  setWebAddressEvent(
    projectId: string,
    index: number,
    kind: EventKind,
    on: boolean,
  ): ProjectIntegrations {
    choose(this.requireAddressAt(this.keptOf(projectId), index).events, kind, on);
    return this.of(projectId);
  }

  private requireAddressAt(kept: Kept, index: number) {
    const listening = kept.webAddresses[index];
    if (listening === undefined) {
      throw new BackendError(
        'not_found',
        `there is no web address with the identifier ${(index + 1).toString()}`,
      );
    }
    return listening;
  }

  /** What was delivered: the demo pretends every test reached its address. */
  deliveriesOf(projectId: string): Deliveries {
    const kept = this.keptOf(projectId);
    return {
      channelFailure: null,
      addresses: kept.webAddresses.map((listening) => [
        ...(this.delivered.get(`${projectId} ${listening.address}`) ?? []),
      ]),
    };
  }

  /** Pretends to send a line to the channel. */
  testChannel(projectId: string, at: string): Delivery {
    if (this.keptOf(projectId).slack === null) {
      throw new BackendError('no_channel', 'no Slack channel is set for this project');
    }
    return { at, event: 'test', status: 200, succeeded: true, failure: null };
  }

  /** Pretends to send the event `test` to an address, and keeps it with its deliveries. */
  testWebAddress(projectId: string, index: number, at: string): Delivery {
    const listening = this.requireAddressAt(this.keptOf(projectId), index);
    const delivery: Delivery = { at, event: 'test', status: 200, succeeded: true, failure: null };
    const key = `${projectId} ${listening.address}`;
    this.delivered.set(key, [delivery, ...(this.delivered.get(key) ?? [])].slice(0, 10));
    return delivery;
  }

  private keptOf(projectId: string): Kept {
    let kept = this.kept.get(projectId);
    if (kept === undefined) {
      kept = { repository: null, slack: null, webAddresses: [] };
      this.kept.set(projectId, kept);
    }
    return kept;
  }
}

function choose(events: Set<EventKind>, kind: EventKind, on: boolean) {
  if (on) {
    events.add(kind);
  } else {
    events.delete(kind);
  }
}
