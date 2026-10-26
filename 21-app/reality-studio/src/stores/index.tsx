import React, { createContext, useContext, useState, useEffect } from 'react';
import {
  Project,
  ModelRevision,
  ModelOp,
  RunFrame,
  Value
} from '../contracts';
import { MockEngineClient } from '../mock/MockEngineClient';

// 1. Session Store (Panels, Active Tool, Camera, Layers, Look, Quality)
export interface SessionState {
  panels: {
    leftOpen: boolean;
    rightOpen: boolean;
    bottomOpen: boolean;
    leftWidth: number;
    rightWidth: number;
    bottomHeight: number;
  };
  setPanels: (update: Partial<SessionState['panels']>) => void;
  activeTool: 'select' | 'move' | 'rotate' | 'scale' | 'connect' | 'measure';
  setActiveTool: (tool: SessionState['activeTool']) => void;
  cameraPreset: 'Perspective' | 'Top' | 'Front' | 'Right' | 'Follow' | 'Orbit' | 'FPV';
  setCameraPreset: (preset: SessionState['cameraPreset']) => void;
  look: 'Realistic' | 'Engineering';
  setLook: (look: 'Realistic' | 'Engineering') => void;
  qualityTier: 'Auto' | 'High' | 'Low';
  setQualityTier: (tier: 'Auto' | 'High' | 'Low') => void;
  layers: {
    structure: boolean;
    physics: boolean;
    environment: boolean;
    state: boolean;
    flow: boolean;
    sensors: boolean;
    connections: boolean;
    failures: boolean;
    observations: boolean;
  };
  toggleLayer: (layer: keyof SessionState['layers']) => void;
}

// 2. Selection Store
export interface SelectionState {
  selectedIds: string[];
  hoveredId: string | null;
  select: (id: string, multi?: boolean) => void;
  deselectAll: () => void;
  setHovered: (id: string | null) => void;
}

// 3. Playback Store
export interface PlaybackState {
  t: number;
  duration: number;
  speed: number;
  isPlaying: boolean;
  setT: (t: number) => void;
  setSpeed: (speed: number) => void;
  togglePlay: () => void;
  pause: () => void;
}

// 4. Draft Store (Pending uncommitted ops)
export interface DraftState {
  pendingOps: ModelOp[];
  addPendingOp: (op: ModelOp) => void;
  clearPendingOps: () => void;
  isDraft: boolean;
}

// 5. Query Cache (Read-only projections from EngineClient)
export interface QueryCacheState {
  currentProject: Project | null;
  activeRevision: ModelRevision | null;
  currentFrame: RunFrame | null;
  setCurrentProject: (prj: Project | null) => void;
  setActiveRevision: (rev: ModelRevision | null) => void;
  setCurrentFrame: (frame: RunFrame | null) => void;
}

// 6. Assistant Store (Rule-based proposals - PREVIEW)
export interface Proposal {
  id: string;
  intent: string;
  description: string;
  reasoning: string;
  expectedEffect: string;
  ops: ModelOp[];
  status: 'pending' | 'applied' | 'rejected';
}

export interface AssistantState {
  proposals: Proposal[];
  activeProposal: Proposal | null;
  applyProposal: (id: string) => void;
  rejectProposal: (id: string) => void;
  dismiss: () => void;
}

export interface StudioContextBundle {
  session: SessionState;
  selection: SelectionState;
  playback: PlaybackState;
  draft: DraftState;
  query: QueryCacheState;
  assistant: AssistantState;
}

const StudioContext = createContext<StudioContextBundle | null>(null);

