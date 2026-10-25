import { lazy, Suspense, useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';
import { Activity, ArrowDown, ArrowRight, Box, CarFront, ChevronDown, CircleHelp, Database, FileCheck2, FolderKanban, Gauge, GitBranch, Layers3, Maximize2, Menu, Pause, Play, Plus, Search, Settings2, ShieldCheck, Sparkles, Wrench, X } from 'lucide-react';
import { api, HelqixRequestError, type ActivityEvent, type BoundedSearchCandidate, type BoundedSearchPayload, type ComponentTypeDefinition, type EngineeringBranchPayload, type EngineeringReality, type EngineeringRequirement, type EVBatteryVehicleConfiguration, type EVDriveCyclePoint, type IntegrationRun, type IntentAnalysis, type ProjectPayload, type RealityComponent, type Resource, type User, type VerificationRecord, type VerificationRule } from './api';

type Mode = 'Build' | 'Simulate' | 'Run' | 'Understand' | 'Improve' | 'Verify';
type SaveStatus = 'saved' | 'saving' | 'error';
type ServiceStatus = 'checking' | 'connected' | 'unavailable';
type ViewPreset = 'perspective' | 'top' | 'front' | 'side' | 'isometric';
type ConnectedTool = { id: string; label: string; status: string; available: boolean; note?: string };
type StudioSearchEntry = { id: string; kind: 'project' | 'component' | 'requirement' | 'run' | 'branch' | 'search' | 'evidence'; title: string; detail: string; searchable: string };
const EMPTY_RUNTIME_ENTITIES: Array<{ id: string; name: string; componentType: string }> = [];
const RuntimeCanvas = lazy(() => import('./RuntimeCanvas').then((module) => ({ default: module.RuntimeCanvas })));
const VehicleRuntimeCanvas = lazy(() => import('./RuntimeCanvas').then((module) => ({ default: module.VehicleRuntimeCanvas })));
const ScenarioExecutionBoard = lazy(() => import('./ScenarioExecutionBoard').then((module) => ({ default: module.ScenarioExecutionBoard })));
const modes: Array<{ name: Mode; icon: typeof Box; label: string }> = [
  { name: 'Build', icon: Box, label: 'Define reality' },
  { name: 'Simulate', icon: Gauge, label: 'Prepare analysis' },
  { name: 'Run', icon: Activity, label: 'Execute models' },
  { name: 'Understand', icon: Layers3, label: 'Inspect behavior' },
  { name: 'Improve', icon: GitBranch, label: 'Explore alternatives' },
  { name: 'Verify', icon: ShieldCheck, label: 'Verify evidence' },
];

export function App() {
  useEffect(() => {
    if (window.location.pathname === '/login' || window.location.pathname === '/signup') {
      window.history.replaceState(null, '', '/');
    }
  }, []);
  const [user, setUser] = useState<User | null>(null);
  const [projects, setProjects] = useState<Resource<ProjectPayload>[]>([]);
  const [selectedProject, setSelectedProject] = useState<Resource<ProjectPayload> | null>(null);
  const [mode, setMode] = useState<Mode>('Build');
  const [authLoading, setAuthLoading] = useState(true);
  const [workspaceAttempt, setWorkspaceAttempt] = useState(0);
  const [serviceStatus, setServiceStatus] = useState<ServiceStatus>('checking');
  const [busy, setBusy] = useState(false);
  const [saveStatus, setSaveStatus] = useState<SaveStatus>('saved');
  const [error, setError] = useState('');
  const [showNewProject, setShowNewProject] = useState(false);
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
    const requirements = (reality?.requirements ?? []).map((item) => ({
      id: item.id, kind: 'requirement' as const, title: item.statement,
      detail: `${item.priority} · ${item.reviewStatus}${item.verificationRule ? ` · ${item.verificationRule.metric.replaceAll('_', ' ')}` : ''}`,
      searchable: `${item.statement} ${item.priority} ${item.reviewStatus} ${item.id}`,
    }));
    const runEntries = runs.map((item) => ({
      id: item.id, kind: 'run' as const,
      title: item.operation === 'simulate_ev_drive_cycle' ? 'Vehicle drive-cycle analysis' : item.operation === 'simulate_bound_cell' ? 'Battery-cell discharge' : item.operation.replaceAll('_', ' '),
      detail: `${item.tool} · ${new Date(item.created_at * 1000).toLocaleString()} · ${item.id.slice(0, 8)}`,
      searchable: `${item.operation} ${item.tool} ${item.provenance} ${item.id}`,
    }));
    const branchEntries = branches.map((item) => ({
      id: item.id, kind: 'branch' as const, title: item.payload.name,
      detail: `Branch · reality revision ${item.payload.base_reality_revision} · ${item.payload.changes.length} declared change${item.payload.changes.length === 1 ? '' : 's'}`,
      searchable: `${item.payload.name} ${item.payload.description} ${item.payload.changes.map((change) => `${change.property_name} ${change.value} ${change.unit ?? ''}`).join(' ')} ${item.id}`,
    }));
    const searchEntries = searches.map((item) => ({
      id: item.id, kind: 'search' as const, title: item.payload.name,
      detail: `Bounded search · ${item.payload.completed_count}/${item.payload.candidate_count} candidates completed · ${item.payload.status}`,
      searchable: `${item.payload.name} ${item.payload.objective.metric} ${item.payload.variable.property_name} ${item.payload.candidates.map((candidate) => `${candidate.current_a} ${candidate.state}`).join(' ')} ${item.id}`,
    }));
    const evidenceEntries = verificationRecords.map((item) => ({
      id: item.id, kind: 'evidence' as const, title: item.payload.requirementTitle || item.payload.requirement_id || 'Verification evidence',
      detail: `Verification · ${item.payload.verdict ?? 'inconclusive'} · source run ${item.payload.source_run_id?.slice(0, 8) ?? 'unavailable'}`,
      searchable: `${item.payload.requirementTitle ?? ''} ${item.payload.requirement_id ?? ''} ${item.payload.verdict ?? ''} ${item.payload.observedValue ?? ''} ${item.payload.conditionFormula ?? ''} ${item.payload.source_run_id ?? ''} ${item.id}`,
    }));
    const projectEntry: StudioSearchEntry[] = selectedProject ? [{
      id: selectedProject.id, kind: 'project', title: selectedProject.payload.name,
      detail: `Project · revision ${selectedProject.version} · ${selectedProject.payload.domain || 'engineering system'}`,
      searchable: `${selectedProject.payload.name} ${selectedProject.payload.description ?? ''} ${selectedProject.payload.intent ?? ''} ${selectedProject.payload.domain ?? ''} ${selectedProject.id}`,
    }] : [];
    return [...projectEntry, ...components, ...requirements, ...runEntries, ...branchEntries, ...searchEntries, ...evidenceEntries];
  }, [selectedProject, runs, branches, searches, verificationRecords]);
  const matchingStudioResults = useMemo(() => {
    const query = searchQuery.trim().toLocaleLowerCase();
    if (!query) return studioSearchIndex.slice(0, 8);
    return studioSearchIndex.filter((item) => `${item.title} ${item.detail} ${item.searchable} ${item.kind}`.toLocaleLowerCase().includes(query)).slice(0, 12);
  }, [searchQuery, studioSearchIndex]);

  useEffect(() => {
    const handleShortcut = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        if (!showNewProject && !showAddComponent && !showTopology && !showRequirement && !showEVConfig) {
          setSearchQuery('');
          setShowSearch(true);
        }
      } else if (event.key === 'Escape') {
        if (showProjectSettings) setShowProjectSettings(false);
        else if (showEVConfig) setShowEVConfig(false);
        else if (showRequirement) setShowRequirement(false);
        else if (showTopology) setShowTopology(false);
        else if (showAddComponent) setShowAddComponent(false);
        else if (showNewProject) setShowNewProject(false);
        else if (showHelp) setShowHelp(false);
        else if (showIntegrations) setShowIntegrations(false);
        else if (showActivity) setShowActivity(false);
        else if (showSearch) setShowSearch(false);
        else if (mobileNav) setMobileNav(false);
      }
    };
    window.addEventListener('keydown', handleShortcut);
    return () => window.removeEventListener('keydown', handleShortcut);
  }, [showNewProject, showAddComponent, showTopology, showRequirement, showEVConfig, showProjectSettings, showHelp, showIntegrations, showActivity, showSearch, mobileNav]);

  const loadProjects = useCallback(async () => {
    const response = await api.projects();
    setProjects(response.data);
    setSelectedProject((current) => current ? response.data.find((p) => p.id === current.id) ?? current : response.data[0] ?? null);
  }, []);

  useEffect(() => {
    if (!selectedProject) { setRuns([]); setSelectedRun(null); setVerificationRecords([]); setBranches([]); setSearches([]); return; }
    let active = true;
    Promise.all([api.runs(selectedProject.id), api.verifications(selectedProject.id), api.resources<EngineeringBranchPayload>('branches', selectedProject.id), api.resources<BoundedSearchPayload>('searches', selectedProject.id)]).then(([{ runs: projectRuns }, { data }, { data: projectBranches }, { data: projectSearches }]) => {
      if (!active) return;
      setRuns(projectRuns);
      setSelectedRun((previous) => previous ? projectRuns.find((run) => run.id === previous.id) ?? previous : projectRuns[0] ?? null);
      setVerificationRecords(data);
      setBranches(projectBranches);
      setSearches(projectSearches);
    }).catch(() => { if (active) { setRuns([]); setSelectedRun(null); setVerificationRecords([]); setBranches([]); setSearches([]); } });
    return () => { active = false; };
  }, [selectedProject?.id, selectedProject?.version]);

  const selectedRunSamples = Array.isArray(selectedRun?.result.series) ? selectedRun.result.series : [];
  const selectedRunSupportsReplay = selectedRun?.operation === 'simulate_ev_drive_cycle' || selectedRun?.operation === 'simulate_bound_cell';
  useEffect(() => {
    setPlaybackIndex(0);
    setPlaybackPlaying(false);
  }, [selectedRun?.id]);
  useEffect(() => {
    if (mode !== 'Understand') setPlaybackPlaying(false);
  }, [mode]);
  useEffect(() => {
    if (!playbackPlaying || !selectedRunSupportsReplay || selectedRunSamples.length < 2) return;
    if (playbackIndex >= selectedRunSamples.length - 1) { setPlaybackPlaying(false); return; }
    const timer = window.setTimeout(() => setPlaybackIndex((index) => index + 1), Math.max(24, 12_000 / selectedRunSamples.length));
    return () => window.clearTimeout(timer);
  }, [playbackPlaying, playbackIndex, selectedRun?.id, selectedRunSupportsReplay, selectedRunSamples.length]);

  useEffect(() => {
    let alive = true;
    setAuthLoading(true);
    setError('');
    api.me().catch(() => api.localWorkspace()).then(async ({ user: current }) => {
      await loadProjects();
      if (!alive) return;
      setUser(current);
      void api.integrations().then((res) => { if (alive) setTools(res.integrations ?? []); }).catch(() => undefined);
    }).catch((cause) => {
      if (alive) setError(cause instanceof Error ? cause.message : 'Could not open the local workspace.');
    }).finally(() => { if (alive) setAuthLoading(false); });
    return () => { alive = false; };
  }, [loadProjects, workspaceAttempt]);

  useEffect(() => {
    let alive = true;
    let checking = false;
    const checkService = async () => {
      if (checking) return;
      checking = true;
      try {
        const health = await api.health();
        if (alive) setServiceStatus(health.status === 'live' ? 'connected' : 'unavailable');
      } catch {
        if (alive) setServiceStatus('unavailable');
      } finally {
        checking = false;
      }
    };
    void checkService();
    const timer = window.setInterval(() => void checkService(), 15_000);
    return () => { alive = false; window.clearInterval(timer); };
  }, []);

  useEffect(() => {
    if (!tools.some((tool) => tool.status === 'checking')) return;
    const timer = window.setInterval(() => {
      api.integrations().then((response) => setTools(response.integrations ?? [])).catch(() => undefined);
    }, 4000);
    return () => window.clearInterval(timer);
  }, [tools]);

  const createProject = async (payload: ProjectPayload) => {
    setBusy(true); setError('');
    try {
      const domain = (payload.domain || 'general').trim().toLowerCase().replace(/[^a-z0-9._-]+/g, '_').replace(/^[^a-z]+/, 'g').slice(0, 80);
      const initialReality: EngineeringReality = {
        schema: 'helqix.engineering-reality.v1', name: payload.name,
        domain, description: payload.description || '', revision: 1, unitsSystem: 'SI',
        components: [], interfaces: [], connections: [], requirements: [], modelBindings: [],
      };
      const result = await api.createProject({ ...payload, engineering_reality: initialReality });
      setProjects((items) => [result.data, ...items]); setSelectedProject(result.data); setSaveStatus('saved'); setShowNewProject(false); setMode('Build');
    } catch (e) { setError(e instanceof Error ? e.message : 'Could not create project.'); }
    finally { setBusy(false); }
  };

  const addComponent = async (name: string, componentType: string, cellInputs?: { currentA: number; cutoffVoltageV: number }) => {
    if (!selectedProject) return;
    setBusy(true); setError('');
    try {
      const current = selectedProject.payload as ProjectPayload & { engineering_reality?: EngineeringReality };
      const reality: EngineeringReality = current.engineering_reality ?? {
        schema: 'helqix.engineering-reality.v1',
        name: current.name,
        domain: (current.domain || 'general').trim().toLowerCase().replace(/[^a-z0-9._-]+/g, '_').replace(/^[^a-z]+/, 'g').slice(0, 80),
        description: current.description || '', revision: 1, unitsSystem: 'SI',
        components: [], interfaces: [], connections: [], requirements: [], modelBindings: [],
      };
      const idBase = name.trim().toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^\d/, 'c_').replace(/^_+|_+$/g, '') || 'component';
      const id = `${idBase.slice(0, 115)}_${crypto.randomUUID().slice(0, 8)}`;
      const normalizedType = componentType.trim();
      const isCell = ['battery_cell', 'lithium_ion_cell'].includes(normalizedType.toLowerCase());
      const component: RealityComponent = {
        id, name: name.trim(), componentType: normalizedType,
        properties: isCell && cellInputs ? [
          { name: 'current_a', value: cellInputs.currentA, unit: 'A', origin: 'user' },
          { name: 'cutoff_voltage_v', value: cellInputs.cutoffVoltageV, unit: 'V', origin: 'user' },
        ] : [],
      };
      const bindings = isCell && cellInputs ? [...reality.modelBindings, {
        id: `binding_${id.slice(0, 100)}`, modelId: 'pybamm.lithium-ion.spm', modelVersion: 'Chen2020',
        scopeComponentId: id, declaredFidelity: 'Single-cell discharge SPM with Chen2020 parameters; use only when the physical cell matches that parameterization; no pack or product-level coupling',
      }] : reality.modelBindings;
      const updatedReality = { ...reality, revision: reality.revision + 1, components: [...reality.components, component], modelBindings: bindings };
      setSaveStatus('saving');
      const result = await api.updateProject(selectedProject, { ...current, engineering_reality: updatedReality });
      setSelectedProject(result.data); setProjects((items) => items.map((item) => item.id === result.data.id ? result.data : item)); setSaveStatus('saved');
      setSelectedComponentId(id); setShowAddComponent(false);
    } catch (e) { setSaveStatus('error'); setError(e instanceof Error ? e.message : 'Could not save this component.'); }
    finally { setBusy(false); }
  };

  const runCellAnalysis = async (bindingId?: string) => {
    if (!selectedProject) return;
    const reality = selectedProject.payload.engineering_reality;
    const binding = reality?.modelBindings.find((item) => item.modelId === 'pybamm.lithium-ion.spm' && item.modelVersion === 'Chen2020' && (!bindingId || item.id === bindingId));
    if (!binding) { setError('Add a battery_cell with explicit current and cutoff voltage before running the cell model.'); setMode('Build'); return; }
    setBusy(true); setError('');
    try {
      const { run } = await api.execute('pybamm', 'simulate_bound_cell', { project_id: selectedProject.id, binding_id: binding.id });
      setRuns((previous) => [run, ...previous.filter((item) => item.id !== run.id)]);
      setSelectedRun(run); setMode('Understand');
    } catch (e) { await refreshRecordedFailure(e, selectedProject.id); setError(e instanceof Error ? e.message : 'The engineering model did not complete.'); }
    finally { setBusy(false); }
  };

  const runVehicleAnalysis = async () => {
    if (!selectedProject) return;
    if (!selectedProject.payload.ev_vehicle) { setError('Save an explicit vehicle, battery-pack, and drive-cycle model in Build before running this analysis.'); return; }
    setBusy(true); setError('');
    try {
      const { run } = await api.execute('pybamm', 'simulate_ev_drive_cycle', { project_id: selectedProject.id });
      setRuns((previous) => [run, ...previous.filter((item) => item.id !== run.id)]); setSelectedRun(run); setMode('Understand');
    } catch (e) { await refreshRecordedFailure(e, selectedProject.id); setError(e instanceof Error ? e.message : 'The vehicle drive-cycle model did not complete.'); }
    finally { setBusy(false); }
  };

  const runAnalysis = async () => {
    if (selectedProject?.payload.ev_vehicle) await runVehicleAnalysis();
    else await runCellAnalysis();
  };

  const saveEVConfiguration = async (configuration: EVBatteryVehicleConfiguration) => {
    if (!selectedProject) return;
    setBusy(true); setError('');
    try {
      setSaveStatus('saving');
      const result = await api.updateProject(selectedProject, { ...selectedProject.payload, ev_vehicle: configuration });
      setSelectedProject(result.data); setProjects((items) => items.map((item) => item.id === result.data.id ? result.data : item)); setSaveStatus('saved'); setShowEVConfig(false);
    } catch (e) { setSaveStatus('error'); setError(e instanceof Error ? e.message : 'Could not save the vehicle model.'); }
    finally { setBusy(false); }
  };

  const saveProjectSettings = async (name: string, domain: string, description: string): Promise<boolean> => {
    const project = selectedProject;
    if (!project) return false;
    const nextName = name.trim();
    const nextDomain = normalizeDomain(domain || 'general');
    const nextDescription = description.trim();
    if (!nextName || !nextDomain) {
      setError('Project name and domain are required.');
      return false;
    }
    setBusy(true); setError(''); setSaveStatus('saving');
    try {
      const payload: ProjectPayload = {
        ...project.payload,
        name: nextName,
        domain: nextDomain,
        description: nextDescription,
        ...(project.payload.engineering_reality ? {
          engineering_reality: {
            ...project.payload.engineering_reality,
            name: nextName,
            domain: nextDomain,
            description: nextDescription,
            revision: project.payload.engineering_reality.revision + 1,
          },
        } : {}),
      };
      const result = await api.updateProject(project, payload);
      setSelectedProject(result.data);
      setProjects((items) => items.map((item) => item.id === result.data.id ? result.data : item));
      setSaveStatus('saved');
      return true;
    } catch (cause) {
      setSaveStatus('error');
      setError(cause instanceof Error ? cause.message : 'Could not save project settings.');
      return false;
    } finally {
      setBusy(false);
    }
  };

  const saveReality = async (transform: (reality: EngineeringReality) => EngineeringReality): Promise<boolean> => {
    if (!selectedProject) return false;
    setBusy(true); setError('');
    try {
      setSaveStatus('saving');
      const payload = selectedProject.payload;
      const current = payload.engineering_reality ?? { schema: 'helqix.engineering-reality.v1' as const, name: payload.name, domain: payload.domain || 'general', description: payload.description || '', revision: 0, unitsSystem: 'SI' as const, components: [], interfaces: [], connections: [], requirements: [], modelBindings: [] };
      const next = transform(current);
      const result = await api.updateProject(selectedProject, { ...payload, engineering_reality: { ...next, revision: current.revision + 1 } });
      setSelectedProject(result.data); setProjects((items) => items.map((item) => item.id === result.data.id ? result.data : item)); setSaveStatus('saved');
      return true;
    } catch (e) { setSaveStatus('error'); setError(e instanceof Error ? e.message : 'Could not save the engineering reality.'); return false; }
    finally { setBusy(false); }
  };

  const addInterface = async (componentId: string, name: string, direction: 'in' | 'out' | 'bidirectional', domain: string) => {
    const id = `port_${crypto.randomUUID().replaceAll('-', '')}`;
    return saveReality((reality) => ({ ...reality, interfaces: [...reality.interfaces, { id, componentId, name: name.trim(), direction, domain: domain.trim() }] }));
  };

  const addConnection = async (sourceInterfaceId: string, targetInterfaceId: string, connectionType: string) => {
    const id = `connection_${crypto.randomUUID().replaceAll('-', '')}`;
    return saveReality((reality) => ({ ...reality, connections: [...reality.connections, { id, sourceInterfaceId, targetInterfaceId, connectionType: connectionType.trim() || 'direct' }] }));
  };

  const removeConnection = async (id: string) => {
    await saveReality((reality) => ({ ...reality, connections: reality.connections.filter((item) => item.id !== id) }));
  };

  const removeInterface = async (id: string) => {
    const reality = selectedProject?.payload.engineering_reality;
    if (reality?.connections.some((item) => item.sourceInterfaceId === id || item.targetInterfaceId === id)) {
      setError('Disconnect this interface before removing it.');
      return;
    }
    await saveReality((current) => ({ ...current, interfaces: current.interfaces.filter((item) => item.id !== id) }));
  };

  const addRequirement = async (statement: string, priority: EngineeringRequirement['priority'], rule: VerificationRule) => {
    const requirement: EngineeringRequirement = { id: `req_${crypto.randomUUID()}`, statement: statement.trim(), priority, reviewStatus: 'draft', verificationRule: rule };
    if (await saveReality((reality) => ({ ...reality, requirements: [...reality.requirements, requirement] }))) setShowRequirement(false);
  };

  const reviewRequirement = async (id: string, reviewStatus: EngineeringRequirement['reviewStatus']) => {
    await saveReality((reality) => ({ ...reality, requirements: reality.requirements.map((item) => item.id === id ? { ...item, reviewStatus } : item) }));
  };

  const compatibleRun = runs.find((run) => run.tool === 'pybamm' && run.operation === 'simulate_bound_cell' && run.result.status === 'completed' && run.project_snapshot?.project_version === selectedProject?.version) ?? null;
  const verifyRequirements = async () => {
    if (!selectedProject || !compatibleRun) { setError('Run the model again after accepting requirements. Verification requires a run from the current saved revision.'); return; }
    setBusy(true); setError('');
    try {
      await api.execute('helqix_reality', 'verify_bound_run', { project_id: selectedProject.id, run_id: compatibleRun.id });
      const { data } = await api.verifications(selectedProject.id); setVerificationRecords(data);
    } catch (e) { setError(e instanceof Error ? e.message : 'Verification could not be completed.'); }
    finally { setBusy(false); }
  };

  const exportEvidence = async () => {
    if (!selectedProject) return;
    setBusy(true); setError('');
    try {
      const blob = await api.exportEvidence(selectedProject.id);
      const href = URL.createObjectURL(blob); const anchor = document.createElement('a'); anchor.href = href; anchor.download = `helqix-evidence-${selectedProject.id}.json`; anchor.click(); URL.revokeObjectURL(href);
    } catch (e) { setError(e instanceof Error ? e.message : 'Evidence export failed.'); }
    finally { setBusy(false); }
  };

  const runBoundedSweep = async (bindingId: string, currentsA: number[], minimumDurationH: number | null) => {
    if (!selectedProject) return;
    const reality = selectedProject.payload.engineering_reality;
    const binding = reality?.modelBindings.find((item) => item.id === bindingId);
    if (!reality || !binding || currentsA.length < 2 || currentsA.length > 5 || new Set(currentsA).size !== currentsA.length || currentsA.some((value) => !Number.isFinite(value) || value < 0.05 || value > 100)) {
      setError('Choose a supported cell model and enter 2–5 unique currents between 0.05 and 100 A.');
      return;
    }
    const component = reality.components.find((item) => item.id === binding.scopeComponentId);
    if (!component || !['battery_cell', 'lithium_ion_cell'].includes(component.componentType.toLowerCase())) {
      setError('The supported sweep applies only to a saved battery-cell model binding.');
      return;
    }

    setBusy(true);
    setError('');
    setSweepProgress({ index: 0, total: currentsA.length, currentA: currentsA[0] });
    const candidates: BoundedSearchCandidate[] = [];
    try {
      for (const [index, currentA] of currentsA.entries()) {
        setSweepProgress({ index: index + 1, total: currentsA.length, currentA });
        let branch: Resource<EngineeringBranchPayload> | null = null;
        try {
          const branchPayload: EngineeringBranchPayload = {
            schema: 'helqix.engineering-branch.v1',
            name: `Cell current ${currentA} A`,
            description: `Candidate ${index + 1} of ${currentsA.length}; created from reality revision ${reality.revision} for the declared current sweep.`,
            base_reality_revision: reality.revision,
            changes: [{ component_id: component.id, property_name: 'current_a', value: currentA, unit: 'A' }],
          };
          const savedBranch = await api.createResource('branches', selectedProject.id, branchPayload);
          branch = savedBranch.data;
          setBranches((items) => [savedBranch.data, ...items.filter((item) => item.id !== savedBranch.data.id)]);

          const { run } = await api.execute('pybamm', 'simulate_bound_cell', { project_id: selectedProject.id, binding_id: binding.id, branch_id: branch.id });
          setRuns((items) => [run, ...items.filter((item) => item.id !== run.id)]);
          setSelectedRun(run);
          const duration = Number(run.result.duration_h);
          const capacity = Number(run.result.discharged_capacity_ah);
          const voltage = Number(run.result.final_voltage_v);
          if (![duration, capacity, voltage].every((value) => Number.isFinite(value) && value > 0)) {
            throw new Error('The completed tool run did not return the three required finite cell metrics.');
          }
          candidates.push({ current_a: currentA, state: 'completed', branch_id: branch.id, run_id: run.id, duration_h: duration, discharged_capacity_ah: capacity, final_voltage_v: voltage });
        } catch (candidateError) {
          await refreshRecordedFailure(candidateError, selectedProject.id);
          candidates.push({ current_a: currentA, state: 'failed', ...(branch ? { branch_id: branch.id } : {}), error: (candidateError instanceof Error ? candidateError.message : 'Candidate execution failed').slice(0, 1000) });
        }
      }

      const completed = candidates.filter((candidate) => candidate.state === 'completed');
      const eligible = completed.filter((candidate) => minimumDurationH === null || (candidate.duration_h ?? 0) >= minimumDurationH);
      const best = eligible.reduce<BoundedSearchCandidate | null>((winner, candidate) => !winner || (candidate.discharged_capacity_ah ?? 0) > (winner.discharged_capacity_ah ?? 0) ? candidate : winner, null);
      const payload: BoundedSearchPayload = {
        schema: 'helqix.bounded-search.v1',
        name: `Cell-current sweep · ${new Date().toLocaleString()}`,
        status: completed.length === candidates.length ? 'completed' : completed.length ? 'partial' : 'failed',
        base_reality_revision: reality.revision,
        binding_id: binding.id,
        variable: { component_id: component.id, property_name: 'current_a', unit: 'A', values: currentsA },
        objective: { metric: 'discharged_capacity_ah', direction: 'maximize' },
        constraint: { metric: 'duration_h', operator: '>=', value_h: minimumDurationH },
        candidate_count: candidates.length,
        completed_count: completed.length,
        best_evaluated_candidate_a: best?.current_a ?? null,
        candidates,
        applicability: 'Finite sweep of a single bound battery cell using PyBaMM SPM / Chen2020. It does not optimize or predict a battery pack, vehicle, or product system.',
      };
      const savedSearch = await api.createResource('searches', selectedProject.id, payload);
      setSearches((items) => [savedSearch.data, ...items.filter((item) => item.id !== savedSearch.data.id)]);
    } catch (searchError) {
      setError(searchError instanceof Error ? searchError.message : 'The sweep could not be recorded. Completed candidate runs remain in the run history.');
    } finally {
      setSweepProgress(null);
      setBusy(false);
    }
  };

  const openActivity = async () => {
    if (!selectedProject) return;
    setShowIntegrations(false); setShowHelp(false);
    setShowActivity(true); setActivityLoading(true); setActivityError('');
    try { const result = await api.activity(selectedProject.id); setActivityEvents(result.events); }
    catch (e) { setActivityError(e instanceof Error ? e.message : 'Could not load project activity.'); }
    finally { setActivityLoading(false); }
  };

  const openIntegrations = async () => {
    setShowHelp(false);
    setShowActivity(false);
    setShowIntegrations(true); setIntegrationLoading(true); setIntegrationError('');
    try { const result = await api.integrations(); setTools(result.integrations ?? []); }
    catch (e) { setIntegrationError(e instanceof Error ? e.message : 'Could not load tool availability.'); }
    finally { setIntegrationLoading(false); }
  };

  const openHelp = () => {
    setShowActivity(false); setShowIntegrations(false); setShowHelp(true);
  };

  const openSearchEntry = (entry: StudioSearchEntry) => {
    if (entry.kind === 'project') {
      setMode('Build');
      setSelectedComponentId(null);
    } else if (entry.kind === 'component') {
      setSelectedComponentId(entry.id);
      setMode('Build');
    } else if (entry.kind === 'requirement') {
      setSelectedComponentId(null);
      setMode('Verify');
    } else if (entry.kind === 'run') {
      const found = runs.find((item) => item.id === entry.id);
      if (found) { setSelectedRun(found); setMode('Understand'); }
    } else if (entry.kind === 'branch' || entry.kind === 'search') {
      setMode('Improve');
    } else {
      setMode('Verify');
    }
    setMobileNav(false);
    setShowSearch(false);
    setSearchQuery('');
  };

  if (authLoading) return <div className="loading-screen"><Brand /><span className="loading-orbit" /><p>Opening your local engineering workspace</p></div>;
  if (!user) return <WorkspaceUnavailable error={error} onRetry={() => setWorkspaceAttempt((attempt) => attempt + 1)} />;
  if (!selectedProject) return <ProjectHome projects={projects} error={error} busy={busy} showNewProject={showNewProject} onCreate={() => setShowNewProject(true)} onCloseNewProject={() => setShowNewProject(false)} onCreateProject={createProject} onRefresh={loadProjects} onOpen={(p) => { setSelectedProject(p); setSaveStatus('saved'); setMode('Build'); }} />;

  return <main className="studio">
    <header className="topbar">
      <button className="icon-button mobile-menu" onClick={() => setMobileNav(!mobileNav)} aria-label="Toggle navigation"><Menu size={18} /></button>
      <Brand compact />
      <div className="top-divider" />
      <button className="project-switcher" onClick={() => { setSaveStatus('saved'); setSelectedProject(null); }}><span className="project-mark">{selectedProject.payload.name.slice(0, 1).toUpperCase()}</span><span className="project-name">{selectedProject.payload.name}</span><ChevronDown size={14} /></button>
      <span className="revision-pill">REV {String(selectedProject.version).padStart(2, '0')}</span>
      <div className="top-spacer" />
      <div className={`save-state save-${saveStatus}`} role="status" aria-live="polite"><span className="save-dot" /> {saveStatus === 'saving' ? 'Saving changes…' : saveStatus === 'error' ? 'Save failed' : 'Saved'}</div>
      <button className="search-trigger" aria-label="Search this project" title="Search this project (Ctrl/⌘ K)" onClick={() => { setSearchQuery(''); setShowSearch(true); }}><Search size={15} /><span>Search</span><kbd>Ctrl K</kbd></button>
    </header>

    <div className="workflow-bar" aria-label="Engineering modes">
      {modes.map(({ name, icon: Icon, label }, i) => <button key={name} className={`mode-step ${mode === name ? 'active' : ''}`} onClick={() => setMode(name)} title={label}>
        <span className="mode-index">{String(i + 1).padStart(2, '0')}</span><Icon size={15} /><span>{name}</span>
      </button>)}
      <div className="workflow-caption">Engineering Reality Studio</div>
    </div>

    <section className="workspace">
      <aside className={`left-rail ${mobileNav ? 'open' : ''}`}>
        <div className="rail-heading"><span>{mode === 'Build' ? 'REALITY' : mode.toUpperCase()}</span><button className="icon-button rail-more" title="Project settings" aria-label="Project settings" onClick={() => { setError(''); setShowProjectSettings(true); }}><Settings2 size={15} /></button></div>
        {mode === 'Build' ? <BuildTree project={selectedProject} selectedComponentId={selectedComponentId} onSelectComponent={setSelectedComponentId} onOpenTopology={() => setShowTopology(true)} /> : <ModeTree mode={mode} project={selectedProject} tools={tools} />}
        <button className="rail-add" onClick={() => { setMode('Build'); setShowAddComponent(true); }}><Plus size={14} /> Add to reality</button>
        <div className="rail-footer"><button type="button" onClick={openHelp} aria-expanded={showHelp} aria-controls="studio-help-drawer"><CircleHelp size={15} /> Help & guides</button><button type="button" onClick={() => void openIntegrations()} aria-expanded={showIntegrations} aria-controls="integration-drawer"><Wrench size={15} /> Integrations <span className="mini-count">{tools.length}</span></button></div>
      </aside>

      <section className="canvas-column">
        <div className="canvas-toolbar"><div className="canvas-breadcrumb"><span>{selectedProject.payload.domain || 'ENGINEERING SYSTEM'}</span><ArrowRight size={12} /><span>{mode.toUpperCase()}</span></div><div className="canvas-actions">{mode === 'Build' && supportsEVVehicleModel(selectedProject) && <button className="tool-button" onClick={() => setShowEVConfig(true)}><CarFront size={14} /> Vehicle model</button>}<ViewportControls viewPreset={viewPreset} onViewChange={setViewPreset} showGrid={showGrid} onShowGrid={setShowGrid} showAxes={showAxes} onShowAxes={setShowAxes} onFit={() => setFitRevision((value) => value + 1)} /></div></div>
        <CanvasStage mode={mode} project={selectedProject} run={selectedRun} playbackIndex={playbackIndex} playbackPlaying={playbackPlaying} onPlaybackIndexChange={setPlaybackIndex} onPlaybackPlayingChange={setPlaybackPlaying} requirements={selectedProject.payload.engineering_reality?.requirements ?? []} records={verificationRecords} compatibleRun={compatibleRun} selectedComponentId={selectedComponentId} onSelectComponent={setSelectedComponentId} viewPreset={viewPreset} showGrid={showGrid} showAxes={showAxes} fitRevision={fitRevision} busy={busy} tools={tools} branches={branches} searches={searches} runs={runs} sweepProgress={sweepProgress} onRunSweep={runBoundedSweep} onRunVehicle={runVehicleAnalysis} onRunCell={runCellAnalysis} onOpenRun={(item) => { setSelectedRun(item); setMode('Understand'); }} onGoBuild={() => setMode('Build')} onGoSimulate={() => setMode('Simulate')} onAddRequirement={() => setShowRequirement(true)} onReview={reviewRequirement} onVerify={verifyRequirements} onExport={exportEvidence} />
        <Timeline mode={mode} run={selectedRun} runs={runs} playbackIndex={playbackIndex} playing={playbackPlaying} onIndexChange={setPlaybackIndex} onPlayingChange={setPlaybackPlaying} />
      </section>

      <Inspector mode={mode} project={selectedProject} tools={tools} selectedComponentId={selectedComponentId} onAddComponent={() => setShowAddComponent(true)} onRun={runAnalysis} onAddRequirement={() => setShowRequirement(true)} busy={busy} />
    </section>
    <footer className="statusbar"><span className={`service-status connection-${serviceStatus}`} role="status" aria-live="polite"><span className="status-led" /> {serviceStatus === 'checking' ? 'Checking service' : serviceStatus === 'connected' ? 'Service connected' : 'Service unavailable'}</span><span>SI units</span><span>Local project</span><button aria-expanded={showActivity} aria-controls="project-activity-drawer" onClick={() => void openActivity()}><Activity size={13} /> Activity</button></footer>

    {showSearch && <div className="studio-search-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setShowSearch(false); }}><section className="studio-search" role="dialog" aria-modal="true" aria-labelledby="studio-search-title"><div className="studio-search-input-row"><Search size={17}/><label className="sr-only" id="studio-search-title" htmlFor="studio-search-input">Search this project</label><input id="studio-search-input" autoFocus value={searchQuery} onChange={(event) => { setSearchQuery(event.target.value); setSelectedSearchIndex(0); }} onKeyDown={(event) => { if (event.key === 'ArrowDown') { event.preventDefault(); setSelectedSearchIndex((index) => Math.max(0, Math.min(index + 1, matchingStudioResults.length - 1))); } else if (event.key === 'ArrowUp') { event.preventDefault(); setSelectedSearchIndex((index) => Math.max(0, index - 1)); } else if (event.key === 'Enter' && matchingStudioResults[selectedSearchIndex]) { event.preventDefault(); openSearchEntry(matchingStudioResults[selectedSearchIndex]); } }} placeholder="Search project, components, evidence, and runs…"/><kbd>ESC</kbd><button className="icon-button" onClick={() => setShowSearch(false)} aria-label="Close search"><X size={16}/></button></div><div className="studio-search-heading"><span>{searchQuery.trim() ? 'TOP MATCHES' : 'PROJECT RECORDS'}</span><small>{selectedProject.payload.name}</small></div><div className="studio-search-results" role="listbox" aria-label="Project search results">{matchingStudioResults.length ? matchingStudioResults.map((entry, index) => <button type="button" role="option" aria-selected={selectedSearchIndex === index} className="studio-search-result" key={`${entry.kind}:${entry.id}`} onMouseEnter={() => setSelectedSearchIndex(index)} onClick={() => openSearchEntry(entry)}><span className="studio-search-kind">{entry.kind === 'project' || entry.kind === 'component' ? <Box size={15}/> : entry.kind === 'requirement' || entry.kind === 'evidence' ? <FileCheck2 size={15}/> : entry.kind === 'run' ? <Activity size={15}/> : <GitBranch size={15}/>}</span><span className="studio-search-copy"><b>{entry.title}</b><small>{entry.detail}</small></span><span className="studio-search-type">{entry.kind}</span><ArrowRight size={14}/></button>) : <div className="studio-search-empty"><Search size={18}/><span>No matching records in this project.</span><small>Search covers this project, components, requirements, branches, sweeps, evidence, and runs.</small></div>}</div><footer className="studio-search-footer"><span><kbd>↑</kbd><kbd>↓</kbd> to navigate</span><span><kbd>ENTER</kbd> to open</span><span><kbd>ESC</kbd> to close</span></footer></section></div>}
    {showActivity && <ActivityDrawer project={selectedProject} events={activityEvents} loading={activityLoading} error={activityError} runs={runs} verifications={verificationRecords} tools={tools} selectedRun={selectedRun} onOpenRun={(item) => { setSelectedRun(item); setMode('Understand'); setShowActivity(false); }} onExport={exportEvidence} onClose={() => setShowActivity(false)} />}
    {showIntegrations && <div className="integration-drawer-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setShowIntegrations(false); }}><aside id="integration-drawer" className="integration-drawer" role="dialog" aria-modal="true" aria-labelledby="integration-title"><header className="integration-drawer-header"><div><span className="eyebrow"><span className="eyebrow-line"/> ENGINEERING TOOLCHAIN</span><h2 id="integration-title">Integrations</h2><p>Availability reported by the local gateway. HELQIX does not assume an unavailable tool is installed.</p></div><button className="icon-button" onClick={() => setShowIntegrations(false)} aria-label="Close integrations"><X size={16}/></button></header><div className="integration-drawer-actions"><span>{tools.filter((tool) => tool.available).length} of {tools.length} available</span><button className="secondary-button" disabled={integrationLoading} onClick={() => void openIntegrations()}>{integrationLoading ? 'Checking…' : 'Refresh status'}</button></div>{integrationError ? <div className="form-error">{integrationError}</div> : integrationLoading && !tools.length ? <div className="activity-state">Checking configured tool availability…</div> : tools.length ? <div className="integration-tool-list">{tools.map((tool) => <article className="integration-tool-row" key={tool.id}><span className={`integration-tool-mark ${tool.available ? 'is-available' : ''}`}><Wrench size={15}/></span><div className="integration-tool-copy"><b>{tool.label}</b><small>{tool.note || (tool.available ? 'The gateway reports this tool is available.' : 'Install and configure this tool to enable its supported operations.')}</small><code>{tool.id}</code></div><span className={`integration-tool-status ${tool.available ? 'is-available' : ''}`}><i/>{tool.available ? 'READY' : tool.status.toUpperCase()}</span></article>)}</div> : <div className="activity-state">No integrations are registered with the gateway.</div>}<footer className="integration-drawer-footer"><span>Tool availability does not establish model validity or engineering applicability.</span><button className="secondary-button" onClick={() => setShowIntegrations(false)}>Close</button></footer></aside></div>}

    {showHelp && <div className="integration-drawer-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setShowHelp(false); }}><aside id="studio-help-drawer" className="integration-drawer help-guide-drawer" role="dialog" aria-modal="true" aria-labelledby="studio-help-title"><header className="integration-drawer-header"><div><span className="eyebrow"><span className="eyebrow-line"/> REALITY STUDIO</span><h2 id="studio-help-title">A guide to this workspace</h2><p>One saved engineering reality, carried through six focused modes.</p></div><button className="icon-button" onClick={() => setShowHelp(false)} aria-label="Close help and guides"><X size={16}/></button></header><div className="help-guide-list">{[
      ['01', 'Build', 'Define components, interfaces, and explicit model bindings. Visual proxies are not CAD geometry.'],
      ['02', 'Simulate', 'Choose a compatible saved model and review its inputs and scope before running it.'],
      ['03', 'Run', 'Execute a configured tool and inspect the saved status, inputs, and result.'],
      ['04', 'Understand', 'Inspect and replay recorded output. Movement is driven by saved samples.'],
      ['05', 'Improve', 'Compare evaluated branches or bounded searches only when the same supported model ran.'],
      ['06', 'Verify', 'Check accepted measurable requirements against compatible recorded evidence.'],
    ].map(([number, title, description]) => <article className="help-guide-row" key={number}><span>{number}</span><div><b>{title}</b><small>{description}</small></div></article>)}</div><footer className="integration-drawer-footer"><span>Available analyses depend on the saved model and configured tools.</span><button className="secondary-button" onClick={() => setShowHelp(false)}>Close</button></footer></aside></div>}
    {showNewProject && <NewProjectModal busy={busy} error={error} onClose={() => setShowNewProject(false)} onCreate={createProject} />}
    {showProjectSettings && <ProjectSettingsModal project={selectedProject} busy={busy} error={error} onClose={() => setShowProjectSettings(false)} onSave={saveProjectSettings} />}
    {showAddComponent && <AddComponentModal busy={busy} domain={selectedProject.payload.engineering_reality?.domain ?? selectedProject.payload.domain} onClose={() => setShowAddComponent(false)} onCreate={addComponent} />}
    {showTopology && <TopologyModal busy={busy} reality={selectedProject.payload.engineering_reality} onClose={() => setShowTopology(false)} onAddInterface={addInterface} onAddConnection={addConnection} onRemoveConnection={removeConnection} onRemoveInterface={removeInterface} />}
    {showRequirement && <RequirementModal busy={busy} onClose={() => setShowRequirement(false)} onCreate={addRequirement} />}
    {showEVConfig && <EVConfigurationModal busy={busy} existing={selectedProject.payload.ev_vehicle} onClose={() => setShowEVConfig(false)} onSave={saveEVConfiguration} />}
    {error && !showProjectSettings && <div className="toast-error"><span>{error}</span><button onClick={() => setError('')} aria-label="Dismiss"><X size={14} /></button></div>}
  </main>;
}

