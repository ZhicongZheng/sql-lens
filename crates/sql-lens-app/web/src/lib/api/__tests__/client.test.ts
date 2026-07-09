import { describe, it, expect, vi, beforeEach } from "vitest";
import { getSqlEvent } from "@/lib/api/client";
import { ApiClientError, isApiClientError } from "@/lib/api/errors";

// Mock the config module so apiBaseUrl is predictable
vi.mock("@/lib/api/config", () => ({
  apiBaseUrl: "http://localhost:5173",
}));

const mockFetch = vi.fn();

beforeEach(() => {
  vi.stubGlobal("fetch", mockFetch);
  mockFetch.mockReset();
});

describe("getSqlEvent", () => {
  it("returns typed event on 200", async () => {
    const event = {
      id: "evt_test123",
      timestamp: "2026-07-03T12:00:00Z",
      target_name: "mysql-local",
      protocol: "mysql",
      database_type: "mysql",
      connection_id: "conn_test",
      client_addr: "127.0.0.1:51000",
      backend_addr: "127.0.0.1:3306",
      user: "app",
      database: "app",
      kind: "statement_execute",
      status: "ok",
      duration_ms: 3.4,
      original_sql: "SELECT * FROM users WHERE id = ?",
      expanded_sql: "SELECT * FROM users WHERE id = 42",
      fingerprint: "select * from users where id = ?",
      rows: { affected: 0, returned: 1 },
      metadata: { mysql: { command: "COM_STMT_EXECUTE", statement_id: 12 } },
    };

    mockFetch.mockResolvedValueOnce(
      new Response(JSON.stringify(event), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    const result = await getSqlEvent("evt_test123");

    expect(result.id).toBe("evt_test123");
    expect(result.status).toBe("ok");
    expect(result.duration_ms).toBe(3.4);
    expect(result.rows.returned).toBe(1);
    expect(mockFetch).toHaveBeenCalledOnce();

    const calledUrl = mockFetch.mock.calls[0][0] as string;
    expect(calledUrl).toContain("/api/v1/sql-events/evt_test123");
  });

  it("throws ApiClientError with NOT_FOUND on 404", async () => {
    const errorBody = {
      error: {
        code: "NOT_FOUND",
        message: "Event not found",
        request_id: "req_test456",
      },
    };

    mockFetch.mockResolvedValueOnce(
      new Response(JSON.stringify(errorBody), {
        status: 404,
        headers: { "Content-Type": "application/json" },
      }),
    );

    try {
      await getSqlEvent("evt_missing");
      expect.fail("should have thrown");
    } catch (err) {
      expect(isApiClientError(err)).toBe(true);
      expect((err as ApiClientError).code).toBe("NOT_FOUND");
      expect((err as ApiClientError).status).toBe(404);
      expect((err as ApiClientError).requestId).toBe("req_test456");
      expect((err as ApiClientError).message).toBe("Event not found");
    }
  });
});
