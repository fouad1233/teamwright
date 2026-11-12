-- Teamwright database, version 5: questions a specialist asks the manager.
--
-- The conventions of version 1 hold.
--
-- Changes are logged in `operations` with the entity 'decision', whose
-- identifier is the identifier of the turn that asked. Its state is the
-- whole question: the question, its options and the answer. The deletion
-- of a turn stands for the deletion of its question.

-- The question a turn asked, one row per turn that asked one.
CREATE TABLE turn_decisions (
    turn_id         TEXT PRIMARY KEY REFERENCES turns (id) ON DELETE CASCADE,
    question        TEXT NOT NULL,
    -- Why it matters. Empty if the specialist did not say.
    reason          TEXT NOT NULL,
    -- The recommended option, by its position counted from 0, or NULL.
    recommended     INTEGER,
    -- The answer of the manager: the chosen option by its position, or NULL
    -- for an answer in words only; the words; and when it was given.
    -- `answered_at` is NULL while the question is open, and then so are the
    -- other two.
    answered_option INTEGER,
    answer_note     TEXT,
    answered_at     TEXT
) STRICT, WITHOUT ROWID;

-- The options of a question, in order, counted from 0.
CREATE TABLE turn_decision_options (
    turn_id     TEXT NOT NULL REFERENCES turn_decisions (turn_id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    label       TEXT NOT NULL,
    -- What choosing it leads to. May be empty.
    consequence TEXT NOT NULL,
    PRIMARY KEY (turn_id, position)
) STRICT, WITHOUT ROWID;
