import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { installFetchMock, type FetchMock } from "../test/fetch-mock";
import { request, setAccessToken, endpoints } from "./client";
import { ApiError } from "./errors";

describe("typed API boundary", () => {
  let mock: FetchMock;
  beforeEach(() => { mock = installFetchMock(); setAccessToken(null); });
  afterEach(() => mock.restore());

  it("sends bearer token when a session exists", async () => {
    mock.respond("/projects", () => ({ status: 200, body: [] }));
    setAccessToken("tok-123");
    await request(endpoints.projects);
    const call = mock.calls[0];
    const headers = (call?.init?.headers ?? {}) as Record<string, string>;
    expect(headers.Authorization).toBe("Bearer tok-123");
  });

  it("maps missing routes to BACKEND_UNAVAILABLE when tolerated", async () => {
    await expect(request(endpoints.model("p1"), { tolerateUnavailable: true })).rejects.toMatchObject({
      category: "BACKEND_UNAVAILABLE",
    });
  });

  it("maps 401 to AUTH_ERROR", async () => {
    mock.respond("/auth/login", () => ({ status: 401, body: null }));
    await expect(request(endpoints.login, { method: "POST", body: {} })).rejects.toMatchObject({
      category: "AUTH_ERROR",
    });
  });

  it("maps network failure to NETWORK_ERROR", async () => {
    mock.respond("/projects", () => { throw new TypeError("Failed to fetch"); });
    await expect(request(endpoints.projects)).rejects.toBeInstanceOf(ApiError);
  });

  it("parses real JSON payloads without transformation", async () => {
    const projects = [{ id: "p1", name: "Alpha", description: "", domain: "aerospace", lastActivity: { nanos: "1000" }, status: "active" }];
    mock.respond("/projects", () => ({ status: 200, body: projects }));
    await expect(request(endpoints.projects)).resolves.toEqual(projects);
  });
});
