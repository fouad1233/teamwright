-- Teamwright database, version 24: the custom fields of the projects, their
-- choices, the values the tasks hold, and views that show only the tasks
-- holding one value of a field.
--
-- The conventions of version 1 hold.
--
-- Changes to a field are logged in `operations` with the entity 'field',
-- whose state is the field with one key for each of its choices,
-- `choice:<identifier of the choice>`, an object with `name`, `color` and
-- `rank`, or null for a choice that was removed (decision 0035). The values
-- a task holds are parts of the task: its logged state has one key for each
-- field it holds or held a value of, `field:<identifier of the field>`, the
-- value, or null once it was cleared. Every such key merges on its own, so
-- two computers that change different choices of a field, or different
-- fields of a task, both keep their change. The tables `field_choices` and
-- `task_field_values` are the rows written from those states.

-- The properties of tasks the manager defines for each project.
CREATE TABLE fields (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    -- Unique within the project, whatever the capitals.
    name        TEXT NOT NULL,
    -- 'text', 'number', 'date', 'yes_no', 'choice' or 'link'. It never
    -- changes once the field is made.
    kind        TEXT NOT NULL,
    -- The unit after the numbers of a number field, NULL for none.
    unit        TEXT,
    -- The place of the field among the fields of its project, the smallest
    -- first.
    position    INTEGER NOT NULL
) STRICT, WITHOUT ROWID;

CREATE INDEX fields_by_project ON fields (project_id);

-- The choices of each choice field. They go with their field.
CREATE TABLE field_choices (
    id        TEXT PRIMARY KEY,
    field_id  TEXT NOT NULL REFERENCES fields (id) ON DELETE CASCADE,
    -- Unique within the field, whatever the capitals.
    name      TEXT NOT NULL,
    -- A color of the domain, by name.
    color     TEXT NOT NULL,
    -- The place of the choice among the choices of its field, the smallest
    -- first. Two choices at one place are ordered by their identifiers.
    rank      REAL NOT NULL
) STRICT, WITHOUT ROWID;

CREATE INDEX field_choices_by_field ON field_choices (field_id);

-- The value each task holds for a field, as the log keeps it: JSON text
-- such as {"text":"Two reels"}, {"number":12.5}, {"date":"2026-11-30"},
-- {"yes":true}, {"choice":"<identifier>"} or {"link":"https://…"}. The use
-- case that removes a field, and syncing, clear its values with logged
-- changes of the tasks before it goes; the cascade only keeps the table
-- tidy.
CREATE TABLE task_field_values (
    task_id   TEXT NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
    field_id  TEXT NOT NULL REFERENCES fields (id) ON DELETE CASCADE,
    value     TEXT NOT NULL,
    PRIMARY KEY (task_id, field_id)
) STRICT, WITHOUT ROWID;

CREATE INDEX task_field_values_by_field ON task_field_values (field_id);

-- The value of a field the tasks a view shows hold, NULL in both columns
-- for a view that shows the tasks whatever they hold: the field, and the
-- identifier of a choice, or 'yes' or 'no'. No reference is enforced, as
-- for the tag of the filter: the use cases that remove a field or a
-- choice, and syncing, change the filter before they go. Nothing is
-- logged: a view state logged before this version has neither key and is
-- read as a view of the tasks whatever they hold.
ALTER TABLE views ADD COLUMN field_id TEXT;
ALTER TABLE views ADD COLUMN field_value TEXT;
