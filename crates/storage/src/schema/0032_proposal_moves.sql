-- Teamwright database: a proposal of the Advisor that regroups tasks that
-- exist (decision 0046).
--
-- The conventions of version 1 hold.
--
-- A task of a proposal may be a part of a task the project has:
-- `part_of_task` is the identifier of that task, and `part_of` is then
-- NULL. Neither is enforced as a reference: the task may be deleted while
-- the proposal waits, and approving checks the board as it is then.
--
-- `turn_proposal_moves` holds the tasks of the project that a proposal
-- moves, in order, counted from 0. Each goes under a task of the same
-- proposal (`part_of`, its position, counted from 0), under another task of
-- the project (`part_of_task`), or, with both NULL, becomes a task of its
-- own.
--
-- The proposal is logged in `operations` as one entity whose state is the
-- whole of it. A state logged before this version has no `part_of_task`
-- and no `moves`, and is read as a plan that moves nothing, on every
-- computer; a version that does not know them shows the plan without them.
ALTER TABLE turn_proposal_tasks ADD COLUMN part_of_task TEXT;

CREATE TABLE turn_proposal_moves (
    turn_id      TEXT NOT NULL REFERENCES turn_proposals (turn_id) ON DELETE CASCADE,
    position     INTEGER NOT NULL,
    task_id      TEXT NOT NULL,
    part_of      INTEGER,
    part_of_task TEXT,
    PRIMARY KEY (turn_id, position)
) STRICT, WITHOUT ROWID;