function Brand({ compact = false }: { compact?: boolean }) { return <div className={`brand ${compact ? 'compact' : ''}`}><span className="brand-symbol"><span /><span /><span /></span><span className="brand-word">HELQIX</span>{!compact && <span className="brand-subtitle">ENGINEERING REALITY</span>}</div>; }
function RuntimeFallback() { return <div className="runtime-placeholder" aria-label="Preparing visual viewport" />; }
function normalizeDomain(value: string): string {
  return value.trim().toLowerCase().replace(/[^a-z0-9._-]+/g, '_').replace(/^[^a-z]+/, 'g').slice(0, 80);
}
function supportsEVVehicleModel(project: Resource<ProjectPayload>): boolean {
  if (project.payload.ev_vehicle) return true;
  const domain = normalizeDomain(project.payload.engineering_reality?.domain ?? project.payload.domain ?? '');
  return /(^|_)(ev|electric_vehicle|vehicle|automotive)(_|$)/.test(domain);
}

function useModalFocus() {
  useEffect(() => {
    const dialogs = document.querySelectorAll<HTMLElement>('[role="dialog"][aria-modal="true"]');
    const dialog = dialogs.item(dialogs.length - 1);
    const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const getFocusable = () => Array.from(dialog?.querySelectorAll<HTMLElement>(
      'a[href],button:not([disabled]),input:not([disabled]),select:not([disabled]),textarea:not([disabled]),[tabindex]:not([tabindex="-1"])',
    ) ?? []).filter((element) => element.getAttribute('aria-hidden') !== 'true' && element.getClientRects().length > 0);
    const initial = getFocusable().find((element) => element.hasAttribute('autofocus')) ?? getFocusable()[0];
    if (initial) initial.focus();
    else dialog?.focus();

    const keepFocusInside = (event: KeyboardEvent) => {
      if (event.key !== 'Tab' || !dialog) return;
      const focusable = getFocusable();
      if (!focusable.length) { event.preventDefault(); dialog.focus(); return; }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && (document.activeElement === first || !dialog.contains(document.activeElement))) {
        event.preventDefault(); last.focus();
      } else if (!event.shiftKey && (document.activeElement === last || !dialog.contains(document.activeElement))) {
        event.preventDefault(); first.focus();
      }
    };
    document.addEventListener('keydown', keepFocusInside);
    return () => {
      document.removeEventListener('keydown', keepFocusInside);
      if (previousFocus?.isConnected) previousFocus.focus();
    };
  }, []);
}

