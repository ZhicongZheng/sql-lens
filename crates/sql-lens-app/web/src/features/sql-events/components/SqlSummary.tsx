import { Badge } from "@/components/ui/badge";
import type { SqlEvent } from "@/types";

interface SqlSummaryProps {
  event: SqlEvent;
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

function fmtTimestamp(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleString(undefined, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}

export function SqlSummary({ event }: SqlSummaryProps) {
  return (
    <div className="rounded-lg border p-4 space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold">Summary</h3>
        <Badge variant="outline" className={statusClass(event.status)}>
          {event.status}
        </Badge>
      </div>

      <div className="grid grid-cols-2 gap-x-4 gap-y-2 text-sm">
        <div className="text-muted-foreground">Timestamp</div>
        <div className="font-mono text-xs">{fmtTimestamp(event.timestamp)}</div>

        <div className="text-muted-foreground">Protocol</div>
        <div>{event.protocol}</div>

        <div className="text-muted-foreground">Database Type</div>
        <div>{event.database_type}</div>

        <div className="text-muted-foreground">Target</div>
        <div className="font-mono text-xs">{event.target_name || "—"}</div>

        <div className="text-muted-foreground">User</div>
        <div>{event.user}</div>

        <div className="text-muted-foreground">Database</div>
        <div>{event.database}</div>

        <div className="text-muted-foreground">Client</div>
        <div className="font-mono text-xs">{event.client_addr}</div>

        <div className="text-muted-foreground">Backend</div>
        <div className="font-mono text-xs">{event.backend_addr}</div>

        <div className="text-muted-foreground">Duration</div>
        <div className="font-mono">{event.duration_ms}ms</div>

        <div className="text-muted-foreground">Fingerprint</div>
        <div className="font-mono text-xs truncate" title={event.fingerprint}>
          {event.fingerprint}
        </div>
      </div>
    </div>
  );
}
