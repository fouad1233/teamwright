-- Teamwright database, version 22: the work-in-progress limits of statuses.
--
-- The conventions of version 1 hold.
--
-- The limit is part of the status, and the statuses are part of the state of
-- their project in `operations`, so a change of a limit is logged, merged and
-- synced with the workflow (decision 0033). A state logged before this
-- version has no limit for its statuses, and is read as statuses without one.

-- The largest number of tasks that should stand in the status at once, from
-- 1 to 999, or NULL when the status has no limit. A status of the category
-- 'done' never has one.
ALTER TABLE statuses ADD COLUMN task_limit INTEGER;
