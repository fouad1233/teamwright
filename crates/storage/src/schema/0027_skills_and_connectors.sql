-- Teamwright database: the library of skills and connectors of each
-- project, and what each specialist was given of it (decision 0039).
--
-- The conventions of version 1 hold.
--
-- Changes are logged in `operations` with the entities 'skill', 'connector'
-- and 'member_kit', whose states are the whole row, and they reach the
-- other computers. What belongs to one computer is not here: the copy of a
-- skill's folder lies on the disk of each computer, and the values of the
-- environment variables a connector needs come from the environment the
-- program was started in. No secret is ever stored.

-- The skills of the library of a project.
CREATE TABLE skills (
    id           TEXT PRIMARY KEY,
    project_id   TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    -- Lower-case letters, digits and hyphens; unique within the project.
    name         TEXT NOT NULL,
    -- From its SKILL.md, at most 1024 characters.
    description  TEXT NOT NULL,
    added_at     TEXT NOT NULL
) STRICT, WITHOUT ROWID;

CREATE INDEX skills_by_project ON skills (project_id);

-- The connectors of the library of a project.
CREATE TABLE connectors (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    -- Lower-case letters, digits and hyphens; unique within the project.
    name        TEXT NOT NULL,
    -- How it is started, as JSON: {"kind": "program", "program", "arguments",
    -- "variables"} or {"kind": "address", "url", "headers": [{"name",
    -- "variable", "bearer"}]}. Variables are named, never given.
    start       TEXT NOT NULL,
    -- The names of its tools, as a JSON array.
    tools       TEXT NOT NULL,
    added_at    TEXT NOT NULL
) STRICT, WITHOUT ROWID;

CREATE INDEX connectors_by_project ON connectors (project_id);

-- What a specialist was given. The identifier is the member's. No reference
-- to the member, the skills or the connectors is enforced: a kit may name
-- something another computer removed, and what it names is read only
-- together with the library.
CREATE TABLE member_kits (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    -- The identifiers of its skills, as a JSON array.
    skills      TEXT NOT NULL,
    -- Its connectors, as a JSON array of {"connector_id", "tools"}.
    connectors  TEXT NOT NULL
) STRICT, WITHOUT ROWID;

CREATE INDEX member_kits_by_project ON member_kits (project_id);
