import { useQuery } from "@tanstack/react-query";
import { getConnections, getConnection } from "@/lib/api/client";
import type { SqlConnection } from "@/types";

export type ConnectionFilter = "active" | "closed";

export function useConnections(filter?: ConnectionFilter) {
  return useQuery({
    queryKey: ["connections", filter],
    queryFn: async () => {
      const response = await getConnections();
      const allConnections = response.items;

      if (!filter) {
        return allConnections;
      }

      return allConnections.filter((conn: SqlConnection) => conn.state === filter);
    },
  });
}

export function useConnection(id: string) {
  return useQuery({
    queryKey: ["connections", id],
    queryFn: () => getConnection(id),
    enabled: !!id,
  });
}
