import { useQuery } from "@tanstack/react-query";
import { getProtocols } from "@/lib/api/client";

export function useProtocols() {
  return useQuery({
    queryKey: ["protocols"],
    queryFn: getProtocols,
  });
}
