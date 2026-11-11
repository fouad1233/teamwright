-- Teamwright database, version 2: the team of a project.
--
-- The conventions of version 1 hold: identifiers are UUIDs as text, moments
-- are UTC in RFC 3339 form with milliseconds, tables are STRICT.

-- The members of the teams. A member is on the team of one project and can
-- be given tasks of that project: tasks.assignee_member_id then holds the
-- identifier of the member.
--
-- Changes to members are logged in `operations` with the entity 'member'.
CREATE TABLE members (
    id             TEXT PRIMARY KEY,
    project_id     TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    name           TEXT NOT NULL,
    -- 'agent' for an AI specialist, 'human' for a person.
    kind           TEXT NOT NULL,
    -- What the member is on the team for. May be empty.
    responsibility TEXT NOT NULL,
    color          TEXT NOT NULL,
    joined_at      TEXT NOT NULL,
    updated_at     TEXT NOT NULL
) STRICT, WITHOUT ROWID;

CREATE INDEX members_by_project ON members (project_id);