function ProjectSettingsModal({ project, busy, error, onClose, onSave }: { project: Resource<ProjectPayload> | null; busy: boolean; error: string; onClose: () => void; onSave: (name: string, domain: string, description: string) => Promise<boolean> }) {
  useModalFocus();
  const [name, setName] = useState(project?.payload.name ?? '');
  const [domain, setDomain] = useState(project?.payload.domain ?? 'general');
  const [description, setDescription] = useState(project?.payload.description ?? '');
  if (!project) return null;
  return <div className="modal-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget && !busy) onClose(); }}>
    <form className="new-project-modal project-settings-modal" role="dialog" aria-modal="true" aria-labelledby="project-settings-title" tabIndex={-1} onSubmit={async (event) => { event.preventDefault(); if (!busy && await onSave(name, domain, description)) onClose(); }}>
      <div className="modal-head"><div><span className="panel-overline">PROJECT CONFIGURATION</span><h2 id="project-settings-title">Project settings</h2><p>Update the project identity while keeping its engineering model attached to the same saved revision history.</p></div><button type="button" className="icon-button" onClick={onClose} disabled={busy} aria-label="Close project settings"><X size={17}/></button></div>
      {error && <div className="form-error" role="alert">{error}</div>}
      <label>Project name<input autoFocus value={name} onChange={(event) => setName(event.target.value)} maxLength={160} required/></label>
      <label>Engineering domain<input value={domain} onChange={(event) => setDomain(event.target.value)} maxLength={120} required aria-describedby="project-domain-help"/><small id="project-domain-help">Used to select domain catalogs and review rules. Changing it does not convert existing component data.</small></label>
      <label>Description<textarea rows={4} value={description} onChange={(event) => setDescription(event.target.value)} maxLength={2000} placeholder="Purpose, scope, or project context"/><small>{description.length} / 2,000 characters</small></label>
      <div className="settings-record"><span>PROJECT ID</span><code>{project.id}</code><span>MODEL REVISION</span><b>{project.payload.engineering_reality?.revision ?? 'No engineering model'}</b><span>UNITS</span><b>{project.payload.engineering_reality?.unitsSystem ?? 'Not set'}</b></div>
      <div className="modal-foot"><span>Saving increments the project version and, when present, the engineering model revision.</span><button type="button" className="secondary-button" onClick={onClose} disabled={busy}>Cancel</button><button className="primary-button" disabled={busy || !name.trim() || !domain.trim()}>{busy ? 'Saving…' : 'Save settings'}<ArrowRight size={15}/></button></div>
    </form>
  </div>;
}

function WorkspaceUnavailable({ error, onRetry }: { error: string; onRetry: () => void }) {
  return <main className="loading-screen"><Brand /><div className="empty-projects gateway-unavailable" role="alert"><div className="empty-glyph"><Wrench size={22} /></div><h2>Workspace gateway unavailable</h2><p>{error || 'Start the local HELQIX gateway to open your workspace.'}</p><button className="primary-button" onClick={onRetry}>Retry connection <ArrowRight size={15} /></button></div></main>;
}

function ProjectHome({ projects, error, busy, showNewProject, onCreate, onCloseNewProject, onCreateProject, onRefresh, onOpen }: { projects: Resource<ProjectPayload>[]; error: string; busy: boolean; showNewProject: boolean; onCreate: () => void; onCloseNewProject: () => void; onCreateProject: (payload: ProjectPayload) => void; onRefresh: () => Promise<void>; onOpen: (p: Resource<ProjectPayload>) => void }) {
  const [filter, setFilter] = useState(''); const filtered = projects.filter((p) => `${p.payload.name} ${p.payload.domain ?? ''}`.toLowerCase().includes(filter.toLowerCase()));
  return <main className="home-page"><header className="home-topbar"><Brand compact /><div className="top-spacer" /><span className="welcome-user">LOCAL WORKSPACE</span></header><section className="home-content"><div className="home-intro"><div><div className="eyebrow"><span className="eyebrow-line" /> YOUR ENGINEERING WORKSPACE</div><h1>Build your next<br /><em>engineering reality.</em></h1><p>Open a project or describe a new product to begin.</p></div><button className="primary-button" onClick={onCreate}><Plus size={16} /> New project</button></div><div className="home-toolbar"><div><h2>Projects <span>{projects.length.toString().padStart(2, '0')}</span></h2><p>Your product realities and engineering studies</p></div><div className="home-filters"><div className="home-search"><Search size={15} /><input value={filter} onChange={(e) => setFilter(e.target.value)} placeholder="Find a project" /></div><button className="icon-button" title="Refresh" onClick={() => void onRefresh()}><Activity size={15} /></button></div></div>{error && <div className="form-error">{error}</div>}
    {filtered.length ? <div className="project-grid">{filtered.map((project, i) => <button className="project-card" key={project.id} onClick={() => onOpen(project)}><div className={`project-card-art art-${i % 3}`}><div className="art-orbit orbit-a" /><div className="art-orbit orbit-b" /><div className="art-cube"><span /><span /><span /></div><span className="art-tag">{project.payload.domain || 'ENGINEERING SYSTEM'}</span></div><div className="project-card-body"><div className="card-topline"><span>REV {String(project.version).padStart(2, '0')}</span><span className="project-active"><i /> SAVED</span></div><h3>{project.payload.name}</h3><p>{project.payload.description || 'Engineering reality workspace'}</p><div className="card-bottom"><span>Updated {new Date(project.updated_at).toLocaleDateString()}</span><ArrowRight size={15} /></div></div></button>)}</div> : <div className="empty-projects"><div className="empty-glyph"><FolderKanban size={23} /></div><h3>{projects.length ? 'No matching projects' : 'Start with an engineering question'}</h3><p>{projects.length ? 'Try another name or product type.' : 'Create a project, describe the product you want to build, then develop its engineering reality in one workspace.'}</p><button className="primary-button" onClick={onCreate}><Plus size={16} /> Create a project</button></div>}
    <div className="home-activity"><div className="activity-marker"><Sparkles size={15} /></div><div><b>A project is a living engineering model.</b><p>HELQIX keeps product structure, connected models, execution history, alternatives, and evidence together.</p></div><button onClick={onCreate}>Get started <ArrowRight size={14} /></button></div></section>
    {showNewProject && <NewProjectModal busy={busy} error={error} onClose={onCloseNewProject} onCreate={onCreateProject} />}
    {busy && <div className="modal-backdrop"><div className="loading-card"><span className="loading-orbit" /><p>Opening project…</p></div></div>}
  </main>;
}

function BuildTree({ project, selectedComponentId, onSelectComponent, onOpenTopology }: { project: Resource<ProjectPayload>; selectedComponentId: string | null; onSelectComponent: (id: string) => void; onOpenTopology: () => void }) { const reality = project.payload.engineering_reality; const components = reality?.components ?? []; return <div className="tree"><div className="tree-item selected"><ChevronDown size={13} /><span className="tree-cube"><Box size={14} /></span><span>{project.payload.name}</span><span className="tree-rev">v{project.version}</span></div><div className="tree-children"><TreeNode icon={Layers3} label="Components" count={String(components.length)} />{components.map((component) => <button key={component.id} type="button" className={`tree-node component-node ${selectedComponentId === component.id ? 'component-selected' : ''}`} onClick={() => onSelectComponent(component.id)}><Box size={13} /><span>{component.name}</span><small title={component.componentType}>{component.componentType}</small></button>)}<TreeNode icon={GitBranch} label="Interfaces & connections" count={`${reality?.interfaces.length ?? 0} / ${reality?.connections.length ?? 0}`} onClick={onOpenTopology} /><TreeNode icon={Database} label="Models & assumptions" count={String(reality?.modelBindings.length ?? 0)} /><TreeNode icon={FileCheck2} label="Requirements" count={String(reality?.requirements.length ?? 0)} /></div><div className="tree-section-label">REPRESENTATIONS</div><TreeNode icon={Box} label="3D / CAD" status="No model linked" quiet /><TreeNode icon={Wrench} label="Engineering tools" status="Check availability" quiet /></div>; }
function TreeNode({ icon: Icon, label, count, status, quiet = false, onClick }: { icon: typeof Box; label: string; count?: string; status?: string; quiet?: boolean; onClick?: () => void }) { return <button type="button" className={`tree-node ${quiet ? 'quiet' : ''}`} onClick={onClick}><Icon size={14} /><span>{label}</span>{count && <small>{count}</small>}{status && <small className="tree-status">{status}</small>}</button>; }
function ModeTree({ mode, project, tools }: { mode: Mode; project: Resource<ProjectPayload>; tools: Array<{ id: string; label: string; status: string; available: boolean }> }) { const map: Record<Mode, Array<[typeof Box, string, string?]>> = { Build: [], Simulate: [[Gauge, 'Analysis setup'], [Layers3, 'Model compatibility'], [Wrench, 'Available solvers']], Run: [[Activity, 'Execution queue'], [Database, 'Inputs & outputs']], Understand: [[Activity, 'Timeline'], [Layers3, 'Observations'], [GitBranch, 'Causal links']], Improve: [[GitBranch, 'Branches'], [Gauge, 'Search space'], [Layers3, 'Comparison']], Verify: [[FileCheck2, 'Requirements'], [ShieldCheck, 'Checks & evidence']] }; return <div className="tree"><div className="tree-item selected"><ChevronDown size={13} /><span className="tree-cube"><Box size={14} /></span><span>{project.payload.name}</span></div><div className="tree-children">{map[mode].map(([Icon, label, detail]) => <TreeNode key={label} icon={Icon} label={label} status={detail} quiet />)}</div><div className="tree-section-label">CONNECTED TOOLS</div>{tools.length ? tools.slice(0, 5).map((t) => <TreeNode key={t.id} icon={Wrench} label={t.label} status={t.available ? 'Available' : t.status} quiet />) : <div className="tree-empty">Tool status unavailable</div>}</div>; }

function ViewportControls({ viewPreset, onViewChange, showGrid, onShowGrid, showAxes, onShowAxes, onFit }: { viewPreset: ViewPreset; onViewChange: (view: ViewPreset) => void; showGrid: boolean; onShowGrid: (visible: boolean) => void; showAxes: boolean; onShowAxes: (visible: boolean) => void; onFit: () => void }) {
  const [openMenu, setOpenMenu] = useState<'layers' | 'view' | null>(null);
  const views: Array<{ value: ViewPreset; label: string }> = [
    { value: 'isometric', label: 'Isometric' },
    { value: 'perspective', label: 'Perspective' },
    { value: 'top', label: 'Top' },
    { value: 'front', label: 'Front' },
    { value: 'side', label: 'Side' },
  ];
  const label = views.find(({ value }) => value === viewPreset)?.label ?? 'Isometric';
  return <>
    <div className="viewport-menu-wrap">
      <button className="tool-button" aria-haspopup="menu" aria-expanded={openMenu === 'layers'} onClick={() => setOpenMenu(openMenu === 'layers' ? null : 'layers')}><Layers3 size={14} /> Layers <ChevronDown size={12} /></button>
      {openMenu === 'layers' && <div className="viewport-popover" role="menu"><span className="viewport-popover-title">VIEW LAYERS</span><label><input type="checkbox" checked={showGrid} onChange={(event) => onShowGrid(event.target.checked)} /> Reference grid</label><label><input type="checkbox" checked={showAxes} onChange={(event) => onShowAxes(event.target.checked)} /> World axes</label></div>}
    </div>
    <div className="viewport-menu-wrap">
      <button className="tool-button" aria-haspopup="menu" aria-expanded={openMenu === 'view'} onClick={() => setOpenMenu(openMenu === 'view' ? null : 'view')}><span className="view-cube">{viewPreset === 'isometric' ? 'ISO' : viewPreset.slice(0, 2).toUpperCase()}</span> {label} <ChevronDown size={12} /></button>
      {openMenu === 'view' && <div className="viewport-popover view-popover" role="menu"><span className="viewport-popover-title">CAMERA VIEW</span>{views.map(({ value, label: viewLabel }) => <button key={value} role="menuitemradio" aria-checked={viewPreset === value} className={viewPreset === value ? 'selected-view' : ''} onClick={() => { onViewChange(value); setOpenMenu(null); }}>{viewLabel}</button>)}</div>}
    </div>
    <button className="icon-button fit-view-button" title="Fit all components in view" aria-label="Fit all components in view" onClick={onFit}><Maximize2 size={15} /></button>
  </>;
}

