# Observe MySQL client handshake response design

## Boundary

Implement in `crates/sql-lens-protocol-mysql`.

This task adds safe parsing and state tracking for the MySQL client handshake response. It must not parse passwords, store auth response bytes, detect auth success/failure, update shared core connection models, emit SQL events, or implement stream buffering.

## Current State

- `parse_mysql_packet` parses one complete MySQL packet envelope.
- `parse_initial_handshake` parses backend Protocol 10 initial handshake metadata.
- `MysqlConnectionState` has `AwaitingInitialHandshake` and `InitialHandshakeSeen` phases.
- `observe_client_bytes` currently only counts bytes.

## Protocol Shape

For a modern Protocol 41 client handshake response, safe fields appear before sensitive auth data:

```text
client_capability_flags: int<4>
max_packet_size:         int<4>
character_set:           int<1>
reserved:                string[23]
username:                string<NUL>
auth_response:           string[...]     # skip, do not store
database:                string<NUL>     # only if CLIENT_CONNECT_WITH_DB
auth_plugin_name:        string<NUL>     # only if CLIENT_PLUGIN_AUTH
```

Authentication response length depends on capability flags:

- `CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA`: length-encoded integer followed by auth bytes.
- `CLIENT_SECURE_CONNECTION`: 1-byte length followed by auth bytes.
- Otherwise: NUL-terminated auth bytes.

The parser should skip the auth response according to the flags and expose only safe metadata.

## Constants

Keep MySQL capability flags local to the MySQL crate:

```rust
const CLIENT_CONNECT_WITH_DB: u32 = 0x0000_0008;
const CLIENT_PROTOCOL_41: u32 = 0x0000_0200;
const CLIENT_SSL: u32 = 0x0000_0800;
const CLIENT_SECURE_CONNECTION: u32 = 0x0000_8000;
const CLIENT_PLUGIN_AUTH: u32 = 0x0008_0000;
const CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA: u32 = 0x0020_0000;
```

`CLIENT_SSL` marks an SSLRequest packet shape and should not be treated as a full client handshake response in this task.

## Public API

Add parser contracts next to handshake parsing, either in `handshake.rs` or a small `client_handshake.rs` if the file becomes crowded:

```rust
pub struct MysqlClientHandshakeResponse {
    pub capability_flags: u32,
    pub max_packet_size: u32,
    pub character_set: u8,
    pub username: Option<String>,
    pub database: Option<String>,
    pub auth_plugin_name: Option<String>,
}

pub fn parse_client_handshake_response(
    payload: &[u8],
) -> Result<MysqlClientHandshakeResponse, MysqlClientHandshakeParseError>;

pub enum MysqlClientHandshakeParseError {
    IncompletePayload { field: &'static str, needed: usize, available: usize },
    UnsupportedProtocol { message: &'static str },
    MissingNullTerminator { field: &'static str },
    InvalidUtf8 { field: &'static str },
    InvalidLengthEncodedInteger { field: &'static str },
}
```

Extend phase/state:

```rust
pub enum MysqlConnectionPhase {
    AwaitingInitialHandshake,
    InitialHandshakeSeen,
    ClientHandshakeSeen,
}

impl MysqlConnectionState {
    pub fn client_handshake(&self) -> Option<&MysqlClientHandshakeResponse>;
}
```

## Adapter Behavior

`observe_client_bytes` should:

- Always increment `client_bytes_observed` by `bytes.len()`.
- Attempt client handshake parsing only when state phase is `InitialHandshakeSeen`.
- Parse a complete MySQL packet using `parse_mysql_packet`.
- Accept the normal client response sequence used after the initial handshake.
- If parsing succeeds, store safe metadata and move phase to `ClientHandshakeSeen`.
- Keep malformed/incomplete bytes non-fatal and non-transitioning.
- Emit zero events.

`observe_backend_bytes` should keep Issue 040 behavior unchanged.

## Security Rules

- Never store auth response bytes.
- Never log raw payloads, username/password pairs, or auth plugin response data.
- Username and database are considered safe metadata for local debugging, but still must be parsed as text and stored as optional values.
- TLS/SSLRequest parsing is deferred because it changes the next bytes on the wire and needs a separate design.

## Tests

Parser tests:

- Parses Protocol 41 handshake response with username, database, and auth plugin name.
- Parses response without database and without auth plugin name.
- Skips secure-connection auth response bytes.
- Skips length-encoded auth response bytes.
- Rejects incomplete fixed header.
- Rejects missing username terminator.
- Rejects invalid UTF-8 username/database/plugin name.
- Does not expose auth response bytes in debug output.

Adapter tests:

- Client response after `InitialHandshakeSeen` moves phase to `ClientHandshakeSeen`.
- Stored metadata contains username/database/plugin name and no auth response.
- Client response before initial handshake does not update state.
- Malformed client bytes stay non-fatal and emit zero events.

## Compatibility

This is additive inside `sql-lens-protocol-mysql`.

Existing packet parser and initial handshake APIs remain valid. Existing adapter tests should continue to pass.

## Rollback

If Protocol 41 parsing becomes too broad, keep fixed header + username + secure-connection auth skipping, and defer database/plugin parsing. Do not implement TLS or auth result detection in this task.
