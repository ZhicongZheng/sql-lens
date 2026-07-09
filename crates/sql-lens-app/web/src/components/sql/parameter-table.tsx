import { useState } from "react";
import { ShieldCheckIcon } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { SqlParameter, SqlParameterValue } from "@/types";

// ---------------------------------------------------------------------------
// Value formatting
// ---------------------------------------------------------------------------

const TRUNCATE_LENGTH = 80;

/** Format a value for display, with truncation support. */
function useFormattedValue(
  pv: SqlParameterValue,
  expanded: boolean,
): string {
  const { type, value } = pv;

  if (type === "null" || value === null) {
    return "NULL";
  }

  let raw: string;
  switch (type) {
    case "boolean":
      raw = String(value);
      break;
    case "json":
      try {
        raw =
          typeof value === "string"
            ? JSON.stringify(JSON.parse(value), null, 2)
            : String(value);
      } catch {
        raw = String(value);
      }
      break;
    default:
      raw = String(value);
  }

  if (!expanded && raw.length > TRUNCATE_LENGTH) {
    return raw.slice(0, TRUNCATE_LENGTH) + "…";
  }
  return raw;
}

function typeLabel(type: string): string {
  // Map Rust enum variant casing to frontend label
  const map: Record<string, string> = {
    integer: "integer",
    unsigned: "unsigned",
    float: "float",
    boolean: "boolean",
    string: "string",
    date: "date",
    time: "time",
    timestamp: "timestamp",
    json: "json",
    binary_summary: "binary",
    unsupported: "unsupported",
    null: "null",
  };
  return map[type] ?? type;
}

// ---------------------------------------------------------------------------
// Parameter row
// ---------------------------------------------------------------------------

function ParameterRow({ param }: { param: SqlParameter }) {
  const [expanded, setExpanded] = useState(false);
  const formatted = useFormattedValue(param.value, expanded);
  const isLong = formatted.length > TRUNCATE_LENGTH || param.value.type === "json";

  return (
    <TableRow>
      <TableCell className="font-mono text-xs">{param.index}</TableCell>
      <TableCell className="text-xs">
        {param.name ? (
          <span className="font-mono">{param.name}</span>
        ) : (
          <span className="text-muted-foreground">—</span>
        )}
      </TableCell>
      <TableCell className="text-xs text-muted-foreground">
        {typeLabel(param.value.type)}
      </TableCell>
      <TableCell className="max-w-xs">
        {param.redacted ? (
          <div className="flex items-center gap-1.5">
            <Badge
              variant="secondary"
              className="gap-1 text-xs text-status-ok"
            >
              <ShieldCheckIcon className="size-3" />
              Redacted
            </Badge>
            <span className="text-xs text-muted-foreground">
              {String(param.value.value)}
            </span>
          </div>
        ) : param.value.type === "null" || param.value.value === null ? (
          <span className="text-xs italic text-muted-foreground">NULL</span>
        ) : (
          <div className="space-y-1">
            <pre className="overflow-x-auto whitespace-pre-wrap break-all font-mono text-xs">
              {formatted}
            </pre>
            {isLong && (
              <Button
                variant="ghost"
                size="sm"
                className="h-6 text-xs"
                onClick={() => setExpanded((v) => !v)}
              >
                {expanded ? "Show less" : "Show more"}
              </Button>
            )}
          </div>
        )}
      </TableCell>
    </TableRow>
  );
}

// ---------------------------------------------------------------------------
// ParameterTable component
// ---------------------------------------------------------------------------

interface ParameterTableProps {
  parameters: SqlParameter[];
}

export function ParameterTable({ parameters }: ParameterTableProps) {
  if (parameters.length === 0) {
    return (
      <p className="text-xs text-muted-foreground">No parameters available.</p>
    );
  }

  return (
    <div className="overflow-x-auto rounded-md border">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="w-[48px]">Index</TableHead>
            <TableHead className="w-[96px]">Name</TableHead>
            <TableHead className="w-[72px]">Type</TableHead>
            <TableHead className="min-w-[160px]">Value</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {parameters.map((param) => (
            <ParameterRow key={param.index} param={param} />
          ))}
        </TableBody>
      </Table>
    </div>
  );
}