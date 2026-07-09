import {
  Table,
  TableBody,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Skeleton } from "@/components/ui/skeleton";
import type { SqlConnection } from "@/types";
import { ConnectionRow } from "./ConnectionRow";

interface ConnectionTableProps {
  connections: SqlConnection[];
  isLoading: boolean;
  onRowClick: (id: string) => void;
  emptyMessage?: string;
}

export function ConnectionTable({
  connections,
  isLoading,
  onRowClick,
  emptyMessage = "No connections found",
}: ConnectionTableProps) {
  if (isLoading) {
    return (
      <div className="space-y-2">
        {Array.from({ length: 5 }).map((_, i) => (
          <Skeleton key={i} className="h-12 w-full" />
        ))}
      </div>
    );
  }

  if (connections.length === 0) {
    return (
      <div className="flex h-32 items-center justify-center rounded-lg border border-dashed">
        <p className="text-sm text-muted-foreground">{emptyMessage}</p>
      </div>
    );
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>ID</TableHead>
          <TableHead>Protocol</TableHead>
          <TableHead>Client</TableHead>
          <TableHead>Backend</TableHead>
          <TableHead>User</TableHead>
          <TableHead>Database</TableHead>
          <TableHead>State</TableHead>
          <TableHead>Connected</TableHead>
          <TableHead>Last Activity</TableHead>
          <TableHead className="text-right">Queries</TableHead>
          <TableHead className="text-right">Bytes In/Out</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {connections.map((conn) => (
          <ConnectionRow
            key={conn.id}
            connection={conn}
            onClick={onRowClick}
          />
        ))}
      </TableBody>
    </Table>
  );
}
