-- Teamwright database, version 1.
--
-- Conventions
--   * Identifiers are UUIDs stored as text.
--   * Moments are UTC in RFC 3339 form with milliseconds, e.g.
--     2026-11-11T09:00:00.000Z. Text in this form sorts in time order.
--   * Calendar dates are YYYY-MM-DD.
--   * Tables are STRICT: SQLite rejects a value of the wrong type.

-- Facts about this database file itself, such as the identity of the
-- computer it belongs to.
CREATE TABLE meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT, WITHOUT ROWID;

CREATE TABLE projects (
    id               TEXT PRIMARY KEY,
    key              TEXT NOT NULL,
    name             TEXT NOT NULL,
    description      TEXT NOT NULL,
    next_task_number INTEGER NOT NULL,
    created_at       TEXT NOT NULL,
    archived_at      TEXT
) STRICT, WITHOUT ROWID;

-- The statuses of a project's workflow, in board order.
CREATE TABLE statuses (
    id         TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    position   INTEGER NOT NULL,
    name       TEXT NOT NULL,
    category   TEXT NOT NULL,
    color      TEXT NOT NULL
) STRICT, WITHOUT ROWID;

CREATE INDEX statuses_by_project ON statuses (project_id, position);

CREATE TABLE tasks (
    id                 TEXT PRIMARY KEY,
    project_id         TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    number             INTEGER NOT NULL,
    title              TEXT NOT NULL,
    description        TEXT NOT NULL,
    status_id          TEXT NOT NULL REFERENCES statuses (id),
    priority           TEXT NOT NULL,
    -- Who is responsible: NULL for nobody, 'manager', or 'member' together
    -- with the member's identifier.
    assignee_kind      TEXT,
    assignee_member_id TEXT,
    parent_id          TEXT REFERENCES tasks (id) ON DELETE SET NULL,
    estimate_minutes   INTEGER,
    start_on           TEXT,
    due_on             TEXT,
    rank               REAL NOT NULL,
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL,
    completed_at       TEXT
) STRICT, WITHOUT ROWID;

CREATE INDEX tasks_by_project ON tasks (project_id);

-- "task_id waits for blocker_id".
CREATE TABLE task_dependencies (
    task_id    TEXT NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
    blocker_id TEXT NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
    PRIMARY KEY (task_id, blocker_id)
) STRICT, WITHOUT ROWID;

-- The operation log: every change ever made, in order. It is what one
-- computer hands to another when they synchronize.
CREATE TABLE operations (
    sequence  INTEGER PRIMARY KEY AUTOINCREMENT,
    -- The computer that made the change.
    device_id TEXT NOT NULL,
    -- A hybrid logical clock stamp. Stamps sort in the order changes
    -- happened, even between computers whose clocks disagree.
    stamp     TEXT NOT NULL,
    -- What changed: 'project' or 'task'.
    entity    TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    -- 'upsert' or 'delete'.
    kind      TEXT NOT NULL,
    -- For an upsert, the complete new state as JSON.
    payload   TEXT
) STRICT;

CREATE INDEX operations_by_entity ON operations (entity, entity_id);
