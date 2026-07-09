import { useCallback, useState } from "react";
import { AlertTriangleIcon, ChevronDownIcon } from "lucide-react";

import { useDetailDrawer } from "@/app/providers/detail-drawer-provider";
import { useSqlEvents } from "@/lib/api/hooks";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { SqlEvent } from "@/types";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Format ISO timestamp to HH:MM:SS. */
function fmtTime(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}

/** Map status string to a tailwind token class. */
function statusClass(status: string): string {
  switch (status) {
    case "ok":
      return "text-status-ok";
    case "slow":
      return "text-status-slow";
    case "error":
      return "text-status-error";
    default:
      return "text-status-unknown";
  }
}

/** Row left-border accent for slow/error rows. */
function rowBorderClass(status: string): string {
  switch (status) {
    case "slow":
      return "border-l-2 border-status-slow";
    case "error":
      return "border-l-2 border-status-error";
    default:
      return "";
  }
}

// ---------------------------------------------------------------------------
// Columns definition
// ---------------------------------------------------------------------------

const COLUMNS = [
  { key: "time", label: "Time", className: "w-[72px]" },
  { key: "protocol", label: "Protocol", className: "w-[80px]" },
  { key: "database", label: "Database", className: "w-[100px]" },
  { key: "user", label: "User", className: "w-[80px]" },
  { key: "client", label: "Client", className: "w-[120px]" },
  { key: "duration", label: "Duration", className: "w-[80px]" },
  { key: "status", label: "Status", className: "w-[72px]" },
  { key: "rows", label: "Rows", className: "w-[56px]" },
  { key: "sql", label: "SQL preview", className: "min-w-[200px]" },
] as const;

// ---------------------------------------------------------------------------
// Skeleton rows
// ---------------------------------------------------------------------------

function LoadingRows() {
  return (
    <>
      {Array.from({ length: 5 }).map((_, rowIdx) => (
        <TableRow key={rowIdx}>
          {COLUMNS.map((col) => (
            <TableCell key={col.key}>
              <Skeleton className="h-4 w-full" />
            </TableCell>
          ))}
        </TableRow>
      ))}
    </>
  );
}

// ---------------------------------------------------------------------------
// Single event row
// ---------------------------------------------------------------------------

function EventRow({
  event,
  onSelect,
}: {
  event: SqlEvent;
  onSelect: (id: string) => void;
}) {
  return (
    <TableRow
      className={`cursor-pointer ${rowBorderClass(event.status)}`}
      onClick={() => onSelect(event.id)}
    >
      <TableCell className="font-mono text-xs">{fmtTime(event.timestamp)}</TableCell>
      <TableCell className="text-xs">{event.protocol}</TableCell>
      <TableCell className="text-xs">{event.database}</TableCell>
      <TableCell className="text-xs">{event.user}</TableCell>
      <TableCell className="font-mono text-xs">{event.client_addr}</TableCell>
      <TableCell className="font-mono text-xs">{event.duration_ms}ms</TableCell>
      <TableCell>
        <Badge variant="outline" className={`${statusClass(event.status)} text-xs`}>
          {event.status}
        </Badge>
      </TableCell>
      <TableCell className="font-mono text-xs">
        {event.rows.returned}
      </TableCell>
      <TableCell className="max-w-xs truncate font-mono text-xs">
        {event.original_sql}
      </TableCell>
    </TableRow>
  );
}

// ---------------------------------------------------------------------------
// SQL Events page
// ---------------------------------------------------------------------------

export function SqlEventsRoute() {
  const [cursor, setCursor] = useState<string | undefined>(undefined);
  const [allItems, setAllItems] = useState<SqlEvent[]>([]);
  const [hasLoadedFirst, setHasLoadedFirst] = useState(false);

  const { data, isLoading, isError, error, isFetching } = useSqlEvents(
    cursor ? { cursor } : undefined,
  );

  const { openDrawer } = useDetailDrawer();

  // Accumulate pages into allItems when data arrives.
  // We use a ref-like pattern with state to detect new pages.
  const currentNextCursor = data?.next_cursor;

  // On first successful load, seed allItems.
  if (data && !hasLoadedFirst) {
    setAllItems(data.items);
    setHasLoadedFirst(true);
  }

  // When cursor changes and new data arrives, append.
  // This is intentionally simple — a real implementation would use
  // TanStack Query's infinite queries, but that's a follow-up.
  const handleLoadMore = useCallback(() => {
    if (currentNextCursor) {
      // Append current page items before fetching next.
      if (data?.items) {
        setAllItems((prev) => {
          const newItems = data.items.filter(
            (item) => !prev.some((p) => p.id === item.id),
          );
          return [...prev, ...newItems];
        });
      }
      setCursor(currentNextCursor);
    }
  }, [currentNextCursor, data]);

  const handleSelectEvent = useCallback(
    (id: string) => {
      // Store the selected event ID for future SQL Detail content.
      // For now, just open the drawer (shows placeholder).
      void id;
      openDrawer();
    },
    [openDrawer],
  );

  // Build display list: allItems + current page items not yet in allItems.
  const displayItems = allItems.length > 0 ? allItems : (data?.items ?? []);
  const showLoadMore = !!currentNextCursor && !isLoading;

  // --- Error state ---
  if (isError) {
    return (
      <div className="space-y-4">
        <h1 className="text-lg font-semibold tracking-tight">SQL Events</h1>
        <div className="flex items-center gap-3 rounded-md border p-6">
          <AlertTriangleIcon className="size-5 shrink-0 text-status-error" />
          <div>
            <p className="font-medium text-status-error">
              Failed to load SQL events
            </p>
            <p className="text-sm text-muted-foreground">
              {error instanceof Error ? error.message : "Unknown error"}
            </p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-lg font-semibold tracking-tight">SQL Events</h1>
        {isFetching && !isLoading && (
          <span className="text-xs text-muted-foreground">Refreshing…</span>
        )}
      </div>

      <div className="overflow-x-auto rounded-md border">
        <Table>
          <TableHeader>
            <TableRow>
              {COLUMNS.map((col) => (
                <TableHead key={col.key} className={col.className}>
                  {col.label}
                </TableHead>
              ))}
            </TableRow>
          </TableHeader>
          <TableBody>
            {isLoading ? (
              <LoadingRows />
            ) : displayItems.length === 0 ? (
              <TableRow>
                <TableCell
                  colSpan={COLUMNS.length}
                  className="h-24 text-center text-sm text-muted-foreground"
                >
                  No SQL events captured yet.
                </TableCell>
              </TableRow>
            ) : (
              displayItems.map((event) => (
                <EventRow
                  key={event.id}
                  event={event}
                  onSelect={handleSelectEvent}
                />
              ))
            )}
          </TableBody>
        </Table>
      </div>

      {showLoadMore && (
        <div className="flex justify-center">
          <Button
            variant="outline"
            size="sm"
            onClick={handleLoadMore}
            disabled={isFetching}
          >
            <ChevronDownIcon className="size-4" />
            {isFetching ? "Loading…" : "Load more"}
          </Button>
        </div>
      )}
    </div>
  );
}
