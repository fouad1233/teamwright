# teamwright-integrations

Teamwright's ways out to other services
([decision 0027](../../docs/decisions/0027-integrations.md)). It implements
ports the application crate declares, and is the only crate that knows the
options and the output of GitHub's command line program.

## What is in it

| Module | Contents |
|---|---|
| `locate` | `GhLocator`: finds `gh`: the program the manager named, then the search path, then the places it is usually installed (on Windows `%LOCALAPPDATA%\Programs\gh\bin`, `%ProgramFiles%\GitHub CLI`, scoop; elsewhere Homebrew, `/usr/local/bin`, `/usr/bin`, `~/.local/bin`) |
| `command` | Runs a short command with a time limit, its arguments as arguments (never through a shell), without a console window, with prompts and colours turned off |
| `github` | `GhCommandLine`, which implements `GitHubProgram`: `gh auth status`, `git remote get-url origin` of the work folder, `gh repo view`, `gh pr list` and `gh issue list` as JSON (state all, at most 200 each), `gh issue create` |
| `web` | `HttpsSender`, which implements `WebSender` with `ureq` (blocking, rustls, a time limit of 10 seconds): `{"text": …}` to a Slack channel, and an event as JSON (`payload_json`) with the headers `X-Teamwright-Event`, `X-Teamwright-Version` and, with a secret, `X-Teamwright-Signature` (`event_request`). An answer that is not a success is its status; a failure is said without the address |
| `signature` | `sha256`, `hmac_sha256`, `hex`: small, and checked against the published test vectors, so that no cryptography crate is added for one signature |

## Rules

- Teamwright keeps no token. Nothing here asks `gh` for one, and no error
  repeats what `gh` was given.
- Every call has a time limit of 20 seconds; a call that runs longer is
  stopped and said to have taken too long.
- A failure is said in plain words: the first line `gh` wrote, or a
  sentence of ours for the cases that are known (no such repository, not
  signed in). A repository whose issues are turned off has none.

## Tests

The unit tests read JSON in the shape `gh` writes it, the locator works on
folders of its own, and the signature is checked against FIPS 180-4 and
RFC 4231. `tests/listener.rs` starts a listener on this computer and sends
it a signed event, as an address would get it; nothing leaves the
computer. `tests/real_gh.rs` reads a real repository through
the `gh` of this computer; it is ignored by default:

```bash
cargo test -p teamwright-integrations --test real_gh -- --ignored --nocapture
```

`TEAMWRIGHT_GH_REPOSITORY` names another repository than
`fouad1233/teamwright`. The test only reads.
