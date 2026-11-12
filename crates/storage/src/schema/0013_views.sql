-- Teamwright database, version 13: the views of the projects.
--
-- The conventions of version 1 hold.
--
-- Changes to a view are logged in `operations` with the entity 'view',
-- whose state is the whole view with its filter (decision 0021).

-- The ways the manager looks at the tasks of each project.
CREATE TABLE views (
    id                  TEXT PRIMARY KEY,
    project_id          TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    -- 'board', 'backlog', 'list', 'gantt', 'calendar' or 'dashboard'.
    kind                TEXT NOT NULL,
    -- Unique within the project, whatever the capitals.
    name                TEXT NOT NULL,
    -- Whose tasks the view shows: 'anyone', 'manager', 'member' or
    -- 'nobody'. For 'member', `assignee_member_id` names the member; no
    -- reference is enforced, because the use case that takes a member off
    -- the team, and syncing, change the filter before the member goes.
    assignee_kind       TEXT NOT NULL,
    assignee_member_id  TEXT,
    -- 1 if tasks in a status of the category done are left out, else 0.
    hides_finished      INTEGER NOT NULL,
    -- The place of the view among the views of its project, the smallest
    -- first. Two views at one place are ordered by their identifiers.
    position            INTEGER NOT NULL
) STRICT, WITHOUT ROWID;

CREATE INDEX views_by_project ON views (project_id);

-- Every project of an earlier version gets the six views a new project
-- starts with, each named after its kind and showing every task. Their
-- identifiers follow from the project and the kind: the identifier of the
-- project with its version digit replaced by a digit of the kind. Two
-- computers that both had the project before this version thus give it the
-- same six views, which merge into one set when they keep in step.
INSERT INTO views (
    id, project_id, kind, name, assignee_kind, assignee_member_id, hides_finished, position
)
SELECT
    substr(project.id, 1, 14) || starting.digit || substr(project.id, 16),
    project.id,
    starting.kind,
    starting.name,
    'anyone',
    NULL,
    0,
    starting.position
FROM projects AS project
CROSS JOIN (
    SELECT 'board' AS kind, 'Board' AS name, '1' AS digit, 0 AS position
    UNION ALL SELECT 'backlog', 'Backlog', '2', 1
    UNION ALL SELECT 'list', 'List', '3', 2
    UNION ALL SELECT 'gantt', 'Gantt', '4', 3
    UNION ALL SELECT 'calendar', 'Calendar', '5', 4
    UNION ALL SELECT 'dashboard', 'Dashboard', '6', 5
) AS starting;

-- The new views are logged as changes of this computer, so that they reach
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
    'view',
    numbered.id,
    'upsert',
    json_object(
        'id', numbered.id,
        'project_id', numbered.project_id,
        'kind', numbered.kind,
        'name', numbered.name,
        'assignee_kind', 'anyone',
        'assignee_member_id', NULL,
        'hides_finished', json('false'),
        'position', numbered.position
    )
FROM (
    SELECT listed.*, row_number() OVER (ORDER BY listed.project_id, listed.position) AS n
    FROM views AS listed
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

-- Each new view gets its merged state: the state just logged, all of whose
-- fields have the stamp of that operation.
INSERT INTO merged_states (entity, entity_id, state, base_stamp)
SELECT entity, entity_id, payload, stamp FROM operations WHERE entity = 'view';
