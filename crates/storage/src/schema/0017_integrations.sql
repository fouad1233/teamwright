-- Teamwright database, version 17: the integrations of a project.
--
-- The conventions of version 1 hold.
--
-- The integrations are facts about this computer, and their addresses are
-- secrets: they are not logged, so they never reach the operation log or
-- another computer (decision 0027).

-- The GitHub repository and the Slack channel of a project, one row per
-- project for which the manager set something.
CREATE TABLE project_integrations (
    project_id    TEXT PRIMARY KEY REFERENCES projects (id) ON DELETE CASCADE,
    -- The repository the manager named, `owner/name`. NULL to read it from
    -- the work folder.
    repository    TEXT,
    -- The incoming webhook address of the Slack channel. NULL for none.
    slack_address TEXT,
    -- The kinds of events told in the channel, separated by spaces.
    slack_events  TEXT NOT NULL DEFAULT ''
) STRICT, WITHOUT ROWID;

-- The web addresses that listen for the events of a project, in the order
-- they were added, counted from 0.
CREATE TABLE project_web_addresses (
    project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    position   INTEGER NOT NULL,
    address    TEXT NOT NULL,
    -- The secret the body is signed with. NULL for none.
    secret     TEXT,
    -- The kinds of events sent, separated by spaces.
    events     TEXT NOT NULL,
    PRIMARY KEY (project_id, position)
) STRICT, WITHOUT ROWID;
