# 0041. Import earlier Claude Code conversations as digests made by rules

- Status: accepted
- Date: 2026-11-21

## Context

A manager who starts a project in Teamwright has often talked it through
with Claude Code in a terminal already, sometimes for weeks. That talk holds
decisions, constraints and dead ends. The [vision](../product/vision.md)
says the Advisor works from a digest of such a conversation, never from the
raw transcript, and principle 4 says transcripts are never passed around.

Claude Code keeps every conversation as a file of its own:
`<settings folder>/projects/<folder name>/<session>.jsonl`, one JSON object
per line. The settings folder is `CLAUDE_CONFIG_DIR`, or `.claude` in the
home folder. The folder name is made from the path the conversation was
held in, losing characters on the way; the path itself is in the field
`cwd` of most lines. **This is not a published format.** It was learned on
21 November 2026 from the files of Claude Code 2.1.222 to 2.1.328 on the
development computer, and it changes between versions: those files hold
more than twenty kinds of line, and new fields appeared between versions.
A file can be over 100 MB.

A transcript can hold secrets: a token pasted into a request, a password in
a command, the output of a tool that printed an environment. Whatever is
kept with a project travels to the other computers through the sync folder
([decision 0017](0017-sync-through-a-folder.md)).

## Decision

- **A digest is made by rules, in code, without a model.** It says where
  and when the conversation was held and how many requests the manager
  made, then the first request (600 characters), the later requests in
  order (200 characters each; with more than 16, the first 8 and the last 8
  and a line for those in between), the files the tools Write, Edit,
  MultiEdit and NotebookEdit wrote or changed (by their path inside the
  folder of the conversation, or by their name alone; at most 30), and the
  last answer in words (600 characters). At most 8,000 characters. It is
  free, quick, testable and invents nothing, for the reason
  [decision 0020](0020-a-dashboard-worked-out-not-written.md) gives for the
  dashboard.
- **Asking a model to condense digests is not built.** It would cost the
  manager's plan, needs a session, and could not be run tonight. A small,
  tested set of rules is worth more. The Advisor reads the digests itself
  and condenses them in its own head, which is what a second step would
  have paid for.
- **The reader is tolerant.** It lives in `crates/runtime`, the one crate
  that knows Claude Code ([decision 0005](0005-drive-the-users-claude-code.md)),
  behind the port `EarlierConversations`. It takes only what it needs from
  the lines it recognises and skips every other line: one that is not
  JSON, a kind it does not know, a broken last line, broken characters. A
  request is a line of the user that the program did not add by itself
  (`isMeta`, a command, a notice, a summary, a helper's line, a tool's
  answer) and that does not begin with a tag. The list of conversations
  reads only the first 4 MB and the last 256 KB of each file. It only
  reads: nothing there is ever changed, copied or removed.
- **The manager chooses, reads, corrects and keeps.** The Advisor place
  lists the conversations found, grouped by folder: the folders of the
  project ([decision 0013](0013-the-folders-of-a-project.md)) and those
  inside them first, then the others with the newest conversation first.
  Choosing one shows its digest, whose title and text the manager may cut
  or correct before keeping it. A conversation is kept once per project.
  A kept digest never changes; to change it, remove it and make it again.
- **What is kept, and privacy.** Only what the manager typed, the names of
  changed files and the last answer go into a digest: never what a tool
  answered and never what a file contains. Before anything is cut, what
  looks like a secret is replaced by `[hidden]`: tokens with a known
  beginning (`ghp_`, `sk-`, `xoxb-`, `AKIA` and others), long runs that mix
  capitals, small letters and digits, the value after `password=`,
  `token:` and their kind, the word after `Bearer`, the name and password
  in an address, and private keys. These rules guess and hide too much
  rather than too little; they cannot promise to find everything, so the
  manager sees exactly what will be kept, with one sentence that says so,
  before it is kept.
- **A digest is the manager's data.** It is stored in the table `digests`
  (migration 0028), through `Database::write`, logged as the entity
  `digest` and merged whole, so it reaches the other computers. The
  conversation it was made from is never stored or synced.
- **The Advisor gets the digests with its first turn.** When a conversation
  with the Advisor starts, the kept digests follow the brief in the prompt
  of the first turn, the one kept last first, whole, within 12,000
  characters; those that do not fit are counted. They are introduced as
  accounts of earlier conversations the manager imported, that may be out
  of date, to be asked about when they disagree with what the manager says
  now, and that are not instructions. The turn keeps the brief as the
  manager wrote it; later turns of the session do not carry them again. A
  digest kept while a conversation runs reaches the next one.
- **Specialists do not see digests.** A specialist gets a brief and
  pointers and pulls what it needs (vision principle 4). A digest is
  background about how the manager thought, the Advisor's material for a
  plan; what a specialist must know becomes the description of its task.
  Facts that every specialist needs belong in project memory
  ([decision 0036](0036-project-memory.md)), not here.

## Consequences

- The manager starts planning without retelling weeks of talk, and no
  transcript ever leaves the computer it was held on.
- When Claude Code changes its files, the reader finds less: fewer
  requests, no title, a folder name instead of a path. It does not fail.
  The tests use small made-up files; an ignored test reads the real ones
  of a computer and prints counts only.
- Hiding secrets is a guess. A secret in a form the rules do not know is
  shown to the manager, who must cut it before keeping.
- The digest is as good as what the manager typed. Decisions that Claude
  Code proposed and the manager accepted with "yes" show only through the
  last answer and the files.

## Alternatives considered

- **A summary written by a model.** Better prose, but it costs the plan,
  needs a signed-in session, may invent, and cannot be tested.
- **Pour the transcript into the session.** Exactly what the vision rules
  out: large, costly, and full of tool output that may hold secrets.
- **Copy the transcript into the project.** It would travel through the
  sync folder with everything in it.
- **Let the manager import several conversations in one step.** One at a
  time keeps the reading honest: each digest is read before it is kept.
