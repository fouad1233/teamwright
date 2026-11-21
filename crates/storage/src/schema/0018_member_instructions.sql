-- Teamwright database, version 18: the instructions of a specialist.
--
-- The conventions of version 1 hold.

-- What an AI specialist is told about how it works, in the manager's words.
-- It reaches every turn of the specialist after Teamwright's own
-- instructions. Empty when there are none, and always empty for a person.
-- Lines are separated by a single line feed.
ALTER TABLE members ADD COLUMN instructions TEXT NOT NULL DEFAULT '';
