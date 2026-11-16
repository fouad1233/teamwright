import type { EventKind, ProjectIntegrations } from '@/shared/api/types';

/** Every kind of event, in the order they are offered, with how the interface names it. */
export const EVENT_LABELS: readonly { kind: EventKind; label: string }[] = [
  { kind: 'task.created', label: 'A task is opened' },
  { kind: 'task.status_changed', label: 'A task changes its status' },
  { kind: 'task.finished', label: 'A specialist finishes a task' },
  { kind: 'agent.waiting', label: 'Someone waits for you' },
  { kind: 'agent.stopped', label: 'A specialist stops' },
  { kind: 'sprint.started', label: 'A sprint starts' },
  { kind: 'sprint.completed', label: 'A sprint is completed' },
];

/** How many of the kinds of events are chosen, in words for the button that changes them. */
export function chosenInWords(events: readonly EventKind[]): string {
  if (events.length === 0) {
    return 'No events';
  }
  if (events.length === EVENT_LABELS.length) {
    return 'Every event';
  }
  return events.length === 1 ? '1 kind of event' : `${events.length.toString()} kinds of events`;
}

/** The one line that says whether the Slack channel is on. */
export function slackInWords(integrations: ProjectIntegrations): string {
  return integrations.slack === null ? 'Off' : `On: ${integrations.slack.address}`;
}

/** The one line that says whether events are sent to web addresses. */
export function webAddressesInWords(integrations: ProjectIntegrations): string {
  const count = integrations.webAddresses.length;
  if (count === 0) {
    return 'Off';
  }
  return count === 1 ? 'On: 1 address' : `On: ${count.toString()} addresses`;
}
