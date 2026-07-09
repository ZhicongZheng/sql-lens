import { useCallback, useEffect, useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { AlertTriangleIcon, ChevronDownIcon, PauseIcon, PlayIcon, SearchIcon, XIcon } from "lucide-react";

import { useDetailDrawer } from "@/app/providers/detail-drawer-provider";
import { useSqlEvents } from "@/lib/api/hooks";
import { useSqlStream } from "@/lib/websocket";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { SqlEvent, SqlEventQueryParams } from "@/types";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function fmtTime(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}

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
// URL filter keys — must match API param names.
// ---------------------------------------------------------------------------

const FILTER_KEYS = [
  "q",
  "target_name",
  "protocol",
  "status",
  "database",
  "user",
  "min_duration_ms",
  "max_duration_ms",
] as const;

type FilterKey = (typeof FILTER_KEYS)[number];

/** Read active filter count from URL search params. */
function countActiveFilters(params: URLSearchParams): number {
  let count = 0;
  for (const key of FILTER_KEYS) {
    if (params.get(key)) count++;
  }
  return count;
}

/** Build SqlEventQueryParams from URL search params. */
function filtersFromParams(
  params: URLSearchParams,
): SqlEventQueryParams | undefined {
  const filters: SqlEventQueryParams = {};
  const q = params.get("q");
  if (q) filters.q = q;
  const target_name = params.get("target_name");
  if (target_name) filters.target_name = target_name;
  const protocol = params.get("protocol");
  if (protocol) filters.protocol = protocol;
  const status = params.get("status");
  if (status) filters.status = status;
  const database = params.get("database");
  if (database) filters.database = database;
  const user = params.get("user");
  if (user) filters.user = user;
  const min = params.get("min_duration_ms");
  if (min) filters.min_duration_ms = Number(min);
  const max = params.get("max_duration_ms");
  if (max) filters.max_duration_ms = Number(max);
  return Object.keys(filters).length > 0 ? filters : undefined;
}

// ---------------------------------------------------------------------------
// Columns
// ---------------------------------------------------------------------------

const COLUMNS = [
  { key: "time", label: "Time", className: "w-[72px]" },
  { key: "target", label: "Target", className: "w-[100px]" },
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
// Sub-components
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
      <TableCell className="font-mono text-xs">{event.target_name || "—"}</TableCell>
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
      <TableCell className="font-mono text-xs">{event.rows.returned}</TableCell>
      <TableCell className="max-w-xs truncate font-mono text-xs">
        {event.original_sql}
      </TableCell>
    </TableRow>
  );
}

// ---------------------------------------------------------------------------
// Filter bar
// ---------------------------------------------------------------------------

