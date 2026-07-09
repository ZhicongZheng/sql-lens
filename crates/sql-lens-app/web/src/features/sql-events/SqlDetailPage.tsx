import { useNavigate, useParams } from "react-router-dom";
import { ArrowLeftIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useSqlEvent } from "./hooks/useSqlEvent";
import { SqlSummary } from "./components/SqlSummary";
import { SqlMonacoViewer } from "./components/SqlMonacoViewer";
import { SqlParameterTable } from "./components/SqlParameterTable";
import { SqlError } from "./components/SqlError";
import { SqlConnectionInfo } from "./components/SqlConnectionInfo";
import { SqlReplaySection } from "./components/SqlReplaySection";

export function SqlDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const { data: event, isLoading, error } = useSqlEvent(id || "");

  const handleBack = () => {
    navigate("/sql-events");
  };

  if (!id) {
    return (
      <div className="p-6">
        <p className="text-destructive">Invalid event ID</p>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="p-6 space-y-4">
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={handleBack}>
            <ArrowLeftIcon className="size-4 mr-1" />
            Back
          </Button>
        </div>
        <Skeleton className="h-8 w-48" />
        <Skeleton className="h-64 w-full" />
        <Skeleton className="h-48 w-full" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-6 space-y-4">
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={handleBack}>
            <ArrowLeftIcon className="size-4 mr-1" />
            Back
          </Button>
        </div>
        <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-4">
          <p className="text-sm text-destructive">
            Failed to load SQL event: {error.message}
          </p>
        </div>
      </div>
    );
  }

  if (!event) {
    return (
      <div className="p-6 space-y-4">
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={handleBack}>
            <ArrowLeftIcon className="size-4 mr-1" />
            Back
          </Button>
        </div>
        <div className="flex h-32 items-center justify-center rounded-lg border border-dashed">
          <p className="text-sm text-muted-foreground">SQL event not found</p>
        </div>
      </div>
    );
  }

  const showExpanded = event.expanded_sql && event.expanded_sql !== event.original_sql;

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={handleBack}>
            <ArrowLeftIcon className="size-4 mr-1" />
            Back to SQL List
          </Button>
        </div>
        <div className="text-sm text-muted-foreground font-mono">
          ID: {event.id.slice(0, 16)}...
        </div>
      </div>

      <div>
        <h1 className="text-2xl font-semibold tracking-tight">SQL Event Detail</h1>
        <p className="text-sm text-muted-foreground">
          {event.protocol} query on {event.database}
        </p>
      </div>

      <SqlSummary event={event} />

      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold">Original SQL</h3>
        </div>
        <SqlMonacoViewer sql={event.original_sql} />
      </div>

      {showExpanded && (
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-semibold">Expanded SQL</h3>
          </div>
          <SqlMonacoViewer sql={event.expanded_sql} />
        </div>
      )}

      <div className="space-y-2">
        <h3 className="text-sm font-semibold">Parameters</h3>
        <SqlParameterTable parameters={event.parameters} />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <SqlConnectionInfo event={event} />
        <SqlError event={event} />
      </div>

      <SqlReplaySection event={event} />
    </div>
  );
}
