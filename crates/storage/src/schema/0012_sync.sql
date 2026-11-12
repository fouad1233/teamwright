-- Teamwright database, version 12: the bookkeeping of syncing.
--
-- The conventions of version 1 hold.
--
-- What this computer knows of the others, and which files it wrote into
-- the sync folder. This describes this computer's view of the folder; it
-- is bookkeeping of the log and is not logged. The settings are kept in
-- `meta`: 'sync_folder' (the chosen folder, missing while syncing is off),
-- 'files_folder' (the folder the written files are in), 'computer_name',
-- 'written_through' (the number of the last operation of this computer
-- written into the folder) and 'copied_before_sync' (set once the database
-- was copied before the first take-in).

-- The other computers found in the sync folder.
CREATE TABLE computers (
    id            TEXT PRIMARY KEY,
    -- What its card says, once it was read.
    name          TEXT,
    version       TEXT,
    format        INTEGER,
    -- NULL while the manager has not decided, 1 when its operations are
    -- taken in, 0 when they are not.
    accepted      INTEGER,
    -- The number of the last of its operations taken in.
    read_up_to    INTEGER NOT NULL DEFAULT 0,
    -- When it last wrote into the folder, by its card.
    last_heard    TEXT,
    -- How far its clock is ahead of this computer's, in milliseconds, if
    -- it is ahead too far.
    clock_ahead   INTEGER,
    -- The format of a file it wrote that this version cannot read.
    newer_format  INTEGER
) STRICT, WITHOUT ROWID;

-- The files this computer wrote into the sync folder, by name. A file in
-- its own folder that is not here means that another computer uses its
-- identity.
CREATE TABLE written_files (
    name       TEXT PRIMARY KEY,
    written_at TEXT NOT NULL
) STRICT, WITHOUT ROWID;
