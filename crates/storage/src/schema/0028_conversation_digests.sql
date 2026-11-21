-- Teamwright database: the digests of earlier conversations.
--
-- The conventions of version 1 hold.
--
-- A digest is logged in `operations` with the entity 'digest', whose state
-- is the whole digest; it never changes once it is kept (decision 0041).
-- The conversation it was made from stays where Claude Code keeps it: only
-- the digest is stored, and only the digest travels to other computers.

-- The accounts of earlier conversations the manager kept with a project.
CREATE TABLE digests (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    -- How the conversation was found on the computer it was read on.
    source      TEXT NOT NULL,
    -- The folder the conversation was held in. Empty if it is not known.
    held_in     TEXT NOT NULL,
    title       TEXT NOT NULL,
    -- The account, as the manager corrected it: plain lines.
    text        TEXT NOT NULL,
    -- When the conversation began and last changed. NULL if not known.
    started_at  TEXT,
    ended_at    TEXT,
    kept_at     TEXT NOT NULL
) STRICT, WITHOUT ROWID;

CREATE INDEX digests_by_project ON digests (project_id, kept_at);
