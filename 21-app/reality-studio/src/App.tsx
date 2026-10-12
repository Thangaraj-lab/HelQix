import { lazy, Suspense, useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';
import { Activity, ArrowDown, ArrowRight, Box, CarFront, ChevronDown, CircleHelp, Database, FileCheck2, FolderKanban, Gauge, GitBranch, Layers3, LogOut, Maximize2, Menu, Pause, Play, Plus, Search, Settings2, ShieldCheck, Sparkles, Wrench, X } from 'lucide-react';
import { api, HelqixRequestError, type ActivityEvent, type BoundedSearchCandidate, type BoundedSearchPayload, type ComponentTypeDefinition, type EngineeringBranchPayload, type EngineeringReality, type EngineeringRequirement, type EVBatteryVehicleConfiguration, type EVDriveCyclePoint, type IntegrationRun, type IntentAnalysis, type ProjectPayload, type RealityComponent, type Resource, type User, type VerificationRecord, type VerificationRule } from './api';
import { ScenarioExecutionBoard } from './ScenarioExecutionBoard';

type Mode = 'Build' | 'Simulate' | 'Run' | 'Understand' | 'Improve' | 'Verify';
type SaveStatus = 'saved' | 'saving' | 'error';
type ServiceStatus = 'checking' | 'connected' | 'unavailable';
type ViewPreset = 'perspective' | 'top' | 'front' | 'side' | 'isometric';
type ConnectedTool = { id: string; label: string; status: string; available: boolean; note?: string };
type StudioSearchEntry = { id: string; kind: 'project' | 'component' | 'requirement' | 'run' | 'branch' | 'search' | 'evidence'; title: string; detail: string; searchable: string };
const EMPTY_RUNTIME_ENTITIES: Array<{ id: string; name: string; componentType: string }> = [];
const RuntimeCanvas = lazy(() => import('./RuntimeCanvas').then((module) => ({ default: module.RuntimeCanvas })));
const VehicleRuntimeCanvas = lazy(() => import('./RuntimeCanvas').then((module) => ({ default: module.VehicleRuntimeCanvas })));
const modes: Array<{ name: Mode; icon: typeof Box; label: string }> = [
  { name: 'Build', icon: Box, label: 'Define reality' },
  { name: 'Simulate', icon: Gauge, label: 'Prepare analysis' },
  { name: 'Run', icon: Activity, label: 'Execute models' },
  { name: 'Understand', icon: Layers3, label: 'Inspect behavior' },
  { name: 'Improve', icon: GitBranch, label: 'Explore alternatives' },
  { name: 'Verify', icon: ShieldCheck, label: 'Verify evidence' },
];

export function App() {
  const [user, setUser] = useState<User | null>(null);
  const [projects, setProjects] = useState<Resource<ProjectPayload>[]>([]);
  const [selectedProject, setSelectedProject] = useState<Resource<ProjectPayload> | null>(null);
  const [mode, setMode] = useState<Mode>('Build');
  const [authLoading, setAuthLoading] = useState(true);
  const [serviceStatus, setServiceStatus] = useState<ServiceStatus>('checking');
  const [busy, setBusy] = useState(false);
  const [saveStatus, setSaveStatus] = useState<SaveStatus>('saved');
  const [error, setError] = useState('');
  const [showNewProject, setShowNewProject] = useState(false);
  const [showProfile, setShowProfile] = useState(false);
  const [showProjectSettings, setShowProjectSettings] = useState(false);
  const [showActivity, setShowActivity] = useState(false);
  const [showIntegrations, setShowIntegrations] = useState(false);
  const [showHelp, setShowHelp] = useState(false);
  const [integrationLoading, setIntegrationLoading] = useState(false);
  const [integrationError, setIntegrationError] = useState('');
  const [showSearch, setShowSearch] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedSearchIndex, setSelectedSearchIndex] = useState(0);
  const [activityEvents, setActivityEvents] = useState<ActivityEvent[]>([]);
  const [activityLoading, setActivityLoading] = useState(false);
  const [activityError, setActivityError] = useState('');
  const [mobileNav, setMobileNav] = useState(false);
  const [showAddComponent, setShowAddComponent] = useState(false);
  const [showTopology, setShowTopology] = useState(false);
  const [showRequirement, setShowRequirement] = useState(false);
  const [showEVConfig, setShowEVConfig] = useState(false);
  const [selectedComponentId, setSelectedComponentId] = useState<string | null>(null);
  const [viewPreset, setViewPreset] = useState<ViewPreset>('isometric');
  const [showGrid, setShowGrid] = useState(true);
  const [showAxes, setShowAxes] = useState(true);
  const [fitRevision, setFitRevision] = useState(0);
  const [runs, setRuns] = useState<IntegrationRun[]>([]);
  const refreshRecordedFailure = async (cause: unknown, projectId?: string) => {
    if (!(cause instanceof HelqixRequestError) || !cause.runId || !projectId) return;
    try {
      const latest = await api.runs(projectId);
      setRuns(latest.runs);
    } catch { /* Preserve the original tool error; the saved run remains available on the next history refresh. */ }
  };
  const [selectedRun, setSelectedRun] = useState<IntegrationRun | null>(null);
  const [playbackIndex, setPlaybackIndex] = useState(0);
  const [playbackPlaying, setPlaybackPlaying] = useState(false);
  const [verificationRecords, setVerificationRecords] = useState<VerificationRecord[]>([]);
  const [branches, setBranches] = useState<Resource<EngineeringBranchPayload>[]>([]);
  const [searches, setSearches] = useState<Resource<BoundedSearchPayload>[]>([]);
  const [sweepProgress, setSweepProgress] = useState<{ index: number; total: number; currentA: number } | null>(null);
  const [tools, setTools] = useState<ConnectedTool[]>([]);

  const studioSearchIndex = useMemo<StudioSearchEntry[]>(() => {
    const reality = selectedProject?.payload.engineering_reality;
    const components = (reality?.components ?? []).map((item) => ({
      id: item.id, kind: 'component' as const, title: item.name,
      detail: `${item.componentType} · ${item.properties.length} declared properties`,
      searchable: `${item.name} ${item.componentType} ${item.id} ${item.properties.map((property) => property.name).join(' ')}`,
    }));
    const requirements = (selectedProject?.payload.requirements ?? []).map((item) => ({ id: item.id, kind: 'requirement' as const, title: item.title, detail: `${item.status} · ${item.reviewStatus}`, searchable: `${item.title} ${item.id} ${item.statement}` }));
    const projectEntry = selectedProject ? [{ id: selectedProject.id, kind: 'project' as const, title: selectedProject.payload.name, detail: `${selectedProject.payload.domain || 'Engineering'} · revision ${selectedProject.payload.engineering_reality?.revision ?? '—'}`, searchable: `${selectedProject.payload.name} ${selectedProject.payload.domain || ''}` }] : [];
    const runEntries = runs.map((item) => ({ id: item.id, kind: 'run' as const, title: item.operation.replaceAll('_', ' '), detail: `${item.tool} · ${item.result.status}`, searchable: `${item.id} ${item.operation} ${item.tool} ${item.result.status}` }));
    const branchEntries = branches.map((item) => ({ id: item.id, kind: 'branch' as const, title: item.payload.name, detail: `branch · ${item.payload.status}`, searchable: `${item.payload.name} ${item.id} ${item.payload.status}` }));
    const searchEntries = searches.map((item) => ({ id: item.id, kind: 'search' as const, title: item.payload.name, detail: `bounded search · ${item.payload.status}`, searchable: `${item.payload.name} ${item.id} ${item.payload.status}` }));
    const evidenceEntries = verificationRecords.map((item) => ({ id: item.id, kind: 'evidence' as const, title: item.ruleId, detail: `verification · ${item.status}`, searchable: `${item.id} ${item.ruleId} ${item.status}` }));
    return [...projectEntry, ...components, ...requirements, ...runEntries, ...branchEntries, ...searchEntries, ...evidenceEntries];
  }, [selectedProject, runs, branches, searches, verificationRecords]);

  // ... existing App logic unchanged ...
