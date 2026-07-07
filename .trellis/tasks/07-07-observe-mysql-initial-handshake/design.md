# Observe MySQL initial handshake design

## Boundary

Implement in `crates/sql-lens-protocol-mysql`.

This task adds MySQL initial server handshake parsing and a small adapter state transition. It does not add client authentication parsing, auth result handling, command parsing, capture event emission, logging, or TCP packet buffering.

## Current State

- `MysqlProtocolAdapter` currently counts observed client/backend bytes and emits no events.
- `MysqlConnectionState` stores only observed byte counters.
- `parse_mysql_packet` parses one complete MySQL packet envelope and returns a payload slice.
- The adapter has no handshake phase yet.

## Protocol Shape

The server initial handshake packet is a backend-to-client packet whose payload starts with protocol version `10` for the classic MySQL Protocol 10 handshake.

The task should parse these safe fields:

```text
protocol_version:        int<1>
server_version:          string<NUL>
connection_id:           int<4>
auth_plugin_data_part_1: string[8]     # skip, do not store
filler:                  int<1>
capability_flags_1:      int<2>        # optional when payload is long enough
character_set:           int<1>        # optional
status_flags:            int<2>        # optional
capability_flags_2:      int<2>        # optional
auth_plugin_data_len:    int<1>        # optional
reserved:                string[10]    # optional
auth_plugin_data_part_2: string[...]   # skip, do not store
auth_plugin_name:        string<NUL>   # optional
```

Only safe metadata is exposed. Authentication challenge bytes are protocol setup data but should be treated as sensitive enough to avoid storing or logging.

## Public API

Add a `handshake.rs` module and re-export parser contracts from `lib.rs`:

```rust
pub struct MysqlInitialHandshake {
    pub protocol_version: u8,
    pub server_version: String,
    pub connection_id: u32,
    pub capability_flags: Option<u32>,
    pub character_set: Option<u8>,
    pub status_flags: Option<u16>,
    pub auth_plugin_name: Option<String>,
}

pub fn parse_initial_handshake(
    payload: &[u8],
) -> Result<MysqlInitialHandshake, MysqlHandshakeParseError>;

pub enum MysqlHandshakeParseError {
    EmptyPayload,
    UnsupportedProtocolVersion { version: u8 },
    MissingServerVersionTerminator,
    IncompletePayload { field: &'static str, needed: usize, available: usize },
    InvalidUtf8 { field: &'static str },
}
```

Add a connection phase enum in the MySQL crate:

```rust
pub enum MysqlConnectionPhase {
    AwaitingInitialHandshake,
    InitialHandshakeSeen,
}
```

Extend `MysqlConnectionState`:

```rust
pub struct MysqlConnectionState {
    client_bytes_observed: usize,
    backend_bytes_observed: usize,
    phase: MysqlConnectionPhase,
    initial_handshake: Option<MysqlInitialHandshake>,
}
```

Expose read-only accessors for tests and future tasks:

```rust
impl MysqlConnectionState {
    pub fn phase(&self) -> MysqlConnectionPhase;
    pub fn initial_handshake(&self) -> Option<&MysqlInitialHandshake>;
}
```

## Adapter Behavior

`observe_backend_bytes` should:

- Always increment `backend_bytes_observed` by `bytes.len()`.
- If phase is not `AwaitingInitialHandshake`, leave handshake state unchanged.
- If phase is `AwaitingInitialHandshake`, attempt to parse a complete MySQL packet using `parse_mysql_packet`.
- If packet parsing succeeds and `sequence_id == 0`, parse the packet payload as an initial handshake.
- If handshake parsing succeeds, store the sanitized `MysqlInitialHandshake` and set phase to `InitialHandshakeSeen`.
- Emit no capture events for handshake observation.

For incomplete or malformed packets, keep the state in `AwaitingInitialHandshake` and return a successful `ProtocolObservation` for the observed bytes. This keeps the observer tolerant until a dedicated stream buffering task owns partial-packet handling.

`observe_client_bytes` should continue counting bytes only in this task.

## Parser Rules

- Empty payload returns `EmptyPayload`.
- Protocol versions other than `10` return `UnsupportedProtocolVersion`.
- Server version must be NUL-terminated UTF-8.
- Connection ID is little-endian `u32`.
- The first 8 bytes of auth plugin data and the later auth plugin data are skipped.
- If capability flags are unavailable because the packet ends after required early fields, parse succeeds with optional fields set to `None`.
- If a field is partially present but incomplete, return `IncompletePayload`.
- Authentication plugin name is optional and parsed as UTF-8 when present and NUL-terminated.

## Tests

Unit parser tests:

- Parses a representative Protocol 10 handshake.
- Rejects empty payload.
- Rejects unsupported protocol version.
- Rejects missing server-version terminator.
- Rejects incomplete required connection ID.
- Does not expose auth challenge bytes.

Adapter tests:

- New connection state starts in `AwaitingInitialHandshake`.
- Observing a complete backend handshake packet transitions to `InitialHandshakeSeen`.
- Stored handshake metadata is sanitized and matches expected safe fields.
- Observing client bytes does not transition the handshake phase.
- Handshake observation emits zero events.

## Compatibility

This is additive to `sql-lens-protocol-mysql`.

Existing packet parser APIs and no-op event behavior remain valid. Existing byte counter tests should continue to pass after expected state assertions are updated.

## Rollback

If handshake parsing becomes too broad, keep only the required Protocol 10 safe fields and defer optional capability/plugin parsing. Do not add stream buffering or auth response parsing to solve parser complexity in this task.
