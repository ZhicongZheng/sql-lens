import { useQuery } from "@tanstack/react-query";
import { getConnections, getConnection } from "@/lib/api/client";

export function useConnections() {
  return useQuery({
    queryKey: ["connections"],
    queryFn: getConnections,
  });
}

export function useConnection(id: string) {
  return useQuery({
    queryKey: ["connections", id],
    queryFn: () => getConnection(id),
    enabled: !!id,
  });
}