function CanvasStage({ mode, project, run, playbackIndex, playbackPlaying, onPlaybackIndexChange, onPlaybackPlayingChange, requirements, records, compatibleRun, selectedComponentId, onSelectComponent, viewPreset, showGrid, showAxes, fitRevision, busy, tools, branches, searches, runs, sweepProgress, onRunSweep, onRunVehicle, onRunCell, onOpenRun, onGoBuild, onGoSimulate, onAddRequirement, onReview, onVerify, onExport }: { mode: Mode; project: Resource<ProjectPayload>; run: IntegrationRun | null; playbackIndex: number; playbackPlaying: boolean; onPlaybackIndexChange: (index: number) => void; onPlaybackPlayingChange: (playing: boolean) => void; requirements: EngineeringRequirement[]; records: VerificationRecord[]; compatibleRun: IntegrationRun | null; selectedComponentId: string | null; onSelectComponent: (id: string | null) => void; viewPreset: 'perspective' | 'top' | 'front' | 'side' | 'isometric'; showGrid: boolean; showAxes: boolean; fitRevision: number; busy: boolean; tools: ConnectedTool[]; branches: Resource<EngineeringBranchPayload>[]; searches: Resource<BoundedSearchPayload>[]; runs: IntegrationRun[]; sweepProgress: { index: number; total: number; currentA: number } | null; onRunSweep: (bindingId: string, currentsA: number[], minimumDurationH: number | null) => Promise<void>; onRunVehicle: () => Promise<void>; onRunCell: (bindingId?: string) => Promise<void>; onOpenRun: (run: IntegrationRun) => void; onGoBuild: () => void; onGoSimulate: () => void; onAddRequirement: () => void; onReview: (id: string, status: EngineeringRequirement['reviewStatus']) => void; onVerify: () => void; onExport: () => void }) {
  const content = useMemo(() => ({
    Build: ['A system begins here.', 'Add components, define their interfaces, and connect validated models. Your reality remains a versioned engineering source of truth.', 'No 3D or CAD representation linked'],
    Simulate: ['Choose what to analyze.', 'Select a compatible engineering model and define explicit inputs before execution. HELQIX will not infer missing physics.', 'No compatible model is configured'],
    Run: ['No execution in progress.', 'When a supported model runs, its state and outputs will appear here, linked to this project revision and saved inputs.', 'No runs recorded'],
    Understand: ['Behavior follows execution.', 'Inspect measured outputs, events, and state against the run timeline. Every visible trace comes from recorded evidence.', 'No observations recorded'],
    Improve: ['Explore a controlled alternative.', 'Create a branch from a saved baseline, declare the changes, and compare only compatible, completed analyses.', 'No evaluated branches'],
    Verify: ['Evidence starts with a claim.', 'Connect a requirement to an applicable run and an explicit check. A preview or saved scenario cannot establish verification.', 'No checks recorded'],
  } as Record<Mode, string[]>), []);
  const [title, description, empty] = content[mode];
  const chartSeries = Array.isArray(run?.result.series) ? run.result.series as Array<{ time_h: number; voltage_v: number; capacity_ah: number }> : null;
  const vehicleSeries = Array.isArray(run?.result.series) ? run.result.series as Array<{ time_s: number; speed_m_s: number; distance_m: number; pack_voltage_v: number; pack_current_a: number; pack_power_w: number }> : null;
  const runtimeEntities = project.payload.engineering_reality?.components ?? EMPTY_RUNTIME_ENTITIES;
  const renderRealityViewport = (authoritativeState?: import('./engineering/authoritativeState').AuthoritativeEngineeringState) => <Suspense fallback={<RuntimeFallback />}><RuntimeCanvas entities={runtimeEntities} reality={project.payload.engineering_reality} authoritativeState={authoritativeState} interfaces={project.payload.engineering_reality?.interfaces} connections={project.payload.engineering_reality?.connections} selectedEntityId={selectedComponentId} onSelectEntity={onSelectComponent} viewPreset={viewPreset} showGrid={showGrid} showAxes={showAxes} fitRevision={fitRevision} /></Suspense>;
  const realityViewport = renderRealityViewport();
  if (mode === 'Simulate') return <SimulationSetupBoard project={project} tools={tools} busy={busy} realityViewport={realityViewport} onRunVehicle={onRunVehicle} onRunCell={onRunCell} onGoBuild={onGoBuild} />;
  if (mode === 'Run') return <ExecutionBoard project={project} runs={runs} selectedRun={run} busy={busy} realityViewport={realityViewport} renderRealityViewport={renderRealityViewport} selectedComponentId={selectedComponentId} onSelectComponent={onSelectComponent} onConfigure={onGoSimulate} onOpenRun={onOpenRun} />;
  if (mode === 'Improve') return <ImproveBoard project={project} branches={branches} searches={searches} runs={runs} busy={busy} progress={sweepProgress} realityViewport={realityViewport} onRunSweep={onRunSweep} onGoBuild={onGoBuild} onGoSimulate={onGoSimulate} />;
  if (mode === 'Verify') return <VerifyBoard project={project} requirements={requirements} records={records} compatibleRun={compatibleRun} busy={busy} realityViewport={realityViewport} onAdd={onAddRequirement} onReview={onReview} onVerify={onVerify} onExport={onExport} />;
  if (mode === 'Understand' && run?.operation === 'simulate_ev_drive_cycle' && vehicleSeries?.length) return <EVRunView run={run} series={vehicleSeries} playbackIndex={playbackIndex} playbackPlaying={playbackPlaying} onPlaybackIndexChange={onPlaybackIndexChange} onPlaybackPlayingChange={onPlaybackPlayingChange} realityViewport={realityViewport} />;
  if (mode === 'Understand' && run?.result.status === 'failed') return <div className="canvas-stage stage-understand run-failure-stage">{realityViewport}<div className="stage-grid"/><section className="run-failure-panel"><div className="run-failure-mark"><Activity size={17}/></div><span className="stage-kicker">RECORDED TOOL ATTEMPT · {run.tool.toUpperCase()}</span><h1>Analysis did not complete</h1><p>The tool attempt and bounded output are recorded. HELQIX has no usable engineering result to display for this run.</p><dl><div><dt>Operation</dt><dd>{run.operation.replaceAll('_', ' ')}</dd></div><div><dt>Failure stage</dt><dd>{String(run.result.failure_stage ?? 'tool execution')}</dd></div><div><dt>Attempt ID</dt><dd>{run.id}</dd></div>{run.result.exit_code !== undefined && <div><dt>Exit code</dt><dd>{String(run.result.exit_code)}</dd></div>}</dl>{typeof run.result.error === 'string' && <p className="run-failure-error">{run.result.error}</p>}{(typeof run.result.stderr === 'string' && run.result.stderr || typeof run.result.stdout === 'string' && run.result.stdout) && <details className="run-failure-logs"><summary>Show tool output</summary>{typeof run.result.stderr === 'string' && run.result.stderr && <pre>{run.result.stderr}</pre>}{typeof run.result.stdout === 'string' && run.result.stdout && <pre>{run.result.stdout}</pre>}</details>}</section></div>;
  if (mode === 'Understand' && run && chartSeries?.length) return <div className="canvas-stage stage-understand">{realityViewport}<div className="stage-grid" /><div className="result-view"><div className="result-heading"><div><span className="stage-kicker">RECORDED SOLVER OUTPUT <i /> PYBAMM · SPM / CHEN2020</span><h1>Cell discharge behavior</h1><p>{String(run.result.component_name ?? 'Battery cell')} · run {run.id.slice(0, 8)}</p></div><span className="result-status"><i /> COMPLETED</span></div><ResultChart series={chartSeries} currentIndex={playbackIndex} /><div className="result-metrics"><Metric label="DISCHARGE DURATION" value={`${Number(run.result.duration_h).toFixed(2)} h`} /><Metric label="DISCHARGED CAPACITY" value={`${Number(run.result.discharged_capacity_ah).toFixed(2)} Ah`} /><Metric label="FINAL VOLTAGE" value={`${Number(run.result.final_voltage_v).toFixed(2)} V`} /></div><div className="result-scope"><ShieldCheck size={14} /><span>{String(run.result.applicability ?? 'Results apply only to this saved single-cell model and its stated conditions.')}</span></div></div><div className="stage-fidelity"><span className="fidelity-icon"><Activity size={13} /></span><div><b>Recorded model output</b><small>Source: {run.provenance}</small></div></div><div className="stage-corner stage-corner-top">{project.payload.domain || 'ENGINEERING REALITY'} <span>·</span> RUN {run.id.slice(0, 8)}</div></div>;
  return <div className={`canvas-stage stage-${mode.toLowerCase()}`}>{realityViewport}<div className="stage-grid" /><div className="stage-glow" /><div className={`stage-center ${runtimeEntities.length ? 'stage-with-components' : ''}`}><div className="stage-graphic"><span className="graphic-ring ring-one" /><span className="graphic-ring ring-two" /><span className="graphic-ring ring-three" /><div className="graphic-core"><span className="core-mark"><span /><span /><span /></span></div><span className="graphic-point point-a" /><span className="graphic-point point-b" /><span className="graphic-point point-c" /></div><div className="stage-copy"><span className="stage-kicker">{project.payload.domain || 'PROJECT REALITY'} <i /> {mode.toUpperCase()}</span><h1>{title}</h1><p>{description}</p><div className="stage-empty-tag"><span /> {runtimeEntities.length ? `${runtimeEntities.length} typed component${runtimeEntities.length === 1 ? '' : 's'} shown as visual proxies` : empty}</div></div></div>{mode === 'Build' && runtimeEntities.length > 0 && <div className="viewport-help">DRAG TO ORBIT <i /> SCROLL TO ZOOM <i /> CLICK A PART TO INSPECT</div>}{mode === 'Build' && (project.payload.engineering_reality?.connections.length ?? 0) > 0 && <div className="topology-legend"><span className="topology-legend-line"/> Declared interface connection</div>}<div className="stage-scale"><span>0</span><i /><i /><i /><i /><span>1 m</span></div><div className="stage-fidelity"><span className="fidelity-icon"><Box size={13} /></span><div><b>{runtimeEntities.length ? 'Provisional component geometry' : 'Representation not linked'}</b><small>{runtimeEntities.length ? 'Visual reference only · CAD not linked' : 'Visual fidelity: none'}</small></div><button type="button" aria-label="Inspect the first component" title={runtimeEntities.length ? 'Select the first component in the context inspector' : 'No components are available to inspect'} disabled={!runtimeEntities.length} onClick={() => onSelectComponent(runtimeEntities[0]?.id ?? null)}><ArrowRight size={14} /></button></div><div className="stage-corner stage-corner-top">{project.payload.domain ? project.payload.domain.toUpperCase() : 'ENGINEERING REALITY'} <span>·</span> SI</div><div className="stage-corner stage-corner-bottom">WORLD ORIGIN <span>0, 0, 0</span></div></div>;
}

function SimulationSetupBoard({ project, tools, busy, realityViewport, onRunVehicle, onRunCell, onGoBuild }: { project: Resource<ProjectPayload>; tools: ConnectedTool[]; busy: boolean; realityViewport: ReactNode; onRunVehicle: () => Promise<void>; onRunCell: (bindingId?: string) => Promise<void>; onGoBuild: () => void }) {
  const reality = project.payload.engineering_reality;
  const solver = tools.find((tool) => tool.id === 'pybamm');
  const cellBindings = (reality?.modelBindings ?? []).filter((binding) => {
    const component = reality?.components.find((item) => item.id === binding.scopeComponentId);
    return binding.modelId === 'pybamm.lithium-ion.spm' && binding.modelVersion === 'Chen2020' && Boolean(component && ['battery_cell', 'lithium_ion_cell'].includes(component.componentType.toLowerCase()));
  });
  const vehicle = project.payload.ev_vehicle;
  const canRunVehicle = Boolean(vehicle && solver?.available);
  const canRunCell = Boolean(cellBindings.length && solver?.available);
  return <div className="canvas-stage stage-simulate">{realityViewport}<div className="stage-grid"/><section className="simulation-board">
    <header className="simulation-header"><div><span className="stage-kicker">MODEL SETUP <i/> {project.payload.domain || 'ENGINEERING REALITY'}</span><h1>Choose an analysis</h1><p>Select a model already bound to this saved project reality. Inputs and scope are shown before each run.</p></div><span className={`tool-status ${solver?.available ? 'tool-ready' : ''}`}><i/>{solver?.available ? 'PYBAMM READY' : solver?.status === 'checking' ? 'CHECKING SOLVER' : 'PYBAMM UNAVAILABLE'}</span></header>
    {!solver?.available && <div className="simulation-notice"><Activity size={15}/><span>{solver?.note || 'The PyBaMM integration is unavailable. Configure the local solver integration to enable analyses.'}</span></div>}
    <div className="simulation-card-grid">
      {vehicle && <article className="simulation-model-card"><div className="simulation-card-head"><span className="simulation-model-icon"><CarFront size={17}/></span><span className="simulation-tag">VEHICLE DRIVE CYCLE</span></div><h2>{vehicle.vehicleName}</h2><p>PyBaMM SPM · Chen2020 cell behavior embedded in a prescribed longitudinal drive-cycle estimate.</p><div className="simulation-input-grid"><Metric label="VEHICLE MASS" value={`${vehicle.vehicleMassKg.toLocaleString()} kg`}/><Metric label="PACK TOPOLOGY" value={`${vehicle.batteryPack.seriesCells}S · ${vehicle.batteryPack.parallelCells}P`}/><Metric label="DRIVE-CYCLE SAMPLES" value={`${vehicle.driveCycle.length}`}/><Metric label="INITIAL SOC" value={`${Math.round(vehicle.batteryPack.initialSoc * 100)}%`}/></div><div className="simulation-card-scope"><ShieldCheck size={13}/><span>Identical-cell pack scaling and prescribed longitudinal demand. No pack electrical network, thermal coupling, or validated whole-vehicle dynamics.</span></div><button className="primary-button simulation-run" disabled={!canRunVehicle || busy} onClick={() => void onRunVehicle()}>{busy ? <span className="button-spinner"/> : <Play size={14}/>} Run vehicle analysis <ArrowRight size={14}/></button></article>}
      {cellBindings.map((binding) => {
        const component = reality?.components.find((item) => item.id === binding.scopeComponentId);
        const current = component?.properties.find((property) => property.name === 'current_a');
        const cutoff = component?.properties.find((property) => property.name === 'cutoff_voltage_v');
        return <article className="simulation-model-card" key={binding.id}><div className="simulation-card-head"><span className="simulation-model-icon"><Database size={16}/></span><span className="simulation-tag">SINGLE-CELL DISCHARGE</span></div><h2>{component?.name ?? binding.scopeComponentId}</h2><p>PyBaMM lithium-ion single-particle model using the Chen2020 parameter set.</p><div className="simulation-input-grid"><Metric label="DISCHARGE CURRENT" value={`${current?.value ?? '—'} ${current?.unit ?? 'A'}`}/><Metric label="CUTOFF VOLTAGE" value={`${cutoff?.value ?? '—'} ${cutoff?.unit ?? 'V'}`}/><Metric label="PARAMETER SET" value="Chen2020"/><Metric label="MODEL" value="SPM"/></div><div className="simulation-card-scope"><ShieldCheck size={13}/><span>{binding.declaredFidelity || 'Applies only to this saved single-cell model; no pack or vehicle coupling.'}</span></div><button className="primary-button simulation-run" disabled={!canRunCell || busy} onClick={() => void onRunCell(binding.id)}>{busy ? <span className="button-spinner"/> : <Play size={14}/>} Run cell analysis <ArrowRight size={14}/></button></article>;
      })}
      {!vehicle && !cellBindings.length && <div className="simulation-empty"><div className="empty-glyph"><Gauge size={21}/></div><h2>No supported model is configured</h2><p>Add a battery cell with explicit discharge inputs or configure the current EV drive-cycle model in Build. HELQIX will not invent missing model inputs.</p><button className="primary-button" onClick={onGoBuild}><Box size={14}/> Open reality builder</button></div>}
    </div>
    <footer className="simulation-footer"><span><ShieldCheck size={13}/> Every run records its model, inputs, result, and project revision.</span><span>Unavailable integrations stay disabled until their health check passes.</span></footer>
  </section></div>;
}

function ExecutionBoard({ project, runs, selectedRun, busy, realityViewport, renderRealityViewport, selectedComponentId, onSelectComponent, onConfigure, onOpenRun }: { project: Resource<ProjectPayload>; runs: IntegrationRun[]; selectedRun: IntegrationRun | null; busy: boolean; realityViewport: ReactNode; renderRealityViewport: (state?: import('./engineering/authoritativeState').AuthoritativeEngineeringState) => ReactNode; selectedComponentId: string | null; onSelectComponent: (id: string | null) => void; onConfigure: () => void; onOpenRun: (run: IntegrationRun) => void }) {
  const [showScenarios, setShowScenarios] = useState(false);
  const recentRuns = [...runs].sort((a, b) => b.created_at - a.created_at);
  if (showScenarios) return <Suspense fallback={<RuntimeFallback />}><ScenarioExecutionBoard project={project} realityViewport={renderRealityViewport} selectedComponentId={selectedComponentId} onSelectComponent={onSelectComponent} onBackToRuns={() => setShowScenarios(false)} /></Suspense>;
  return <div className="canvas-stage stage-run">{realityViewport}<div className="stage-grid"/><section className="execution-board">
    <header className="simulation-header"><div><span className="stage-kicker">EXECUTION RECORD <i/> REVISION-LINKED</span><h1>Model runs</h1><p>Completed and failed tool attempts for this project, with the recorded output and provenance.</p></div><div className="execution-actions"><button className="secondary-button" onClick={() => setShowScenarios(true)}><GitBranch size={14}/> State scenarios</button><button className="secondary-button" disabled={busy} onClick={onConfigure}><Gauge size={14}/> Configure analysis</button></div></header>
    {recentRuns.length ? <div className="execution-list">{recentRuns.map((item) => <article className={`execution-row ${selectedRun?.id === item.id ? 'execution-selected' : ''}`} key={item.id}><span className="execution-mark"><Activity size={16}/></span><div className="execution-main"><b>{item.operation === 'simulate_ev_drive_cycle' ? 'Vehicle drive-cycle analysis' : item.operation === 'simulate_bound_cell' ? 'Battery-cell discharge' : item.operation.replaceAll('_', ' ')}</b><small>{item.tool} · {item.provenance} · {new Date(item.created_at * 1000).toLocaleString()}</small></div><span className={`execution-state ${item.result.status === 'failed' ? 'state-failed' : ''}`}><i/> {item.result.status === 'failed' ? 'FAILED' : 'RECORDED'}</span><button className="secondary-button" onClick={() => onOpenRun(item)}>{item.result.status === 'failed' ? 'View details' : 'View result'} <ArrowRight size={13}/></button></article>)}</div> : <div className="execution-empty"><div className="empty-glyph"><Activity size={21}/></div><h2>No model runs yet</h2><p>Choose a supported analysis, check its declared inputs, and run it. The resulting record will appear here.</p><button className="primary-button" disabled={busy} onClick={onConfigure}><Gauge size={14}/> Choose an analysis</button></div>}
    <footer className="simulation-footer"><span><Database size={13}/> {recentRuns.length} recorded run{recentRuns.length === 1 ? '' : 's'}</span><span>Run records are scoped to this project.</span></footer>
  </section></div>;
}

function ImproveBoard({ project, branches, searches, runs, busy, progress, realityViewport, onRunSweep, onGoBuild, onGoSimulate }: { project: Resource<ProjectPayload>; branches: Resource<EngineeringBranchPayload>[]; searches: Resource<BoundedSearchPayload>[]; runs: IntegrationRun[]; busy: boolean; progress: { index: number; total: number; currentA: number } | null; realityViewport: ReactNode; onRunSweep: (bindingId: string, currentsA: number[], minimumDurationH: number | null) => Promise<void>; onGoBuild: () => void; onGoSimulate: () => void }) {
  const reality = project.payload.engineering_reality;
  const cellBindings = (reality?.modelBindings ?? []).filter((binding) => {
    const component = reality?.components.find((item) => item.id === binding.scopeComponentId);
    return binding.modelId === 'pybamm.lithium-ion.spm' && binding.modelVersion === 'Chen2020' && !!component && ['battery_cell', 'lithium_ion_cell'].includes(component.componentType.toLowerCase());
  });
  const [bindingId, setBindingId] = useState(cellBindings[0]?.id ?? '');
  const [currentsText, setCurrentsText] = useState('0.5, 1, 2');
  const [minimumDurationText, setMinimumDurationText] = useState('');
  const [formError, setFormError] = useState('');
  const hasEVContext = !!project.payload.ev_vehicle || runs.some((run) => run.operation === 'simulate_ev_drive_cycle');
  const [analysisView, setAnalysisView] = useState<'cell' | 'vehicle'>(() => hasEVContext ? 'vehicle' : 'cell');
  useEffect(() => {
    if (!cellBindings.some((item) => item.id === bindingId)) setBindingId(cellBindings[0]?.id ?? '');
  }, [project.id, project.version, bindingId]);
  useEffect(() => setAnalysisView(hasEVContext ? 'vehicle' : 'cell'), [project.id, hasEVContext]);

  const selectedBinding = cellBindings.find((item) => item.id === bindingId);
  const component = selectedBinding ? reality?.components.find((item) => item.id === selectedBinding.scopeComponentId) : null;
  const latestSearch = searches[0]?.payload;
  const latestRuns = new Map(runs.map((item) => [item.id, item]));
  const baselineCurrent = component?.properties.find((property) => property.name === 'current_a');
  const currentValue = (value: string) => {
    const fields = value.split(',').map((part) => part.trim());
    if (fields.some((part) => !part)) return null;
    const parsed = fields.map(Number);
    if (parsed.length < 2 || parsed.length > 5 || parsed.some((item) => !Number.isFinite(item) || item < 0.05 || item > 100) || new Set(parsed).size !== parsed.length) return null;
    return parsed;
  };
  const submit = (event: React.FormEvent) => {
    event.preventDefault();
    setFormError('');
    if (!selectedBinding) { setFormError('Select a saved PyBaMM SPM / Chen2020 cell binding.'); return; }
    const currents = currentValue(currentsText);
    if (!currents) { setFormError('Enter 2–5 unique current values from 0.05 to 100 A, separated by commas.'); return; }
    const minimum = minimumDurationText.trim() ? Number(minimumDurationText) : null;
    if (minimum !== null && (!Number.isFinite(minimum) || minimum < 0)) { setFormError('Minimum duration must be a finite value of zero hours or greater.'); return; }
    void onRunSweep(selectedBinding.id, currents, minimum);
  };

  if (hasEVContext && (!cellBindings.length || analysisView === 'vehicle')) return <EVRunComparisonBoard project={project} runs={runs} realityViewport={realityViewport} onGoBuild={onGoBuild} onGoSimulate={onGoSimulate} onGoCellSweep={cellBindings.length ? () => setAnalysisView('cell') : undefined} />;
  if (!cellBindings.length) return <div className="canvas-stage stage-improve">{realityViewport}<div className="stage-grid"/><section className="improve-empty"><div className="empty-glyph"><GitBranch size={22}/></div><span className="stage-kicker">CONTROLLED ALTERNATIVES <i/> MODEL REQUIRED</span><h1>No compatible model runs yet</h1><p>Improve can compare supported, recorded model runs. Configure an explicit engineering model in Build, then execute it in Simulate.</p><button className="primary-button" onClick={onGoBuild}><Box size={14}/> Open reality builder</button><small>Only completed, compatible results can be compared.</small></section></div>;

  return <div className="canvas-stage stage-improve">{realityViewport}<div className="stage-grid"/><section className="improve-board">
    <header className="improve-header"><div><span className="stage-kicker">CONTROLLED ALTERNATIVES <i/> REAL MODEL RUNS</span><h1>Compare cell operating points</h1><p>Each candidate is saved as a revision-bound branch, run through PyBaMM, and retained with its own input and output provenance.</p></div><div className="ev-compare-header-actions"><span className="improve-scope">CELL MODEL ONLY</span>{hasEVContext && <button className="secondary-button" type="button" onClick={() => setAnalysisView('vehicle')}><CarFront size={13}/> Vehicle runs</button>}</div></header>
    <div className="improve-scope-note"><ShieldCheck size={14}/><span>This finite sweep varies a single cell's discharge current. It does not evaluate an EV pack, whole vehicle, thermal behavior, or an optimal product design.</span></div>
    <div className="improve-layout"><form className="sweep-config" onSubmit={submit}><div className="sweep-config-heading"><span>SEARCH DEFINITION</span><small>2–5 candidates · serial execution</small></div>
      <label>Bound cell model<select value={bindingId} onChange={(event) => setBindingId(event.target.value)}>{cellBindings.map((binding) => { const bound = reality?.components.find((item) => item.id === binding.scopeComponentId); return <option key={binding.id} value={binding.id}>{bound?.name ?? binding.scopeComponentId}</option>; })}</select></label>
      <div className="sweep-bound-model"><b>{component?.name ?? 'Battery cell'}</b><span>{selectedBinding?.modelId} · {selectedBinding?.modelVersion}</span><small>Saved baseline current: {typeof baselineCurrent?.value === 'number' ? `${baselineCurrent.value} ${baselineCurrent.unit ?? 'A'}` : 'not declared'}</small></div>
      <label>Candidate discharge currents <small>COMMA-SEPARATED · A</small><input inputMode="decimal" value={currentsText} onChange={(event) => setCurrentsText(event.target.value)} placeholder="0.5, 1, 2"/><small>Choose 2–5 unique values, 0.05–100 A. Include the baseline to compare it directly.</small></label>
      <label>Minimum discharge duration <small>OPTIONAL · h</small><input type="number" min="0" step="any" value={minimumDurationText} onChange={(event) => setMinimumDurationText(event.target.value)} placeholder="No minimum"/><small>Only completed candidates meeting this threshold can be selected as best evaluated.</small></label>
      {formError && <div className="form-error">{formError}</div>}
      <button className="primary-button sweep-submit" disabled={busy || !selectedBinding}>{busy ? <><span className="button-spinner"/> {progress ? `Candidate ${progress.index}/${progress.total} · ${progress.currentA} A` : 'Preparing sweep…'}</> : <><Play size={13}/> Run bounded sweep</>}</button>
      <small className="sweep-duration-note">Runs are sequential; the local solver can take several minutes per candidate.</small>
    </form>

    <div className="sweep-results"><div className="sweep-results-head"><div><span>RECORDED COMPARISON</span><small>{latestSearch ? `Reality revision ${latestSearch.base_reality_revision} · ${latestSearch.completed_count}/${latestSearch.candidate_count} runs complete` : 'Run a sweep to create a recorded comparison'}</small></div>{latestSearch && <span className={`search-status status-${latestSearch.status}`}>{latestSearch.status}</span>}</div>
      {latestSearch ? <><div className="sweep-chart-card"><div className="sweep-chart-title"><span>DISCHARGED CAPACITY BY CURRENT</span><small>PyBaMM output · Chen2020</small></div><SweepChart candidates={latestSearch.candidates} bestCurrentA={latestSearch.best_evaluated_candidate_a}/></div>
        <div className="candidate-table-wrap"><table className="candidate-table"><thead><tr><th>CURRENT</th><th>CAPACITY</th><th>DURATION</th><th>FINAL V</th><th>RESULT</th></tr></thead><tbody>{latestSearch.candidates.map((candidate) => { const best = candidate.current_a === latestSearch.best_evaluated_candidate_a; const sourceRun = candidate.run_id ? latestRuns.get(candidate.run_id) : undefined; return <tr key={`${candidate.current_a}-${candidate.run_id ?? candidate.branch_id}`} className={best ? 'candidate-best' : ''}><td>{candidate.current_a} A</td><td>{candidate.state === 'completed' ? `${candidate.discharged_capacity_ah?.toFixed(3)} Ah` : '—'}</td><td>{candidate.state === 'completed' ? `${candidate.duration_h?.toFixed(3)} h` : '—'}</td><td>{candidate.state === 'completed' ? `${candidate.final_voltage_v?.toFixed(3)} V` : '—'}</td><td><span className={`candidate-state ${candidate.state}`}>{best ? 'Best evaluated' : candidate.state === 'completed' ? 'Completed' : 'Failed'}</span>{candidate.state === 'failed' && <small title={candidate.error ?? ''}>{candidate.error}</small>}{sourceRun && <small>Run {sourceRun.id.slice(0, 8)}</small>}</td></tr>; })}</tbody></table></div>
        <div className="sweep-objective"><Gauge size={14}/><span>Objective: maximize discharged capacity{latestSearch.constraint.value_h === null ? '' : `, subject to at least ${latestSearch.constraint.value_h} h`}. This is the best among eligible completed points, not a global optimum.</span></div>
      </> : <div className="sweep-no-result"><div className="empty-glyph"><Activity size={19}/></div><h3>No evaluated candidates</h3><p>Define a small, explicit current sweep. HELQIX will record each branch and its actual solver result here.</p></div>}
    </div></div>
    <div className="improve-history"><div className="improve-history-column"><span>RECENT BRANCHES <small>{branches.length}</small></span>{branches.length ? branches.slice(0, 5).map((branch) => <div className="improve-history-row" key={branch.id}><GitBranch size={13}/><div><b>{branch.payload.name}</b><small>Reality rev {branch.payload.base_reality_revision} · {branch.payload.changes[0]?.property_name.replaceAll('_', ' ')} {String(branch.payload.changes[0]?.value)} {branch.payload.changes[0]?.unit}</small></div></div>) : <small>No alternative branches saved.</small>}</div><div className="improve-history-column"><span>SWEEP HISTORY <small>{searches.length}</small></span>{searches.length ? searches.slice(0, 5).map((search) => <div className="improve-history-row" key={search.id}><Activity size={13}/><div><b>{search.payload.name}</b><small>{search.payload.completed_count}/{search.payload.candidate_count} completed · {search.payload.best_evaluated_candidate_a === null ? 'no eligible candidate' : `best evaluated ${search.payload.best_evaluated_candidate_a} A`}</small></div><span className={`search-status status-${search.payload.status}`}>{search.payload.status}</span></div>) : <small>No persisted sweeps yet.</small>}</div></div>
  </section></div>;
}

