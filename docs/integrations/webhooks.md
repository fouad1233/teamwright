# Web addresses that listen

A project can send what happens in it to up to five web addresses. This is
what Zapier ("Catch Hook"), Make, n8n and a company's own tools take in.
The manager adds an address under "Integrations…" in the project's menu,
chooses which events it hears, and can send it a test event. Why it works
this way is in [decision 0027](../decisions/0027-integrations.md).

An address is kept on the computer where it was added, and is not synced.
Events are sent only while Teamwright runs, and only those that happened on
that computer.

## The request

Every event is one `POST` to the address, over HTTPS, with a time limit of
ten seconds. Teamwright does not send it again if it fails; the integrations
dialog shows the last ten deliveries of each address with the answer.

| Header | Value |
|---|---|
| `Content-Type` | `application/json` |
| `User-Agent` | `Teamwright/<version of the program>` |
| `X-Teamwright-Event` | The name of the event, such as `task.finished` |
| `X-Teamwright-Version` | The version of the shape of the body, now `1` |
| `X-Teamwright-Signature` | Only with a secret: `sha256=` and the HMAC-SHA256 of the body, with the secret as the key, in small hexadecimal digits |

An answer in the 200s counts as taken. Any other answer is shown with its
status.

### Checking the signature

Compute the HMAC-SHA256 of the raw body as it arrived (before reading it as
JSON) with the secret, write it in hexadecimal, put `sha256=` in front, and
compare it with the header in constant time. In Node:

```js
import { createHmac, timingSafeEqual } from 'node:crypto';

function isFromTeamwright(rawBody, header, secret) {
  const expected = `sha256=${createHmac('sha256', secret).update(rawBody).digest('hex')}`;
  return header.length === expected.length && timingSafeEqual(Buffer.from(header), Buffer.from(expected));
}
```

## The body

Every field is always there. A field that does not apply to an event is
`null`. A new field may be added without changing the version; the version
changes only when a field is taken away or changes its meaning.

| Field | Type | What it is |
|---|---|---|
| `version` | number | The version of the shape: `1` |
| `event` | text | The name of the event (below) |
| `at` | text | When it happened, in UTC, as `2026-11-16T09:30:00.000Z` |
| `project.id` | text | The project's identifier |
| `project.key` | text | The project's key, such as `SSN` |
| `project.name` | text | The project's name when it happened |
| `task` | object or null | The task, where the event has one |
| `task.id` | text | The task's identifier |
| `task.key` | text | The task's key, such as `SSN-4` |
| `task.title` | text | Its title |
| `task.status` | object or null | Its status after the event: `name` (as the manager named it) and `category` (`todo`, `active`, `review` or `done`). Go by the category; names change |
| `task.assignee` | text or null | The name of who it is given to, `The manager`, or null |
| `agent` | object or null | The specialist or the Advisor the event is about: `kind` (`specialist` or `advisor`), `id` (the member, or null), `name` |
| `from_status` | object or null | For `task.status_changed`: the status the task left, `name` and `category` |
| `waits_for` | text or null | For `agent.waiting`: `decision`, `answers` or `approval` |
| `question` | text or null | For `agent.waiting`: the first words of the question |
| `reason` | text or null | For `agent.stopped`: why, in its first words |
| `sprint` | object or null | For the events of a sprint: `id`, `name`, `goal` (when it started), `unit` (`points` or `tasks`), `committed` and `completed` (when it was completed) |

## The events

| Name | When |
|---|---|
| `task.created` | The manager opened a task |
| `task.status_changed` | The manager moved a task to another status (not when cards are only reordered) |
| `task.finished` | A specialist finished its work on a task |
| `agent.waiting` | A specialist or the Advisor waits for the manager |
| `agent.stopped` | A specialist or the Advisor could not go on |
| `sprint.started` | The manager started a sprint |
| `sprint.completed` | The manager completed a sprint |
| `test` | The manager pressed "Send a test event" |

Tasks opened from a plan the manager approved, and tasks moved by the work
of a specialist, are not told as `task.created` or `task.status_changed`.

### `task.created`

```json
{
  "version": 1,
  "event": "task.created",
  "at": "2026-11-16T09:30:00.000Z",
  "project": { "id": "0193…", "key": "SSN", "name": "Smart sensor node" },
  "task": {
    "id": "0193…",
    "key": "SSN-22",
    "title": "Measure the battery life",
    "status": { "name": "To do", "category": "todo" },
    "assignee": "Hardware engineer"
  },
  "agent": null,
  "from_status": null,
  "waits_for": null,
  "question": null,
  "reason": null,
  "sprint": null
}
```

