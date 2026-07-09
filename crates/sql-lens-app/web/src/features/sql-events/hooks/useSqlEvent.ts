import { useQuery } from "@tanstack/react-query";
import { getSqlEvent } from "@/lib/api/client";

export function useSqlEvent(id: string) {
  return useQuery({
    queryKey: ["sql-event", id],
    queryFn: () => getSqlEvent(id),
    enabled: !!id,
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}
