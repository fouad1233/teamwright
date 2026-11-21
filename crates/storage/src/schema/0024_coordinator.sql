-- Teamwright database, version 24: the coordinator of each project, its
-- budgets in tokens, and the record of what it did (decision 0037).
--
-- The conventions of version 1 hold.
--
-- All three are logged and travel to the other computers. A coordinator is
-- logged with the entity 'coordinator' and merges by field: its limit, and
-- its switch (the computer it runs on with the moment it was switched). A
-- budget is logged with the entity 'budget' and merges by field: its
-- tokens, and the question asked about it. An entry of the record is
-- logged with the entity 'coordinator_entry' and never changes.
--
-- Each goes with its project: the program removes and logs them first. The
-- member or the task a budget or an entry is about is not enforced as a
-- reference: what the record says stays readable after a task was deleted.

-- The coordinator of a project. Its identifier is the project's. A project
-- without a row has a coordinator that is off, with the default limit.
CREATE TABLE coordinators (
    id                TEXT PRIMARY KEY,
    project_id        TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    -- How many specialists may work at once, 1 to 8.
    concurrency_limit INTEGER NOT NULL,
    -- The computer it runs on, or NULL while it is off.
    runs_on           TEXT,
    -- When it was last switched on or off. NULL if it never was.
    switched_at       TEXT
) STRICT, WITHOUT ROWID;

-- A budget in tokens. Its identifier is that of what it is for: the
-- project, a member or a task.
CREATE TABLE budgets (
    id         TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    -- 'project', 'member' or 'task'.
    scope      TEXT NOT NULL,
    -- The tokens it allows. NULL once the manager took it away.
    tokens     INTEGER,
    -- The question asked about it when it was used up, with its answer, as
    -- JSON. NULL if nothing was asked.
    question   TEXT
) STRICT, WITHOUT ROWID;

CREATE INDEX budgets_by_project ON budgets (project_id);

-- What a coordinator did and why, one sentence each.
CREATE TABLE coordinator_entries (
    id         TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    at         TEXT NOT NULL,
    -- 'started', 'woke', 'held', 'asked', 'switched_on' or 'switched_off'.
    kind       TEXT NOT NULL,
    text       TEXT NOT NULL,
    task_id    TEXT,
    member_id  TEXT,
    -- The computer the coordinator ran on.
    computer   TEXT NOT NULL
) STRICT, WITHOUT ROWID;

CREATE INDEX coordinator_entries_by_project ON coordinator_entries (project_id, at);
