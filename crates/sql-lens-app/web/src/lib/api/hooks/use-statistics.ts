import { useQuery } from "@tanstack/react-query";
import { getStatistics } from "@/lib/api/client";

/**
 * Statistics query with 5-second polling (interim until WebSocket integration).
 */
export function useStatistics(window?: string) {
  return useQuery({
    queryKey: ["statistics", window],
    queryFn: () => getStatistics(window),
    refetchInterval: 5_000,
  });
}
