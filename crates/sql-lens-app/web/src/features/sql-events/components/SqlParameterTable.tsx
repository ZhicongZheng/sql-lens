import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { SqlParameter } from "@/types";

interface SqlParameterTableProps {
  parameters: SqlParameter[];
}

function formatParameterValue(value: unknown): string {
  if (value === null) return "NULL";
  if (typeof value === "object") return JSON.stringify(value);
  return String(value);
}

export function SqlParameterTable({ parameters }: SqlParameterTableProps) {
  if (parameters.length === 0) {
    return (
      <div className="rounded-lg border p-4 text-sm text-muted-foreground">
        No parameters
      </div>
    );
  }

  return (
    <div className="rounded-lg border overflow-x-auto">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="w-16">Index</TableHead>
            <TableHead className="w-32">Name</TableHead>
            <TableHead className="w-24">Type</TableHead>
            <TableHead>Value</TableHead>
            <TableHead className="w-24">Redacted</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {parameters.map((param, idx) => (
            <TableRow key={idx}>
              <TableCell className="font-mono text-xs">{param.index}</TableCell>
              <TableCell className="font-mono text-xs">{param.name || "—"}</TableCell>
              <TableCell>
                <Badge variant="outline" className="text-xs">
                  {param.value.type}
                </Badge>
              </TableCell>
              <TableCell className="font-mono text-xs max-w-md truncate">
                {param.redacted ? (
                  <span className="text-muted-foreground">REDACTED</span>
                ) : (
                  formatParameterValue(param.value.value)
                )}
              </TableCell>
              <TableCell>
                {param.redacted ? (
                  <Badge variant="secondary" className="text-xs">Yes</Badge>
                ) : (
                  <Badge variant="outline" className="text-xs">No</Badge>
                )}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}
