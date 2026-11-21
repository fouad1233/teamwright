-- Teamwright database: the memory of a project (decision 0036).
--
-- The conventions of version 1 hold.
--
-- An entry is logged in `operations` with the entity 'memory_entry', whose
-- state is the whole entry. Only `replaced_by` changes once it is written,
-- when a newer entry replaces it.
--
-- Only the project is enforced as a reference: an entry goes with its
-- project, and the program removes and logs it first. The task an entry
-- belongs to is not enforced: a decision or a contract stays true of the
-- project after the task it came from is deleted. Neither are the
-- specialist, conversation and turn it came from, nor the entry that
-- replaced it, for the same reasons as a handoff's (version 18).

CREATE TABLE memory_entries (
    id              TEXT PRIMARY KEY,
    project_id      TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    -- 'decision', 'fact' or 'contract'.
    kind            TEXT NOT NULL,
    title           TEXT NOT NULL,
    text            TEXT NOT NULL,
    -- 'manager', 'specialist', 'decision_request' or 'handoff'.
    source          TEXT NOT NULL,
    -- The specialist, conversation and turn it came from. All three are
    -- NULL when the manager wrote it, and none is otherwise.
    member_id       TEXT,
    conversation_id TEXT,
    turn_id         TEXT,
    -- The task it belongs to, or NULL for the whole project.
    task_id         TEXT,
    recorded_at     TEXT NOT NULL,
    -- The newer entry that replaced it, or NULL while it is current.
    replaced_by     TEXT
) STRICT, WITHOUT ROWID;

CREATE INDEX memory_entries_by_project ON memory_entries (project_id, recorded_at);
