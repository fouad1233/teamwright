-- Teamwright database, version 4: the model and the effort of a specialist.
--
-- The conventions of version 1 hold.

-- The model an AI specialist runs on, by the name the program that runs it
-- understands: one word, such as 'sonnet'. NULL leaves it to the program.
-- Always NULL for a person.
ALTER TABLE members ADD COLUMN model TEXT;

-- How hard the model of an AI specialist thinks: 'low', 'medium', 'high',
-- 'xhigh' or 'max'. NULL leaves it to the program. Always NULL for a person.
ALTER TABLE members ADD COLUMN effort TEXT;
