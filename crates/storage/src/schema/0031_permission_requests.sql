-- Teamwright database: what specialists ask to be allowed, and what the
-- manager answers (decision 0038).
--
-- The conventions of version 1 hold.
--
-- A request is logged in `operations` with the entity
-- 'permission_request', whose state is the whole request. What is asked
-- never changes once it is written. The answer is written once; only
-- `withdrawn_at` changes after it, when the manager takes back what was
-- allowed for a task or for always.
--
-- Only the project is enforced as a reference: a request goes with its
-- project, and the program removes and logs it first. The specialist, the
-- conversation, the turns and the task are not, for the same reasons as a
-- handoff's (version 18): what was asked and allowed stays readable.
--
-- `answered_on` is the computer the manager answered on. An allowed
-- permission holds only there: a command is a command of that computer
-- (decisions 0013 and 0027 treat what belongs to one computer the same way).

CREATE TABLE permission_requests (
    id              TEXT PRIMARY KEY,
    project_id      TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    -- The specialist who asked, and the conversation and the turn it asked
    -- in.
    member_id       TEXT NOT NULL,
    conversation_id TEXT NOT NULL,
    turn_id         TEXT NOT NULL,
    -- The task the conversation is about, or NULL.
    task_id         TEXT,
    -- 'command', 'command_prefix' or 'web_domain', and the command, the
    -- prefix or the domain.
    kind            TEXT NOT NULL,
    what            TEXT NOT NULL,
    reason          TEXT NOT NULL,
    asked_at        TEXT NOT NULL,
    -- NULL while the request is open; 'allowed' or 'refused' once answered.
    decision        TEXT,
    -- What was allowed and for how long ('once', 'task' or 'always'). NULL
    -- unless the decision is 'allowed'.
    scope           TEXT,
    allowed_kind    TEXT,
    allowed_what    TEXT,
    -- What the manager added in words. NULL while the request is open.
    note            TEXT,
    answered_at     TEXT,
    answered_on     TEXT,
    -- The turn the answer started.
    next_turn_id    TEXT,
    withdrawn_at    TEXT
) STRICT, WITHOUT ROWID;

CREATE INDEX permission_requests_by_project ON permission_requests (project_id, asked_at);
