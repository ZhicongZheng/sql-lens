import { wsUrl } from "@/lib/websocket/url";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type ConnectionState =
  | "closed"
  | "connecting"
  | "open"
  | "reconnecting";

/** Summary payload from the WebSocket `sql_event.created` message. */
export interface WsSqlEventSummary {
  id: string;
  timestamp: string;
  target_name: string;
  protocol: string;
  status: string;
  duration_ms: number;
  sql_preview: string;
}

// ---------------------------------------------------------------------------
// Module-level state (singleton)
// ---------------------------------------------------------------------------

let ws: WebSocket | null = null;
let state: ConnectionState = "closed";
let backoff = 1000; // ms, doubles on each failure, caps at 30s
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let intentionalClose = false;

type StateListener = (state: ConnectionState) => void;
type EventListener = (event: WsSqlEventSummary) => void;

const stateListeners = new Set<StateListener>();
const eventListeners = new Set<EventListener>();

const MAX_BACKOFF = 30_000;
const SUBSCRIBE_MESSAGE = JSON.stringify({ type: "subscribe", version: 1 });

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

function setState(next: ConnectionState) {
  if (state === next) return;
  state = next;
  for (const listener of stateListeners) {
    try {
      listener(state);
    } catch {
      // Listener errors are swallowed — don't break other listeners.
    }
  }
}

function scheduleReconnect() {
  if (intentionalClose) return;
  setState("reconnecting");
  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    connect();
  }, backoff);
  backoff = Math.min(backoff * 2, MAX_BACKOFF);
}

function handleMessage(data: string) {
  let parsed: { type?: string; version?: number; payload?: unknown };
  try {
    parsed = JSON.parse(data);
  } catch {
    return; // Ignore malformed messages.
  }

  if (parsed.type === "sql_event.created" && parsed.payload) {
    const payload = parsed.payload as WsSqlEventSummary;
    for (const listener of eventListeners) {
      try {
        listener(payload);
      } catch {
        // Listener errors are swallowed.
      }
    }
  } else if (parsed.type === "subscription.error") {
    console.warn(
      "[ws/sql] subscription error:",
      (parsed.payload as { message?: string })?.message ?? parsed,
    );
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/** Open the WebSocket connection if not already open. */
export function connect() {
  if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) {
    return;
  }

  intentionalClose = false;
  setState("connecting");

  const socket = new WebSocket(wsUrl("/ws/sql"));
  ws = socket;

  socket.onopen = () => {
    setState("open");
    backoff = 1000; // Reset backoff on success.
    try {
      socket.send(SUBSCRIBE_MESSAGE);
    } catch {
      // Send failed — will reconnect on close.
    }
  };

  socket.onmessage = (ev) => {
    if (typeof ev.data === "string") {
      handleMessage(ev.data);
    }
  };

  socket.onclose = () => {
    ws = null;
    if (!intentionalClose) {
      scheduleReconnect();
    } else {
      setState("closed");
    }
  };

  socket.onerror = () => {
    // The browser will fire `onclose` after `onerror`, which handles reconnect.
    // Nothing to do here.
  };
}

/** Intentionally close the connection. No auto-reconnect. */
export function disconnect() {
  intentionalClose = true;
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
  if (ws) {
    ws.close();
    ws = null;
  }
  setState("closed");
}

/** Subscribe to connection state changes. Returns an unsubscribe function. */
export function onStateChange(listener: StateListener): () => void {
  stateListeners.add(listener);
  return () => stateListeners.delete(listener);
}

/** Subscribe to incoming SQL events. Returns an unsubscribe function. */
export function onSqlEvent(listener: EventListener): () => void {
  eventListeners.add(listener);
  return () => eventListeners.delete(listener);
}

/** Get the current connection state. */
export function getConnectionState(): ConnectionState {
  return state;
}
