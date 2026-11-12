-- Teamwright database, version 8: sprints, a backlog and story points.
--
-- The conventions of version 1 hold.
--
-- Changes to a sprint are logged in `operations` with the entity 'sprint',
-- whose state is the whole sprint with its summary. The state of a task in
-- the log gains its sprint and its points; states logged before this
-- version have neither, which means the backlog and no points.

-- The sprints of the projects, planned, active or completed.
CREATE TABLE sprints (
    id                  TEXT PRIMARY KEY,
    project_id          TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    name                TEXT NOT NULL,
    -- What the sprint is for. May be empty.
    goal                TEXT NOT NULL,
    -- The first and the last day. Either may be NULL while it is planned.
    start_on            TEXT,
    end_on              TEXT,
    -- 'planned', 'active' or 'completed'.
    state               TEXT NOT NULL,
    created_at          TEXT NOT NULL,
    started_at          TEXT,
    completed_at        TEXT,
    -- What a completed sprint achieved: the unit, 'points' or 'tasks', and
    -- the work committed and completed in it. All three are NULL until the
    -- sprint is completed.
    summary_unit        TEXT,
    summary_committed   INTEGER,
    summary_completed   INTEGER
) STRICT, WITHOUT ROWID;

CREATE INDEX sprints_by_project ON sprints (project_id);

-- The sprint a task is planned into, or NULL for the backlog. A sprint is
-- only removed once its tasks have left it, so the database never has to
-- set this column to NULL by itself.
ALTER TABLE tasks ADD COLUMN sprint_id TEXT REFERENCES sprints (id) ON DELETE SET NULL;
-- The story points of a task, from 0 to 999, or NULL for none.
ALTER TABLE tasks ADD COLUMN points INTEGER;

CREATE INDEX tasks_by_sprint ON tasks (sprint_id);

-- The number the next sprint of a project gets. A sprint left without a
-- name is called after it. Projects of earlier versions had no sprints.
ALTER TABLE projects ADD COLUMN next_sprint_number INTEGER NOT NULL DEFAULT 1;
