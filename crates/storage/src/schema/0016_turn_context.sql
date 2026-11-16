-- Teamwright database, version 16: how full the context of a session was
-- after a turn.
--
-- The conventions of version 1 hold.
--
-- A conversation is one session of the agent program, and the session has
-- a window of a fixed size. After a turn the program says how many tokens
-- its last request held, and the size of the window of the model. Either
-- may be missing: NULL where the program did not say. A window without
-- what was held is damaged data.
--
-- Nothing is logged here: every turn of an earlier version has no context,
-- and a logged state of a turn without these fields is read as a turn
-- without one, on every computer.
ALTER TABLE turns ADD COLUMN context_tokens INTEGER;
ALTER TABLE turns ADD COLUMN context_window_tokens INTEGER;
