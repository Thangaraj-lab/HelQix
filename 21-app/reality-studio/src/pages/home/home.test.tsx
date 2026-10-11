/** Home screen — entry point behavior: empty state, real projects, new-project action. */
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { AuthProvider } from "../../state/auth";
import { AppStateProvider } from "../../state/AppState";
import { ProtectedRoute } from "../../app/ProtectedRoute";
import { AppShell } from "../../app/AppShell";
import { LoginPage } from "../LoginPage";
import { HomePage } from "./HomePage";
import { installFetchMock, type FetchMock } from "../../test/fetch-mock";
import type { ProjectSummary } from "../../api/types";

function project(overrides: Partial<ProjectSummary> = {}): ProjectSummary {
  return {
    id: "p-1",
    name: "Thermal Loop Rig",
    description: "",
    domain: "aerospace",
    status: "active",
    lastActivity: { nanos: "1700000000000000000" },
    ...overrides,
  };
}

function renderHome() {
  const router = createMemoryRouter(
    [
      { path: "/login", element: <LoginPage /> },
      { element: <ProtectedRoute />, children: [{ element: <AppShell />, children: [{ path: "/home", element: <HomePage /> }] }] },
      { path: "/projects", element: <div>PROJECTS PAGE</div> },
      { path: "/settings", element: <div>SETTINGS PAGE</div> },
    ],
    { initialEntries: ["/home"] },
  );
  return render(
    <AppStateProvider>
      <AuthProvider>
        <RouterProvider router={router} />
      </AuthProvider>
    </AppStateProvider>,
  );
}

async function signIn(mock: FetchMock) {
  mock.respond("/auth/login", () => ({
    status: 200,
    body: { token: "t-1", user: { email: "eng@helqix.io", name: "Engineer" } },
  }));
  await userEvent.click(screen.getByRole("button", { name: /SIGN IN/i }));
  // Wait until the Home screen has finished rendering and settled its data phase.
  await waitFor(
    () => expect(screen.getByRole("button", { name: /NEW PROJECT/i })).toBeInTheDocument(),
    { timeout: 3000 },
  );
}

describe("HELQIX Home", () => {
  let mock: FetchMock;

  beforeEach(() => {
    sessionStorage.clear();
    mock = installFetchMock();
    mock.respond("/auth/session", () => ({ status: 401, body: null }));
  });

  afterEach(() => {
    mock.restore();
  });

  it("shows the honest empty state when there are no projects", async () => {
    mock.respond("/api/projects", () => ({ status: 200, body: [] }));
    renderHome();
    await screen.findByText("Welcome back");
    await userEvent.type(screen.getByLabelText(/^Email/i), "eng@helqix.io");
    await userEvent.type(screen.getByLabelText(/^Password/i), "hunter2pass");
    await signIn(mock);

    expect(screen.getByRole("heading", { level: 1 })).toHaveAccessibleName("HELQIX");
    expect(screen.getByText(/Engineering Reality Studio/i)).toBeInTheDocument();
    expect(screen.getByText(/Build.*Execute.*Understand.*Verify/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /NEW PROJECT/i })).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("No projects yet");
    // No fabricated project rows.
    expect(screen.queryByText("Defence Drone")).not.toBeInTheDocument();
    expect(screen.queryByText("Demo Project")).not.toBeInTheDocument();
  });

  it("lists real backend projects and links to each one", async () => {
    mock.respond("/api/projects", () => ({ status: 200, body: [project()] }));
    renderHome();
    await screen.findByText("Welcome back");
    await userEvent.type(screen.getByLabelText(/^Email/i), "eng@helqix.io");
    await userEvent.type(screen.getByLabelText(/^Password/i), "hunter2pass");
    await signIn(mock);

    const row = await screen.findByRole("link", { name: /Thermal Loop Rig/i }, { timeout: 3000 });
    expect(row).toHaveAttribute("href", "/projects/p-1");
    expect(screen.queryByText("No projects yet")).not.toBeInTheDocument();
  });

  it("opens the new-project flow from the primary action", async () => {
    renderHome();
    await screen.findByText("Welcome back");
    await userEvent.type(screen.getByLabelText(/^Email/i), "eng@helqix.io");
    await userEvent.type(screen.getByLabelText(/^Password/i), "hunter2pass");
    await signIn(mock);

    await userEvent.click(screen.getByRole("button", { name: /NEW PROJECT/i }));
    expect(await screen.findByRole("dialog", { name: /Create project/i })).toBeInTheDocument();
  });
});
