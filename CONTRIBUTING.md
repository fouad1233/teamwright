# Contributing to Teamwright

Thank you for wanting to help. This guide applies to everyone who changes the
repository, whether a person or an AI agent.

The project is at an early stage. Before writing a large change, open an issue
and describe what you want to do, so the work fits the
[roadmap](docs/product/roadmap.md) and the
[architecture](docs/architecture/overview.md).

## Ground rules

1. **Small commits.** One logical change per commit. A commit builds, passes
   the tests and can be understood on its own.
2. **Code that explains itself.** A reader should understand what a piece of
   code does from its names and structure. Comments say why, not what.
3. **Documentation moves with the code.** A change that affects behavior
   updates the README of the crate or feature it touches, and the user manual
   if users will notice it.
4. **Decisions are recorded.** A change that reverses or adds a long-lived
   decision adds a record in [`docs/decisions`](docs/decisions/README.md).
5. **Honest status.** Do not describe something as working until it has been
   run. The roadmap marks what exists.

## Commit messages

Commits follow [Conventional Commits](https://www.conventionalcommits.org/).

```
<type>(<optional scope>): <summary in the imperative, lower case>

<body: what changed and why, wrapped at 72 characters>
```

| Type | Use it for |
|---|---|
| `feat` | A new capability |
| `fix` | A bug fix |
| `docs` | Documentation only |
| `refactor` | A change in structure with no change in behavior |
| `test` | Tests only |
| `build` | Build system, dependencies |
| `ci` | Continuous integration |
| `chore` | Repository upkeep |

Scopes are crate or feature names: `domain`, `storage`, `runtime`, `bus`,
`desktop`, `board`, `office`.

## Code standards

### All code

- Names are full words that say what a thing is: `decision_request`, not
  `dr`; `remaining_tokens`, not `rem`.
- A function does one thing. If its name needs "and", split it.
- No dead code, no commented-out code, no unexplained constants.
- Errors are handled where something can be done about them and passed up
  otherwise. They are never silently dropped.
- Every behavior has a test that would fail without it.

### Rust

- Format with `cargo fmt`. Lint with `cargo clippy --all-targets -- -D warnings`.
- `unsafe` is denied. An exception needs `#[expect(unsafe_code, reason = "…")]`
  and has to convince a reviewer.
- No `unwrap` or `expect` outside tests. Return a typed error.
- Each crate has its own error type built with `thiserror`.
- Public items have documentation comments.
- Types protect their own rules: fields are private, and constructors return
  an error for invalid input.
- IDs are distinct types, never bare strings.
- Respect the dependency rule in the
  [architecture overview](docs/architecture/overview.md): inner crates never
  depend on outer ones.

### TypeScript

- Strict mode. No `any`.
- Lint with ESLint and format with Prettier.
- Code is organized by feature. Shared building blocks live in `shared`.
- Components stay small and contain no business rules.
- The backend is reached only through the `Backend` interface.

### The interface

- Follow the [design philosophy](docs/design/philosophy.md). Use the design
  tokens; never a raw color or size.
- Use the existing components for menus, dialogs, dates and tooltips. Do not
  put a browser's own drop-down or date field on screen.
- Look at what you built, in both themes, before you call it done.

## Working on the code

[Development](docs/development/README.md) explains how to build, run and check
the project. Every crate and the interface have a README that says how that
part is organized and how to extend it.

## License

By contributing you agree that your contribution is licensed under the
[Apache License 2.0](LICENSE).
