-- Teamwright database, version 19: what specialists hand each other.
--
-- The conventions of version 1 hold.
--
-- A handoff is logged in `operations` with the entity 'handoff', whose
-- state is the whole handoff with its files; it never changes once it is
-- written. A message is logged with the entity 'team_message', whose state
-- is the whole message; only its delivery changes later (decision 0029).
--
-- The specialists, conversations and turns a handoff or a message names
-- are not enforced as references: a specialist may leave the team, and a
-- conversation may be deleted with its task, while what was handed over
-- stays readable. Only the project and the task are: a handoff goes with
-- its task and a message with its project, and the program removes and
-- logs them first, so that the database never removes one by itself.

-- The result of a task, handed over by the specialist who did it.
CREATE TABLE handoffs (
    id              TEXT PRIMARY KEY,
    project_id      TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    task_id         TEXT NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
    -- The specialist who published it, and the conversation and the turn
    -- it was published in.
    author_id       TEXT NOT NULL,
    conversation_id TEXT NOT NULL,
    turn_id         TEXT NOT NULL,
    summary         TEXT NOT NULL,
    -- The interface contract. Empty if there is none.
    contract        TEXT NOT NULL,
    published_at    TEXT NOT NULL
) STRICT, WITHOUT ROWID;

CREATE INDEX handoffs_by_project ON handoffs (project_id, published_at);
CREATE INDEX handoffs_by_task ON handoffs (task_id);

-- The files a handoff names, in order, counted from 0: paths inside the
-- work folder of the project, with '/' between their parts. These are the
-- registered artifacts of the task.
CREATE TABLE handoff_files (
    handoff_id TEXT NOT NULL REFERENCES handoffs (id) ON DELETE CASCADE,
    position   INTEGER NOT NULL,
    path       TEXT NOT NULL,
    PRIMARY KEY (handoff_id, position)
) STRICT, WITHOUT ROWID;

-- A question or a note from one specialist to another.
CREATE TABLE team_messages (
    id                TEXT PRIMARY KEY,
    project_id        TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    -- Who sent it, and the conversation and the turn it was sent in.
    from_member_id    TEXT NOT NULL,
    conversation_id   TEXT NOT NULL,
    turn_id           TEXT NOT NULL,
    to_member_id      TEXT NOT NULL,
    -- 'question' or 'note'.
    kind              TEXT NOT NULL,
    text              TEXT NOT NULL,
    sent_at           TEXT NOT NULL,
    -- The turn of the receiver that started with it, and when. Both are
    -- NULL while it waits.
    delivered_turn_id TEXT,
    delivered_at      TEXT
) STRICT, WITHOUT ROWID;

CREATE INDEX team_messages_by_project ON team_messages (project_id, sent_at);
