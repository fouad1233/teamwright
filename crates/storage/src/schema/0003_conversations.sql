-- Teamwright database, version 3: the work of AI specialists.
--
-- The conventions of version 1 hold: identifiers are UUIDs as text, moments
-- are UTC in RFC 3339 form with milliseconds, tables are STRICT.
--
-- Changes are logged in `operations` with the entities 'conversation',
-- 'turn' and 'turn_step'. A step's entity identifier is the identifier of
-- its turn, a slash and its position: '<turn id>/3'. The deletion of a turn
-- stands for the deletion of its steps, and the deletion of a conversation
-- is logged after the deletion of each of its turns.

-- A thread of work with one specialist: about a task, or about a question
-- the manager asked. It is one session of the agent program.
CREATE TABLE conversations (
    id         TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    -- NULL once the specialist has left the team. What it did stays.
    member_id  TEXT REFERENCES members (id) ON DELETE SET NULL,
    -- NULL for a conversation that is not about a task.
    task_id    TEXT REFERENCES tasks (id) ON DELETE CASCADE,
    started_at TEXT NOT NULL,
    -- The identifier of the agent program's session, once it is known.
    session_id TEXT
) STRICT, WITHOUT ROWID;

CREATE INDEX conversations_by_project ON conversations (project_id, started_at);
CREATE INDEX conversations_by_task ON conversations (task_id);
CREATE INDEX conversations_by_member ON conversations (member_id);

-- One request to a specialist and what came of it.
CREATE TABLE turns (
    id                   TEXT PRIMARY KEY,
    conversation_id      TEXT NOT NULL REFERENCES conversations (id) ON DELETE CASCADE,
    request              TEXT NOT NULL,
    -- 'brief' or 'message'.
    origin               TEXT NOT NULL,
    -- 'running', 'finished', 'failed' or 'stopped'.
    state                TEXT NOT NULL,
    started_at           TEXT NOT NULL,
    ended_at             TEXT,
    -- The specialist's answer. Empty until it has one.
    answer               TEXT NOT NULL,
    -- Why the turn failed or was stopped, in words.
    failure              TEXT,
    model                TEXT,
    -- The tokens the turn used, all models together. Either all four are
    -- NULL, before the turn has ended, or none is.
    input_tokens         INTEGER,
    output_tokens        INTEGER,
    cache_read_tokens    INTEGER,
    cache_written_tokens INTEGER
) STRICT, WITHOUT ROWID;

CREATE INDEX turns_by_conversation ON turns (conversation_id, started_at);
CREATE INDEX turns_by_state ON turns (state);

-- What happened during a turn, one row per step, in order.
CREATE TABLE turn_steps (
    turn_id     TEXT NOT NULL REFERENCES turns (id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    at          TEXT NOT NULL,
    -- 'text', 'tool', 'denied' or 'notice'.
    kind        TEXT NOT NULL,
    -- What was written, for 'text'; the message, for 'notice'.
    text        TEXT,
    -- For 'tool': the identifier of the use, its summary and its outcome
    -- ('running', 'done' or 'failed').
    tool_use_id TEXT,
    -- For 'tool' and 'denied': the name of the tool.
    tool        TEXT,
    summary     TEXT,
    outcome     TEXT,
    PRIMARY KEY (turn_id, position)
) STRICT, WITHOUT ROWID;
