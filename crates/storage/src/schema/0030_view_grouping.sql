-- Teamwright database, version 30: what a view groups its tasks by.
--
-- The conventions of version 1 hold.
--
-- The grouping is part of the state of a view in `operations`, so it is
-- logged, merged and synced with the view like its filter (decision 0021).
-- A state logged before this version has no `grouping` and is read as a view
-- grouped by status. Nothing is logged here.

-- 'status', 'assignee', 'priority', 'area', 'tag' or 'sprint'. Only a board
-- and a list are grouped by anything but 'status'.
ALTER TABLE views ADD COLUMN grouping TEXT NOT NULL DEFAULT 'status';
