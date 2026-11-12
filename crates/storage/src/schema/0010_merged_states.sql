-- Teamwright database, version 10: the stamps of the fields, and the
-- merged state of every thing.
--
-- The conventions of version 1 hold.
--
-- Computers that keep in step merge what they changed field by field
-- (decision 0017). For that, every field of every stored thing keeps the
-- stamp of the write that set it, and every operation carries the stamps of
-- the fields of its state that are older than itself.

-- The number of the operation on the computer that made it. For an
-- operation of this computer it equals `sequence`; an operation taken in
-- from another computer keeps the number it has there.
ALTER TABLE operations ADD COLUMN device_sequence INTEGER;
UPDATE operations SET device_sequence = sequence;

-- For an upsert: the stamps of the fields of its state that are older than
-- the operation, as a JSON object from the name of the field to its stamp.
-- NULL when every field has the stamp of the operation, as every operation
-- written before this version has.
ALTER TABLE operations ADD COLUMN field_stamps TEXT;

-- For a delete: for each computer, by its identity, the stamp of the latest
-- change of the thing the deleting computer had seen, as a JSON object.
ALTER TABLE operations ADD COLUMN seen TEXT;

-- NULL once the operation is applied. Otherwise why it waits: 'parent' (it
-- refers to something that has not arrived yet), 'unknown entity' (a newer
-- version wrote it), 'damaged' (its state breaks a rule), or 'never' (it
-- describes another computer and is not applied here).
ALTER TABLE operations ADD COLUMN waiting TEXT;

CREATE UNIQUE INDEX operations_by_device ON operations (device_id, device_sequence);
CREATE INDEX operations_by_stamp ON operations (stamp);

-- The merged state of every thing that was ever stored, with the stamps of
-- its fields. It is what an operation from another computer is merged
-- into.
CREATE TABLE merged_states (
    entity        TEXT NOT NULL,
    entity_id     TEXT NOT NULL,
    -- The merged state as JSON, keys this version does not know included.
    -- NULL for a thing that is only known by its deletion.
    state         TEXT,
    -- The latest stamp of any field.
    base_stamp    TEXT NOT NULL,
    -- The stamps of the fields that are older than `base_stamp`, as a JSON
    -- object from the name of the field to its stamp. NULL if there are
    -- none.
    field_stamps  TEXT,
    -- The stamp of the deletion, for a thing that was deleted. A deleted
    -- thing stays deleted.
    deleted_stamp TEXT,
    -- For each computer, the stamp of the latest change of the thing that
    -- the deleting computer had seen, as a JSON object.
    deleted_seen  TEXT,
    PRIMARY KEY (entity, entity_id)
) STRICT, WITHOUT ROWID;

-- Every thing in the log gets its merged state: the state of its latest
-- upsert, all of whose fields have the stamp of that upsert, and the
-- deletion if its latest operation is one.
INSERT INTO merged_states (
    entity, entity_id, state, base_stamp, field_stamps, deleted_stamp, deleted_seen
)
SELECT
    latest.entity,
    latest.entity_id,
    (
        SELECT upsert.payload FROM operations AS upsert
        WHERE upsert.entity = latest.entity AND upsert.entity_id = latest.entity_id
          AND upsert.payload IS NOT NULL
        ORDER BY upsert.stamp DESC
        LIMIT 1
    ),
    coalesce(
        (
            SELECT max(upsert.stamp) FROM operations AS upsert
            WHERE upsert.entity = latest.entity AND upsert.entity_id = latest.entity_id
              AND upsert.payload IS NOT NULL
        ),
        latest.stamp
    ),
    NULL,
    CASE WHEN latest.kind = 'delete' THEN latest.stamp END,
    CASE WHEN latest.kind = 'delete' THEN (
        SELECT json_group_object(seen.device_id, seen.stamp) FROM operations AS seen
        WHERE seen.entity = latest.entity AND seen.entity_id = latest.entity_id
          AND seen.kind = 'upsert'
          AND seen.stamp = (
              SELECT max(other.stamp) FROM operations AS other
              WHERE other.entity = seen.entity AND other.entity_id = seen.entity_id
                AND other.device_id = seen.device_id AND other.kind = 'upsert'
          )
    ) END
FROM operations AS latest
WHERE latest.stamp = (
    SELECT max(other.stamp) FROM operations AS other
    WHERE other.entity = latest.entity AND other.entity_id = latest.entity_id
);

-- A deletion of a thing that was never upserted saw nothing.
UPDATE merged_states SET deleted_seen = NULL WHERE deleted_seen = '{}';