type EVRecordedSample = { time_s: number; speed_m_s: number; distance_m: number; pack_voltage_v: number };
type RecordedEVRun = { run: IntegrationRun; config: EVBatteryVehicleConfiguration; series: EVRecordedSample[] };

function isEVConfiguration(value: unknown): value is EVBatteryVehicleConfiguration {
  if (!value || typeof value !== 'object') return false;
  const config = value as Partial<EVBatteryVehicleConfiguration>;
  return config.schema === 'helqix.ev-battery-vehicle.v1'
    && typeof config.modelId === 'string' && typeof config.parameterSet === 'string'
    && Array.isArray(config.driveCycle) && !!config.batteryPack
    && typeof config.vehicleMassKg === 'number';
}

function isEVRecordedSeries(value: unknown): value is EVRecordedSample[] {
  return Array.isArray(value) && value.length >= 2 && value.every((sample) => !!sample && typeof sample === 'object'
    && Number.isFinite((sample as EVRecordedSample).time_s)
    && Number.isFinite((sample as EVRecordedSample).speed_m_s)
    && Number.isFinite((sample as EVRecordedSample).distance_m)
    && Number.isFinite((sample as EVRecordedSample).pack_voltage_v));
}

function EVRunComparisonBoard({ project, runs, realityViewport, onGoBuild, onGoSimulate, onGoCellSweep }: { project: Resource<ProjectPayload>; runs: IntegrationRun[]; realityViewport: ReactNode; onGoBuild: () => void; onGoSimulate: () => void; onGoCellSweep?: () => void }) {
  const recordedRuns: RecordedEVRun[] = runs.flatMap((run) => {
    const config = run.inputs.ev_vehicle_configuration;
    const series = run.result.series;
    if (run.operation !== 'simulate_ev_drive_cycle' || run.result.status !== 'completed'
      || typeof run.result.model_input_sha256 !== 'string' || !isEVConfiguration(config) || !isEVRecordedSeries(series)) return [];
    return [{ run, config, series }];
  }).sort((left, right) => right.run.created_at - left.run.created_at);
  const groups = new Map<string, RecordedEVRun[]>();
  for (const item of recordedRuns) {
    const signature = JSON.stringify({
      tool: item.run.tool, provenance: item.run.provenance,
      modelId: item.config.modelId, parameterSet: item.config.parameterSet,
      driveCycle: item.config.driveCycle,
    });
    groups.set(signature, [...(groups.get(signature) ?? []), item]);
  }
  const comparableRuns = [...groups.values()].sort((left, right) => right.length - left.length || right[0].run.created_at - left[0].run.created_at)[0] ?? [];
  const [baselineId, setBaselineId] = useState('');
  const [candidateId, setCandidateId] = useState('');
  const baseline = comparableRuns.find((item) => item.run.id === baselineId) ?? comparableRuns[1] ?? comparableRuns[0] ?? null;
  const candidate = comparableRuns.find((item) => item.run.id === candidateId && item.run.id !== baseline?.run.id)
    ?? comparableRuns.find((item) => item.run.id !== baseline?.run.id) ?? null;
  const comparablePair = baseline && candidate && baseline.run.id !== candidate.run.id;
  const metrics: Array<{ key: 'distance_m' | 'peak_pack_power_w' | 'minimum_cell_voltage_v' | 'final_pack_voltage_v'; label: string; unit: string; digits: number }> = [
    { key: 'distance_m', label: 'Recorded travel distance', unit: 'm', digits: 1 },
    { key: 'peak_pack_power_w', label: 'Peak pack power', unit: 'kW', digits: 2 },
    { key: 'minimum_cell_voltage_v', label: 'Minimum cell voltage', unit: 'V', digits: 3 },
    { key: 'final_pack_voltage_v', label: 'Final pack voltage', unit: 'V', digits: 1 },
  ];
  const massDelta = comparablePair ? candidate.config.vehicleMassKg - baseline.config.vehicleMassKg : 0;
  return <div className="canvas-stage stage-improve">{realityViewport}<div className="stage-grid"/><section className="ev-compare-board">
    <header className="improve-header"><div><span className="stage-kicker">CONTROLLED ALTERNATIVES <i/> RECORDED VEHICLE RUNS</span><h1>Compare vehicle configurations</h1><p>Compare compatible PyBaMM runs from the same prescribed speed trace. Each run retains its exact validated inputs and model digest.</p></div><div className="ev-compare-header-actions"><span className="improve-scope">EV DRIVE-CYCLE MODEL</span>{onGoCellSweep && <button className="secondary-button" type="button" onClick={onGoCellSweep}><Activity size={13}/> Cell search</button>}</div></header>
    <div className="ev-compare-scope"><ShieldCheck size={14}/><span>Vehicle mass changes road load and battery demand. Speed is prescribed by the drive cycle, so this comparison does not establish free-running acceleration or vehicle performance.</span></div>
    {comparablePair ? <>
      <div className="ev-compare-selectors">
        <label>BASELINE<select value={baseline.run.id} onChange={(event) => setBaselineId(event.target.value)}>{comparableRuns.filter((item) => item.run.id !== candidate.run.id).map((item) => <option key={item.run.id} value={item.run.id}>{evRunLabel(item)}</option>)}</select><small>Run {baseline.run.id.slice(0, 8)} · {new Date(baseline.run.created_at * 1000).toLocaleString()}</small></label>
        <ArrowRight size={16}/>
        <label>CONFIGURATION B<select value={candidate.run.id} onChange={(event) => setCandidateId(event.target.value)}>{comparableRuns.filter((item) => item.run.id !== baseline.run.id).map((item) => <option key={item.run.id} value={item.run.id}>{evRunLabel(item)}</option>)}</select><small>Run {candidate.run.id.slice(0, 8)} · {new Date(candidate.run.created_at * 1000).toLocaleString()}</small></label>
      </div>
      <div className="ev-compare-compatibility"><i/> Same drive-cycle points · {baseline.config.modelId} · {baseline.config.parameterSet} · recorded tool: {baseline.run.provenance}</div>
      <div className="ev-compare-chart-grid">
        <EVCompareChart title="VEHICLE SPEED" unit="m/s" baseline={baseline} candidate={candidate} field="speed_m_s"/>
        <EVCompareChart title="PACK VOLTAGE" unit="V" baseline={baseline} candidate={candidate} field="pack_voltage_v"/>
      </div>
      <div className="ev-compare-metrics"><div className="ev-compare-metric-head"><span>RECORDED OUTCOMES</span><small>Measured output only · no winner inferred</small></div>{metrics.map((metric) => {
        const left = Number(baseline.run.result[metric.key]); const right = Number(candidate.run.result[metric.key]);
        const toValue = (value: number) => `${metric.key === 'peak_pack_power_w' ? (value / 1000).toFixed(metric.digits) : value.toFixed(metric.digits)} ${metric.unit}`;
        const delta = right - left;
        return <div className="ev-compare-metric" key={metric.key}><span>{metric.label}</span><b>{toValue(left)} <i>→</i> {toValue(right)}</b><small>{delta > 0 ? '+' : ''}{metric.key === 'peak_pack_power_w' ? (delta / 1000).toFixed(metric.digits) : delta.toFixed(metric.digits)} {metric.unit} · {metric.key === 'peak_pack_power_w' ? 'B − baseline' : 'B − baseline'}</small></div>;
      })}</div>
      <footer className="ev-compare-footer"><span>Mass: {baseline.config.vehicleMassKg.toLocaleString()} kg → {candidate.config.vehicleMassKg.toLocaleString()} kg ({massDelta > 0 ? '+' : ''}{massDelta.toLocaleString()} kg)</span><span>Model input digests: {String(baseline.run.result.model_input_sha256).slice(0, 12)} · {String(candidate.run.result.model_input_sha256).slice(0, 12)}</span></footer>
    </> : <div className="ev-compare-empty"><div className="empty-glyph"><GitBranch size={20}/></div><h2>{recordedRuns.length ? 'Run a second compatible configuration' : 'No reproducible vehicle runs yet'}</h2><p>{recordedRuns.length ? 'To compare alternatives, edit the explicit vehicle or pack inputs in Build, then run the same drive cycle again. Runs with different drive-cycle points or model settings are kept separate.' : 'Save the explicit vehicle, pack, and drive-cycle inputs, then execute them. New runs will retain their exact configuration for later comparison.'}</p><div><button className="secondary-button" onClick={onGoBuild}><Box size={14}/> Edit configuration</button><button className="primary-button" onClick={onGoSimulate}><Gauge size={14}/> Open simulation</button></div>{recordedRuns.length < runs.filter((run) => run.operation === 'simulate_ev_drive_cycle' && run.result.status === 'completed').length && <small>Older runs without captured configuration are not eligible for reproducible comparison.</small>}</div>}
    <div className="ev-compare-revision">Project {project.payload.name} · v{project.version} · {comparableRuns.length} compatible recorded run{comparableRuns.length === 1 ? '' : 's'}</div>
  </section></div>;
}

function evRunLabel(item: RecordedEVRun): string {
  return `${item.config.vehicleName} · ${item.config.vehicleMassKg.toLocaleString()} kg · ${item.config.batteryPack.seriesCells}s${item.config.batteryPack.parallelCells}p`;
}

function EVCompareChart({ title, unit, baseline, candidate, field }: { title: string; unit: string; baseline: RecordedEVRun; candidate: RecordedEVRun; field: 'speed_m_s' | 'pack_voltage_v' }) {
  const width = 480; const height = 150; const pad = { left: 42, right: 12, top: 12, bottom: 25 };
  const samples = [...baseline.series, ...candidate.series];
  const maxTime = Math.max(1, ...samples.map((sample) => sample.time_s));
  const values = samples.map((sample) => sample[field]);
  const rawMin = Math.min(...values); const rawMax = Math.max(...values); const spread = Math.max(rawMax - rawMin, 0.1);
  const minY = rawMin - spread * 0.08; const maxY = rawMax + spread * 0.08;
  const x = (time: number) => pad.left + time / maxTime * (width - pad.left - pad.right);
  const y = (value: number) => height - pad.bottom - (value - minY) / (maxY - minY) * (height - pad.top - pad.bottom);
  const makePath = (series: EVRecordedSample[]) => series.map((sample, index) => `${index ? 'L' : 'M'}${x(sample.time_s).toFixed(1)},${y(sample[field]).toFixed(1)}`).join(' ');
  return <section className="ev-compare-chart"><div><span>{title}</span><small>{unit} · recorded solver samples</small></div><svg viewBox={`0 0 ${width} ${height}`} role="img" aria-label={`${title.toLowerCase()} comparison across two recorded vehicle runs`}><line x1={pad.left} x2={width - pad.right} y1={height - pad.bottom} y2={height - pad.bottom} stroke="#484b3b"/><line x1={pad.left} x2={width - pad.right} y1={pad.top} y2={pad.top} stroke="#303229"/><text x={pad.left - 7} y={pad.top + 4} textAnchor="end">{maxY.toFixed(1)}</text><text x={pad.left - 7} y={height - pad.bottom + 3} textAnchor="end">{minY.toFixed(1)}</text><path d={makePath(baseline.series)} fill="none" stroke="#c2ed74" strokeWidth="2.2" strokeLinejoin="round" strokeLinecap="round"/><path d={makePath(candidate.series)} fill="none" stroke="#8ec7dc" strokeWidth="2.2" strokeLinejoin="round" strokeLinecap="round"/><text x={pad.left} y={height - 7}>0 s</text><text x={width - pad.right} y={height - 7} textAnchor="end">{maxTime.toFixed(0)} s</text></svg><footer><span><i className="ev-legend-baseline"/>{baseline.config.vehicleName} · A</span><span><i className="ev-legend-candidate"/>{candidate.config.vehicleName} · B</span></footer></section>;
}

function SweepChart({ candidates, bestCurrentA }: { candidates: BoundedSearchCandidate[]; bestCurrentA: number | null }) {
  const points = candidates.filter((candidate) => candidate.state === 'completed' && Number.isFinite(candidate.discharged_capacity_ah));
  if (!points.length) return <div className="sweep-chart-empty">No completed solver points to plot.</div>;
  const width = 540; const height = 160; const pad = { left: 44, right: 18, top: 14, bottom: 28 };
  const minX = Math.min(...points.map((point) => point.current_a)); const maxX = Math.max(...points.map((point) => point.current_a));
  const values = points.map((point) => point.discharged_capacity_ah ?? 0); const minY = Math.min(...values); const maxY = Math.max(...values);
  const x = (value: number) => pad.left + (value - minX) / Math.max(maxX - minX, 1e-9) * (width - pad.left - pad.right);
  const y = (value: number) => height - pad.bottom - (value - minY) / Math.max(maxY - minY, 1e-9) * (height - pad.top - pad.bottom);
  const line = [...points].sort((a, b) => a.current_a - b.current_a).map((point, index) => `${index ? 'L' : 'M'}${x(point.current_a).toFixed(1)},${y(point.discharged_capacity_ah ?? 0).toFixed(1)}`).join(' ');
  return <svg className="sweep-chart" viewBox={`0 0 ${width} ${height}`} role="img" aria-label="Recorded discharged capacity by cell current">{[0, 1, 2, 3].map((index) => { const value = minY + (maxY - minY) * index / 3; return <g key={index}><line x1={pad.left} x2={width - pad.right} y1={y(value)} y2={y(value)} stroke="#777b5d30"/><text x={pad.left - 7} y={y(value) + 3} textAnchor="end" fill="#838579" fontSize="9">{value.toFixed(2)}</text></g>; })}<path d={line} fill="none" stroke="#c2ed74" strokeWidth="2.2" strokeLinejoin="round" strokeLinecap="round"/>{points.map((point) => <g key={point.run_id ?? point.current_a}><circle cx={x(point.current_a)} cy={y(point.discharged_capacity_ah ?? 0)} r="4" fill={point.current_a === bestCurrentA ? '#d9f59a' : '#9fac76'} stroke="#1c2114" strokeWidth="2"/><text x={x(point.current_a)} y={height - 8} textAnchor="middle" fill="#838579" fontSize="8">{point.current_a} A</text></g>)}</svg>;
}

function Timeline({ mode, run, runs, playbackIndex, playing, onIndexChange, onPlayingChange }: { mode: Mode; run: IntegrationRun | null; runs: IntegrationRun[]; playbackIndex: number; playing: boolean; onIndexChange: (index: number) => void; onPlayingChange: (playing: boolean) => void }) {
  const [collapsed, setCollapsed] = useState(false);
  const selectedIndex = run ? runs.findIndex((entry) => entry.id === run.id) : -1;
  const vehicleSeries = run?.operation === 'simulate_ev_drive_cycle' && Array.isArray(run.result.series)
    ? run.result.series as Array<{ time_s: number; distance_m: number }> : [];
  const cellSeries = run?.operation === 'simulate_bound_cell' && Array.isArray(run.result.series)
    ? run.result.series as Array<{ time_h: number; voltage_v: number }> : [];
  const sampleCount = vehicleSeries.length || cellSeries.length;
  const supportsReplay = mode === 'Understand' && sampleCount > 1;
  const safeIndex = Math.max(0, Math.min(playbackIndex, sampleCount - 1));
  const currentTime = vehicleSeries.length ? Number(vehicleSeries[safeIndex]?.time_s) : Number(cellSeries[safeIndex]?.time_h);
  const endTime = vehicleSeries.length
    ? Number(run?.result.drive_cycle_duration_s ?? vehicleSeries[vehicleSeries.length - 1]?.time_s)
    : Number(cellSeries[cellSeries.length - 1]?.time_h);
  const timeUnit = vehicleSeries.length ? 's' : 'h';
  const timeText = Number.isFinite(currentTime) && Number.isFinite(endTime) ? `${currentTime.toFixed(2)} / ${endTime.toFixed(2)} ${timeUnit}` : '—';
  const progress = supportsReplay && sampleCount > 1 ? safeIndex / (sampleCount - 1) : run ? 1 : 0;
  const togglePlayback = () => {
    if (playing) { onPlayingChange(false); return; }
    if (safeIndex >= sampleCount - 1) onIndexChange(0);
    onPlayingChange(true);
  };
  return <div className={`timeline ${collapsed ? 'timeline-collapsed' : ''}`}><div className="timeline-top"><div><span className="timeline-title">{mode === 'Understand' ? 'RUN TIMELINE' : mode === 'Improve' ? 'BRANCH HISTORY' : 'PROJECT HISTORY'}</span><span className="timeline-hint">{run ? `RUN ${Math.max(1, selectedIndex + 1)} OF ${runs.length} · ${run.operation.replaceAll('_', ' ')} · ${new Date(run.created_at * 1000).toLocaleString()}` : 'No recorded activity yet'}</span></div><div className="timeline-controls"><button className="disabled-control" disabled={!supportsReplay} onClick={() => { onPlayingChange(false); onIndexChange(0); }} aria-label="Restart recorded run">↺</button><button className="play-button" disabled={!supportsReplay} onClick={togglePlayback} aria-label={playing ? 'Pause recorded run' : 'Play recorded run'}>{playing ? <Pause size={12}/> : <Play size={12}/>}</button><button className="disabled-control" disabled={!supportsReplay || safeIndex >= sampleCount - 1} onClick={() => { onPlayingChange(false); onIndexChange(Math.min(sampleCount - 1, safeIndex + 1)); }} aria-label="Advance one recorded sample">▶</button><span className="time-readout">{supportsReplay ? timeText : '— : —'}</span></div><button className="icon-button" onClick={() => { if (!collapsed) onPlayingChange(false); setCollapsed(!collapsed); }} aria-expanded={!collapsed} aria-label={collapsed ? 'Expand timeline' : 'Collapse timeline'}><ArrowDown size={14} /></button></div>{!collapsed && <div className="timeline-track"><div className="track-base" /><div className="track-event start-event" />{run ? <div className="run-track-range" style={{ width: `${progress * 100}%` }} /> : null}<span className="timeline-playhead" style={{ left: `${progress * 100}%` }}/><span className="track-label">{run ? 'RECORDED SOLVER SAMPLES' : 'PROJECT CREATED'}</span>{!run ? <div className="track-empty">Timeline populates from saved executions and evidence</div> : supportsReplay ? <input className="timeline-scrubber" type="range" min={0} max={sampleCount - 1} step={1} value={safeIndex} onChange={(event) => { onPlayingChange(false); onIndexChange(Number(event.target.value)); }} aria-label={`Scrub recorded run time in ${timeUnit}`} /> : <div className="track-empty">This run has no supported recorded time-series for replay.</div>}</div>}</div>;
}

