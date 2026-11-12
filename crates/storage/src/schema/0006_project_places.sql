-- Teamwright database, version 6: the folders of a project.
--
-- The conventions of version 1 hold.
--
-- The folders are facts about this computer: a folder here means nothing on
-- another computer. Changes are logged in `operations` with the entity
-- 'project_places', whose identifier is the identifier of the project and
-- whose state is the work folder with the whole list of reference folders.
-- When computers synchronize, these operations must not be applied on
-- another computer.

-- The folders chosen for a project, one row per project for which the
-- manager chose something.
CREATE TABLE project_places (
    project_id  TEXT PRIMARY KEY REFERENCES projects (id) ON DELETE CASCADE,
    -- The work folder the manager chose, as it was given. NULL for the
    -- folder Teamwright gives the project.
    work_folder TEXT
) STRICT, WITHOUT ROWID;

-- The folders the specialists of a project may read and must not change,
-- in the order they were added, counted from 0.
CREATE TABLE project_reference_folders (
    project_id TEXT NOT NULL REFERENCES project_places (project_id) ON DELETE CASCADE,
    position   INTEGER NOT NULL,
    folder     TEXT NOT NULL,
    PRIMARY KEY (project_id, position)
) STRICT, WITHOUT ROWID;
