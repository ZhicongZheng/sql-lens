import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { SqlEventsRoute } from "../sql-events";
import type { PaginatedResponse, SqlEvent } from "@/types";

// Mock useSqlEvents hook
vi.mock("@/lib/api/hooks/use-sql-events", () => ({
  useSqlEvents: vi.fn(),
}));

// Mock useSqlStream hook
vi.mock("@/lib/websocket", () => ({
  useSqlStream: vi.fn(() => ({ connectionState: "connected", queuedCount: 0 })),
}));

// Mock DetailDrawerProvider
vi.mock("@/app/providers/detail-drawer-provider", () => ({
  useDetailDrawer: vi.fn(() => ({ openDrawer: vi.fn() })),
}));

import { useSqlEvents } from "@/lib/api/hooks/use-sql-events";

describe("SQL List XSS prevention", () => {
  const xssPayloads = [
    { name: "script tag", payload: "<script>alert('xss')</script>" },
    { name: "img onerror", payload: "<img src=x onerror=alert('xss')>" },
    { name: "svg onload", payload: "<svg onload=alert('xss')>" },
    { name: "javascript URL", payload: "<a href=\"javascript:alert('xss')\">click</a>" },
    { name: "html entity", payload: "&lt;script&gt;alert('xss')&lt;/script&gt;" },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
  });

  xssPayloads.forEach(({ name, payload }) => {
    it(`escapes ${name} in SQL preview column`, () => {
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

      const mockResponse: PaginatedResponse<SqlEvent> = {
        items: [maliciousEvent],
      };

      vi.mocked(useSqlEvents).mockReturnValue({
        data: mockResponse,
        isLoading: false,
        isError: false,
        error: null,
        isFetching: false,
      } as ReturnType<typeof useSqlEvents>);

      render(
        <MemoryRouter>
          <SqlEventsRoute />
        </MemoryRouter>
      );

      // Payload should appear as text content, not as parsed HTML
      const sqlCell = screen.getByText((content: string) => content.includes(payload.substring(0, 20)));
      expect(sqlCell).toBeTruthy();

      // No script elements should be injected
      const scripts = document.querySelectorAll("script:not([type])");
      expect(scripts.length).toBe(0);
    });
  });
});