type ActivityTab = 'events' | 'telemetry' | 'state' | 'compute' | 'provenance' | 'evidence';
const activityTabs: Array<{ id: ActivityTab; label: string }> = [
  { id: 'events', label: 'Events' }, { id: 'telemetry', label: 'Telemetry' },
  { id: 'state', label: 'State' }, { id: 'compute', label: 'Compute' },
  { id: 'provenance', label: 'Provenance' }, { id: 'evidence', label: 'Evidence' },
];

function ActivityDrawer({ project, events, loading, error, runs, verifications, tools, selectedRun, onOpenRun, onExport, onClose }: {
  project: Resource<ProjectPayload>; events: ActivityEvent[]; loading: boolean; error: string;
  runs: IntegrationRun[]; verifications: VerificationRecord[]; tools: ConnectedTool[];
  selectedRun: IntegrationRun | null; onOpenRun: (run: IntegrationRun) => void;
  onExport: () => Promise<void>; onClose: () => void;
}) {
  useModalFocus();
  const [tab, setTab] = useState<ActivityTab>('events');
  const [telemetryRunId, setTelemetryRunId] = useState('');
  const telemetryRuns = runs.filter((run) => run.result.status === 'completed' && Array.isArray(run.result.series) && run.result.series.length > 0);
  const telemetryRun = telemetryRuns.find((run) => run.id === telemetryRunId)
    ?? telemetryRuns.find((run) => run.id === selectedRun?.id) ?? telemetryRuns[0];
  const samples = telemetryRun && Array.isArray(telemetryRun.result.series)
    ? telemetryRun.result.series.filter((sample): sample is Record<string, unknown> => Boolean(sample) && typeof sample === 'object' && !Array.isArray(sample)) : [];
  const numericChannels = samples[0] ? Object.keys(samples[0]).filter((key) => samples.some((sample) => typeof sample[key] === 'number')).slice(0, 5) : [];
  const reality = project.payload.engineering_reality;
  const selectedSnapshot = selectedRun?.project_snapshot as { project_version?: number; models?: { engineering_reality?: { revision?: number; sha256?: string } } } | null | undefined;

  const chooseAdjacentTab = (event: React.KeyboardEvent<HTMLButtonElement>, current: ActivityTab) => {
    if (event.key !== 'ArrowRight' && event.key !== 'ArrowLeft') return;
    event.preventDefault();
    const index = activityTabs.findIndex((item) => item.id === current);
    const direction = event.key === 'ArrowRight' ? 1 : -1;
    const next = activityTabs[(index + direction + activityTabs.length) % activityTabs.length];
    setTab(next.id);
    event.currentTarget.parentElement?.querySelector<HTMLButtonElement>(`[data-activity-tab="${next.id}"]`)?.focus();
  };

  return <div className="activity-drawer-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
    <section id="project-activity-drawer" className="activity-drawer" role="dialog" aria-modal="true" aria-labelledby="activity-title">
      <div className="activity-drawer-grip"/><header className="activity-drawer-header"><div><span className="eyebrow"><span className="eyebrow-line"/> PROJECT RECORD · ENGINEERING TRACE</span><h2 id="activity-title">Activity</h2><p>Events, runs, state, and evidence for {project.payload.name}.</p></div><button className="icon-button" onClick={onClose} aria-label="Close activity"><X size={16}/></button></header>
      <nav className="activity-tabs" role="tablist" aria-label="Project activity categories">{activityTabs.map((item) => <button key={item.id} type="button" role="tab" data-activity-tab={item.id} id={`activity-tab-${item.id}`} aria-selected={tab === item.id} aria-controls="activity-tab-panel" tabIndex={tab === item.id ? 0 : -1} className={tab === item.id ? 'active' : ''} onClick={() => setTab(item.id)} onKeyDown={(event) => chooseAdjacentTab(event, item.id)}>{item.label}{item.id === 'provenance' ? runs.length : item.id === 'evidence' ? verifications.length : ''}</button>)}</nav>
      <div id="activity-tab-panel" className="activity-tab-panel" role="tabpanel" aria-labelledby={`activity-tab-${tab}`}>
        {tab === 'events' && (loading ? <div className="activity-state">Loading saved project events…</div> : error ? <div className="form-error">{error}</div> : events.length ? <div className="activity-list">{events.map((event) => <article className="activity-row" key={event.id}><span className={`activity-dot ${event.outcome === 'failed' ? 'is-failed' : ''}`}/><div><b>{event.action.replaceAll('.', ' ').replaceAll('_', ' ')}</b><small>{event.subject_type.replaceAll('_', ' ')}{event.subject_id ? ` · ${event.subject_id.slice(0, 8)}` : ''}{event.details.tool ? ` · ${event.details.tool}` : ''}{event.details.operation ? ` / ${event.details.operation}` : ''}</small></div><time>{new Date(event.created_at * 1000).toLocaleString()}</time></article>)}</div> : <div className="activity-state">No saved activity for this project yet.</div>)}
        {tab === 'telemetry' && <div className="activity-tab-content"><header><div><b>Recorded signals</b><small>Only values saved by a completed model run appear here.</small></div><select aria-label="Choose a telemetry run" value={telemetryRun?.id ?? ''} onChange={(event) => setTelemetryRunId(event.target.value)} disabled={!telemetryRuns.length}><option value="">{telemetryRuns.length ? 'Choose a recorded run' : 'No recorded time series'}</option>{telemetryRuns.map((run) => <option key={run.id} value={run.id}>{run.operation.replaceAll('_', ' ')} · {run.id.slice(0, 8)}</option>)}</select></header>{telemetryRun ? <><div className="activity-telemetry-summary"><Activity size={16}/><div><b>{samples.length.toLocaleString()} recorded samples</b><small>{telemetryRun.tool} · {telemetryRun.provenance}</small></div><button className="secondary-button" onClick={() => onOpenRun(telemetryRun)}>Open run</button></div><div className="activity-channel-grid">{numericChannels.map((key) => <div key={key}><span>{key.replaceAll('_', ' ')}</span><b>{String(samples[0][key])}<i> → </i>{String(samples[samples.length - 1][key])}</b><small>first → last recorded value</small></div>)}</div></> : <div className="activity-state">Run a supported analysis to create an inspectable time series.</div>}</div>}
        {tab === 'state' && <div className="activity-tab-content"><div className="activity-state-summary"><div><span>SAVED PROJECT REVISION</span><b>v{project.version}</b></div><div><span>REALITY REVISION</span><b>{reality?.revision ?? 'Not defined'}</b></div><div><span>DECLARED COMPONENTS</span><b>{reality?.components.length ?? 0}</b></div><div><span>DECLARED CONNECTIONS</span><b>{reality?.connections.length ?? 0}</b></div></div><section className="activity-state-source"><b>{selectedRun ? 'Selected run snapshot' : 'Current saved project'}</b><small>{selectedRun ? `Run ${selectedRun.id.slice(0, 8)} · project revision ${selectedSnapshot?.project_version ?? 'not captured'}` : `Project ${project.id}`}</small><code>{selectedSnapshot?.models?.engineering_reality?.sha256 ?? 'No selected run model digest'}</code></section></div>}
        {tab === 'compute' && <div className="integration-tool-list activity-compute-list">{tools.length ? tools.map((tool) => <article className="integration-tool-row" key={tool.id}><span className={`integration-tool-mark ${tool.available ? 'is-available' : ''}`}><Wrench size={15}/></span><div className="integration-tool-copy"><b>{tool.label}</b><small>{tool.note || (tool.available ? 'Available from the local gateway.' : 'Install and configure this tool to enable its operations.')}</small></div><span className={`integration-tool-status ${tool.available ? 'is-available' : ''}`}><i/>{tool.available ? 'READY' : tool.status.toUpperCase()}</span></article>) : <div className="activity-state">Compute and integration status is unavailable.</div>}</div>}
        {tab === 'provenance' && <div className="activity-run-list">{runs.length ? runs.slice(0, 20).map((run) => <article className="activity-run-row" key={run.id}><div><b>{run.operation.replaceAll('_', ' ')}</b><small>{run.tool} · {run.provenance} · {new Date(run.created_at * 1000).toLocaleString()}</small></div><span className={`activity-run-status ${run.result.status === 'failed' ? 'is-failed' : ''}`}>{String(run.result.status ?? 'recorded')}</span><details><summary>Inputs & model snapshot</summary><pre>{JSON.stringify({ inputs: run.inputs, snapshot: run.project_snapshot ?? null }, null, 2)}</pre></details><button type="button" className="secondary-button" onClick={() => onOpenRun(run)}>Inspect run</button></article>) : <div className="activity-state">No tool runs have been recorded.</div>}</div>}
        {tab === 'evidence' && <div className="activity-tab-content"><header><div><b>Verification records</b><small>Traceability is not certification.</small></div><button className="secondary-button" onClick={() => void onExport()}>Export evidence</button></header>{verifications.length ? <div className="activity-list">{verifications.map((record) => <article className="activity-row" key={record.id}><span className={`activity-dot ${record.payload.verdict === 'fail' ? 'is-failed' : ''}`}/><div><b>{record.payload.requirementTitle || record.payload.requirement_id || 'Requirement check'}</b><small>{record.payload.verdict ?? 'inconclusive'} · source run {record.payload.source_run_id?.slice(0, 8) ?? 'not linked'}</small></div><time>{new Date(record.created_at).toLocaleString()}</time></article>)}</div> : <div className="activity-state">No verification records exist for this project yet.</div>}</div>}
      </div>
      <footer className="activity-drawer-footer"><span>Data comes from this project’s saved records and configured integrations.</span><button className="secondary-button" onClick={onClose}>Close activity</button></footer>
    </section>
  </div>;
}

function Inspector({ mode, project, tools, selectedComponentId, onAddComponent, onRun, onAddRequirement, busy }: { mode: Mode; project: Resource<ProjectPayload>; tools: Array<{ id: string; label: string; status: string; available: boolean }>; selectedComponentId: string | null; onAddComponent: () => void; onRun: () => void; onAddRequirement: () => void; busy: boolean }) {
  const reality = (project.payload as ProjectPayload & { engineering_reality?: EngineeringReality }).engineering_reality;
  const selectedComponent = reality?.components.find((component) => component.id === selectedComponentId);
  const selectedInterfaces = (reality?.interfaces ?? []).filter((item) => item.componentId === selectedComponent?.id);
  const selectedInterfaceIds = new Set(selectedInterfaces.map((item) => item.id));
  const selectedConnections = (reality?.connections ?? []).filter((item) => selectedInterfaceIds.has(item.sourceInterfaceId) || selectedInterfaceIds.has(item.targetInterfaceId));
  const interfaceOwners = new Map((reality?.interfaces ?? []).map((item) => [item.id, item.componentId]));
  const componentNames = new Map((reality?.components ?? []).map((item) => [item.id, `${item.name} (${item.componentType})`]));
  const linkedComponents = [...new Set(selectedConnections.map((item) => {
    const linkedInterfaceId = selectedInterfaceIds.has(item.sourceInterfaceId) ? item.targetInterfaceId : item.sourceInterfaceId;
    return interfaceOwners.get(linkedInterfaceId);
  }).filter((id): id is string => Boolean(id && id !== selectedComponent?.id)))];
  const scopedBindings = (reality?.modelBindings ?? []).filter((item) => item.scopeComponentId === selectedComponent?.id);
  const selectedBuildSections: Array<[string, string]> = selectedComponent ? [
    ['COMPONENT ID', selectedComponent.id],
    ['TYPE', selectedComponent.componentType],
    ['GEOMETRY', 'Procedural visual proxy; no linked CAD dimensions or source asset'],
    ['BEHAVIOR & MODELS', scopedBindings.length ? scopedBindings.map((item) => `${item.modelId} · ${item.modelVersion} · ${item.declaredFidelity}`).join('; ') : 'No model or behavior binding declared for this component'],
    ['INTERFACES', selectedInterfaces.length ? selectedInterfaces.map((item) => `${item.name} · ${item.direction} · ${item.domain}`).join('; ') : 'No interfaces declared'],
    ['DEPENDENCIES', linkedComponents.length ? linkedComponents.map((id) => componentNames.get(id) ?? id).join('; ') : 'No connected component dependencies declared'],
    ['CONSTRAINTS', reality?.requirements.length ? `${reality.requirements.length} project requirement(s); none are linked to this component` : 'No project requirements declared'],
    ['INITIAL STATE', 'Not stored for this component in the current engineering reality'],
  ] : [];
  const info: Record<Mode, { title: string; description: string; sections: Array<[string, string]> }> = {
    Build: { title: selectedComponent?.name || 'Project reality', description: selectedComponent ? selectedComponent.componentType : 'Versioned source of truth', sections: selectedComponent ? selectedBuildSections : [['IDENTITY', project.payload.domain || 'Engineering system'], ['REVISION', `v${project.version} · saved project record`], ['DESCRIPTION', project.payload.description || 'No description supplied'], ['COMPONENTS', `${reality?.components.length ?? 0} saved in engineering reality`]] },
    Simulate: { title: 'Analysis readiness', description: 'Readiness from saved model and integration gateway', sections: [['MODEL', project.payload.ev_vehicle ? 'PyBaMM EV drive cycle · Chen2020' : reality?.modelBindings.some((binding) => binding.modelId === 'pybamm.lithium-ion.spm') ? 'PyBaMM SPM · Chen2020' : 'No compatible model binding'], ['TOOL STATUS', tools.find((tool) => tool.id === 'pybamm')?.status || 'Gateway unavailable'], ['SCOPE', project.payload.ev_vehicle ? 'Prescribed speed · road load · identical cell pack' : 'Bound single-cell discharge; not pack or vehicle simulation']] },
    Run: { title: 'Execution status', description: 'Only actual solver outputs appear here', sections: [['MODEL', project.payload.ev_vehicle ? 'PyBaMM EV drive cycle · Chen2020' : reality?.modelBindings.length ? 'PyBaMM single-cell · Chen2020' : 'No model configured'], ['STATUS', busy ? 'Running model…' : 'Ready when the saved inputs and solver are available'], ['PROJECT REVISION', `v${project.version}`], ['LAST RECORDED RUN', 'Select Understand to inspect saved output']] },
    Understand: { title: 'Selected run', description: 'Recorded output with solver provenance', sections: [['RUN', 'Cell discharge'], ['MODEL', 'PyBaMM SPM · Chen2020'], ['RESULT SOURCE', 'Saved project snapshot and inputs']] },
    Improve: { title: 'Alternative design', description: 'Branch from a controlled baseline', sections: [['BASELINE', `Project revision v${project.version}`], ['OBJECTIVE', 'Not declared'], ['EVALUATION', 'No candidates evaluated']] },
    Verify: { title: 'Evidence & checks', description: 'Traceability does not imply certification', sections: [['REQUIREMENTS', 'None recorded'], ['VERIFICATION', 'No checks recorded'], ['EVIDENCE', 'No project evidence exported']] },
  };
  const details = info[mode];
  const modelReady = Boolean(project.payload.ev_vehicle || reality?.modelBindings.some((binding) => binding.modelId === 'pybamm.lithium-ion.spm' && binding.modelVersion === 'Chen2020'));
  const solverReady = Boolean(tools.find((tool) => tool.id === 'pybamm')?.available);
  return <aside className="inspector"><div className="inspector-header"><div><span className="panel-overline">{mode === 'Build' ? 'SYSTEM' : mode.toUpperCase()}</span><h2>{details.title}</h2><p>{details.description}</p></div></div><div className="inspector-content">{mode === 'Build' && <div className="selected-object"><div className="selected-object-icon"><Box size={17} /></div><div><b>{selectedComponent?.name || project.payload.name}</b><span>{selectedComponent?.componentType || 'Engineering system'}</span></div><span className="object-more">···</span></div>}{details.sections.map(([label, value]) => <div className="property-row" key={label}><span>{label}</span><b>{value}</b></div>)}{mode === 'Build' && selectedComponent && <section className="inspector-properties" aria-label="Declared engineering properties"><header><span>DECLARED PROPERTIES</span><small>{selectedComponent.properties.length}</small></header>{selectedComponent.properties.length ? <dl>{selectedComponent.properties.map((property, index) => <div className="inspector-property" key={`${property.name}:${index}`}><dt>{property.name.replaceAll('_', ' ')}</dt><dd>{String(property.value)}{property.unit ? ` ${property.unit}` : ''}</dd><small>{property.origin.replaceAll('_', ' ')}{property.sourceRef ? ` · ${property.sourceRef}` : ''}</small></div>)}</dl> : <p className="inspector-properties-empty">No properties have been declared for this component.</p>}</section>}<div className="inspector-divider" /><div className="inspector-note"><Sparkles size={14} /><div><b>Engineering assistant · not connected</b><p>Project-grounded model assistance is not configured. This inspector shows saved project data only.</p><button type="button" disabled title="No intelligence provider is configured">Ask about this project <ArrowRight size={13} /></button></div></div></div><div className="inspector-bottom"><button className="inspector-primary" disabled={mode === 'Build' ? false : mode === 'Verify' ? busy : mode !== 'Run' || busy || !modelReady || !solverReady} onClick={mode === 'Build' ? onAddComponent : mode === 'Verify' ? onAddRequirement : mode === 'Run' ? onRun : undefined}><Plus size={14} /> {mode === 'Build' ? 'Add component' : mode === 'Run' ? (busy ? 'Running…' : project.payload.ev_vehicle ? 'Run vehicle drive cycle' : 'Run cell analysis') : mode === 'Verify' ? 'Add requirement' : 'Select item'} {mode === 'Run' && !solverReady && <span>PyBaMM unavailable</span>}</button><p>{mode === 'Build' ? 'Adds a typed component to the saved engineering reality.' : mode === 'Run' ? project.payload.ev_vehicle ? 'Runs the saved EV, pack, and drive-cycle inputs through the configured adapter.' : 'Runs PyBaMM using the saved single-cell model and project inputs.' : mode === 'Verify' ? 'Requirements are saved as drafts until explicitly accepted.' : 'Actions unlock when the required project data is present.'}</p></div></aside>;
}