### `task.status_changed`

```json
{
  "version": 1,
  "event": "task.status_changed",
  "at": "2026-11-16T09:41:00.000Z",
  "project": { "id": "0193…", "key": "SSN", "name": "Smart sensor node" },
  "task": {
    "id": "0193…",
    "key": "SSN-4",
    "title": "Draw the schematic",
    "status": { "name": "In test", "category": "review" },
    "assignee": "Hardware engineer"
  },
  "agent": null,
  "from_status": { "name": "In progress", "category": "active" },
  "waits_for": null,
  "question": null,
  "reason": null,
  "sprint": null
}
```

### `task.finished`

```json
{
  "version": 1,
  "event": "task.finished",
  "at": "2026-11-16T10:02:00.000Z",
  "project": { "id": "0193…", "key": "SSN", "name": "Smart sensor node" },
  "task": {
    "id": "0193…",
    "key": "SSN-4",
    "title": "Draw the schematic",
    "status": { "name": "In test", "category": "review" },
    "assignee": "Hardware engineer"
  },
  "agent": { "kind": "specialist", "id": "0193…", "name": "Hardware engineer" },
  "from_status": null,
  "waits_for": null,
  "question": null,
  "reason": null,
  "sprint": null
}
```

### `agent.waiting`

```json
{
  "version": 1,
  "event": "agent.waiting",
  "at": "2026-11-16T10:05:00.000Z",
  "project": { "id": "0193…", "key": "SSN", "name": "Smart sensor node" },
  "task": {
    "id": "0193…",
    "key": "SSN-5",
    "title": "Choose the regulator",
    "status": { "name": "In progress", "category": "active" },
    "assignee": "Hardware engineer"
  },
  "agent": { "kind": "specialist", "id": "0193…", "name": "Hardware engineer" },
  "from_status": null,
  "waits_for": "decision",
  "question": "Linear or switching regulator for the sensor board?",
  "reason": null,
  "sprint": null
}
```

When the Advisor waits for the approval of a plan, `task`, `question` and
`agent.id` are null, `agent.kind` is `advisor` and `waits_for` is
`approval`.

### `agent.stopped`

```json
{
  "version": 1,
  "event": "agent.stopped",
  "at": "2026-11-16T10:12:00.000Z",
  "project": { "id": "0193…", "key": "SSN", "name": "Smart sensor node" },
  "task": {
    "id": "0193…",
    "key": "SSN-7",
    "title": "Write the firmware for the sensor",
    "status": { "name": "In progress", "category": "active" },
    "assignee": "Firmware engineer"
  },
  "agent": { "kind": "specialist", "id": "0193…", "name": "Firmware engineer" },
  "from_status": null,
  "waits_for": null,
  "question": null,
  "reason": "The agent program ran out of time.",
  "sprint": null
}
```

### `sprint.started`

```json
{
  "version": 1,
  "event": "sprint.started",
  "at": "2026-11-12T08:00:00.000Z",
  "project": { "id": "0193…", "key": "SSN", "name": "Smart sensor node" },
  "task": null,
  "agent": null,
  "from_status": null,
  "waits_for": null,
  "question": null,
  "reason": null,
  "sprint": {
    "id": "0193…",
    "name": "Sprint 2",
    "goal": "A first prototype that measures",
    "unit": null,
    "committed": null,
    "completed": null
  }
}
```

### `sprint.completed`

```json
{
  "version": 1,
  "event": "sprint.completed",
  "at": "2026-11-25T17:00:00.000Z",
  "project": { "id": "0193…", "key": "SSN", "name": "Smart sensor node" },
  "task": null,
  "agent": null,
  "from_status": null,
  "waits_for": null,
  "question": null,
  "reason": null,
  "sprint": {
    "id": "0193…",
    "name": "Sprint 2",
    "goal": null,
    "unit": "points",
    "committed": 21,
    "completed": 18
  }
}
```

### `test`

```json
{
  "version": 1,
  "event": "test",
  "at": "2026-11-16T09:00:00.000Z",
  "project": { "id": "0193…", "key": "SSN", "name": "Smart sensor node" },
  "task": null,
  "agent": null,
  "from_status": null,
  "waits_for": null,
  "question": null,
  "reason": null,
  "sprint": null
}
```

## Slack

A Slack channel gets one plain line for each event it is told, with the
project's name first, such as:

```text
Smart sensor node: Hardware engineer finished SSN-4 Draw the schematic.
```

The line is posted as `{"text": "…"}` to the channel's incoming webhook,
which Slack gives a channel when an incoming webhook is added to it.
