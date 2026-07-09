import { useEffect, useRef, useSyncExternalStore } from "react";

import { queryClient } from "@/lib/query-client";
import {
  connect,
  getConnectionState,
  onSqlEvent,
  onStateChange,
  type ConnectionState,
  type WsSqlEventSummary,
} from "@/lib/websocket/sql-stream";
import type { PaginatedResponse, SqlEvent } from "@/types";

/**
 * Hook that connects to the SQL WebSocket stream and integrates incoming
 * events with the TanStack Query cache.
 *
 * The WebSocket connection is a module-level singleton — multiple hook
 * instances share the same connection. Only the first mount triggers
 * `connect()`.
 */
export function useSqlStream(): {
  connectionState: ConnectionState;
  latestEvent: WsSqlEventSummary | null;
} {
  const latestEventRef = useRef<WsSqlEventSummary | null>(null);

  // Connect on mount (idempotent if already connected).
  useEffect(() => {
    connect();

    const unsubEvent = onSqlEvent((event) => {
      latestEventRef.current = event;

      // Prepend the event into the TanStack Query cache for ["sql-events"].
      // This makes the SQL List table update without a refetch.
      queryClient.setQueryData(
        ["sql-events", undefined],
        (old: PaginatedResponse<SqlEvent> | undefined) => {
          const incoming: SqlEvent = {
            id: event.id,
            timestamp: event.timestamp,
            target_name: event.target_name,
            protocol: event.protocol,
            database_type: "",
            connection_id: "",
            client_addr: "",
            backend_addr: "",
            user: "",
            database: "",
            kind: "",
            status: event.status,
            duration_ms: event.duration_ms,
            original_sql: event.sql_preview,
            expanded_sql: "",
            fingerprint: "",
            rows: { affected: 0, returned: 0 },
            metadata: {},
          };

          if (!old) {
            return { items: [incoming] };
          }

          // Deduplicate by ID.
          if (old.items.some((item) => item.id === incoming.id)) {
            return old;
          }

          return { ...old, items: [incoming, ...old.items] };
        },
      );
    });

    return () => {
      unsubEvent();
      // Do NOT disconnect — the connection is global/shared.
    };
  }, []);

  // Use useSyncExternalStore for the connection state so React re-renders
  // efficiently when the WS state changes.
  const connectionState = useSyncExternalStore(
    onStateChange,
    getConnectionState,
  );

  return { connectionState, latestEvent: latestEventRef.current };
}
