# Contributing to SQL Lens

## Welcome

SQL Lens is intended to be friendly to human contributors and AI coding agents. The project values small changes, clear boundaries, and tests that prove behavior.

## Principles

- Keep changes small.
- Prefer simple designs.
- Do not add abstractions before they remove real complexity.
- Preserve protocol boundaries.
- Do not leak secrets.
- Test protocol behavior with fixtures.
- Update documentation when changing public behavior.

## Development Workflow

1. Pick or create an issue.
2. Discuss scope for large changes.
3. Create a branch.
4. Implement the smallest useful change.
5. Add tests.
6. Update docs.
7. Open a pull request.

## Branch Names

Recommended:

- `feature/mysql-query-capture`
- `fix/redaction-binary-params`
- `docs/protocol-state-machine`
- `test/mysql-prepared-fixtures`

## Commits

Use clear commit messages.

Recommended format:

```text
area: short imperative summary
```

Examples:

- `protocol-mysql: parse com_query packets`
- `storage: add ring buffer eviction tests`
- `web: add sql detail parameter table`
- `docs: document plugin hook contracts`

## Pull Requests

Each PR should include:

- Summary.
- Why the change is needed.
- Testing performed.
- Screenshots for UI changes.
- Compatibility notes for protocol or API changes.
- Security notes for redaction, auth, replay, or plugins.

## Code Style

Rust:

- Format with `rustfmt`.
- Lint with `clippy`.
- Prefer explicit domain types.
- Avoid large modules.
- Keep protocol parsing separate from proxy forwarding.

Frontend:

- Use TypeScript.
- Use shadcn/ui for base components.
- Use TanStack Query for server state.
- Keep feature-specific code under `features/`.
- Avoid untyped API payloads.

## Testing Requirements

Protocol changes:

- Unit tests.
- Golden packet fixture tests.
- Integration test when possible.

Storage changes:

- Retention tests.
- Query filter tests.
- Capacity and eviction tests.

API changes:

- Schema tests.
- Error response tests.

UI changes:

- Component tests for logic-heavy components.
- Playwright smoke test for key flows.

Security-sensitive changes:

- Redaction tests.
- Auth tests.
- XSS or CSRF tests when relevant.

## Issues

Good issues include:

- Problem statement.
- Expected behavior.
- Acceptance criteria.
- Suggested files or modules.
- Test expectations.

## Reviews

Reviewers should check:

- Correct module ownership.
- Protocol correctness.
- Error handling.
- Redaction behavior.
- Test coverage.
- Documentation updates.
- Public API compatibility.

## License

By contributing, you agree that your contributions are licensed under the project license.

