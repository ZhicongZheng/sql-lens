# Guarded Replay Design

Replay execution must be an explicit app/API workflow, not a side effect of preview. The request identifies a configured target and either an event or SQL text. The app resolves the target, validates the mutation confirmation policy, opens a bounded MySQL-compatible client session, executes the statement, and maps the result to a protocol-neutral response.

Never reuse the proxy's backend connection or bypass configured target resolution. The execution path must be isolated from packet forwarding and must use redacted event data where the source is a captured event.
