import { useState } from "react";
import { usePreviewReplay } from "@/lib/api/hooks/use-replay";
import type { ReplayPreviewResponse } from "@/types";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { AlertTriangle, Play } from "lucide-react";

export function ReplayPage() {
  const [mode, setMode] = useState<"event" | "sql">("event");
  const [eventId, setEventId] = useState("");
  const [sql, setSql] = useState("");
  const [preview, setPreview] = useState<ReplayPreviewResponse | null>(null);

  const { mutate: previewReplay, isPending, error } = usePreviewReplay();

  const handlePreview = () => {
    setPreview(null);
    if (mode === "event" && eventId.trim()) {
      previewReplay(
        { event_id: eventId.trim() },
        {
          onSuccess: (data) => setPreview(data),
        }
      );
    } else if (mode === "sql" && sql.trim()) {
      previewReplay(
        { sql: sql.trim() },
        {
          onSuccess: (data) => setPreview(data),
        }
      );
    }
  };

  const canPreview = (mode === "event" && eventId.trim()) || (mode === "sql" && sql.trim());

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Replay</h1>
        <p className="text-sm text-muted-foreground">
          Preview and execute captured SQL against a target with explicit confirmation for mutations.
        </p>
      </div>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        {/* Input Panel */}
        <Card>
          <CardHeader>
            <CardTitle>Preview Request</CardTitle>
            <CardDescription>Provide an event ID or raw SQL to preview</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex gap-2">
              <Button
                variant={mode === "event" ? "default" : "outline"}
                onClick={() => setMode("event")}
              >
                From Event
              </Button>
              <Button
                variant={mode === "sql" ? "default" : "outline"}
                onClick={() => setMode("sql")}
              >
                Raw SQL
              </Button>
            </div>

            {mode === "event" ? (
              <div className="space-y-2">
                <Label htmlFor="eventId">Event ID</Label>
                <Input
                  id="eventId"
                  placeholder="evt_12345"
                  value={eventId}
                  onChange={(e) => setEventId(e.target.value)}
                />
              </div>
            ) : (
              <div className="space-y-2">
                <Label htmlFor="sql">SQL Statement</Label>
                <Textarea
                  id="sql"
                  placeholder="SELECT * FROM users WHERE id = 42"
                  value={sql}
                  onChange={(e) => setSql(e.target.value)}
                  rows={4}
                  className="font-mono text-sm"
                />
              </div>
            )}

            <Button onClick={handlePreview} disabled={!canPreview || isPending} className="w-full">
              {isPending ? "Previewing..." : "Preview"}
            </Button>

            {error && (
              <Alert variant="destructive">
                <AlertTitle>Preview Failed</AlertTitle>
                <AlertDescription>{error.message}</AlertDescription>
              </Alert>
            )}
          </CardContent>
        </Card>

        {/* Preview Result */}
        <Card>
          <CardHeader>
            <CardTitle>Preview Result</CardTitle>
            <CardDescription>SQL and mutation risk classification</CardDescription>
          </CardHeader>
          <CardContent>
            {!preview ? (
              <div className="flex h-48 items-center justify-center text-sm text-muted-foreground">
                Submit a preview request to see results
              </div>
            ) : (
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <div>
                    <div className="text-sm text-muted-foreground">Source</div>
                    <div className="font-mono text-sm">{preview.source}</div>
                  </div>
                  <Badge variant={preview.is_mutation ? "destructive" : "secondary"}>
                    {preview.is_mutation ? "Mutation" : "Read-only"}
                  </Badge>
                </div>

                {preview.event_id && (
                  <div>
                    <div className="text-sm text-muted-foreground">Event ID</div>
                    <div className="font-mono text-sm">{preview.event_id}</div>
                  </div>
                )}

                <div>
                  <div className="text-sm text-muted-foreground mb-1">SQL</div>
                  <pre className="rounded-md border bg-muted/50 p-3 text-sm font-mono overflow-x-auto">
                    {preview.sql}
                  </pre>
                </div>

                {preview.warning && (
                  <Alert variant="destructive">
                    <AlertTriangle className="h-4 w-4" />
                    <AlertTitle>Warning</AlertTitle>
                    <AlertDescription>{preview.warning}</AlertDescription>
                  </Alert>
                )}

                <div className="pt-4 border-t">
                  <Button disabled className="w-full" variant="default">
                    <Play className="mr-2 h-4 w-4" />
                    Execute (Coming Soon)
                  </Button>
                  <p className="mt-2 text-xs text-center text-muted-foreground">
                    Execution endpoint not yet available
                  </p>
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
