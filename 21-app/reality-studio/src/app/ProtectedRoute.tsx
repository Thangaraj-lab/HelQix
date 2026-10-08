/** Auth gate: unauthenticated users are redirected to /login. */
import { Navigate, Outlet, useLocation } from "react-router-dom";
import { useAuth } from "../state/auth";
import { LoadingBlock } from "../components/common";

export function ProtectedRoute() {
  const { status } = useAuth();
  const location = useLocation();
  if (status === "checking") return <LoadingBlock rows={6} />;
  if (status !== "signedIn") {
    return <Navigate to="/login" replace state={{ from: location.pathname }} />;
  }
  return <Outlet />;
}
