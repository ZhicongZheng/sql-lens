import { Badge } from "@/components/ui/badge";
import { TableCell, TableRow } from "@/components/ui/table";
import type { SqlConnection } from "@/types";

interface ConnectionRowProps {
  connection: SqlConnection;
  onClick: (id: string) => void;
}

function fmtBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function fmtTimestamp(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleString(undefined, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}

export function ConnectionRow({ connection, onClick }: ConnectionRowProps) {
  const isActive = connection.state === "active";

  return (
    <TableRow
      className="cursor-pointer hover:bg-muted/50"
      onClick={() => onClick(connection.id)}
    >
      <TableCell className="font-mono text-xs">{connection.id.slice(0, 8)}</TableCell>
      <TableCell>
        <Badge variant="outline">{connection.protocol}</Badge>
      </TableCell>
      <TableCell className="font-mono text-xs">{connection.client_addr}</TableCell>
      <TableCell className="font-mono text-xs">{connection.backend_addr}</TableCell>
      <TableCell>{connection.user}</TableCell>
      <TableCell>{connection.database}</TableCell>
      <TableCell>
        <Badge variant={isActive ? "default" : "secondary"}>
          {connection.state}
        </Badge>
      </TableCell>
      <TableCell className="text-xs text-muted-foreground">
        {fmtTimestamp(connection.connected_at)}
      </TableCell>
      <TableCell className="text-xs text-muted-foreground">
        {fmtTimestamp(connection.last_activity_at)}
      </TableCell>
      <TableCell className="text-right">{connection.query_count}</TableCell>
      <TableCell className="text-right text-xs">
        {fmtBytes(connection.bytes_in)} / {fmtBytes(connection.bytes_out)}
      </TableCell>
    </TableRow>
  );
}