export const StudioStoreProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  // 1. Session Store
  const [panels, setPanelsInternal] = useState<SessionState['panels']>({
    leftOpen: true,
    rightOpen: true,
    bottomOpen: true,
    leftWidth: 280,
    rightWidth: 320,
    bottomHeight: 220
  });
  const [activeTool, setActiveTool] = useState<SessionState['activeTool']>('select');
  const [cameraPreset, setCameraPreset] = useState<SessionState['cameraPreset']>('Perspective');
  const [look, setLook] = useState<'Realistic' | 'Engineering'>('Realistic');
  const [qualityTier, setQualityTier] = useState<'Auto' | 'High' | 'Low'>('High');
  const [layers, setLayers] = useState<SessionState['layers']>({
    structure: true,
    physics: true,
    environment: false,
    state: true,
    flow: true,
    sensors: true,
    connections: true,
    failures: true,
    observations: false
  });

  const setPanels = (update: Partial<SessionState['panels']>) => {
    setPanelsInternal(prev => ({ ...prev, ...update }));
  };

  const toggleLayer = (l: keyof SessionState['layers']) => {
    setLayers(prev => ({ ...prev, [l]: !prev[l] }));
  };

  // 2. Selection Store
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [hoveredId, setHovered] = useState<string | null>(null);

  const select = (id: string, multi: boolean = false) => {
    if (multi) {
      setSelectedIds(prev => (prev.includes(id) ? prev.filter(x => x !== id) : [...prev, id]));
    } else {
      setSelectedIds([id]);
    }
  };
  const deselectAll = () => setSelectedIds([]);

  // 3. Playback Store
  const [t, setT] = useState<number>(0);
  const duration = 12.0;
  const [speed, setSpeed] = useState<number>(1.0);
  const [isPlaying, setIsPlaying] = useState<boolean>(false);

  const togglePlay = () => setIsPlaying(prev => !prev);
  const pause = () => setIsPlaying(false);

  // 4. Draft Store
  const [pendingOps, setPendingOps] = useState<ModelOp[]>([]);
  const addPendingOp = (op: ModelOp) => setPendingOps(prev => [...prev, op]);
  const clearPendingOps = () => setPendingOps([]);

  // 5. Query Cache
  const [currentProject, setCurrentProject] = useState<Project | null>(null);
  const [activeRevision, setActiveRevision] = useState<ModelRevision | null>(null);
  const [currentFrame, setCurrentFrame] = useState<RunFrame | null>(null);

  // 6. Assistant Store (Rule-based suggestions - PREVIEW)
  const [proposals, setProposals] = useState<Proposal[]>([
    {
      id: 'prop-cooling-bypass',
      intent: 'Mitigate Stator Winding Thermal Limit',
      description: 'Add secondary liquid cooling loop to stator sleeve',
      reasoning: 'Winding temperature exceeds 120.0 °C threshold at t=8.42s under high-load step.',
      expectedEffect: 'Estimated peak temperature drops by 14.2 °C (from 120.0 °C to 105.8 °C).',
      ops: [],
      status: 'pending'
    }
  ]);
  const [activeProposal, setActiveProposal] = useState<Proposal | null>(null);

  const applyProposal = (id: string) => {
    setProposals(prev => prev.map(p => (p.id === id ? { ...p, status: 'applied' } : p)));
    setActiveProposal(null);
  };
  const rejectProposal = (id: string) => {
    setProposals(prev => prev.map(p => (p.id === id ? { ...p, status: 'rejected' } : p)));
    setActiveProposal(null);
  };
  const dismiss = () => setActiveProposal(null);

  // Sync playback frame from MockEngineClient
  useEffect(() => {
    let active = true;
    MockEngineClient.getFrame('default-run', t).then(frame => {
      if (active) setCurrentFrame(frame);
    });
    return () => {
      active = false;
    };
  }, [t]);

  // Tick simulation time when playing
  useEffect(() => {
    if (!isPlaying) return;
    const interval = setInterval(() => {
      setT(prev => {
        if (prev >= duration) {
          setIsPlaying(false);
          return 0;
        }
        return parseFloat((prev + 0.05 * speed).toFixed(2));
      });
    }, 50);
    return () => clearInterval(interval);
  }, [isPlaying, speed, duration]);

  const bundle: StudioContextBundle = {
    session: {
      panels,
      setPanels,
      activeTool,
      setActiveTool,
      cameraPreset,
      setCameraPreset,
      look,
      setLook,
      qualityTier,
      setQualityTier,
      layers,
      toggleLayer
    },
    selection: {
      selectedIds,
      hoveredId,
      select,
      deselectAll,
      setHovered
    },
    playback: {
      t,
      duration,
      speed,
      isPlaying,
      setT,
      setSpeed,
      togglePlay,
      pause
    },
    draft: {
      pendingOps,
      addPendingOp,
      clearPendingOps,
      isDraft: pendingOps.length > 0
    },
    query: {
      currentProject,
      activeRevision,
      currentFrame,
      setCurrentProject,
      setActiveRevision,
      setCurrentFrame
    },
    assistant: {
      proposals,
      activeProposal,
      applyProposal,
      rejectProposal,
      dismiss
    }
  };

  return <StudioContext.Provider value={bundle}>{children}</StudioContext.Provider>;
};

export const useStudio = (): StudioContextBundle => {
  const ctx = useContext(StudioContext);
  if (!ctx) throw new Error('useStudio must be used within StudioStoreProvider');
  return ctx;
};
