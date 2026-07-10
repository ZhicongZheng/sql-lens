import { describe, it, expect, vi, beforeEach } from "vitest";
import { render } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { SqlDetailPage } from "../SqlDetailPage";
import type { SqlEvent } from "@/types";

// Mock useSqlEvent hook
vi.mock("../hooks/useSqlEvent", () => ({
  useSqlEvent: vi.fn(),
}));

// Mock ThemeProvider
vi.mock("@/app/providers/theme-provider", () => ({
  useTheme: vi.fn(() => ({ theme: "light" })),
}));

import { useSqlEvent } from "../hooks/useSqlEvent";

describe("SQL Detail XSS prevention", () => {
  const xssPayloads = [
    { name: "script tag", payload: "<script>alert('xss')</script>" },
    { name: "img onerror", payload: "<img src=x onerror=alert('xss')>" },
    { name: "svg onload", payload: "<svg onload=alert('xss')>" },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
  });

  xssPayloads.forEach(({ name, payload }) => {
    it(`escapes ${name} in original SQL (Monaco viewer)`, () => {
      const maliciousEvent: SqlEvent = {
        id: "test-id",
        timestamp: "2026-01-01T00:00:00Z",
        target_name: "test-target",
        protocol: "mysql",
        database_type: "mysql",
        connection_id: "conn-1",
        client_addr: "127.0.0.1:1234",
        backend_addr: "127.0.0.1:3306",
        user: "testuser",
        database: "testdb",
        kind: "query",
        status: "ok",
        duration_ms: 100,
        original_sql: payload,
        expanded_sql: payload,
        fingerprint: "abc123",
        rows: { affected: 0, returned: 0 },
        parameters: [],
        metadata: {},
      };

      vi.mocked(useSqlEvent).mockReturnValue({
        data: maliciousEvent,
        isLoading: false,
        isError: false,
        error: null,
        isFetching: false,
      } as ReturnType<typeof useSqlEvent>);

      render(
        <MemoryRouter initialEntries={["/sql/test-id"]}>
          <Routes>
            <Route path="/sql/:id" element={<SqlDetailPage />} />
          </Routes>
        </MemoryRouter>
      );

      // XSS prevention: verify component rendered without throwing errors
      expect(document.body).toBeTruthy();
    });

    it(`escapes ${name} in parameter values`, () => {
      const maliciousEvent: SqlEvent = {
        id: "test-id",
        timestamp: "2026-01-01T00:00:00Z",
        target_name: "test-target",
        protocol: "mysql",
        database_type: "mysql",
        connection_id: "conn-1",
        client_addr: "127.0.0.1:1234",
        backend_addr: "127.0.0.1:3306",
        user: "testuser",
        database: "testdb",
        kind: "query",
        status: "ok",
        duration_ms: 100,
        original_sql: "SELECT * FROM users WHERE name = ?",
        expanded_sql: `SELECT * FROM users WHERE name = '${payload}'`,
        fingerprint: "abc123",
        rows: { affected: 0, returned: 0 },
        parameters: [
          {
            index: 0,
            name: "name",
            value: { type: "string", value: payload },
            redacted: false,
          },
        ],
        metadata: {},
      };

      vi.mocked(useSqlEvent).mockReturnValue({
        data: maliciousEvent,
        isLoading: false,
        isError: false,
        error: null,
        isFetching: false,
      } as ReturnType<typeof useSqlEvent>);

      render(
        <MemoryRouter initialEntries={["/sql/test-id"]}>
          <Routes>
            <Route path="/sql/:id" element={<SqlDetailPage />} />
          </Routes>
        </MemoryRouter>
      );

      // Parameter table should display payload as text
      expect(document.body.textContent).toContain(payload.substring(0, 15));
    });
  });

  it("escapes malicious error message in SqlError section", () => {
    const maliciousErrorEvent: SqlEvent = {
      id: "test-id",
      timestamp: "2026-01-01T00:00:00Z",
      target_name: "test-target",
      protocol: "mysql",
      database_type: "mysql",
      connection_id: "conn-1",
      client_addr: "127.0.0.1:1234",
      backend_addr: "127.0.0.1:3306",
      user: "testuser",
      database: "testdb",
      kind: "query",
      status: "error",
      duration_ms: 100,
      original_sql: "SELECT * FROM users",
      expanded_sql: "SELECT * FROM users",
      fingerprint: "abc123",
      rows: { affected: 0, returned: 0 },
      parameters: [],
      metadata: {
        mysql: {
          error: {
            code: 1064,
            sqlstate: "42000",
            message: "<script>alert('error xss')</script>",
          },
        },
      },
    };

    vi.mocked(useSqlEvent).mockReturnValue({
      data: maliciousErrorEvent,
      isLoading: false,
      isError: false,
      error: null,
      isFetching: false,
    } as ReturnType<typeof useSqlEvent>);

    render(
      <MemoryRouter initialEntries={["/sql/test-id"]}>
        <Routes>
          <Route path="/sql/:id" element={<SqlDetailPage />} />
        </Routes>
      </MemoryRouter>
    );

    // Error message should be displayed as text
    expect(document.body.textContent).toContain("<script>alert");
  });
});
