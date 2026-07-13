# Retention Enforcement Design

## Architecture

### Components

1. **RetentionEnforcer Service**
   - Scheduled background task
   - Queries retention configuration from config layer
   - Calculates cutoff timestamps per table/query
   - Executes DELETE operations against storage layer
   - Emits audit logs

2. **Retention Configuration Reader**
   - Reads global default retention period
   - Reads per-table/per-query overrides
   - Provides effective retention for each data source

3. **Storage Layer Interface**
   - Must support time-based deletion queries
   - Must provide count estimates before deletion

### Data Flow

```
Config Store → RetentionEnforcer → calculate_cutoffs() → Storage.delete_before(cutoff)
                                              ↓
                                        AuditLogger.log_enforcement()
```

### Scheduling

- Uses tokio runtime scheduler or dedicated thread
- Configurable interval (default: every 1 hour)
- Non-blocking: runs in background, does not stall capture path

## Contracts

### RetentionEnforcer API (internal)

```rust
pub struct RetentionEnforcer {
    config: Arc<AppConfig>,
    storage: Arc<dyn Storage>,
    audit: Arc<dyn AuditLogger>,
}

impl RetentionEnforcer {
    pub async fn enforce(&self) -> Result<EnforcementReport, RetentionError>;
}

pub struct EnforcementReport {
    pub tables_scanned: usize,
    pub rows_deleted: u64,
    pub duration_ms: u64,
}
```

### Storage Trait Extension

```rust
#[async_trait]
pub trait Storage {
    async fn delete_before(&self, table: &str, cutoff: DateTime<Utc>) -> Result<u64>;
    async fn count_before(&self, table: &str, cutoff: DateTime<Utc>) -> Result<u64>;
}
```

## Trade-offs

| Decision | Rationale |
|----------|-----------|
| Scheduled batch deletion vs. TTL at insert | Batch deletion simpler, avoids per-row overhead, allows config changes to retroactively apply |
| Single global enforcer vs. per-table | Simpler coordination, single audit trail |
| Blocking vs. non-blocking enforcement | Non-blocking required to not stall capture path |

## Operational Considerations

- Enforcement failures logged but do not crash application
- Large deletion batches may need chunking to avoid long transactions
- Consider adding dry-run mode for validation
