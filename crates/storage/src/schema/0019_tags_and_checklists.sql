-- Teamwright database, version 19: the tags of the projects, the tags of
-- the tasks, the checklists of the tasks, and views that show only the
-- tasks with one tag.
--
-- The conventions of version 1 hold.
--
-- Changes to a tag are logged in `operations` with the entity 'tag', whose
-- state is the whole tag (decision 0030). The tags a task carries and the
-- items of its checklist are parts of the task: its logged state has one
-- key for each tag it carries or carried, `tag:<identifier of the tag>`,
-- true or false, and one for each item of its checklist,
-- `item:<identifier of the item>`, an object with `text`, `done` and
-- `rank`, or null for an item that was removed. Every such key merges on
-- its own, so two computers that change different tags or different items
-- of one task both keep their change. The two tables below are the rows
-- written from that state, like `task_dependencies`.

-- The words the manager puts on the tasks of each project.
CREATE TABLE tags (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    -- Unique within the project, whatever the capitals.
    name        TEXT NOT NULL,
    -- A color of the domain, by name: 'gray', 'blue' and so on.
    color       TEXT NOT NULL
) STRICT, WITHOUT ROWID;

CREATE INDEX tags_by_project ON tags (project_id);

-- The tags each task carries. The use case that removes a tag, and
-- syncing, take it off its tasks before it goes, with logged changes of the
-- tasks; the cascade only keeps the table tidy.
CREATE TABLE task_tags (
    task_id  TEXT NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
    tag_id   TEXT NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
    PRIMARY KEY (task_id, tag_id)
) STRICT, WITHOUT ROWID;

CREATE INDEX task_tags_by_tag ON task_tags (tag_id);

-- The items of the checklist of each task. They go with their task.
CREATE TABLE checklist_items (
    id       TEXT PRIMARY KEY,
    task_id  TEXT NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
    text     TEXT NOT NULL,
    -- 1 if the item is done, else 0.
    done     INTEGER NOT NULL CHECK (done IN (0, 1)),
    -- The place of the item among the items of its checklist, the smallest
    -- first. Two items at one place are ordered by their identifiers.
    rank     REAL NOT NULL
) STRICT, WITHOUT ROWID;

CREATE INDEX checklist_items_by_task ON checklist_items (task_id);

-- The tag the tasks a view shows carry, NULL for a view that shows the
-- tasks with any tag. No reference is enforced, as for the member of the
-- filter: the use case that removes a tag, and syncing, change the filter
-- before the tag goes. Nothing is logged: a view state logged before this
-- version has no `tag_id` and is read as a view of the tasks with any tag.
ALTER TABLE views ADD COLUMN tag_id TEXT;
