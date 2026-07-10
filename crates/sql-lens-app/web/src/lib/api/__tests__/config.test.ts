import { describe, expect, it } from "vitest";

import { resolveApiBaseUrl } from "../config";

describe("resolveApiBaseUrl", () => {
  it("uses the browser origin when no explicit API base is configured", () => {
    expect(resolveApiBaseUrl(undefined, "http://127.0.0.1:7010")).toBe(
      "http://127.0.0.1:7010",
    );
  });

  it("prefers an explicit API base and removes its trailing slash", () => {
    expect(
      resolveApiBaseUrl("https://api.example.test/", "http://127.0.0.1:7010"),
    ).toBe("https://api.example.test");
  });
});
