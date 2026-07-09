import { useMutation } from "@tanstack/react-query";
import { previewReplay } from "@/lib/api/client";
import type { ReplayPreviewRequest } from "@/types";

export function usePreviewReplay() {
  return useMutation({
    mutationFn: (req: ReplayPreviewRequest) => previewReplay(req),
  });
}
