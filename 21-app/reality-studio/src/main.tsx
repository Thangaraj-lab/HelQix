import React from "react";
import ReactDOM from "react-dom/client";
import { AppStateProvider } from "./state/AppState";
import { AuthProvider } from "./state/auth";
import { AppRoutes } from "./app/routes";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/ui.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AppStateProvider>
      <AuthProvider>
        <AppRoutes />
      </AuthProvider>
    </AppStateProvider>
  </React.StrictMode>,
);
