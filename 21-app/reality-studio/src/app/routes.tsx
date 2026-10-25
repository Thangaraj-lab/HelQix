/** Route map for HELQIX Reality Studio. */
import { createBrowserRouter, Navigate, RouterProvider } from "react-router-dom";
import { AppShell } from "./AppShell";
import { ProtectedRoute } from "./ProtectedRoute";
import { LandingPage } from "../pages/LandingPage";
import { LoginPage } from "../pages/LoginPage";
import { SignupPage } from "../pages/SignupPage";
import { PasswordRecoveryPage } from "../pages/PasswordRecoveryPage";
import { WorkspaceHomePage } from "../pages/home/WorkspaceHomePage";
import { HomePage } from "../pages/home/HomePage";
import { ProjectsPage } from "../pages/home/ProjectsPage";
import { ProjectOverviewPage } from "../pages/home/ProjectOverviewPage";
import { SettingsPage } from "../pages/home/SettingsPage";
import { IntentPage } from "../pages/intent/IntentPage";
import { ModelPage } from "../pages/model/ModelPage";
import { ArchitecturePage } from "../pages/architecture/ArchitecturePage";
import { RealityStudioPage } from "../pages/reality/RealityStudioPage";
import { ExecutionPage } from "../pages/execution/ExecutionPage";
import { ObservationPage } from "../pages/execution/ObservationPage";
import { TimelinePage } from "../pages/execution/TimelinePage";
import { FailurePage } from "../pages/failure/FailurePage";
import { CausalPage } from "../pages/causal/CausalPage";
import { BranchesPage } from "../pages/branch/BranchesPage";
import { ComparePage } from "../pages/compare/ComparePage";
import { SearchPage } from "../pages/search/SearchPage";
import { VerifyPage } from "../pages/verify/VerifyPage";
import { EvidencePage } from "../pages/evidence/EvidencePage";

export const router = createBrowserRouter([
  { path: "/", element: <LandingPage /> },
  { path: "/login", element: <LoginPage /> },
  { path: "/signup", element: <SignupPage /> },
  { path: "/recover", element: <PasswordRecoveryPage /> },
  {
    element: <ProtectedRoute />,
    children: [
      {
        element: <AppShell />,
        children: [
          { path: "/home", element: <HomePage /> },
          { path: "/workspace", element: <WorkspaceHomePage /> },
          { path: "/projects", element: <ProjectsPage /> },
          { path: "/projects/:projectId", element: <ProjectOverviewPage /> },
          { path: "/projects/:projectId/intent", element: <IntentPage /> },
          { path: "/projects/:projectId/model", element: <ModelPage /> },
          { path: "/projects/:projectId/architecture", element: <ArchitecturePage /> },
          { path: "/projects/:projectId/reality", element: <RealityStudioPage /> },
          { path: "/projects/:projectId/execution", element: <ExecutionPage /> },
          { path: "/projects/:projectId/observation", element: <ObservationPage /> },
          { path: "/projects/:projectId/timeline", element: <TimelinePage /> },
          { path: "/projects/:projectId/failure", element: <FailurePage /> },
          { path: "/projects/:projectId/causal", element: <CausalPage /> },
          { path: "/projects/:projectId/branches", element: <BranchesPage /> },
          { path: "/projects/:projectId/compare", element: <ComparePage /> },
          { path: "/projects/:projectId/search", element: <SearchPage /> },
          { path: "/projects/:projectId/verify", element: <VerifyPage /> },
          { path: "/projects/:projectId/evidence", element: <EvidencePage /> },
          { path: "/settings", element: <SettingsPage /> },
        ],
      },
    ],
  },
  { path: "*", element: <Navigate to="/" replace /> },
]);

export function AppRoutes() {
  return <RouterProvider router={router} />;
}