function Metric({ label, value }: { label: string; value: string }) { return <div className="metric"><span>{label}</span><b>{value}</b></div>; }
function EVRunView({ run, series, playbackIndex, playbackPlaying, onPlaybackIndexChange, onPlaybackPlayingChange, realityViewport }: { run: IntegrationRun; series: Array<{ time_s: number; speed_m_s: number; distance_m: number; pack_voltage_v: number; pack_current_a: number; pack_power_w: number }>; playbackIndex: number; playbackPlaying: boolean; onPlaybackIndexChange: (index: number) => void; onPlaybackPlayingChange: (playing: boolean) => void; realityViewport: ReactNode }) {
  const frame = Math.max(0, Math.min(playbackIndex, series.length - 1));
  const point = series[frame];
  const totalDistance = Math.max(...series.map((item) => item.distance_m), 0);
  const progress = Number.isFinite(point.distance_m) && Number.isFinite(totalDistance) && totalDistance > 0
    ? Math.max(0, Math.min(1, point.distance_m / totalDistance))
    : 0;
  const maxTime = Number(run.result.drive_cycle_duration_s ?? series[series.length - 1].time_s);
  const display = (value: number | undefined, digits = 1) => Number.isFinite(value) ? value!.toFixed(digits) : '—';
  return <div className="canvas-stage stage-understand ev-result-stage">{realityViewport}<div className="stage-grid"/><section className="ev-result-view"><header className="result-heading"><div><span className="stage-kicker">RECORDED DRIVE-CYCLE OUTPUT <i/> PYBAMM · SPM / CHEN2020</span><h1>{String(run.result.vehicle_name ?? 'Vehicle')} · battery response</h1><p>Run {run.id.slice(0, 8)} · {Number(run.result.sample_count ?? series.length)} recorded samples</p></div><span className="result-status"><i/> COMPLETED</span></header>
    <div className="motion-panel"><div className="motion-panel-head"><div><span>TRAJECTORY REPLAY</span><small>Position from recorded distance over the prescribed speed trace</small></div><div className="motion-play-group"><button className="motion-play-button" disabled={series.length < 2} title={series.length < 2 ? 'Replay needs at least two recorded samples' : undefined} onClick={() => { if (frame >= series.length - 1 && !playbackPlaying) onPlaybackIndexChange(0); onPlaybackPlayingChange(!playbackPlaying); }} aria-label={playbackPlaying ? 'Pause replay' : 'Play recorded vehicle movement'}>{playbackPlaying ? <Pause size={12}/> : <Play size={12}/>} {playbackPlaying ? 'Pause' : frame >= series.length - 1 ? 'Replay' : 'Play'}</button><span>{display(point.time_s, 0)} <small>/ {display(maxTime, 0)} s</small></span></div></div><div className="motion-world"><Suspense fallback={<RuntimeFallback />}><VehicleRuntimeCanvas distanceMeters={point.distance_m} totalDistanceMeters={totalDistance}/></Suspense><div className="motion-skyline"><i/><i/><i/><i/><i/></div><div className="motion-road"><div className="road-lanes"/><div className="motion-car" style={{ left: `${5 + progress * 87}%` }}><svg viewBox="0 0 120 50" role="img" aria-label="Vehicle position along recorded drive-cycle distance"><path d="M12 32h8l9-14c2-3 5-5 9-5h31c5 0 8 2 12 6l9 11 13 3c4 1 6 3 6 7v4H10v-5c0-4 1-6 2-7Z" fill="#c4e47c"/><path d="m33 19-7 11h58l-9-10c-2-2-4-3-7-3H41c-4 0-6 0-8 2Z" fill="#313b25"/><circle cx="31" cy="42" r="7" fill="#11120f" stroke="#879c59" strokeWidth="2"/><circle cx="91" cy="42" r="7" fill="#11120f" stroke="#879c59" strokeWidth="2"/></svg><b>{display(point.speed_m_s * 3.6, 0)} <small>km/h</small></b></div></div><div className="motion-distance"><span>0 m</span><span>{display(point.distance_m, 0)} m traveled</span><span>{display(totalDistance, 0)} m</span></div></div><input className="motion-slider" type="range" min="0" max={series.length - 1} value={frame} disabled={series.length < 2} onChange={(e) => { onPlaybackPlayingChange(false); onPlaybackIndexChange(Number(e.target.value)); }} aria-label="Scrub recorded vehicle trajectory"/></div>
    <div className="result-metrics ev-metrics"><Metric label="DISTANCE TRAVELED" value={`${display(Number(run.result.distance_m), 2)} m`}/><Metric label="PEAK PACK POWER" value={`${display(Number(run.result.peak_pack_power_w) / 1000, 2)} kW`}/><Metric label="MIN CELL VOLTAGE" value={`${display(Number(run.result.minimum_cell_voltage_v), 3)} V`}/><Metric label="PACK VOLTAGE · CURRENT" value={`${display(point.pack_voltage_v, 1)} V · ${display(point.pack_current_a, 1)} A`}/></div>
    <div className="result-scope"><ShieldCheck size={14}/><span>{String(run.result.applicability ?? 'Prescribed-speed longitudinal load coupled to a Chen2020 single-cell model. Not a validated vehicle or battery-pack simulation.')}</span></div>
  </section><div className="stage-fidelity"><span className="fidelity-icon"><CarFront size={14}/></span><div><b>Recorded trajectory</b><small>Position is derived from run distance · scrub to inspect time</small></div></div></div>;
}
function VerifyBoard({ project, requirements, records, compatibleRun, busy, realityViewport, onAdd, onReview, onVerify, onExport }: { project: Resource<ProjectPayload>; requirements: EngineeringRequirement[]; records: VerificationRecord[]; compatibleRun: IntegrationRun | null; busy: boolean; realityViewport: ReactNode; onAdd: () => void; onReview: (id: string, status: EngineeringRequirement['reviewStatus']) => void; onVerify: () => void; onExport: () => void }) {
  const accepted = requirements.filter((item) => item.reviewStatus === 'accepted' && item.verificationRule);
  const byRequirement = new Map([...records].reverse().map((record) => [record.payload.requirement_id, record]));
  return <div className="canvas-stage stage-verify">{realityViewport}<div className="stage-grid" /><section className="verify-board"><header className="verify-header"><div><span className="stage-kicker">PROJECT REVISION {project.version} <i /> TRACEABLE CHECKS</span><h1>Requirements & evidence</h1><p>Review each requirement, then check it against a compatible recorded model run.</p></div><div className="verify-actions"><button className="secondary-button" onClick={onExport} disabled={busy}>Export evidence</button><button className="primary-button" onClick={onVerify} disabled={busy || !compatibleRun || accepted.length === 0}><ShieldCheck size={14} /> {busy ? 'Checking…' : 'Check accepted'}</button></div></header>
    <div className={`verification-source ${compatibleRun ? 'source-ready' : 'source-stale'}`}><span className="source-led"/><div><b>{compatibleRun ? `Current source run · ${compatibleRun.id.slice(0, 8)}` : 'A current source run is required'}</b><small>{compatibleRun ? `PyBaMM · single-cell discharge · project revision ${project.version}` : 'Accepting or changing a requirement changes the saved revision. Run the model again before checking.'}</small></div>{!compatibleRun && <Activity size={15} />}</div>
    <div className="requirements-heading"><div><h2>Design requirements <span>{requirements.length}</span></h2><p>Only accepted requirements with a supported check rule are evaluated.</p></div><button className="tool-button" onClick={onAdd}><Plus size={14}/> Add requirement</button></div>
    {requirements.length ? <div className="requirement-list">{requirements.map((requirement) => { const record = byRequirement.get(requirement.id); const rule = requirement.verificationRule; const verdict = record?.payload.verdict; return <article className="requirement-card" key={requirement.id}><div className="requirement-priority">{requirement.priority.toUpperCase()}</div><div className="requirement-main"><h3>{requirement.statement}</h3><div className="requirement-condition">{rule ? `${rule.metric.replaceAll('_', ' ')} ${rule.operator} ${rule.threshold} ${rule.unit}` : 'No supported numeric check rule'}</div>{record && <div className={`check-result verdict-${verdict ?? 'inconclusive'}`}><span>{verdict ?? 'inconclusive'}</span><span>{record.payload.observedValue ? `Observed ${record.payload.observedValue}` : record.payload.conditionFormula}</span><small>Evidence record · source {record.payload.source_run_id?.slice(0, 8) ?? 'run unavailable'}</small></div>}</div><div className="requirement-side"><span className={`review-pill review-${requirement.reviewStatus}`}>{requirement.reviewStatus}</span>{requirement.reviewStatus === 'draft' ? <button className="review-button" disabled={busy || !rule} onClick={() => onReview(requirement.id, 'accepted')}>Accept for verification</button> : <button className="review-button" disabled={busy} onClick={() => onReview(requirement.id, 'draft')}>Return to draft</button>}</div></article>; })}</div> : <div className="verify-empty"><div className="empty-glyph"><FileCheck2 size={20}/></div><h3>No requirements defined</h3><p>Add a measurable requirement. It will remain a draft until someone accepts it for checking.</p><button className="primary-button" onClick={onAdd}><Plus size={15}/> Add requirement</button></div>}
    <p className="verify-limit"><ShieldCheck size={13}/> HELQIX checks the recorded model output against declared conditions. This is traceability evidence, not certification or a claim that the product is safe.</p>
  </section></div>;
}
function ResultChart({ series, currentIndex }: { series: Array<{ time_h: number; voltage_v: number; capacity_ah: number }>; currentIndex: number }) {
  const width = 680; const height = 250; const pad = { left: 48, right: 18, top: 16, bottom: 32 };
  const times = series.map((point) => point.time_h); const volts = series.map((point) => point.voltage_v);
  const minX = Math.min(...times); const maxX = Math.max(...times); const minY = Math.min(...volts); const maxY = Math.max(...volts);
  const x = (value: number) => pad.left + (value - minX) / Math.max(maxX - minX, 1e-9) * (width - pad.left - pad.right);
  const y = (value: number) => height - pad.bottom - (value - minY) / Math.max(maxY - minY, 1e-9) * (height - pad.top - pad.bottom);
  const sample = series[Math.max(0, Math.min(currentIndex, series.length - 1))];
  const path = series.map((point, index) => `${index ? 'L' : 'M'}${x(point.time_h).toFixed(1)},${y(point.voltage_v).toFixed(1)}`).join(' ');
  const ticks = Array.from({ length: 4 }, (_, i) => minY + (maxY - minY) * i / 3);
  return <div className="chart-wrap"><div className="chart-title">CELL TERMINAL VOLTAGE <span>{sample ? `${sample.time_h.toFixed(3)} h · ${sample.voltage_v.toFixed(3)} V` : 'recorded solver output'}</span></div><svg viewBox={`0 0 ${width} ${height}`} role="img" aria-label="Recorded battery cell terminal voltage over simulation time"><defs><linearGradient id="voltage-line" x1="0" x2="0" y1="0" y2="1"><stop offset="0" stopColor="#c2ed74" stopOpacity=".28"/><stop offset="1" stopColor="#c2ed74" stopOpacity="0"/></linearGradient></defs>{ticks.map((tick) => <g key={tick}><line x1={pad.left} x2={width - pad.right} y1={y(tick)} y2={y(tick)} stroke="#777b5d24"/><text x={pad.left - 9} y={y(tick) + 3} textAnchor="end" fill="#838579" fontSize="9">{tick.toFixed(2)}</text></g>)}<path d={`${path} L${x(maxX)},${height - pad.bottom} L${x(minX)},${height - pad.bottom} Z`} fill="url(#voltage-line)"/><path d={path} fill="none" stroke="#c2ed74" strokeWidth="2.4" strokeLinejoin="round" strokeLinecap="round"/>{sample && <circle cx={x(sample.time_h)} cy={y(sample.voltage_v)} r="4.5" fill="#c2ed74" stroke="#1c2114" strokeWidth="2"/>}<text x={pad.left} y={height - 8} fill="#838579" fontSize="9">0 h</text><text x={width - pad.right} y={height - 8} textAnchor="end" fill="#838579" fontSize="9">{maxX.toFixed(2)} h</text><text x="11" y="14" fill="#838579" fontSize="8">V</text></svg></div>;
}

function NewProjectModal({ busy, error, onClose, onCreate }: { busy: boolean; error: string; onClose: () => void; onCreate: (p: ProjectPayload) => void }) {
  useModalFocus();
  const [name, setName] = useState(''); const [domain, setDomain] = useState(''); const [intent, setIntent] = useState('');
  const [analysis, setAnalysis] = useState<IntentAnalysis | null>(null); const [accepted, setAccepted] = useState<Set<number>>(new Set());
  const [acceptedAI, setAcceptedAI] = useState<Set<number>>(new Set()); const [analyzing, setAnalyzing] = useState(false); const [aiReviewing, setAIReviewing] = useState(false); const [parseError, setParseError] = useState('');
  const analyzeBrief = async () => {
    if (!intent.trim()) return;
    setAnalyzing(true); setParseError('');
    try {
      const result = await api.parseIntent(intent, domain);
      setAnalysis(result); setAccepted(new Set()); setAcceptedAI(new Set());
    } catch (cause) { setParseError(cause instanceof Error ? cause.message : 'The brief could not be analyzed.'); }
    finally { setAnalyzing(false); }
  };
  const requestAIReview = async () => {
    if (!intent.trim() || !analysis?.ai_review_status?.available || !analysis.ai_review_status.provider) return;
    setAIReviewing(true); setParseError('');
    try {
      const result = await api.parseIntent(intent, domain, true);
      setAnalysis(result); setAcceptedAI(new Set());
    } catch (cause) { setParseError(cause instanceof Error ? cause.message : 'AI review could not be completed.'); }
    finally { setAIReviewing(false); }
  };
  const toggleCandidate = (index: number) => setAccepted((previous) => {
    const next = new Set(previous);
    if (next.has(index)) next.delete(index); else next.add(index);
    return next;
  });
  const create = () => {
    const reviewed = analysis ? { ...analysis, user_review: { accepted_candidate_indices: [...accepted].sort((a, b) => a - b), accepted_ai_objective_indices: [...acceptedAI].sort((a, b) => a - b), reviewed_at: new Date().toISOString() } } : undefined;
    onCreate({ name: name.trim(), domain: domain.trim() || undefined, description: intent.trim() || undefined, intent: intent.trim() ? intent : undefined, intent_analysis: reviewed });
  };
  return <div className="modal-backdrop" onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}><form className="new-project-modal project-create-modal" role="dialog" aria-modal="true" aria-label="Create an engineering project" tabIndex={-1} onSubmit={(e) => { e.preventDefault(); create(); }}>
    <div className="modal-head"><div><span className="panel-overline">NEW ENGINEERING REALITY</span><h2>What are you building?</h2><p>Start from your product brief. HELQIX can surface explicitly stated values for your review.</p></div><button type="button" className="icon-button" onClick={onClose}><X size={17} /></button></div>
    <label>Project name<input value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. Electric delivery vehicle" required autoFocus /></label>
    <label>Product type <small>OPTIONAL · USED TO SELECT REVIEW RULES</small><input disabled={analyzing || aiReviewing} value={domain} onChange={(e) => { setDomain(e.target.value); setAnalysis(null); setAccepted(new Set()); setAcceptedAI(new Set()); }} placeholder="e.g. vehicle, battery system, agricultural drone" /></label>
    <label>What should it do? <small>OPTIONAL</small><textarea disabled={analyzing || aiReviewing} rows={4} value={intent} onChange={(e) => { setIntent(e.target.value); setAnalysis(null); setAccepted(new Set()); setAcceptedAI(new Set()); }} placeholder="Describe its purpose, operating conditions, targets, or constraints. Missing details remain unknown." /></label>
    <div className="brief-analysis-actions"><button type="button" className="secondary-button" disabled={!intent.trim() || analyzing || aiReviewing} onClick={() => void analyzeBrief()}>{analyzing ? <span className="button-spinner"/> : <Sparkles size={14}/>} Review explicit values</button><small>Local review extracts only supported values stated in your brief.</small></div>
    {parseError && <div className="form-error">{parseError}</div>}
    {analysis && <section className="intent-review-panel"><div className="intent-review-heading"><div><b>Review extracted statements</b><span>{analysis.scope} · {analysis.parser_version}</span></div><span>{analysis.candidates.length} value{analysis.candidates.length === 1 ? '' : 's'}</span></div>{analysis.conflicts.length > 0 && <div className="intent-conflict-note">Conflicting values found for {analysis.conflicts.join(', ')}. Choose only the statement you intend to keep, or leave the conflict unresolved.</div>}{analysis.candidates.length ? <div className="intent-candidates">{analysis.candidates.map((candidate, index) => <label className="intent-candidate" key={`${candidate.key}-${candidate.start}-${index}`}><input type="checkbox" checked={accepted.has(index)} onChange={() => toggleCandidate(index)}/><span className="intent-candidate-main"><b>{candidate.label}</b><small>{candidate.source_text}</small></span><span className="intent-candidate-value">{candidate.value}{candidate.unit ? ` ${candidate.unit}` : ''}</span><em>{candidate.review_status === 'conflict_review_required' ? 'CONFLICT' : 'REVIEW'}</em></label>)}</div> : <div className="intent-no-candidates">No structured fields are configured for this product type yet. The original brief will still be saved verbatim.</div>}<div className="intent-not-inferred"><ShieldCheck size={13}/><span>Not inferred: {analysis.not_inferred.join(' · ')}</span></div>
      <div className="ai-review-entry"><div><b>Optional AI brief review</b><small>{analysis.ai_review_status?.note ?? 'AI review status is unavailable until the gateway is updated and configured.'}</small></div>{analysis.ai_review_status?.available ? <button type="button" className="secondary-button" disabled={aiReviewing || analyzing} onClick={() => void requestAIReview()}>{aiReviewing ? <span className="button-spinner"/> : <Sparkles size={13}/>} Send brief to {analysis.ai_review_status.provider} for review</button> : <span className="review-pill">{analysis.ai_review_status ? 'NOT CONFIGURED' : 'UNAVAILABLE'}</span>}</div>
      {analysis.ai_review && <div className="ai-review-results"><div className="ai-review-provenance">PROPOSALS FROM {analysis.ai_review.provider.toUpperCase()} · {analysis.ai_review.model} · REVIEW REQUIRED</div>{analysis.ai_review.objectives.length > 0 && <div className="intent-candidates">{analysis.ai_review.objectives.map((proposal, index) => <label className="intent-candidate" key={`${proposal.source_quote}-${index}`}><input type="checkbox" checked={acceptedAI.has(index)} onChange={() => setAcceptedAI((previous) => { const next = new Set(previous); if (next.has(index)) next.delete(index); else next.add(index); return next; })}/><span className="intent-candidate-main"><b>{proposal.statement}</b><small>Source: “{proposal.source_quote}”</small></span><em>AI PROPOSAL</em></label>)}</div>}{analysis.ai_review.questions.length > 0 && <div className="ai-review-questions"><b>Clarifications to consider</b>{analysis.ai_review.questions.map((item, index) => <p key={`${item.question}-${index}`}>{item.question}<small>{item.reason}</small></p>)}</div>}{!analysis.ai_review.objectives.length && !analysis.ai_review.questions.length && <p className="intent-no-candidates">No grounded objective proposals or clarification questions were returned.</p>}</div>}
    </section>}
    {error && <div className="form-error">{error}</div>}
    <div className="modal-foot"><span>{accepted.size || acceptedAI.size ? `${accepted.size + acceptedAI.size} reviewed item${accepted.size + acceptedAI.size === 1 ? '' : 's'} marked for the project record.` : 'No technical values or AI proposals are accepted without your selection.'}</span><button type="button" className="secondary-button" onClick={onClose}>Cancel</button><button className="primary-button" disabled={busy || analyzing || aiReviewing || !name.trim()}>{busy ? 'Creating…' : 'Create project'}<ArrowRight size={15} /></button></div>
  </form></div>;
}

