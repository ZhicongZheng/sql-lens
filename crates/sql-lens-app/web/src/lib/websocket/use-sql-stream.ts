import { useCallback, useEffect, useRef, useSyncExternalStore } from "react";

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

// ---------------------------------------------------------------------------
// Module-level pause queue (shared across hook instances)
// ---------------------------------------------------------------------------

const MAX_QUEUE_SIZE = 200;
let eventQueue: WsSqlEventSummary[] = [];
let isPaused = false;
let queueListeners = new Set<() => void>();

function enqueueEvent(event: WsSqlEventSummary) {
  eventQueue.push(event);
  if (eventQueue.length > MAX_QUEUE_SIZE) {
    eventQueue = eventQueue.slice(eventQueue.length - MAX_QUEUE_SIZE);
  }
  for (const l of queueListeners) l();
}

function flushQueue() {
  if (eventQueue.length === 0) return;

  const events = eventQueue;
  eventQueue = [];
  for (const l of queueListeners) l();

  // Flush all queued events into the TanStack Query cache at once.
  queryClient.setQueryData(
    ["sql-events", undefined],
    (old: PaginatedResponse<SqlEvent> | undefined) => {
      let result = old ?? { items: [] };
      for (const event of events) {
        const incoming = toSqlEvent(event);
        if (!result.items.some((item) => item.id === incoming.id)) {
          result = { ...result, items: [incoming, ...result.items] };
        }
      }
      return result;
    },
  );
}

function setPaused(paused: boolean) {
  isPaused = paused;
  if (!paused) {
    flushQueue();
  }
  for (const l of queueListeners) l();
}

function getQueuedCount(): number {
  return eventQueue.length;
}

/** Convert a WebSocket summary to a stub SqlEvent for cache insertion. */
function toSqlEvent(event: WsSqlEventSummary): SqlEvent {
  return {
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
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export interface UseSqlStreamOptions {
  /** When true, incoming events are queued instead of flushed to the cache. */
  paused?: boolean;
}

export function useSqlStream(options?: UseSqlStreamOptions): {
  connectionState: ConnectionState;
  latestEvent: WsSqlEventSummary | null;
  queuedCount: number;
} {
  const latestEventRef = useRef<WsSqlEventSummary | null>(null);
  const paused = options?.paused ?? false;

  // Sync pause state to module level.
  useEffect(() => {
    setPaused(paused);
    return () => {
      // On unmount, ensure we're not stuck in paused state.
      if (paused) setPaused(false);
    };
  }, [paused]);

  // Connect on mount (idempotent if already connected).
  useEffect(() => {
    connect();

    const unsubEvent = onSqlEvent((event) => {
      latestEventRef.current = event;

      if (isPaused) {
        enqueueEvent(event);
      } else {
        // Direct flush: prepend to cache.
        queryClient.setQueryData(
          ["sql-events", undefined],
          (old: PaginatedResponse<SqlEvent> | undefined) => {
            const incoming = toSqlEvent(event);
            if (!old) {
              return { items: [incoming] };
            }
            if (old.items.some((item) => item.id === incoming.id)) {
              return old;
            }
            return { ...old, items: [incoming, ...old.items] };
          },
        );
      }
    });

    return () => {
      unsubEvent();
      // Do NOT disconnect — the connection is global/shared.
    };
  }, []);

  // Subscribe to queue count changes for re-render.
  const queuedCount = useSyncExternalStore(
    useCallback(
      (onStoreChange: () => void) => {
        queueListeners.add(onStoreChange);
        return () => {
          queueListeners.delete(onStoreChange);
        };
      },
      [],
    ),
    getQueuedCount,
  );

  const connectionState = useSyncExternalStore(
    onStateChange,
    getConnectionState,
  );

  return { connectionState, latestEvent: latestEventRef.current, queuedCount };
}
