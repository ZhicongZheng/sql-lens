import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useConnections } from "@/lib/api/hooks/use-connections";
import type { ConnectionFilter } from "@/lib/api/hooks/use-connections";
import { ConnectionFilters } from "./components/ConnectionFilters";
import { ConnectionTable } from "./components/ConnectionTable";

export function ConnectionsPage() {
  const [filter, setFilter] = useState<ConnectionFilter>("active");
  const navigate = useNavigate();

  const { data: connections = [], isLoading, error } = useConnections(filter);

  const handleRowClick = (id: string) => {
    navigate(`/connections/${id}`);
  };

  const emptyMessage =
    filter === "active"
      ? "No active connections"
      : "No closed connections";

  if (error) {
    return (
      <div className="p-6">
        <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-4">
          <p className="text-sm text-destructive">
            Failed to load connections: {error.message}
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Connections</h1>
          <p className="text-sm text-muted-foreground">
            Active and closed database connections
          </p>
        </div>
        <ConnectionFilters value={filter} onChange={setFilter} />
      </div>

      <ConnectionTable
        connections={connections}
        isLoading={isLoading}
        onRowClick={handleRowClick}
        emptyMessage={emptyMessage}
      />
    </div>
  );
}
