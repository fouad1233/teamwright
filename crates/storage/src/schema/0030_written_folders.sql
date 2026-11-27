-- Teamwright database: the folders besides the work folder that the
-- specialists of a project may write in (decision 0044).
--
-- The conventions of version 1 hold.
--
-- They are facts about this computer, like the reference folders of version
-- 6, and are logged the same way: in the state of the entity
-- 'project_places', which never leaves this computer.
--
-- Whether a project has full access is not kept here: it is a choice of
-- this computer kept in `meta`, outside the log, like the choice about
-- commands.

-- The folders the specialists of a project may read and write besides the
-- work folder, in the order they were added, counted from 0.
CREATE TABLE project_written_folders (
    project_id TEXT NOT NULL REFERENCES project_places (project_id) ON DELETE CASCADE,
    position   INTEGER NOT NULL,
    folder     TEXT NOT NULL,
    PRIMARY KEY (project_id, position)
) STRICT, WITHOUT ROWID;
