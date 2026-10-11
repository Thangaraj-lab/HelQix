/** Protected routing + login behavior tests. */
import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { AuthProvider } from "../state/auth";
import { AppStateProvider } from "../state/AppState";
import { ProtectedRoute } from "./ProtectedRoute";
import { LoginPage } from "../pages/LoginPage";
import { installFetchMock, type FetchMock } from "../test/fetch-mock";

function renderAuth() {
  const router = createMemoryRouter(
    [
      { path: "/login", element: <LoginPage /> },
      { element: <ProtectedRoute />, children: [{ path: "/workspace", element: <div>WORKSPACE VISIBLE</div> }] },
    ],
    { initialEntries: ["/workspace"] },
  );
  return render(
    <AppStateProvider>
      <AuthProvider>
        <RouterProvider router={router} />
      </AuthProvider>
    </AppStateProvider>,
  );
}

describe("authentication & protected routes", () => {
  let mock: FetchMock;
  beforeEach(() => {
    sessionStorage.clear();
    vi.resetModules();
    mock = installFetchMock();
  });

  it("redirects unauthenticated users to the login page", async () => {
    renderAuth();
    expect(await screen.findByText("Welcome back", {}, { timeout: 3000 })).toBeInTheDocument();
    expect(screen.queryByText("WORKSPACE VISIBLE")).not.toBeInTheDocument();
  });

  it("signs in through the backend auth endpoint and reaches the workspace", async () => {
    mock.respond("/auth/login", () => ({
      status: 200,
      body: { token: "t-1", user: { email: "eng@helqix.io", name: "Engineer" } },
    }));
    renderAuth();
    await screen.findByText("Welcome back");
    await userEvent.type(screen.getByLabelText(/^Email/i), "eng@helqix.io");
    await userEvent.type(screen.getByLabelText(/^Password/i), "hunter2pass");
    await userEvent.click(screen.getByRole("button", { name: /SIGN IN/i }));
    await waitFor(() => expect(screen.getByText("WORKSPACE VISIBLE")).toBeInTheDocument(), { timeout: 3000 });
    expect(mock.calls.some((c) => c.url.includes("/api/auth/login"))).toBe(true);
  });

  it("shows an honest error when the auth service is not implemented", async () => {
    // default mock returns 404 without tolerateUnavailable → NOT_FOUND surfaced as error banner
    mock.respond("/auth/login", () => ({ status: 503, body: null }));
    renderAuth();
    await screen.findByText("Welcome back");
    await userEvent.type(screen.getByLabelText(/^Email/i), "a@b.co");
    await userEvent.type(screen.getByLabelText(/^Password/i), "whatever1");
    await userEvent.click(screen.getByRole("button", { name: /SIGN IN/i }));
    expect(await screen.findByRole("alert", {}, { timeout: 3000 })).toBeInTheDocument();
  });
});
