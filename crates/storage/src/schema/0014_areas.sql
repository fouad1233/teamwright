-- Teamwright database, version 14: the areas of the projects.
--
-- The conventions of version 1 hold.
--
-- Changes to an area are logged in `operations` with the entity 'area',
-- whose state is the whole area (decision 0022). The area of a task is a
-- field of the task's state, `area_id`.

-- The parts of each project that have a job of their own.
CREATE TABLE areas (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    -- Unique within the project, whatever the capitals.
    name        TEXT NOT NULL,
    -- The place of the area among the areas of its project, the smallest
    -- first. Two areas at one place are ordered by their identifiers.
    position    INTEGER NOT NULL
) STRICT, WITHOUT ROWID;

CREATE INDEX areas_by_project ON areas (project_id);

-- The area a task is in. The program always fills it. No reference is
-- enforced: the use case that removes an area moves its tasks first, and
-- syncing, which can take in the removal of an area before it learns of a
-- task in it, moves such a task into the first area of its project when the
-- take-in is over.
ALTER TABLE tasks ADD COLUMN area_id TEXT;

CREATE INDEX tasks_by_area ON tasks (area_id);

-- Every project of an earlier version gets the area a new project starts
-- with, "General". Its identifier follows from the project: the identifier
-- of the project with its version digit replaced by 0. Two computers that
-- both had the project before this version thus give it the same area,
-- which is one area when they keep in step.
INSERT INTO areas (id, project_id, name, position)
SELECT substr(id, 1, 14) || '0' || substr(id, 16), id, 'General', 0
FROM projects;

-- Every task goes into that area. These changes are not logged: a state of
-- a task without `area_id`, which is what the log holds for every task of
-- an earlier version, is read as a task in the area its project started
-- with, on every computer, which is exactly what is written here.
UPDATE tasks
SET area_id = substr(project_id, 1, 14) || '0' || substr(project_id, 16);

-- The new areas are logged as changes of this computer, so that they reach
-- the computers it keeps in step with like any other change. Their stamps
-- come after every stamp in the log and after the time of day, and count
-- on within one millisecond; their numbers come after the last operation
-- of this computer.
INSERT INTO operations (device_id, device_sequence, stamp, entity, entity_id, kind, payload)
SELECT
    computer.value,
    start.last_sequence + numbered.n,
    printf(
        '%013d-%05d-%s',
        start.millis + (numbered.n - 1) / 100000,
        (numbered.n - 1) % 100000,
        computer.value
    ),
    'area',
    numbered.id,
    'upsert',
    json_object(
        'id', numbered.id,
        'project_id', numbered.project_id,
        'name', numbered.name,
        'position', numbered.position
    )
FROM (
    SELECT listed.*, row_number() OVER (ORDER BY listed.project_id) AS n
    FROM areas AS listed
) AS numbered
JOIN meta AS computer ON computer.key = 'device_id'
CROSS JOIN (
    SELECT
        max(
            CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER),
            coalesce(
                (SELECT max(CAST(substr(stamp, 1, 13) AS INTEGER)) FROM operations), 0
            ) + 1
        ) AS millis,
        coalesce(
            (
                SELECT max(device_sequence) FROM operations
                WHERE device_id = (SELECT value FROM meta WHERE key = 'device_id')
            ),
            0
        ) AS last_sequence
) AS start;

-- Each new area gets its merged state: the state just logged, all of whose
-- fields have the stamp of that operation.
INSERT INTO merged_states (entity, entity_id, state, base_stamp)
SELECT entity, entity_id, payload, stamp FROM operations WHERE entity = 'area';
