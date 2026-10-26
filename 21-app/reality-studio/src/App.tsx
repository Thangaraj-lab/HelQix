import React from 'react';
import { useAppState, AppStateProvider } from './engineering/stateStore';
import { StudioStoreProvider } from './stores';
import { AppShell } from './app/AppShell';

// All Screens and Core Studio
import { Studio } from './pages/studio/Studio';
import { LandingPage } from './pages/LandingPage';
import { OverviewPage } from './pages/home/OverviewPage';
import { WorkspacePage } from './pages/reality/WorkspacePage';
import { ModelEditorPage } from './pages/model/ModelEditorPage';
import { SetupPage } from './pages/simulate/SetupPage';
import { RunPage } from './pages/simulate/RunPage';
import { ResultsPage } from './pages/simulate/ResultsPage';
import { FailureCausalityPage } from './pages/failure/FailureCausalityPage';
import { ComparePage } from './pages/compare/ComparePage';
import { ScenarioBuilderPage } from './pages/simulate/ScenarioBuilderPage';
import { TimelineReplayPage } from './pages/execution/TimelineReplayPage';
import { BuildOptimizationPage } from './pages/search/BuildOptimizationPage';
import { ComputeResourcesPage } from './pages/compute/ComputeResourcesPage';
import { VerifyValidationPage } from './pages/verify/VerifyValidationPage';
import { DeployExportPage } from './pages/evidence/DeployExportPage';
import { EdgeMonitoringPage } from './pages/edge/EdgeMonitoringPage';

const ScreenRouter: React.FC = () => {
  const { currentScreen } = useAppState();

  // Route: if URL query contains stage=..., prioritize Studio
  const params = new URLSearchParams(window.location.search);
  const stage = params.get('stage');

  if (stage) {
    return <Studio />;
  }

  switch (currentScreen) {
    case 'landing':
      return <LandingPage />;
    case 'home':
      return <AppShell><OverviewPage /></AppShell>;
    case 'workspace':
      return <Studio />;
    case 'model_editor':
      return <AppShell><ModelEditorPage /></AppShell>;
    case 'simulation_setup':
      return <AppShell><SetupPage /></AppShell>;
    case 'simulation_run':
      return <AppShell><RunPage /></AppShell>;
    case 'simulation_results':
      return <AppShell><ResultsPage /></AppShell>;
    case 'failure_causality':
      return <AppShell><FailureCausalityPage /></AppShell>;
    case 'compare':
      return <AppShell><ComparePage /></AppShell>;
    case 'scenario_builder':
      return <AppShell><ScenarioBuilderPage /></AppShell>;
    case 'timeline_replay':
      return <AppShell><TimelineReplayPage /></AppShell>;
    case 'build_optimization':
      return <AppShell><BuildOptimizationPage /></AppShell>;
    case 'compute_resources':
      return <AppShell><ComputeResourcesPage /></AppShell>;
    case 'verify_validation':
      return <AppShell><VerifyValidationPage /></AppShell>;
    case 'deploy_export':
      return <AppShell><DeployExportPage /></AppShell>;
    case 'edge_monitoring':
      return <AppShell><EdgeMonitoringPage /></AppShell>;
    default:
      return <Studio />;
  }
};

export const App: React.FC = () => {
  return (
    <StudioStoreProvider>
      <AppStateProvider>
        <ScreenRouter />
      </AppStateProvider>
    </StudioStoreProvider>
  );
};

export default App;
