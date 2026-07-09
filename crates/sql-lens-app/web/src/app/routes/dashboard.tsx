import { useState } from "react";
import { CopyIcon, TerminalIcon } from "lucide-react";
import { toast } from "sonner";

import { PageStub } from "@/components/layout/page-stub";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Separator } from "@/components/ui/separator";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

// Issue 065 design-system baseline showcase. This is a dense, tool-like
// surface — NOT a marketing page. Real Dashboard widgets (069) replace this.

type SqlStatus = "ok" | "slow" | "error" | "unknown";

interface SampleRow {
  time: string;
  database: string;
  durationMs: number;
  status: SqlStatus;
  preview: string;
}

const SAMPLE_ROWS: SampleRow[] = [
  {
    time: "12:01:04",
    database: "prod_orders",
    durationMs: 2,
    status: "ok",
    preview: "SELECT * FROM orders WHERE id = ?",
  },
  {
    time: "12:01:05",
    database: "prod_orders",
    durationMs: 1840,
    status: "slow",
    preview: "SELECT * FROM orders JOIN items ON ...",
  },
  {
    time: "12:01:06",
    database: "analytics",
    durationMs: 12,
    status: "error",
    preview: "INSERT INTO metrics (col) VALUES (...)",
  },
  {
    time: "12:01:07",
    database: "staging",
    durationMs: 0,
    status: "unknown",
    preview: "SET time_zone = '+00:00'",
  },
];

const STATUS_LABEL: Record<SqlStatus, string> = {
  ok: "OK",
  slow: "Slow",
  error: "Error",
  unknown: "Unknown",
};

function StatusBadge({ status }: { status: SqlStatus }) {
  // Color is never the only signal — pair token with a word (component-guidelines).
  return (
    <Badge
      variant="outline"
      className={
        status === "ok"
          ? "border-status-ok/40 text-status-ok"
          : status === "slow"
            ? "border-status-slow/40 text-status-slow"
            : status === "error"
              ? "border-status-error/40 text-status-error"
              : "border-status-unknown/40 text-status-unknown"
      }
    >
      {STATUS_LABEL[status]}
    </Badge>
  );
}

export function DashboardRoute() {
  const [replayOpen, setReplayOpen] = useState(false);

  return (
    <div className="space-y-6">
      <PageStub
        title="Dashboard"
        description="QPS, latency percentiles, active connections, and recent error timeline."
      />

      <Separator />

      <Card>
        <CardHeader>
          <CardTitle>Recent SQL events</CardTitle>
          <CardDescription>
            Design-system baseline preview (Issue 065). Real widgets land in 069.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Time</TableHead>
                <TableHead>Database</TableHead>
                <TableHead>Duration</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>SQL preview</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {SAMPLE_ROWS.map((row) => (
                <TableRow key={row.time}>
                  <TableCell className="font-mono text-xs">
                    {row.time}
                  </TableCell>
                  <TableCell className="font-mono text-xs">
                    {row.database}
                  </TableCell>
                  <TableCell className="font-mono text-xs">
                    {row.durationMs}ms
                  </TableCell>
                  <TableCell>
                    <StatusBadge status={row.status} />
                  </TableCell>
                  <TableCell className="max-w-[24rem] truncate font-mono text-xs">
                    {row.preview}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Tabs defaultValue="summary">
        <TabsList>
          <TabsTrigger value="summary">Summary</TabsTrigger>
          <TabsTrigger value="latency">Latency</TabsTrigger>
          <TabsTrigger value="errors">Errors</TabsTrigger>
        </TabsList>
        <TabsContent
          value="summary"
          className="text-sm text-muted-foreground"
        >
          p50 4ms · p95 240ms · p99 1.8s across the last 5 minutes (sample).
        </TabsContent>
        <TabsContent
          value="latency"
          className="text-sm text-muted-foreground"
        >
          Latency trend placeholder — ECharts integration is a follow-up issue.
        </TabsContent>
        <TabsContent
          value="errors"
          className="text-sm text-muted-foreground"
        >
          1 error in the window (sample row above).
        </TabsContent>
      </Tabs>

      <div className="flex flex-wrap items-center gap-2">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="outline"
              size="icon"
              aria-label="Copy sample SQL to clipboard"
              onClick={() => toast.success("SQL copied (sample)")}
            >
              <CopyIcon />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Copy sample SQL</TooltipContent>
        </Tooltip>

        <Dialog open={replayOpen} onOpenChange={setReplayOpen}>
          <DialogTrigger asChild>
            <Button variant="outline">
              <TerminalIcon />
              Replay sample
            </Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Replay sample query</DialogTitle>
              <DialogDescription>
                Replay requires explicit confirmation. This is a baseline
                preview only — no query is sent.
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="outline" onClick={() => setReplayOpen(false)}>
                Cancel
              </Button>
              <Button
                onClick={() => {
                  setReplayOpen(false);
                  toast.info("Replay is not wired in the baseline.");
                }}
              >
                Confirm
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        <Button
          variant="secondary"
          onClick={() => toast.success("Baseline mounted", { richColors: true })}
        >
          Toast baseline
        </Button>
      </div>
    </div>
  );
}
