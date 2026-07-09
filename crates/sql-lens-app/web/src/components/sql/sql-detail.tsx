import { lazy, Suspense, useState } from "react";
import {
  AlertTriangleIcon,
  CopyIcon,
  ChevronDownIcon,
  ChevronUpIcon,
  TerminalIcon,
} from "lucide-react";
import { toast } from "sonner";

import { useSqlEvent } from "@/lib/api/hooks";
import { usePreviewReplay } from "@/lib/api/hooks";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function fmtTime(iso: string): string {
  return new Date(iso).toLocaleString();
}

function statusBadgeClass(status: string): string {
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

async function copyToClipboard(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    toast.success("SQL copied");
  } catch {
    toast.error("Failed to copy");
  }
}

// ---------------------------------------------------------------------------
// Section wrapper
// ---------------------------------------------------------------------------

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-1.5">
      <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
        {title}
      </h3>
      {children}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Lazy-loaded Monaco SQL editor (code-split to avoid bloating the main bundle)
// ---------------------------------------------------------------------------

const LazySqlEditor = lazy(() =>
  import("@/components/sql/sql-editor").then((m) => ({
    default: m.SqlEditor,
  })),
);

// ---------------------------------------------------------------------------
// SQL block with Monaco editor + copy button
// ---------------------------------------------------------------------------

