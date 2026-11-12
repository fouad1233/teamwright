-- Teamwright database, version 9: the computer a conversation runs on.
--
-- The conventions of version 1 hold.
--
-- A conversation is a session of the agent program, which lives on the
-- computer it was started on. Only that computer can go on with it. The
-- logged state of a conversation gains `home_computer`.

-- The identity of the computer the conversation was started on. A program
-- of this version never leaves it empty.
ALTER TABLE conversations ADD COLUMN home_computer TEXT;

-- Every conversation in a database of an earlier version was started on
-- this computer, whose identity is in `meta`. A new database has no
-- conversations yet, and gets its identity after the migrations.
UPDATE conversations
SET home_computer = (SELECT value FROM meta WHERE key = 'device_id');

-- The states of conversations in the log say the same, so that a state
-- read back from the log is whole. Only this fact is added; nothing else in
-- the log changes.
UPDATE operations
SET payload = json_set(
    payload,
    '$.home_computer',
    (SELECT value FROM meta WHERE key = 'device_id')
)
WHERE entity = 'conversation'
  AND payload IS NOT NULL
  AND json_extract(payload, '$.home_computer') IS NULL
  AND EXISTS (SELECT 1 FROM meta WHERE key = 'device_id');
