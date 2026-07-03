# SQL Lens Security

## Security Position

SQL Lens observes database traffic. That makes security and privacy part of the core design, not an add-on.

The default product is local-first. Shared or production-like deployments require explicit hardening.

## Threat Model

Assets:

- Database credentials.
- SQL text.
- Query parameters.
- Application data inside SQL literals.
- Connection metadata.
- Capture files.
- Web UI sessions.

Potential attackers:

- Local users on the same machine.
- Users on the same network when SQL Lens binds to non-local addresses.
- Malicious web pages attempting CSRF.
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

## Web Login

Local mode may disable auth only when binding to loopback.

If binding to a non-loopback address:

- Auth should be enabled.
- Session cookies should be `HttpOnly`.
- Session cookies should use `SameSite=Lax` or stricter.
- Secure cookies should be used with HTTPS.

## CSRF

CSRF protection is required for mutating endpoints when cookie auth is enabled.

High-risk endpoints:

- Replay execute.
- Settings mutation.
- Plugin enable or disable.
- Export destination configuration.

## XSS

SQL text is untrusted input.

Rules:

- Render SQL as text, not HTML.
- Escape all dynamic content.
- Avoid `dangerouslySetInnerHTML`.
- Treat error messages from databases as untrusted.
- Monaco content must be supplied as plain text.

## RBAC

Open source MVP can use a simple role model:

- `viewer`: read events, statistics, and connections.
- `operator`: run replay preview, export data.
- `admin`: change settings, manage plugins, run replay execute.

Enterprise or team editions may add:

- SSO.
- Groups.
- Audit logs.
- Project-level permissions.

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