function FilterBar({
  searchParams,
  setSearchParams,
}: {
  searchParams: URLSearchParams;
  setSearchParams: (
    next: URLSearchParams | ((prev: URLSearchParams) => URLSearchParams),
    opts?: { replace?: boolean },
  ) => void;
}) {
  const activeCount = countActiveFilters(searchParams);

  function setFilter(key: FilterKey, value: string) {
    setSearchParams(
      (prev) => {
        const next = new URLSearchParams(prev);
        if (value) {
          next.set(key, value);
        } else {
          next.delete(key);
        }
        // Reset cursor when filters change.
        next.delete("cursor");
        return next;
      },
      { replace: true },
    );
  }

  function clearAll() {
    setSearchParams(new URLSearchParams(), { replace: true });
  }

  return (
    <div className="space-y-2">
      <div className="flex flex-wrap items-center gap-2">
        {/* Text search */}
        <div className="relative">
          <SearchIcon className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search SQL…"
            aria-label="Search SQL"
            className="h-8 w-48 pl-8 text-sm"
            defaultValue={searchParams.get("q") ?? ""}
            onChange={(e) => setFilter("q", e.target.value)}
          />
        </div>

        {/* Target */}
        <Input
          placeholder="Target"
          aria-label="Target"
          className="h-8 w-28 text-xs"
          defaultValue={searchParams.get("target_name") ?? ""}
          onChange={(e) => setFilter("target_name", e.target.value)}
        />

        {/* Protocol */}
        <Select
          value={searchParams.get("protocol") ?? "__all__"}
          onValueChange={(v) => setFilter("protocol", v === "__all__" ? "" : v)}
        >
          <SelectTrigger className="h-8 w-[110px] text-xs" aria-label="Protocol">
            <SelectValue placeholder="Protocol" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="__all__">All protocols</SelectItem>
            <SelectItem value="mysql">mysql</SelectItem>
          </SelectContent>
        </Select>

        {/* Status */}
        <Select
          value={searchParams.get("status") ?? "__all__"}
          onValueChange={(v) => setFilter("status", v === "__all__" ? "" : v)}
        >
          <SelectTrigger className="h-8 w-[100px] text-xs" aria-label="Status">
            <SelectValue placeholder="Status" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="__all__">All status</SelectItem>
            <SelectItem value="ok">ok</SelectItem>
            <SelectItem value="slow">slow</SelectItem>
            <SelectItem value="error">error</SelectItem>
            <SelectItem value="unknown">unknown</SelectItem>
          </SelectContent>
        </Select>

        {/* Database */}
        <Input
          placeholder="Database"
          aria-label="Database"
          className="h-8 w-28 text-xs"
          defaultValue={searchParams.get("database") ?? ""}
          onChange={(e) => setFilter("database", e.target.value)}
        />

        {/* User */}
        <Input
          placeholder="User"
          aria-label="User"
          className="h-8 w-24 text-xs"
          defaultValue={searchParams.get("user") ?? ""}
          onChange={(e) => setFilter("user", e.target.value)}
        />

        {/* Duration range */}
        <Input
          type="number"
          placeholder="Min ms"
          aria-label="Minimum duration (ms)"
          className="h-8 w-20 text-xs"
          defaultValue={searchParams.get("min_duration_ms") ?? ""}
          onChange={(e) => setFilter("min_duration_ms", e.target.value)}
        />
        <span className="text-xs text-muted-foreground">–</span>
        <Input
          type="number"
          placeholder="Max ms"
          aria-label="Maximum duration (ms)"
          className="h-8 w-20 text-xs"
          defaultValue={searchParams.get("max_duration_ms") ?? ""}
          onChange={(e) => setFilter("max_duration_ms", e.target.value)}
        />

        {/* Clear button */}
        {activeCount > 0 && (
          <Button variant="ghost" size="sm" onClick={clearAll} className="h-8 gap-1 text-xs">
            <XIcon className="size-3" />
            Clear ({activeCount})
          </Button>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// SQL Events page
// ---------------------------------------------------------------------------

export function SqlEventsRoute() {
  const [searchParams, setSearchParams] = useSearchParams();
  const [allItems, setAllItems] = useState<SqlEvent[]>([]);
  const [cursor, setCursor] = useState<string | undefined>(undefined);
  const [isPaused, setIsPaused] = useState(false);

  const filters = useMemo(() => filtersFromParams(searchParams), [searchParams]);

  // Reset accumulated items when filters change.
  const filterKey = searchParams.toString();
  useEffect(() => {
    setAllItems([]);
    setCursor(undefined);
  }, [filterKey]);

  // Build query params: filters + cursor.
  const queryParams: SqlEventQueryParams | undefined = useMemo(() => {
    if (!filters && !cursor) return undefined;
    return { ...filters, cursor };
  }, [filters, cursor]);

  const { data, isLoading, isError, error, isFetching } =
    useSqlEvents(queryParams);

  const { openDrawer } = useDetailDrawer();

  // WebSocket live stream.
  const { connectionState, queuedCount } = useSqlStream({ paused: isPaused });

  // Seed allItems on first successful load.
  useEffect(() => {
    if (data?.items && allItems.length === 0 && !cursor) {
      setAllItems(data.items);
    }
  }, [data, allItems.length, cursor]);

  const handleLoadMore = useCallback(() => {
    if (data?.next_cursor) {
      setAllItems((prev) => {
        const newItems = (data?.items ?? []).filter(
          (item) => !prev.some((p) => p.id === item.id),
        );
        return [...prev, ...newItems];
      });
      setCursor(data.next_cursor);
    }
  }, [data]);

  const handleSelectEvent = useCallback(
    (id: string) => {
      openDrawer(id);
    },
    [openDrawer],
  );

  const displayItems = allItems.length > 0 ? allItems : (data?.items ?? []);
  const showLoadMore = !!data?.next_cursor && !isLoading;
  const activeFilterCount = countActiveFilters(searchParams);

  // --- Error state ---
  if (isError) {
    return (
      <div className="space-y-4">
        <h1 className="text-lg font-semibold tracking-tight">SQL Events</h1>
        <FilterBar searchParams={searchParams} setSearchParams={setSearchParams} />
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
        <div className="flex items-center gap-2">
          <h1 className="text-lg font-semibold tracking-tight">SQL Events</h1>
          {activeFilterCount > 0 && (
            <Badge variant="secondary" className="text-xs">
              {activeFilterCount} filter{activeFilterCount > 1 ? "s" : ""} active
            </Badge>
          )}
          {/* Live stream indicator */}
          <span className="flex items-center gap-1.5 text-xs">
            <span
              className={`size-1.5 rounded-full ${
                isPaused
                  ? "bg-status-slow"
                  : connectionState === "open"
                    ? "bg-status-ok"
                    : connectionState === "closed"
                      ? "bg-status-error"
                      : "bg-status-slow"
              }`}
            />
            <span
              className={
                isPaused
                  ? "text-status-slow"
                  : connectionState === "open"
                    ? "text-status-ok"
                    : connectionState === "closed"
                      ? "text-status-error"
                      : "text-status-slow"
              }
            >
              {isPaused
                ? "Paused"
                : connectionState === "open"
                  ? "Live"
                  : connectionState === "closed"
                    ? "Disconnected"
                    : "Connecting…"}
            </span>
          </span>
          {isPaused && queuedCount > 0 && (
            <Badge variant="secondary" className="text-xs">
              {queuedCount} queued
            </Badge>
          )}
          {/* Pause/resume toggle */}
          <Button
            variant="ghost"
            size="icon"
            className="size-7"
            onClick={() => setIsPaused((p) => !p)}
            aria-label={isPaused ? "Resume live updates" : "Pause live updates"}
          >
            {isPaused ? <PlayIcon className="size-3.5" /> : <PauseIcon className="size-3.5" />}
          </Button>
        </div>
        {isFetching && !isLoading && (
          <span className="text-xs text-muted-foreground">Refreshing…</span>
        )}
      </div>

      <FilterBar searchParams={searchParams} setSearchParams={setSearchParams} />

      {connectionState === "closed" && (
        <div className="flex items-center gap-2 rounded-md border border-status-error/30 bg-status-error/5 px-3 py-2 text-xs text-status-error">
          <AlertTriangleIcon className="size-3.5 shrink-0" />
          Live updates disconnected. Reconnecting…
        </div>
      )}

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
