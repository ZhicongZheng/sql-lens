import { Button } from "@/components/ui/button";
import { useNavigate } from "react-router-dom";
import type { SqlEvent } from "@/types";

interface SqlConnectionInfoProps {
  event: SqlEvent;
}

export function SqlConnectionInfo({ event }: SqlConnectionInfoProps) {
  const navigate = useNavigate();

  const handleViewConnection = () => {
    navigate(`/connections/${event.connection_id}`);
  };

  return (
    <div className="rounded-lg border p-4 space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold">Connection</h3>
        <Button
          variant="outline"
          size="sm"
          onClick={handleViewConnection}
        >
          View Connection
        </Button>
      </div>

      <div className="grid grid-cols-2 gap-x-4 gap-y-2 text-sm">
        <div className="text-muted-foreground">Connection ID</div>
        <div className="font-mono text-xs">{event.connection_id}</div>

        <div className="text-muted-foreground">Client Address</div>
        <div className="font-mono text-xs">{event.client_addr}</div>

        <div className="text-muted-foreground">Backend Address</div>
        <div className="font-mono text-xs">{event.backend_addr}</div>
      </div>
    </div>
  );
}
