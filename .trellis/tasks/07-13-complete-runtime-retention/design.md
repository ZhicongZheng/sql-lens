# Runtime Retention Design

Normalize runtime timestamps before implementing age cleanup. Prefer a typed or canonical epoch representation at the storage boundary; do not compare mixed arbitrary timestamp strings.

The enforcer should calculate one immutable enforcement snapshot per cycle, then apply count and age policies to each owned backend. Cleanup operations should be bounded or chunked where the storage API permits it. If dynamic config is implemented, use an `Arc<RwLock<RetentionConfig>>` or equivalent app-owned reader and never hold the config lock during deletion.

Per-table/per-query retention overrides are out of scope until storage exposes a stable source/table identity. The task must either implement that identity or explicitly document global-only behavior rather than pretending overrides exist.
