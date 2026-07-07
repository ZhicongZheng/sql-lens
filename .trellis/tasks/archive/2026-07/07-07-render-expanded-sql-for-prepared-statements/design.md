# Design

## Boundary

Implement rendering in `sql-lens-protocol-mysql` first.

Do not add protocol-neutral renderer APIs until there is a second protocol
consumer or a prepared statement event emission task needs one. Core already
has the necessary value model, so this task should not change `sql-lens-core`
contracts.

## Public Shape

Add a MySQL-local renderer API in `execute.rs`:

```rust
pub fn render_expanded_sql(
    template_sql: &str,
    parameters: &[MysqlDecodedParameter],
) -> Result<String, MysqlExpandedSqlRenderError>;
```

Add a MySQL-local envelope field:

```rust
pub struct MysqlStatementExecuteEnvelope {
    pub expanded_sql: Option<String>,
}
```

Adapter behavior:

- Known statement ID with decoded parameters and successful render stores
  `Some(expanded_sql)`.
- Unknown statement ID stores `None`.
- Unsupported parameter decoding stores `None` because there is no complete
  parameter list.
- Render mismatch is non-fatal at adapter level and should not update the
  execute envelope for malformed known-statement payloads.

## Placeholder Scanner

Scan the template SQL byte-by-byte as UTF-8 text and replace `?` only in normal
SQL context.

Skip placeholders inside:

- single-quoted strings
- double-quoted strings
- backtick-quoted identifiers
- line comments starting with `-- ` or `#`
- block comments `/* ... */`

String quote handling should support doubled quotes such as `''` inside
single-quoted strings.

This scanner is intentionally small and explicit. It is not a full SQL parser.

## Literal Rendering

Render by `SqlParameterValue`:

- `Null` -> `NULL`
- `Integer`, `Unsigned`, `Float` -> decimal string
- `Boolean` -> `TRUE` / `FALSE`
- `String`, `Date`, `Time`, `Timestamp`, `Json`, `BinarySummary`,
  `Unsupported` -> single-quoted display literal

Single-quoted display literals escape embedded single quotes by doubling them.
Control characters may use readable backslash escapes for display, but the
output is not intended to be replayed as exact executable SQL.

Binary values render the existing summary string only, never raw bytes.

## Error Handling

Structured errors should cover:

- not enough parameters for placeholders
- extra parameters after all placeholders are consumed

Errors must not include raw parameter bytes. Error messages may include counts
and indexes.

## Trade-Offs

- Keeping the renderer MySQL-local avoids prematurely generalizing SQL dialect
  behavior.
- The output is readable debug SQL, not an exact replay artifact. Later replay
  can use the original template plus structured parameters.
- Redaction is out of scope because Issue 056 owns the security policy before
  storage or WebSocket exposure.
