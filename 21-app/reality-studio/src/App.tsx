import React from 'react';
import { AppStateProvider } from './engineering/stateStore';
import { StudioStoreProvider } from './stores';
import { AppShell } from './app/AppShell';
import { navigate, useLocation } from './app/router';
import { Studio } from './pages/studio/Studio';
import { LandingPage } from './pages/LandingPage';
import { OverviewPage } from './pages/home/OverviewPage';
import { NewProjectPage } from './pages/NewProjectPage';

// One product, three places (file 35 section 12; file 36 non-goals): Hero, Home and the Studio.
// Everything else is a stage inside the Studio, addressed by the URL query.
const Routes: React.FC = () => {
  const { route } = useLocation();
  switch (route.name) {
    case 'hero':
      return <LandingPage />;
    case 'home':
      return <AppShell><OverviewPage /></AppShell>;
    case 'new':
      return <NewProjectPage />;
    case 'studio':
      return <Studio key={route.projectId} projectId={route.projectId} />;
    default:
      return (
        <div className="h-screen flex flex-col items-center justify-center gap-3 bg-[var(--bg-void)] text-[var(--text-ink)]">
          <p className="text-sm">Nothing at {route.path}.</p>
          <button className="px-3 py-1.5 rounded-md bg-[#2F7BFF] text-white text-xs" onClick={() => navigate('/home')}>Go to Home</button>
        </div>
      );
  }
};

export const App: React.FC = () => (
  <StudioStoreProvider>
    <AppStateProvider>
      <Routes />
    </AppStateProvider>
  </StudioStoreProvider>
);

export default App;