function SqlBlock({
  sql,
  label,
}: {
  sql: string;
  label: string;
}) {
  return (
    <div className="space-y-2">
      <div className="flex justify-end">
        <Button
          variant="ghost"
          size="sm"
          className="h-7 gap-1 text-xs"
          onClick={() => copyToClipboard(sql)}
          aria-label={`Copy ${label}`}
        >
          <CopyIcon className="size-3" />
          Copy
        </Button>
      </div>
      <Suspense
        fallback={
          <div className="flex h-20 items-center justify-center rounded-md bg-muted text-xs text-muted-foreground">
            Loading editor…
          </div>
        }
      >
        <LazySqlEditor value={sql} />
      </Suspense>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Loading skeleton
// ---------------------------------------------------------------------------

function DetailSkeleton() {
  return (
    <div className="space-y-4 p-4">
      {Array.from({ length: 6 }).map((_, i) => (
        <div key={i} className="space-y-2">
          <Skeleton className="h-3 w-16" />
          <Skeleton className="h-4 w-full" />
          <Skeleton className="h-4 w-3/4" />
        </div>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

interface SqlDetailProps {
  eventId: string;
}

export function SqlDetail({ eventId }: SqlDetailProps) {
  const { data: event, isLoading, isError, error } = useSqlEvent(eventId);
  const [showExpanded, setShowExpanded] = useState(false);
  const [showMetadata, setShowMetadata] = useState(false);
  const replayMutation = usePreviewReplay();

  // --- Loading ---
  if (isLoading) {
    return <DetailSkeleton />;
  }

  // --- Error ---
  if (isError) {
    const isNotFound =
      error instanceof Error && error.message.includes("NOT_FOUND");
    return (
      <div className="flex flex-col items-center gap-2 p-8 text-center">
        <AlertTriangleIcon className="size-6 text-status-error" />
        <p className="font-medium text-status-error">
          {isNotFound ? "Event not found" : "Failed to load event"}
        </p>
        <p className="text-sm text-muted-foreground">
          {error instanceof Error ? error.message : "Unknown error"}
        </p>
      </div>
    );
  }

  if (!event) return null;

  const hasExpandedSql =
    event.expanded_sql && event.expanded_sql !== event.original_sql;

  return (
    <div className="space-y-4 overflow-y-auto p-4">
      {/* Summary */}
      <Section title="Summary">
        <div className="flex flex-wrap items-center gap-2">
          <Badge
            variant="outline"
            className={`${statusBadgeClass(event.status)} text-xs`}
          >
            {event.status}
          </Badge>
          {event.target_name && (
            <span className="font-mono text-xs text-muted-foreground">
              {event.target_name}
            </span>
          )}
          <span className="text-xs text-muted-foreground">{event.protocol}</span>
          <span className="text-xs text-muted-foreground">·</span>
          <span className="text-xs text-muted-foreground">{event.database}</span>
          <span className="text-xs text-muted-foreground">·</span>
          <span className="text-xs text-muted-foreground">{event.user}</span>
        </div>
        <div className="mt-1 flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
          <span>
            Duration:{" "}
            <span className="font-mono font-medium text-foreground">
              {event.duration_ms}ms
            </span>
          </span>
          <span>Time: {fmtTime(event.timestamp)}</span>
          <span>
            Rows: {event.rows.returned} returned / {event.rows.affected} affected
          </span>
        </div>
      </Section>
      <Separator />

      {/* SQL */}
      <Section title="SQL">
        {hasExpandedSql && (
          <div className="mb-2">
            <Button
              variant="ghost"
              size="sm"
              className="h-7 gap-1 text-xs"
              onClick={() => setShowExpanded((v) => !v)}
            >
              {showExpanded ? (
                <>
                  <ChevronUpIcon className="size-3" /> Original
                </>
              ) : (
                <>
                  <ChevronDownIcon className="size-3" /> Expanded
                </>
              )}
            </Button>
          </div>
        )}
        <SqlBlock
          sql={
            showExpanded && hasExpandedSql
              ? event.expanded_sql
              : event.original_sql
          }
          label={showExpanded ? "expanded SQL" : "original SQL"}
        />
      </Section>
      <Separator />

      {/* Parameters */}
      <Section title="Parameters">
        <ParametersBlock metadata={event.metadata} />
      </Section>
      <Separator />

      {/* Timings + Connection */}
      <Section title="Connection">
        <div className="space-y-1 text-xs">
          <div className="flex justify-between">
            <span className="text-muted-foreground">Connection ID</span>
            <span className="font-mono">{event.connection_id}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-muted-foreground">Client</span>
            <span className="font-mono">{event.client_addr}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-muted-foreground">Backend</span>
            <span className="font-mono">{event.backend_addr}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-muted-foreground">Target</span>
            <span className="font-mono">{event.target_name}</span>
          </div>
        </div>
      </Section>
      <Separator />

      {/* Error (only when status is error) */}
      {event.status === "error" && (
        <>
          <Section title="Error">
            <div className="flex items-center gap-2 rounded-md border border-status-error/30 bg-status-error/5 p-3 text-xs text-status-error">
              <AlertTriangleIcon className="size-4 shrink-0" />
              <span>Query returned an error. Check protocol metadata for details.</span>
            </div>
          </Section>
          <Separator />
        </>
      )}

      {/* Protocol metadata */}
      <Section title="Protocol Metadata">
        <Button
          variant="ghost"
          size="sm"
          className="h-7 gap-1 text-xs"
          onClick={() => setShowMetadata((v) => !v)}
        >
          {showMetadata ? (
            <>
              <ChevronUpIcon className="size-3" /> Hide
            </>
          ) : (
            <>
              <ChevronDownIcon className="size-3" /> Show raw metadata
            </>
          )}
        </Button>
        {showMetadata && (
          <pre className="mt-2 overflow-x-auto rounded-md bg-muted p-3 text-xs font-mono whitespace-pre-wrap break-all">
            {JSON.stringify(event.metadata, null, 2)}
          </pre>
        )}
      </Section>
      <Separator />

      {/* Replay */}
      <Section title="Replay">
        <Button
          variant="outline"
          size="sm"
          className="gap-1.5"
          onClick={() => {
            replayMutation.mutate(
              { event_id: event.id },
              {
                onSuccess: () =>
                  toast.info("Replay preview is not yet wired."),
                onError: () => toast.error("Replay preview failed."),
              },
            );
          }}
        >
          <TerminalIcon className="size-3.5" />
          Replay
        </Button>
      </Section>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Parameters helper
// ---------------------------------------------------------------------------

function ParametersBlock({ metadata }: { metadata: Record<string, Record<string, unknown>> }) {
  // Try to extract parameters from protocol-specific metadata.
  // MySQL: metadata.mysql.parameters (future).
  // For now, show placeholder if no parameter data is found.
  const protocolKeys = Object.keys(metadata);
  if (protocolKeys.length === 0) {
    return (
      <p className="text-xs text-muted-foreground">No parameters available.</p>
    );
  }

  // Show a simple key-value summary of the metadata keys.
  return (
    <div className="space-y-1 text-xs">
      {protocolKeys.map((key) => (
        <div key={key} className="flex justify-between">
          <span className="text-muted-foreground">{key}</span>
          <span className="font-mono">
            {typeof metadata[key] === "string"
              ? metadata[key]
              : JSON.stringify(metadata[key])}
          </span>
        </div>
      ))}
    </div>
  );
}
