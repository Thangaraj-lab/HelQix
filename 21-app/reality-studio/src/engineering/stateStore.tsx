import React, { createContext, useContext, useState, useEffect } from 'react';

export type ScreenId = 
  | 'landing'
  | 'home'
  | 'workspace'
  | 'model_editor'
  | 'simulation_setup'
  | 'simulation_run'
  | 'simulation_results'
  | 'failure_causality'
  | 'compare'
  | 'scenario_builder'
  | 'timeline_replay'
  | 'build_optimization'
  | 'compute_resources'
  | 'verify_validation'
  | 'deploy_export'
  | 'edge_monitoring';

export type WorkspaceMode = 'Reality' | 'Time' | 'Causality' | 'Branch' | 'Verification';
export type QualityTier = 'Auto' | 'High' | 'Low';
export type DeterminismLevel = 'Bitwise' | 'Tolerance' | 'Statistical';
export type OriginBadge = 'Observed' | 'Simulated' | 'Derived' | 'AI-hypothesis';
export type StatusBadge = 'Verified' | 'Unverified';

export interface AppState {
  currentScreen: ScreenId;
  setCurrentScreen: (screen: ScreenId) => void;
  workspaceMode: WorkspaceMode;
  setWorkspaceMode: (mode: WorkspaceMode) => void;
  qualityTier: QualityTier;
  setQualityTier: (tier: QualityTier) => void;
  determinism: DeterminismLevel;
  setDeterminism: (det: DeterminismLevel) => void;
  
  // Timeline State
  currentTime: number; // in seconds, e.g. 4.2
  totalDuration: number; // e.g. 12.0
  isPlaying: boolean;
  playbackSpeed: number;
  seekTo: (time: number) => void;
  togglePlay: () => void;
  freezeTime: number | null;
  freezeAtCurrent: () => void;
  replayHashMatch: boolean;

  // Selected Engineering Component
  selectedComponentId: string;
  setSelectedComponentId: (id: string) => void;

  // Run Information
  runId: string;
  revisionHash: string;
  isRunActive: boolean;
  setIsRunActive: (active: boolean) => void;

  // Layer Toggles
  layers: {
    structure: boolean;
    physics: boolean;
    state: boolean;
    flow: boolean;
    environment: boolean;
  };
  toggleLayer: (layer: keyof AppState['layers']) => void;

  // Notification Toast
  toast: { message: string; type: 'info' | 'success' | 'warning' | 'error' } | null;
  showToast: (message: string, type?: 'info' | 'success' | 'warning' | 'error') => void;
}

const AppStateContext = createContext<AppState | null>(null);

export const AppStateProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [currentScreen, setCurrentScreen] = useState<ScreenId>('landing');
  const [workspaceMode, setWorkspaceMode] = useState<WorkspaceMode>('Reality');
  const [qualityTier, setQualityTier] = useState<QualityTier>('High');
  const [determinism, setDeterminism] = useState<DeterminismLevel>('Bitwise');
  
  const [currentTime, setCurrentTime] = useState<number>(4.20);
  const totalDuration = 12.00;
  const [isPlaying, setIsPlaying] = useState<boolean>(false);
  const [playbackSpeed, setPlaybackSpeed] = useState<number>(1.0);
  const [freezeTime, setFreezeTime] = useState<number | null>(null);
  const [replayHashMatch, setReplayHashMatch] = useState<boolean>(true);

  const [selectedComponentId, setSelectedComponentId] = useState<string>('CMP-A-001');
  const [runId] = useState<string>('RUN-8421-B7');
  const [revisionHash] = useState<string>('bf318f4');
  const [isRunActive, setIsRunActive] = useState<boolean>(true);

  const [layers, setLayers] = useState({
    structure: true,
    physics: true,
    state: true,
    flow: true,
    environment: false
  });

  const [toast, setToast] = useState<{ message: string; type: 'info' | 'success' | 'warning' | 'error' } | null>(null);

  const showToast = (message: string, type: 'info' | 'success' | 'warning' | 'error' = 'info') => {
    setToast({ message, type });
    setTimeout(() => {
      setToast(null);
    }, 3800);
  };

  const toggleLayer = (layer: keyof typeof layers) => {
    setLayers(prev => ({ ...prev, [layer]: !prev[layer] }));
  };

  const seekTo = (time: number) => {
    const clamped = Math.max(0, Math.min(time, totalDuration));
    setCurrentTime(clamped);
  };

  const togglePlay = () => {
    setIsPlaying(prev => !prev);
  };

  const freezeAtCurrent = () => {
    setFreezeTime(currentTime);
    showToast(`State snapshot frozen at ${currentTime.toFixed(2)}s (Hash: 0x${Math.floor(currentTime * 987654).toString(16)})`, 'success');
  };

  // Timeline ticking simulation
  useEffect(() => {
    if (!isPlaying) return;
    const interval = setInterval(() => {
      setCurrentTime(prev => {
        if (prev >= totalDuration) {
          setIsPlaying(false);
          return 0;
        }
        return prev + 0.05 * playbackSpeed;
      });
    }, 50);
    return () => clearInterval(interval);
  }, [isPlaying, playbackSpeed, totalDuration]);

  const value: AppState = {
    currentScreen,
    setCurrentScreen,
    workspaceMode,
    setWorkspaceMode,
    qualityTier,
    setQualityTier,
    determinism,
    setDeterminism,
    currentTime,
    totalDuration,
    isPlaying,
    playbackSpeed,
    seekTo,
    togglePlay,
    freezeTime,
    freezeAtCurrent,
    replayHashMatch,
    selectedComponentId,
    setSelectedComponentId,
    runId,
    revisionHash,
    isRunActive,
    setIsRunActive,
    layers,
    toggleLayer,
    toast,
    showToast
  };

  return <AppStateContext.Provider value={value}>{children}</AppStateContext.Provider>;
};

export const useAppState = (): AppState => {
  const context = useContext(AppStateContext);
  if (!context) {
    throw new Error('useAppState must be used within an AppStateProvider');
  }
  return context;
};
