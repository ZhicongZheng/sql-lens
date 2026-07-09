import { AlertTriangleIcon } from "lucide-react";

import { useStatistics } from "@/lib/api/hooks";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";

// ---------------------------------------------------------------------------
// Stat card — displays a single metric with label and optional description.
// ---------------------------------------------------------------------------

interface StatCardProps {
  title: string;
  value: string | number;
  description?: string;
  /** Tailwind class for the value text, e.g. "text-status-slow". */
  valueClassName?: string;
}

function StatCard({ title, value, description, valueClassName }: StatCardProps) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardDescription>{title}</CardDescription>
      </CardHeader>
      <CardContent>
        <CardTitle className={`text-2xl tabular-nums ${valueClassName ?? ""}`}>
          {value}
        </CardTitle>
        {description ? (
          <p className="mt-1 text-xs text-muted-foreground">{description}</p>
        ) : null}
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// Skeleton cards — shown while statistics are loading.
// ---------------------------------------------------------------------------

function StatCardSkeleton() {
  return (
    <Card>
      <CardHeader className="pb-2">
        <Skeleton className="h-4 w-20" />
      </CardHeader>
      <CardContent>
        <Skeleton className="h-8 w-24" />
        <Skeleton className="mt-1.5 h-3 w-16" />
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function fmtMs(ms: number): string {
  return ms.toFixed(1);
}

function fmtPct(rate: number): string {
  return `${(rate * 100).toFixed(2)}%`;
}

// ---------------------------------------------------------------------------
// Dashboard page
// ---------------------------------------------------------------------------

export function DashboardRoute() {
  const { data, isLoading, isError, error } = useStatistics();

  // --- Loading state ---
  if (isLoading) {
    return (
      <div className="space-y-4">
        <h1 className="text-lg font-semibold tracking-tight">Dashboard</h1>
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {Array.from({ length: 5 }).map((_, i) => (
            <StatCardSkeleton key={i} />
          ))}
        </div>
      </div>
    );
  }

  // --- Error state ---
  if (isError) {
    return (
      <div className="space-y-4">
        <h1 className="text-lg font-semibold tracking-tight">Dashboard</h1>
        <Card>
          <CardContent className="flex items-center gap-3 py-8">
            <AlertTriangleIcon className="size-5 shrink-0 text-status-error" />
            <div>
              <p className="font-medium text-status-error">
                Failed to load statistics
              </p>
              <p className="text-sm text-muted-foreground">
                {error instanceof Error ? error.message : "Unknown error"}
              </p>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  // --- Data not yet available (defensive guard) ---
  if (!data) {
    return null;
  }

  // --- Empty state (cold start — no traffic yet) ---
  const isEmpty = data.qps === 0 && data.active_connections === 0;

  return (
    <div className="space-y-4">
      <h1 className="text-lg font-semibold tracking-tight">Dashboard</h1>

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <StatCard
          title="QPS"
          value={isEmpty ? "—" : data.qps.toFixed(1)}
          description={isEmpty ? "No data yet" : "Queries per second"}
        />

        <StatCard
          title="Latency"
          value={
            isEmpty
              ? "—"
              : `${fmtMs(data.latency_ms.p50)} / ${fmtMs(data.latency_ms.p95)} / ${fmtMs(data.latency_ms.p99)} ms`
          }
          description="p50 / p95 / p99"
        />

        <StatCard
          title="Active Connections"
          value={isEmpty ? "—" : data.active_connections}
        />

        <StatCard
          title="Slow SQL"
          value={isEmpty ? "—" : data.slow_count}
          valueClassName={!isEmpty && data.slow_count > 0 ? "text-status-slow" : undefined}
        />

        <StatCard
          title="Error Rate"
          value={isEmpty ? "—" : fmtPct(data.error_rate)}
          valueClassName={
            !isEmpty && data.error_rate > 0 ? "text-status-error" : undefined
          }
        />
      </div>
    </div>
  );
}
