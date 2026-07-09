# SQL Lens Security

## Security Position

SQL Lens observes database traffic. That makes security and privacy part of the core design, not an add-on.

The product is local-first and intended for developer machines, local demo
setups, CI, and pre-production debugging. The open source core does not provide
application-layer authentication, RBAC, or CSRF protection and should not be
treated as a shared production web service.

## Threat Model

Assets:

- Database credentials.
- SQL text.
- Query parameters.
- Application data inside SQL literals.
- Connection metadata.
- Capture files.

Potential attackers:

- Local users on the same machine.
- Users on the same network when SQL Lens binds to non-local addresses.
- Malicious SQL contents rendered in UI.
- Plugins with excessive access.

## Passwords And Credentials

Rules:

- Never log database passwords.
- Never persist authentication packet payloads.
- Never expose credentials through API responses.
- Redact credentials in connection strings.
- Do not include secrets in panic messages.

## TLS

Supported modes:

- `disabled`.
- `passthrough`.
- `terminate`.
- `upstream`.

Security rules:

- TLS termination must be explicit.
- Private keys must be read from configured files, not API payloads.
- TLS passthrough limits SQL capture because encrypted payloads cannot be decoded.
- Documentation must be clear when a mode reduces capture visibility.

## Log Redaction

Logs should include:

- Connection IDs.
- Event IDs.
- Protocol.
- State transitions.
- Error categories.

Logs should not include:

- Passwords.
- Raw auth payloads.
- Full SQL with sensitive parameters unless explicitly configured.
- TLS private key paths with contents.

## SQL Redaction

Redaction points:

- Parameter decoding.
- SQL expansion.
- Storage write.
- WebSocket broadcast.
- API serialization.
- Exporters.

Redaction selectors:

- Parameter name.
- Column name heuristic.
- SQL pattern.
- Value classifier.
- Plugin-provided rule.

Default sensitive names:

- `password`.
- `passwd`.
- `token`.
- `secret`.
- `api_key`.
- `access_key`.
- `refresh_token`.

## Local API Boundary

SQL Lens should bind web/API listeners to loopback addresses by default. Binding
to a non-loopback address is a deliberate local-network exposure choice and does
not enable an auth layer. Users who need a shared or internet-facing deployment
must place SQL Lens behind infrastructure that supplies access control.

## XSS

SQL text is untrusted input.

Rules:

- Render SQL as text, not HTML.
- Escape all dynamic content.
- Avoid `dangerouslySetInnerHTML`.
- Treat error messages from databases as untrusted.
- Monaco content must be supplied as plain text.

## Plugin Security

Plugins are high risk.

Rules:

- Plugins disabled by default.
- Timeouts for hook execution.
- Clear network permission flags.
- No secret access unless explicitly granted.
- Plugin errors must not crash proxy sessions.

## Replay Safety

Replay can mutate data.

Rules:

- Replay execution requires explicit user action.
- Mutating SQL requires confirmation.
- Replay target must be visible.
- Replay should show final SQL before execution.
- Replay should be disabled by config in restricted environments.

## Security Disclosure

The project should publish a security policy:

```text
SECURITY.md
```

Recommended process:

- Private vulnerability report email or GitHub Security Advisory.
- Acknowledge within 7 days.
- Fix timeline based on severity.
- Publish advisory for confirmed vulnerabilities.
