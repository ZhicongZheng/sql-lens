# Detect MySQL authentication result design

## Boundary

Implement in `crates/sql-lens-protocol-mysql`.

This task observes backend-to-client packets after the client handshake response and updates MySQL-specific connection state. It must not emit SQL events, parse commands, persist raw auth payloads, implement auth switch flows, or update protocol-neutral core connection models.

## Current State

- `parse_mysql_packet` parses one complete MySQL packet.
- `MysqlConnectionState` can reach `ClientHandshakeSeen` after Issue 041.
- Authentication success/failure phases do not exist yet.
- `observe_backend_bytes` currently handles initial handshake only.

## Packet Shape

After a client handshake response, backend authentication result packets commonly use:

```text
OK packet:
  header: 0x00
  remaining payload: OK packet fields, ignored in this task

ERR packet:
  header: 0xff
  error_code: int<2>
  sql_state_marker: "#"
  sql_state: string[5]
  error_message: string<EOF>
```

Only safe metadata from ERR is stored. Raw packet bytes are not retained.

## Public API

Add auth result types in the MySQL crate:

```rust
pub enum MysqlAuthenticationStatus {
    Succeeded,
    Failed,
}

pub struct MysqlAuthenticationResult {
    pub status: MysqlAuthenticationStatus,
    pub error_code: Option<u16>,
    pub sql_state: Option<String>,
    pub message: Option<String>,
}
```

Extend phase:

```rust
pub enum MysqlConnectionPhase {
    AwaitingInitialHandshake,
    InitialHandshakeSeen,
    ClientHandshakeSeen,
    Authenticated,
    AuthenticationFailed,
}
```

Expose read-only state:

```rust
impl MysqlConnectionState {
    pub fn authentication_result(&self) -> Option<&MysqlAuthenticationResult>;
}
```

## Parser Behavior

Add a small backend auth result parser:

```rust
pub fn parse_authentication_result(
    payload: &[u8],
) -> Result<Option<MysqlAuthenticationResult>, MysqlAuthenticationResultParseError>;
```

Rules:

- Empty payload returns `IncompletePayload`.
- Payload first byte `0x00` returns success.
- Payload first byte `0xff` returns failure and parses optional error code / SQL state / message.
- Other first bytes return `Ok(None)` to represent unsupported auth continuation or non-result packet.
- Invalid UTF-8 in SQL state or message returns a structured parse error.
- Do not retain raw payload bytes.

## Adapter Behavior

`observe_backend_bytes` should:

- Continue initial handshake behavior while awaiting initial handshake.
- Attempt auth result parsing only when phase is `ClientHandshakeSeen`.
- Parse a complete MySQL packet using `parse_mysql_packet`.
- Accept normal backend auth result sequence after client handshake.
- On success, store result and move phase to `Authenticated`.
- On failure, store result and move phase to `AuthenticationFailed`.
- Unsupported result packets stay non-fatal and keep phase `ClientHandshakeSeen`.
- Emit zero events.

## Tests

Parser tests:

- Parses OK packet as success.
- Parses ERR packet with code, SQL state, and message as failure.
- Returns `None` for unsupported first byte.
- Rejects empty payload.
- Rejects invalid UTF-8 in message.

Adapter tests:

- Backend OK after client handshake moves phase to `Authenticated`.
- Backend ERR after client handshake moves phase to `AuthenticationFailed`.
- OK packet before client handshake does not update auth state.
- Unsupported auth continuation packet is non-fatal and does not transition.
- Auth result observation emits zero events.

## Compatibility

This is additive inside `sql-lens-protocol-mysql`.

Later command parsing can start from `Authenticated`. Later auth switch work can refine the unsupported continuation branch without changing the public safe result type.

## Rollback

If ERR packet parsing becomes too broad, keep success/failure state detection by first byte and defer detailed ERR metadata. Do not add command parsing or auth switch handling in this task.
