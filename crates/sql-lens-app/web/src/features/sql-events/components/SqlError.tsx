import { AlertTriangleIcon } from "lucide-react";
import type { SqlEvent } from "@/types";

interface SqlErrorProps {
  event: SqlEvent;
}

export function SqlError({ event }: SqlErrorProps) {
  if (event.status !== "error") {
    return null;
  }

  // Extract error info from metadata if available
  const errorMeta = (event.metadata as Record<string, Record<string, unknown>>)?.mysql?.error ||
                    (event.metadata as Record<string, Record<string, unknown>>)?.error;
  const errorCode = String((errorMeta as Record<string, unknown>)?.code || "Unknown");
  const sqlState = String((errorMeta as Record<string, unknown>)?.sqlstate || "");
  const message = String((errorMeta as Record<string, unknown>)?.message || "Query execution failed");

  return (
    <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-4">
      <div className="flex items-start gap-2">
        <AlertTriangleIcon className="size-4 mt-0.5 shrink-0 text-destructive" />
        <div className="space-y-1">
          <div className="font-semibold text-sm text-destructive">Query Error</div>
          <div className="font-mono text-xs text-muted-foreground">
            Code: {errorCode}
            {sqlState && ` | SQLSTATE: ${sqlState}`}
          </div>
          <div className="text-sm">{message}</div>
        </div>
      </div>
    </div>
  );
}
