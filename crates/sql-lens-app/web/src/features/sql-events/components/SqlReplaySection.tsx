import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { InfoIcon } from "lucide-react";
import type { SqlEvent } from "@/types";

interface SqlReplaySectionProps {
  event: SqlEvent;
}

export function SqlReplaySection({ event }: SqlReplaySectionProps) {
  const isMutation = event.original_sql
    .trim()
    .toUpperCase()
    .match(/^(INSERT|UPDATE|DELETE|REPLACE|CREATE|DROP|ALTER|TRUNCATE)/);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-sm">Replay</CardTitle>
        <CardDescription>
          Replay this query against the target database
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {isMutation && (
          <div className="flex items-start gap-2 rounded-md bg-amber-50 dark:bg-amber-950/30 p-3 text-sm">
            <InfoIcon className="size-4 mt-0.5 shrink-0 text-amber-600 dark:text-amber-500" />
            <div className="text-amber-800 dark:text-amber-200">
              This query appears to be a mutation. Executing it may modify data.
            </div>
          </div>
        )}

        <div className="flex gap-2">
          <Button variant="outline" disabled>
            Preview SQL
          </Button>
          <Button disabled title="Execute functionality coming soon">
            Execute
          </Button>
        </div>

        <p className="text-xs text-muted-foreground">
          Preview shows the final SQL that would be executed. Execute is disabled until
          the replay feature is implemented.
        </p>
      </CardContent>
    </Card>
  );
}