function AddComponentModal({ busy, domain, onClose, onCreate }: { busy: boolean; domain?: string; onClose: () => void; onCreate: (name: string, componentType: string, cellInputs?: { currentA: number; cutoffVoltageV: number }) => void }) {
  useModalFocus();
  const [name, setName] = useState(''); const [typeId, setTypeId] = useState(''); const [customType, setCustomType] = useState(''); const [currentA, setCurrentA] = useState(''); const [cutoffV, setCutoffV] = useState('');
  const [types, setTypes] = useState<ComponentTypeDefinition[]>([]); const [catalogScope, setCatalogScope] = useState(''); const [catalogLoading, setCatalogLoading] = useState(true); const [catalogError, setCatalogError] = useState('');
  useEffect(() => {
    let active = true;
    setCatalogLoading(true); setCatalogError('');
    api.componentTypes(domain).then((catalog) => {
      if (!active) return;
      setTypes(catalog.entries); setCatalogScope(catalog.scope);
      setTypeId((current) => current && catalog.entries.some((item) => item.id === current) ? current : '');
    }).catch((error: unknown) => {
      if (active) setCatalogError(error instanceof Error ? error.message : 'Component type library is unavailable.');
    }).finally(() => { if (active) setCatalogLoading(false); });
    return () => { active = false; };
  }, [domain]);
  const selectedType = types.find((item) => item.id === typeId);
  const custom = typeId === 'custom_component';
  const componentType = custom ? customType.trim().toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_+|_+$/g, '') : selectedType?.id ?? '';
  const cell = ['battery_cell', 'lithium_ion_cell'].includes(componentType);
  const groups = [...new Set(types.map((item) => item.category))];
  return <div className="modal-backdrop" onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}><form className="new-project-modal component-modal" role="dialog" aria-modal="true" aria-label="Add a component" tabIndex={-1} onSubmit={(e) => { e.preventDefault(); if (!componentType) return; onCreate(name, componentType, cell ? { currentA: Number(currentA), cutoffVoltageV: Number(cutoffV) } : undefined); }}><div className="modal-head"><div><span className="panel-overline">ENGINEERING REALITY · {domain || 'GENERAL'}</span><h2>Add a component</h2><p>Choose a structural type from the shared library, or define your own. A type is not a supplier part or engineering specification.</p></div><button type="button" className="icon-button" onClick={onClose}><X size={17} /></button></div><label>Component name<input autoFocus value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. Cell A01" required maxLength={160} /></label><label>Component type<select value={typeId} onChange={(e) => setTypeId(e.target.value)} required><option value="">{catalogLoading ? 'Loading component library…' : 'Choose a component type'}</option>{groups.map((category) => <optgroup label={category} key={category}>{types.filter((item) => item.category === category && item.id !== 'custom_component').map((item) => <option key={item.id} value={item.id}>{item.label}</option>)}</optgroup>)}<option value="custom_component">Custom component type…</option></select></label>{selectedType && <div className="catalog-type-detail"><b>{selectedType.label}</b><span>{selectedType.description}</span>{selectedType.supportedModels.length > 0 ? selectedType.supportedModels.map((model) => <small key={`${model.modelId}:${model.version}`}>Available model binding: {model.modelId} · {model.version} · {model.scope}. Required: {model.requiredProperties.join(', ')}</small>) : <small>No compatible analysis model is currently registered for this type.</small>}</div>}{custom && <label>Custom type identifier<input value={customType} onChange={(e) => setCustomType(e.target.value)} placeholder="e.g. hydraulic_actuator" maxLength={160} required/><small>Saved as a normalized type identifier; it adds no model support.</small></label>}{catalogError && <div className="catalog-warning">The shared type library could not load. You can still save a user-defined type. {catalogError}</div>}{cell && <div className="model-inputs"><div className="model-input-heading"><Gauge size={15} /><div><b>PyBaMM SPM · Chen2020</b><span>Scoped single-cell discharge model</span></div><span className="binding-badge">BINDING</span></div><div className="model-input-grid"><label>Discharge current<input type="number" inputMode="decimal" min="0.05" max="100" step="any" value={currentA} onChange={(e) => setCurrentA(e.target.value)} placeholder="Required" required /><small>A · 0.05–100</small></label><label>Cutoff voltage<input type="number" inputMode="decimal" min="2.5" max="4" step="any" value={cutoffV} onChange={(e) => setCutoffV(e.target.value)} placeholder="Required" required /><small>V · 2.5–4.0</small></label></div><p className="model-disclaimer">These are user-declared inputs. The model applies to one cell; it does not simulate a battery pack or product system.</p></div>}<div className="catalog-scope-note">{catalogScope || 'Custom component classifications carry no performance values or design conclusions.'}</div><div className="modal-foot"><span>Only the entered values and source are saved to this project revision.</span><button type="button" className="secondary-button" onClick={onClose}>Cancel</button><button className="primary-button" disabled={busy || catalogLoading || !componentType || (custom && !customType.trim())}>{busy ? 'Saving…' : 'Add component'}<ArrowRight size={15} /></button></div></form></div>;
}

function TopologyModal({ busy, reality, onClose, onAddInterface, onAddConnection, onRemoveConnection, onRemoveInterface }: { busy: boolean; reality?: EngineeringReality; onClose: () => void; onAddInterface: (componentId: string, name: string, direction: 'in' | 'out' | 'bidirectional', domain: string) => Promise<boolean>; onAddConnection: (sourceId: string, targetId: string, connectionType: string) => Promise<boolean>; onRemoveConnection: (id: string) => void; onRemoveInterface: (id: string) => void }) {
  useModalFocus();
  const components = reality?.components ?? [];
  const interfaces = reality?.interfaces ?? [];
  const connections = reality?.connections ?? [];
  const [componentId, setComponentId] = useState(components[0]?.id ?? '');
  const [portName, setPortName] = useState('');
  const [direction, setDirection] = useState<'in' | 'out' | 'bidirectional'>('in');
  const [domain, setDomain] = useState('electrical_power');
  const [sourceId, setSourceId] = useState('');
  const [targetId, setTargetId] = useState('');
  const [connectionType, setConnectionType] = useState('direct');
  useEffect(() => {
    if (!components.some((item) => item.id === componentId)) setComponentId(components[0]?.id ?? '');
  }, [reality?.revision, componentId]);
  useEffect(() => {
    if (sourceId && !interfaces.some((item) => item.id === sourceId && item.direction !== 'in')) setSourceId('');
    if (targetId && !interfaces.some((item) => item.id === targetId && item.direction !== 'out')) setTargetId('');
  }, [reality?.revision, interfaces, sourceId, targetId]);
  const source = interfaces.find((item) => item.id === sourceId);
  const targetOptions = interfaces.filter((item) => item.direction !== 'out' && item.domain.toLowerCase() === source?.domain.toLowerCase() && item.id !== sourceId);
  const portLabel = (id: string) => {
    const port = interfaces.find((item) => item.id === id);
    const component = components.find((item) => item.id === port?.componentId);
    return `${component?.name ?? 'Unknown component'} · ${port?.name ?? id} (${port?.domain ?? 'unknown'})`;
  };
  const createInterface = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!componentId || !portName.trim() || !domain.trim()) return;
    if (await onAddInterface(componentId, portName, direction, domain)) setPortName('');
  };
  const createConnection = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!sourceId || !targetId || !source || source.domain.toLowerCase() !== interfaces.find((item) => item.id === targetId)?.domain.toLowerCase()) return;
    if (await onAddConnection(sourceId, targetId, connectionType)) setConnectionType('direct');
  };
  return <div className="modal-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}><section className="new-project-modal topology-modal" role="dialog" aria-modal="true" aria-labelledby="topology-title" tabIndex={-1}>
    <header className="modal-head"><div><span className="panel-overline">ENGINEERING REALITY · REV {reality?.revision ?? 0}</span><h2 id="topology-title">Interfaces & connections</h2><p>Describe component ports and connect matching physical or information domains. These links record topology; they do not create solver behavior.</p></div><button type="button" className="icon-button" onClick={onClose} aria-label="Close topology editor"><X size={17}/></button></header>
    {!components.length ? <div className="topology-empty"><div className="empty-glyph"><Box size={20}/></div><b>Add components before defining interfaces.</b><span>Ports belong to a component in the saved engineering reality.</span></div> : <>
      <div className="topology-summary"><span><b>{components.length}</b> components</span><span><b>{interfaces.length}</b> interfaces</span><span><b>{connections.length}</b> connections</span></div>
      <div className="topology-editor-grid">
        <form className="topology-form" onSubmit={(event) => void createInterface(event)}><div className="topology-form-title"><span>DEFINE AN INTERFACE</span><small>PORT ON A COMPONENT</small></div>
          <label>Component<select value={componentId} onChange={(event) => setComponentId(event.target.value)}>{components.map((item) => <option key={item.id} value={item.id}>{item.name} · {item.componentType}</option>)}</select></label>
          <label>Interface name<input value={portName} onChange={(event) => setPortName(event.target.value)} required maxLength={160} placeholder="e.g. DC power out"/></label>
          <div className="topology-form-row"><label>Direction<select value={direction} onChange={(event) => setDirection(event.target.value as typeof direction)}><option value="in">Input</option><option value="out">Output</option><option value="bidirectional">Bidirectional</option></select></label><label>Domain<input value={domain} onChange={(event) => setDomain(event.target.value)} required maxLength={80} placeholder="electrical_power"/></label></div>
          <button className="secondary-button" disabled={busy || !componentId || !portName.trim() || !domain.trim()}><Plus size={13}/> Add interface</button>
        </form>
        <form className="topology-form" onSubmit={(event) => void createConnection(event)}><div className="topology-form-title"><span>CONNECT INTERFACES</span><small>DOMAIN MUST MATCH</small></div>
          <label>Source · output<select value={sourceId} onChange={(event) => { setSourceId(event.target.value); setTargetId(''); }}><option value="">Choose an output interface</option>{interfaces.filter((item) => item.direction !== 'in').map((item) => <option key={item.id} value={item.id}>{portLabel(item.id)}</option>)}</select></label>
          <label>Target · input<select value={targetId} onChange={(event) => setTargetId(event.target.value)}><option value="">{source ? targetOptions.length ? 'Choose a matching input interface' : 'No matching input interface yet' : 'Choose a source first'}</option>{targetOptions.map((item) => <option key={item.id} value={item.id}>{portLabel(item.id)}</option>)}</select></label>
          <label>Connection kind<input value={connectionType} onChange={(event) => setConnectionType(event.target.value)} maxLength={100} placeholder={source?.domain || 'direct'}/></label>
          <button className="primary-button" disabled={busy || !sourceId || !targetId}><Plus size={13}/> Save connection</button>
        </form>
      </div>
      <div className="topology-records"><div className="topology-record-heading"><span>DECLARED TOPOLOGY</span><small>Changes save as a new reality revision</small></div>
        {connections.length > 0 && <div className="topology-record-group"><b>Connections</b>{connections.map((connection) => <div className="topology-record" key={connection.id}><span className="topology-record-mark"><GitBranch size={13}/></span><div><b>{portLabel(connection.sourceInterfaceId)}</b><small>to {portLabel(connection.targetInterfaceId)} · {connection.connectionType}</small></div><button type="button" className="icon-button" disabled={busy} onClick={() => onRemoveConnection(connection.id)} aria-label="Remove connection"><X size={13}/></button></div>)}</div>}
        <div className="topology-record-group"><b>Interfaces</b>{interfaces.length ? interfaces.map((port) => <div className="topology-record" key={port.id}><span className="topology-record-mark"><ArrowRight size={13}/></span><div><b>{portLabel(port.id)}</b><small>{port.direction.toUpperCase()} · {port.domain}</small></div><button type="button" className="icon-button" disabled={busy || connections.some((item) => item.sourceInterfaceId === port.id || item.targetInterfaceId === port.id)} onClick={() => onRemoveInterface(port.id)} aria-label="Remove interface"><X size={13}/></button></div>) : <small className="topology-no-records">No interfaces defined.</small>}</div>
      </div>
    </>}
    <footer className="modal-foot"><span>Ports and wires are topology declarations. They do not imply compatible physics models or simulation.</span><button type="button" className="secondary-button" onClick={onClose}>Done</button></footer>
  </section></div>;
}

function RequirementModal({ busy, onClose, onCreate }: { busy: boolean; onClose: () => void; onCreate: (statement: string, priority: EngineeringRequirement['priority'], rule: VerificationRule) => void }) {
  useModalFocus();
  const [statement, setStatement] = useState(''); const [priority, setPriority] = useState<EngineeringRequirement['priority']>('must');
  const [metric, setMetric] = useState<VerificationRule['metric']>('discharged_capacity_ah'); const [operator, setOperator] = useState<VerificationRule['operator']>('>='); const [threshold, setThreshold] = useState('');
  const unit = metric === 'duration_h' ? 'h' : metric === 'final_voltage_v' ? 'V' : 'Ah';
  return <div className="modal-backdrop" onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}><form className="new-project-modal requirement-modal" role="dialog" aria-modal="true" aria-label="Add a measurable requirement" tabIndex={-1} onSubmit={(e) => { e.preventDefault(); onCreate(statement, priority, { metric, operator, threshold: Number(threshold), unit }); }}><div className="modal-head"><div><span className="panel-overline">DESIGN REQUIREMENT</span><h2>State a measurable condition</h2><p>Saved as a draft. A reviewer must accept it before it can be checked against a model run.</p></div><button type="button" className="icon-button" onClick={onClose}><X size={17}/></button></div><label>Requirement statement<input value={statement} onChange={(e) => setStatement(e.target.value)} maxLength={500} required placeholder="e.g. The cell shall deliver at least 2 Ah" autoFocus/></label><div className="requirement-form-grid"><label>Priority<select value={priority} onChange={(e) => setPriority(e.target.value as EngineeringRequirement['priority'])}><option value="must">Must</option><option value="should">Should</option><option value="could">Could</option></select></label><label>Check metric<select value={metric} onChange={(e) => setMetric(e.target.value as VerificationRule['metric'])}><option value="discharged_capacity_ah">Discharged capacity (Ah)</option><option value="duration_h">Discharge duration (h)</option><option value="final_voltage_v">Final voltage (V)</option></select></label></div><div className="requirement-form-grid"><label>Condition<select value={operator} onChange={(e) => setOperator(e.target.value as VerificationRule['operator'])}><option value=">=">At least (≥)</option><option value="<=">At most (≤)</option><option value=">">Greater than (&gt;)</option><option value="<">Less than (&lt;)</option><option value="==">Exactly (=)</option></select></label><label>Threshold ({unit})<input type="number" inputMode="decimal" min="0" step="any" value={threshold} onChange={(e) => setThreshold(e.target.value)} required placeholder={`Value in ${unit}`}/></label></div><div className="verify-form-note">This starter integration can check these three metrics only for a recorded PyBaMM single-cell discharge run.</div><div className="modal-foot"><span>Saving changes the project revision.</span><button type="button" className="secondary-button" onClick={onClose}>Cancel</button><button className="primary-button" disabled={busy || !statement.trim() || !threshold || Number(threshold) < 0}>{busy ? 'Saving…' : 'Save draft'}<ArrowRight size={15}/></button></div></form></div>;
}

function EVConfigurationModal({ busy, existing, onClose, onSave }: { busy: boolean; existing?: EVBatteryVehicleConfiguration; onClose: () => void; onSave: (configuration: EVBatteryVehicleConfiguration) => void }) {
  useModalFocus();
  const [values, setValues] = useState<Record<string, string>>(() => ({
    vehicleName: existing?.vehicleName ?? '', vehicleMassKg: existing?.vehicleMassKg?.toString() ?? '', dragCoefficient: existing?.dragCoefficient?.toString() ?? '', frontalAreaM2: existing?.frontalAreaM2?.toString() ?? '', rollingResistanceCoefficient: existing?.rollingResistanceCoefficient?.toString() ?? '', drivetrainEfficiency: existing?.drivetrainEfficiency?.toString() ?? '', accessoryPowerW: existing?.accessoryPowerW?.toString() ?? '',
    seriesCells: existing?.batteryPack.seriesCells?.toString() ?? '', parallelCells: existing?.batteryPack.parallelCells?.toString() ?? '', cellCapacityAh: existing?.batteryPack.cellCapacityAh?.toString() ?? '', cellNominalVoltageV: existing?.batteryPack.cellNominalVoltageV?.toString() ?? '', cellCutoffVoltageV: existing?.batteryPack.cellCutoffVoltageV?.toString() ?? '', cellMaxContinuousCurrentA: existing?.batteryPack.cellMaxContinuousCurrentA?.toString() ?? '', initialSoc: existing?.batteryPack.initialSoc?.toString() ?? '',
  }));
  const [cycle, setCycle] = useState(existing ? existing.driveCycle.map((p) => `${p.timeS}, ${p.speedMS}, ${p.roadGradePercent}`).join('\n') : '');
  const [formError, setFormError] = useState('');
  const field = (key: string, label: string, unit: string, limits: string, type = 'number') => <label>{label}<div className="ev-input-with-unit"><input type={type} value={values[key] ?? ''} min="0" step="any" onChange={(e) => setValues((current) => ({ ...current, [key]: e.target.value }))} placeholder="Required" required/><small>{unit}</small></div><small className="ev-limit">{limits}</small></label>;
  const set = (key: string, value: string) => setValues((current) => ({ ...current, [key]: value }));
  const submit = () => {
    try {
      const lines = cycle.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
      const driveCycle: EVDriveCyclePoint[] = lines.map((line, index) => { const parts = line.split(',').map((value) => Number(value.trim())); if (parts.length < 2 || parts.length > 3 || parts.some((value) => !Number.isFinite(value))) throw new Error(`Drive-cycle row ${index + 1} must be time, speed, grade.`); return { timeS: parts[0], speedMS: parts[1], roadGradePercent: parts[2] ?? 0 }; });
      if (driveCycle.length < 2 || driveCycle[0].timeS !== 0 || driveCycle.some((point, index) => point.timeS < 0 || point.speedMS < 0 || Math.abs(point.roadGradePercent) > 20 || (index > 0 && point.timeS - driveCycle[index - 1].timeS < 1))) throw new Error('Use 2–200 points. Start at 0 seconds; times must increase by at least 1 second. Speed must be non-negative; grade ±20% max.');
      const number = (key: string) => { const value = Number(values[key]); if (!Number.isFinite(value)) throw new Error(`Enter a valid value for ${key.replace(/[A-Z]/g, (letter) => ` ${letter.toLowerCase()}`)}.`); return value; };
      const configuration: EVBatteryVehicleConfiguration = { schema: 'helqix.ev-battery-vehicle.v1', vehicleName: values.vehicleName.trim(), vehicleMassKg: number('vehicleMassKg'), dragCoefficient: number('dragCoefficient'), frontalAreaM2: number('frontalAreaM2'), rollingResistanceCoefficient: number('rollingResistanceCoefficient'), drivetrainEfficiency: number('drivetrainEfficiency'), accessoryPowerW: number('accessoryPowerW'), batteryPack: { seriesCells: number('seriesCells'), parallelCells: number('parallelCells'), cellCapacityAh: number('cellCapacityAh'), cellNominalVoltageV: number('cellNominalVoltageV'), cellCutoffVoltageV: number('cellCutoffVoltageV'), cellMaxContinuousCurrentA: number('cellMaxContinuousCurrentA'), initialSoc: number('initialSoc') }, driveCycle, modelId: 'pybamm.lithium-ion.spm', parameterSet: 'Chen2020' };
      if (!configuration.vehicleName) throw new Error('Enter a vehicle name.');
      if (configuration.batteryPack.cellCutoffVoltageV >= configuration.batteryPack.cellNominalVoltageV) throw new Error('Cell cutoff voltage must be lower than nominal voltage.');
      onSave(configuration);
    } catch (e) { setFormError(e instanceof Error ? e.message : 'Check the vehicle model inputs.'); }
  };
  return <div className="modal-backdrop" onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}><form className="new-project-modal ev-config-modal" role="dialog" aria-modal="true" aria-label="Vehicle and battery model configuration" tabIndex={-1} onSubmit={(e) => { e.preventDefault(); submit(); }}><div className="modal-head"><div><span className="panel-overline">SAVED ENGINEERING MODEL</span><h2>Vehicle & battery system</h2><p>Explicit inputs for the bounded drive-cycle adapter. These values are not inferred from the project description.</p></div><button type="button" className="icon-button" onClick={onClose}><X size={17}/></button></div>
    <h3 className="ev-section-title"><CarFront size={15}/> Vehicle & road-load properties</h3><div className="ev-config-grid"><label>Vehicle name<input type="text" maxLength={120} value={values.vehicleName} onChange={(e) => set('vehicleName', e.target.value)} placeholder="e.g. Test vehicle A" required/></label>{field('vehicleMassKg','Running mass','kg','> 100 · includes battery and occupants')}{field('dragCoefficient','Drag coefficient','Cd','0.05–1.5')}{field('frontalAreaM2','Frontal area','m²','0.3–12')}{field('rollingResistanceCoefficient','Rolling resistance','Crr','0.001–0.08')}{field('drivetrainEfficiency','Drivetrain efficiency','fraction','> 0.5–1')}{field('accessoryPowerW','Accessory load','W','0–10,000')}</div>
    <h3 className="ev-section-title"><Box size={15}/> Battery pack definition <small>identical cells assumed</small></h3><div className="ev-config-grid">{field('seriesCells','Series cells','cells','1–200')}{field('parallelCells','Parallel strings','cells','1–100')}{field('cellCapacityAh','Cell capacity','Ah','0.5–50')}{field('cellNominalVoltageV','Cell nominal voltage','V','> 2.5–4.5')}{field('cellCutoffVoltageV','Cell cutoff voltage','V','2.5–4.3 · below nominal')}{field('cellMaxContinuousCurrentA','Cell current limit','A','> 0–500')}{field('initialSoc','Initial state of charge','fraction','0–1')}</div>
    <h3 className="ev-section-title"><Activity size={15}/> Drive cycle <small>one row per time point</small></h3><label className="drive-cycle-label">Enter one point per line: <code>time_seconds, speed_m_per_second, road_grade_percent</code><textarea value={cycle} onChange={(e) => setCycle(e.target.value)} rows={5} required spellCheck={false}/><small>Must begin at 0 s; at least 1 s between points; up to 200 points. The vehicle follows this prescribed speed trace.</small></label>
    <div className="ev-model-limit"><ShieldCheck size={14}/><span>Analysis couples a calculated longitudinal road load to a Chen2020 single-cell model scaled across identical series/parallel counts. It does not simulate free-running acceleration, regeneration, cell mismatch, thermal gradients, BMS behavior, or a measured battery chemistry.</span></div>
    {formError && <div className="form-error">{formError}</div>}<div className="modal-foot"><span>Saving creates a new project revision.</span><button type="button" className="secondary-button" onClick={onClose}>Cancel</button><button className="primary-button" disabled={busy}>{busy ? 'Saving…' : existing ? 'Update model' : 'Save model'}<ArrowRight size={15}/></button></div></form></div>;
}

