import { apiBaseUrl } from "@/lib/api/config";

/**
 * Derive a WebSocket URL from the REST API base URL.
 * e.g. "http://127.0.0.1:5173" + "/ws/sql" → "ws://127.0.0.1:5173/ws/sql"
 */
export function wsUrl(path: string): string {
  const url = new URL(path, apiBaseUrl);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  return url.toString();
}
