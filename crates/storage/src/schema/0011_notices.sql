-- Teamwright database, version 11: what syncing tells the manager.
--
-- The conventions of version 1 hold.
--
-- When operations of another computer are taken in, some things are worth
-- telling the manager: a task or a project that got a new number or key, a
-- change that was dropped because its thing was deleted elsewhere, a task
-- that moved because its status was removed, a computer whose clock is
-- ahead. They describe this computer's view, are bookkeeping of syncing,
-- and are not logged.

CREATE TABLE notices (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    at         TEXT NOT NULL,
    -- 'task_renumbered', 'project_rekeyed', 'change_dropped', 'task_moved'
    -- or 'clock_ahead'.
    kind       TEXT NOT NULL,
    -- The project and the task it concerns, if any. No reference is
    -- enforced: a notice may outlive what it is about.
    project_id TEXT,
    task_id    TEXT,
    message    TEXT NOT NULL
) STRICT;
