import { useQuery } from "@tanstack/react-query";
import { getSqlEvents, getSqlEvent } from "@/lib/api/client";
import type { SqlEventQueryParams } from "@/types";

export function useSqlEvents(params?: SqlEventQueryParams) {
  return useQuery({
    queryKey: ["sql-events", params],
    queryFn: () => getSqlEvents(params),
  });
}

export function useSqlEvent(id: string) {
  return useQuery({
    queryKey: ["sql-events", id],
    queryFn: () => getSqlEvent(id),
    enabled: !!id,
  });
}
