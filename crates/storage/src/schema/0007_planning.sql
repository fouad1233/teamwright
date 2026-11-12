-- Teamwright database, version 7: planning a project with the Advisor.
--
-- The conventions of version 1 hold.
--
-- A conversation is with a specialist or with the Advisor. The Advisor is
-- not a member of the team, so a conversation with it has no member and no
-- task, and keeps the model and the effort the Advisor runs on in it.
--
-- Changes are logged in `operations` with the entities 'interview' and
-- 'proposal', whose identifier is the identifier of the turn that asked or
-- proposed. Their state is the whole interview, with its questions, their
-- options and the answers, and the whole proposal, with its members, its
-- tasks and what each task waits for. The deletion of a turn stands for the
-- deletion of its interview and its proposal.

-- 1 for a conversation with the Advisor, 0 for one with a specialist.
ALTER TABLE conversations ADD COLUMN with_advisor INTEGER NOT NULL DEFAULT 0;
-- The model and the effort of the Advisor in this conversation, or NULL for
-- the defaults of the program. Always NULL in a conversation with a
-- specialist.
ALTER TABLE conversations ADD COLUMN advisor_model TEXT;
ALTER TABLE conversations ADD COLUMN advisor_effort TEXT;

-- The interview a turn asked, one row per turn that asked one.
CREATE TABLE turn_interviews (
    turn_id     TEXT PRIMARY KEY REFERENCES turns (id) ON DELETE CASCADE,
    -- When the manager answered, or NULL while the interview is open.
    answered_at TEXT
) STRICT, WITHOUT ROWID;

-- The questions of an interview, in order, counted from 0, each with its
-- answer.
CREATE TABLE turn_interview_questions (
    turn_id         TEXT NOT NULL REFERENCES turn_interviews (turn_id) ON DELETE CASCADE,
    position        INTEGER NOT NULL,
    question        TEXT NOT NULL,
    -- Why the Advisor asks. Empty if it did not say.
    reason          TEXT NOT NULL,
    -- The recommended option, by its position counted from 0, or NULL.
    recommended     INTEGER,
    -- The answer of the manager: the chosen option by its position, or NULL
    -- for an answer in words only, and the words. `answer_note` is NULL
    -- while the interview is open, and then so is `answered_option`.
    answered_option INTEGER,
    answer_note     TEXT,
    PRIMARY KEY (turn_id, position)
) STRICT, WITHOUT ROWID;

-- The options of a question, in order, counted from 0. A question that is
-- answered in words has none.
CREATE TABLE turn_interview_options (
    turn_id           TEXT NOT NULL,
    question_position INTEGER NOT NULL,
    position          INTEGER NOT NULL,
    label             TEXT NOT NULL,
    -- What choosing it leads to. May be empty.
    consequence       TEXT NOT NULL,
    PRIMARY KEY (turn_id, question_position, position),
    FOREIGN KEY (turn_id, question_position)
        REFERENCES turn_interview_questions (turn_id, position) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;

-- The proposal a turn made, one row per turn that proposed. Once it is
-- approved, its members and tasks are those the manager approved.
CREATE TABLE turn_proposals (
    turn_id     TEXT PRIMARY KEY REFERENCES turns (id) ON DELETE CASCADE,
    -- The project in the Advisor's words.
    summary     TEXT NOT NULL,
    -- When the manager approved it, or NULL while it is open.
    approved_at TEXT
) STRICT, WITHOUT ROWID;

-- The members a proposal adds to the team, in order, counted from 0.
CREATE TABLE turn_proposal_members (
    turn_id        TEXT NOT NULL REFERENCES turn_proposals (turn_id) ON DELETE CASCADE,
    position       INTEGER NOT NULL,
    name           TEXT NOT NULL,
    responsibility TEXT NOT NULL,
    reason         TEXT NOT NULL,
    -- NULL for the default of the program.
    model          TEXT,
    effort         TEXT,
    PRIMARY KEY (turn_id, position)
) STRICT, WITHOUT ROWID;

-- The tasks a proposal creates, in order, counted from 0.
CREATE TABLE turn_proposal_tasks (
    turn_id          TEXT NOT NULL REFERENCES turn_proposals (turn_id) ON DELETE CASCADE,
    position         INTEGER NOT NULL,
    title            TEXT NOT NULL,
    description      TEXT NOT NULL,
    -- 'manager', 'member' or NULL for nobody. A member is named in
    -- `owner_name`, which is NULL otherwise.
    owner_kind       TEXT,
    owner_name       TEXT,
    estimate_minutes INTEGER,
    priority         TEXT NOT NULL,
    PRIMARY KEY (turn_id, position)
) STRICT, WITHOUT ROWID;

-- Which task of a proposal waits for which other task of the same
-- proposal, by their positions.
CREATE TABLE turn_proposal_task_waits (
    turn_id       TEXT NOT NULL,
    task_position INTEGER NOT NULL,
    -- The order in which the task names them, counted from 0.
    position      INTEGER NOT NULL,
    waits_for     INTEGER NOT NULL,
    PRIMARY KEY (turn_id, task_position, position),
    FOREIGN KEY (turn_id, task_position)
        REFERENCES turn_proposal_tasks (turn_id, position) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;
