import React, { useState, useEffect, useRef } from 'react';
import { useStudio } from '../../stores';
import { MockEngineClient } from '../../mock/MockEngineClient';
import { RunFrame, Value } from '../../contracts';
import { MockTag, ValueView, Panel } from '../../components/ui/UiKit';

export interface SimulateStageProps {
  onInvestigateFailure: () => void;
  onForkBranch?: () => void;
}

export const SimulateStage: React.FC<SimulateStageProps> = ({
  onInvestigateFailure,
  onForkBranch
}) => {
  const { session, playback } = useStudio();
  const [engine] = useState(() => MockEngineClient);

  // Simulation Time & Playback
  const [currentTime, setCurrentTime] = useState<number>(0);
  const [totalDuration] = useState<number>(12.0);
  const [isPlaying, setIsPlaying] = useState<boolean>(true);
  const [speedMultiplier, setSpeedMultiplier] = useState<number>(1.0);
  const [activeScenarioId, setActiveScenarioId] = useState<string>('scen-01');

  // Frame Telemetry Data
  const [currentFrame, setCurrentFrame] = useState<RunFrame | null>(null);
  const [isStale, setIsStale] = useState<boolean>(false);
  const [isDraftParam, setIsDraftParam] = useState<boolean>(false);
  const [ambientTempDraft, setAmbientTempDraft] = useState<number>(25.0);
  const [loadTorqueDraft, setLoadTorqueDraft] = useState<number>(53.4);
  const [isRerunning, setIsRerunning] = useState<boolean>(false);

  // Thermal failure flag at t >= 8.42s
  const [isFailed, setIsFailed] = useState<boolean>(false);
  const [showEventStream, setShowEventStream] = useState<boolean>(false);
  const [bottomTab, setBottomTab] = useState<'telemetry' | 'events' | 'inspector'>('telemetry');

  // Scenarios definition (Section 6.10: 01 to 05 with Current and Baseline tags)
  const scenarios = [
    { id: 'scen-01', num: '01', title: 'High-Load Dynamic Cycle', tag: 'Current', duration: '12.0 s', load: '85.4 N·m' },
    { id: 'scen-02', num: '02', title: 'Nominal Continuous Rated', tag: 'Baseline', duration: '120.0 s', load: '32.0 N·m' },
    { id: 'scen-03', num: '03', title: 'Thermal Soak & Cool-down', tag: '', duration: '60.0 s', load: '0.0 N·m' },
    { id: 'scen-04', num: '04', title: 'Harmonic Torque Pulsing', tag: '', duration: '30.0 s', load: '60.0 N·m' },
    { id: 'scen-05', num: '05', title: 'Cold-Start Rapid Ramp', tag: '', duration: '15.0 s', load: '45.0 N·m' }
  ];

  // Fetch deterministic frame whenever currentTime updates
  useEffect(() => {
    let unmounted = false;
    async function loadFrame() {
      const frame = await engine.getFrame('run-1', currentTime);
      if (!unmounted) {
        setCurrentFrame(frame);
        if (currentTime >= 8.42) {
          setIsFailed(true);
        } else {
          setIsFailed(false);
        }
      }
    }
    loadFrame();
    return () => {
      unmounted = true;
    };
  }, [currentTime, engine]);

  // Simulation playback loop
  useEffect(() => {
    if (!isPlaying) return;

    const interval = setInterval(() => {
      setCurrentTime(prev => {
        const next = prev + 0.1 * speedMultiplier;
        if (next >= totalDuration) {
          setIsPlaying(false);
          return totalDuration;
        }
        return parseFloat(next.toFixed(2));
      });
    }, 100);

    return () => clearInterval(interval);
  }, [isPlaying, speedMultiplier, totalDuration]);

  // Handle Scrubbing (reproducible state replay - Section 6.10)
  const handleScrub = (time: number) => {
    const clamped = Math.max(0, Math.min(totalDuration, parseFloat(time.toFixed(2))));
    setCurrentTime(clamped);
  };

  // Handle Re-running when parameters are edited
  const handleApplyRerun = async () => {
    setIsRerunning(true);
    setIsStale(true);
    await new Promise(r => setTimeout(r, 600));
    setIsStale(false);
    setIsDraftParam(false);
    setIsRerunning(false);
  };

  const tempVal = currentFrame?.values.stator_winding_temperature?.v ?? 25.0;
  const radSVal = currentFrame?.values.rotor_angular_velocity?.v ?? 314.2;
  const currentVal = currentFrame?.values.phase_rms_current?.v ?? 42.0;
  const torqueVal = currentFrame?.values.shaft_torque_load?.v ?? 32.0;

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-transparent z-10 pointer-events-none select-none">
      {/* 1. TOP SIMULATION BAR */}
      <div className="h-11 px-5 flex items-center justify-between border-b border-[#22395A]/80 bg-[#0A1422]/90 backdrop-blur-md pointer-events-auto shrink-0">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <span className="text-xs font-semibold text-[#F2F6FB]">Simulate Stage</span>
            <span className="px-2 py-0.5 rounded text-[10px] font-mono font-bold tracking-wider bg-blue-500/15 border border-blue-500/40 text-[#22D3EE]">
              Run 19 · Active
            </span>
            <MockTag />
          </div>

          <span className="text-[#22395A]">|</span>

          {/* Scale Notice (Section 6.10) */}
          <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-purple-500/15 border border-purple-500/30 text-purple-300">
            Visual speed scaled 1:10
          </span>

          {isFailed && (
            <span className="px-2 py-0.5 rounded text-[10px] font-mono font-bold uppercase tracking-wider bg-red-500/20 border border-red-500/60 text-red-400 animate-pulse flex items-center gap-1">
              <span>⚠</span>
              <span>Thermal limit breached at 8.42 s</span>
            </span>
          )}
        </div>

        {/* Right Status / Actions */}
        <div className="flex items-center gap-2.5">
          {isFailed ? (
            <button
              onClick={onInvestigateFailure}
              className="px-4 py-1.5 rounded-[8px] text-xs font-bold text-white bg-red-600 hover:bg-red-500 shadow-[0_0_20px_rgba(239,68,68,0.5)] transition-all flex items-center gap-1.5"
            >
              <span>🔍</span>
              <span>Investigate failure →</span>
            </button>
          ) : (
            <div className="text-[11px] font-mono text-[#7F93AF]">
              Solver: <strong className="text-white">RK4 Bitwise</strong> (dt: 0.001 s)
            </div>
          )}
        </div>
      </div>

      {/* 2. MAIN SIMULATION VIEWPORT */}
      <div className="flex-1 flex overflow-hidden">
        {/* LEFT PANEL: SCENARIOS & RUN SETTINGS (Section 6.10) */}
        <div className="w-72 border-r border-[#22395A]/80 bg-[#0A1422]/85 backdrop-blur-md flex flex-col justify-between shrink-0 pointer-events-auto p-3.5 space-y-3 overflow-y-auto">
          {/* Scenarios List */}
          <div className="space-y-2">
            <div className="flex items-center justify-between pb-1 border-b border-[#22395A]">
              <span className="text-xs font-bold text-[#F2F6FB] uppercase tracking-wider font-mono">
                Scenarios (05)
              </span>
              <span className="text-[10px] font-mono text-[#22D3EE]">Dynamic</span>
            </div>

            <div className="space-y-1.5">
              {scenarios.map(scen => {
                const isSelected = activeScenarioId === scen.id;
                return (
                  <div
                    key={scen.id}
                    onClick={() => setActiveScenarioId(scen.id)}
                    className={`p-2.5 rounded-[8px] border cursor-pointer transition-all ${
                      isSelected
                        ? 'bg-[#2F7BFF]/20 border-[#2F7BFF] shadow-[0_0_12px_rgba(47,123,255,0.25)]'
                        : 'bg-[#0E1B2E]/60 border-[#22395A] hover:bg-[#13253D]/80 text-[#D6E2F3]'
                    }`}
                  >
                    <div className="flex items-center justify-between text-xs">
                      <div className="flex items-center gap-1.5">
                        <span className="font-mono text-[10px] text-slate-400">{scen.num}</span>
                        <span className="font-semibold">{scen.title}</span>
                      </div>
                      {scen.tag && (
                        <span
                          className={`text-[9px] font-mono px-1 rounded ${
                            scen.tag === 'Current'
                              ? 'bg-[#22D3EE]/20 text-[#22D3EE]'
                              : 'bg-slate-700 text-slate-300'
                          }`}
                        >
                          {scen.tag}
                        </span>
                      )}
                    </div>
                    <div className="flex items-center justify-between text-[10px] font-mono text-[#7F93AF] mt-1">
                      <span>Duration: {scen.duration}</span>
                      <span>Load: {scen.load}</span>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Run Settings Summary Card (Section 6.10) */}
          <div className="p-3 rounded-[10px] bg-[#0E1B2E] border border-[#22395A] space-y-1.5 font-mono text-xs">
            <div className="text-[11px] font-bold text-[#F2F6FB] uppercase tracking-wider">
              Run Settings Summary
            </div>
            <div className="flex items-center justify-between text-[10px] text-[#7F93AF]">
              <span>Solver Engine:</span>
              <span className="text-white">RK4 Bitwise</span>
            </div>
            <div className="flex items-center justify-between text-[10px] text-[#7F93AF]">
              <span>Internal dt:</span>
              <span className="text-white">0.001 s</span>
            </div>
            <div className="flex items-center justify-between text-[10px] text-[#7F93AF]">
              <span>Run Seed:</span>
              <span className="text-[#22D3EE]">8421</span>
            </div>
            <div className="flex items-center justify-between text-[10px] text-[#7F93AF]">
              <span>Determinism:</span>
              <span className="text-[#4ADE80]">Bitwise identical</span>
            </div>
          </div>
        </div>

        {/* CENTER REALITY CANVAS HUD & CAMERA CONTROLS */}
        <div className="flex-1 relative flex flex-col justify-between p-4 pointer-events-none">
          {/* Top Floating HUD: 4 Live Values with Badges (Section 6.10) */}
          <div className="flex items-start justify-between pointer-events-auto">
            <div className="grid grid-cols-2 lg:grid-cols-4 gap-2 max-w-2xl">
              {/* Temperature */}
              <div
                className={`p-2.5 rounded-[10px] border backdrop-blur-md transition-all ${
                  tempVal >= 120.0
                    ? 'bg-red-950/80 border-red-500 shadow-[0_0_20px_rgba(239,68,68,0.4)]'
                    : 'bg-[#0A1422]/85 border-[#22395A]'
                }`}
              >
                <div className="flex items-center justify-between text-[10px] text-[#7F93AF] font-mono">
                  <span>Winding Temp</span>
                  <div className="flex items-center gap-1">
                    <MockTag />
                    <span
                      className={`px-1 py-0.2 rounded text-[8px] uppercase ${
                        tempVal >= 120.0 ? 'bg-red-500/20 text-red-300' : 'bg-emerald-500/20 text-emerald-300'
                      }`}
                    >
                      {tempVal >= 120.0 ? 'unverified' : 'verified'}
                    </span>
                  </div>
                </div>
                <div className="flex items-baseline gap-1 mt-0.5 font-mono">
                  <span
                    className={`text-lg font-bold ${
                      tempVal >= 120.0 ? 'text-red-400' : 'text-white'
                    }`}
                  >
                    {tempVal}
                  </span>
                  <span className="text-xs text-[#22D3EE]">°C</span>
                  {tempVal >= 120.0 && <span className="text-[10px] text-red-400 ml-auto font-sans font-bold">! LIMIT</span>}
                </div>
              </div>

              {/* Angular Velocity */}
              <div className="p-2.5 rounded-[10px] bg-[#0A1422]/85 border border-[#22395A] backdrop-blur-md">
                <div className="flex items-center justify-between text-[10px] text-[#7F93AF] font-mono">
                  <span>Rotor Velocity</span>
                  <MockTag />
                </div>
                <div className="flex items-baseline gap-1 mt-0.5 font-mono">
                  <span className="text-lg font-bold text-white">{radSVal}</span>
                  <span className="text-xs text-[#22D3EE]">rad/s</span>
                </div>
              </div>

              {/* RMS Current */}
              <div className="p-2.5 rounded-[10px] bg-[#0A1422]/85 border border-[#22395A] backdrop-blur-md">
                <div className="flex items-center justify-between text-[10px] text-[#7F93AF] font-mono">
                  <span>Phase RMS Current</span>
                  <MockTag />
                </div>
                <div className="flex items-baseline gap-1 mt-0.5 font-mono">
                  <span className="text-lg font-bold text-white">{currentVal}</span>
                  <span className="text-xs text-[#22D3EE]">A</span>
                </div>
              </div>

              {/* Torque Load */}
              <div className="p-2.5 rounded-[10px] bg-[#0A1422]/85 border border-[#22395A] backdrop-blur-md">
                <div className="flex items-center justify-between text-[10px] text-[#7F93AF] font-mono">
                  <span>Shaft Torque</span>
                  <MockTag />
                </div>
                <div className="flex items-baseline gap-1 mt-0.5 font-mono">
                  <span className="text-lg font-bold text-white">{torqueVal}</span>
                  <span className="text-xs text-[#22D3EE]">N·m</span>
                </div>
              </div>
            </div>

            {/* Camera Presets & Follow Camera Mode (Section 6.10) */}
            <div className="p-1.5 rounded-[10px] bg-[#0A1422]/85 backdrop-blur-md border border-[#22395A] flex items-center gap-1 text-xs font-mono">
              {(['Perspective', 'Top', 'Front', 'Right'] as const).map(p => (
                <button
                  key={p}
                  onClick={() => session.setCameraPreset(p)}
                  className={`px-2 py-0.5 rounded-[6px] ${
                    session.cameraPreset === p
                      ? 'bg-[#2F7BFF]/30 text-[#22D3EE] border border-[#2F7BFF]/40'
                      : 'text-[#7F93AF] hover:text-white'
                  }`}
                >
                  {p}
                </button>
              ))}
              <div className="w-[1px] h-3 bg-[#22395A] mx-0.5"></div>
              <span className="text-[10px] text-cyan-400 px-1 font-bold">Follow 🎥</span>
            </div>
          </div>

          {/* Failure Alert Banner (Section 6.10 & 6.11) */}
          {isFailed && (
            <div className="p-3.5 rounded-[12px] bg-red-950/90 border border-red-500/80 backdrop-blur-md max-w-lg mx-auto pointer-events-auto text-center space-y-2 animate-fade-in shadow-[0_0_30px_rgba(239,68,68,0.4)]">
              <div className="flex items-center justify-center gap-2 text-red-400 font-bold text-xs uppercase tracking-wider font-mono">
                <span className="w-2.5 h-2.5 rounded-full bg-red-500 animate-ping"></span>
                <span>Winding temperature above limit at 8.42 s</span>
              </div>
              <p className="text-xs text-[#F2F6FB]">
                Stator temperature reached <strong>{tempVal} °C</strong> (limit 120.0 °C). Thermal protection circuit tripped.
              </p>
              <div className="flex items-center justify-center gap-2 pt-1">
                <button
                  onClick={onInvestigateFailure}
                  className="px-5 py-1.5 rounded-[8px] text-xs font-semibold text-white bg-red-600 hover:bg-red-500 shadow-[0_0_20px_rgba(239,68,68,0.5)] transition-all"
                >
                  Investigate failure →
                </button>
              </div>
            </div>
          )}

          {/* Bottom Dock: Telemetry Graphs & Event Stream */}
          <div className="p-3 rounded-[14px] bg-[#0A1422]/90 backdrop-blur-md border border-[#22395A] pointer-events-auto space-y-2">
            <div className="flex items-center justify-between border-b border-[#22395A]/80 pb-2">
              <div className="flex items-center gap-2">
                {(['telemetry', 'events', 'inspector'] as const).map(tab => (
                  <button
                    key={tab}
                    onClick={() => setBottomTab(tab)}
                    className={`px-3 py-1 rounded-[6px] text-xs font-mono font-medium transition-all ${
                      bottomTab === tab
                        ? 'bg-[#2F7BFF]/30 text-[#22D3EE] font-semibold border border-[#2F7BFF]/50'
                        : 'text-[#7F93AF] hover:text-white'
                    }`}
                  >
                    <span className="capitalize">{tab}</span>
                    {tab === 'events' && isFailed && (
                      <span className="ml-1.5 px-1 py-0.2 rounded-full bg-red-500/20 text-red-400 text-[9px]">
                        Trip
                      </span>
                    )}
                  </button>
                ))}
              </div>

              <div className="flex items-center gap-2 text-[11px] font-mono text-[#7F93AF]">
                <span>t = <strong className="text-white">{currentTime.toFixed(2)} s</strong> / {totalDuration} s</span>
                {isStale && (
                  <span className="px-1.5 py-0.2 rounded text-[9px] bg-amber-500/20 text-amber-300 font-bold">
                    STALE
                  </span>
                )}
              </div>
            </div>

            {/* Content per bottom tab */}
            {bottomTab === 'telemetry' && (
              <div className="grid grid-cols-2 gap-3 max-h-32">
                {/* Temperature Curve vs Threshold */}
                <div className="p-2 rounded-[8px] bg-[#0E1B2E] border border-[#22395A] space-y-1">
                  <div className="flex items-center justify-between text-[10px] font-mono">
                    <span className="text-[#7F93AF]">Thermal Curve (Limit: 120.0 °C)</span>
                    <span className={tempVal >= 120 ? 'text-red-400 font-bold' : 'text-[#4ADE80]'}>
                      {tempVal} °C
                    </span>
                  </div>
                  {/* SVG Sparkline Graph */}
                  <div className="h-14 w-full relative">
                    <svg className="w-full h-full overflow-visible" viewBox="0 0 200 50">
                      {/* Threshold 120C Red Line */}
                      <line x1="0" y1="20" x2="200" y2="20" stroke="#ef4444" strokeWidth="1" strokeDasharray="3 3" />
                      <text x="5" y="16" fill="#ef4444" fontSize="8" fontFamily="monospace">120 °C Limit</text>
                      {/* Dynamic Rising Temperature Polyline */}
                      <polyline
                        fill="none"
                        stroke="#22d3ee"
                        strokeWidth="2"
                        points={`0,45 50,42 100,35 140,20 180,8 200,4`}
                      />
                      {/* Current Time Cursor Marker */}
                      <circle
                        cx={(currentTime / totalDuration) * 200}
                        cy={Math.max(4, 45 - (tempVal - 25) * 0.4)}
                        r="3"
                        fill={tempVal >= 120 ? '#ef4444' : '#22d3ee'}
                        className="animate-pulse"
                      />
                    </svg>
                  </div>
                </div>

                {/* Torque & Current Curve */}
                <div className="p-2 rounded-[8px] bg-[#0E1B2E] border border-[#22395A] space-y-1">
                  <div className="flex items-center justify-between text-[10px] font-mono">
                    <span className="text-[#7F93AF]">Torque (N·m) & Phase Current (A)</span>
                    <span className="text-[#22D3EE]">{torqueVal} N·m / {currentVal} A</span>
                  </div>
                  <div className="h-14 w-full relative">
                    <svg className="w-full h-full overflow-visible" viewBox="0 0 200 50">
                      <polyline fill="none" stroke="#8b5cf6" strokeWidth="1.5" points="0,40 70,40 75,15 200,15" />
                      <polyline fill="none" stroke="#38bdf8" strokeWidth="1.5" points="0,32 70,32 75,22 200,22" />
                      <text x="5" y="48" fill="#8b5cf6" fontSize="8" fontFamily="monospace">Torque Step</text>
                      <text x="120" y="48" fill="#38bdf8" fontSize="8" fontFamily="monospace">Current</text>
                    </svg>
                  </div>
                </div>
              </div>
            )}

            {bottomTab === 'events' && (
              <div className="max-h-28 overflow-y-auto space-y-1 font-mono text-[10px]">
                <div className="text-[#7F93AF]">t = 0.00 s · Solver initialized seed: 8421</div>
                <div className="text-[#7F93AF]">t = 0.50 s · Nominal speed reached: 314.2 rad/s (3000 RPM)</div>
                <div className="text-[#22D3EE]">t = 4.20 s · Applied dynamic torque step: +53.4 N·m</div>
                {currentTime >= 8.42 && (
                  <div className="text-red-400 font-bold">
                    t = 8.42 s · Thermal limit breached: 120.0 °C exceeded (Trip triggered)
                  </div>
                )}
              </div>
            )}

            {bottomTab === 'inspector' && (
              <div className="grid grid-cols-4 gap-2 font-mono text-xs max-h-28">
                <div className="p-2 rounded bg-[#0E1B2E] border border-[#22395A]">
                  <span className="text-[10px] text-[#7F93AF]">State Variables</span>
                  <div className="text-white font-bold">14 active</div>
                </div>
                <div className="p-2 rounded bg-[#0E1B2E] border border-[#22395A]">
                  <span className="text-[10px] text-[#7F93AF]">Energy Inflow</span>
                  <div className="text-[#22D3EE] font-bold">3.12 kW</div>
                </div>
                <div className="p-2 rounded bg-[#0E1B2E] border border-[#22395A]">
                  <span className="text-[10px] text-[#7F93AF]">Dissipated Heat</span>
                  <div className="text-red-400 font-bold">480.2 W</div>
                </div>
                <div className="p-2 rounded bg-[#0E1B2E] border border-[#22395A]">
                  <span className="text-[10px] text-[#7F93AF]">Joule Efficiency</span>
                  <div className="text-[#4ADE80] font-bold">89.4 %</div>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* RIGHT PANEL: ENVIRONMENT & DRAFT PARAMETERS (Section 6.10) */}
        <div className="w-72 border-l border-[#22395A]/80 bg-[#0A1422]/85 backdrop-blur-md flex flex-col justify-between shrink-0 pointer-events-auto p-3.5 space-y-3 overflow-y-auto">
          {/* Environment Card (Only fields scenario supports - Section 6.10) */}
          <div className="space-y-2">
            <div className="flex items-center justify-between pb-1 border-b border-[#22395A]">
              <span className="text-xs font-bold text-[#F2F6FB] uppercase tracking-wider font-mono">
                Environment
              </span>
              <span className="text-[10px] font-mono text-[#22D3EE]">Chamber</span>
            </div>

            {/* Weather / Chamber chips */}
            <div className="flex items-center gap-1 text-[10px] font-mono">
              {(['Clear', 'Cloudy', 'Fog'] as const).map(w => (
                <button
                  key={w}
                  className={`px-2 py-0.5 rounded border ${
                    w === 'Clear'
                      ? 'bg-[#2F7BFF]/20 border-[#2F7BFF] text-cyan-300'
                      : 'bg-[#13253D] border-[#22395A] text-[#7F93AF]'
                  }`}
                >
                  {w}
                </button>
              ))}
            </div>

            <div className="space-y-1.5 text-xs font-mono">
              <div className="p-2 rounded bg-[#0E1B2E] border border-[#22395A] flex items-center justify-between">
                <span className="text-[#7F93AF]">Ambient Temp:</span>
                <span className="text-white">25.0 °C</span>
              </div>
              <div className="p-2 rounded bg-[#0E1B2E] border border-[#22395A] flex items-center justify-between">
                <span className="text-[#7F93AF]">Atm Pressure:</span>
                <span className="text-white">101.3 kPa</span>
              </div>
              <div className="p-2 rounded bg-[#0E1B2E] border border-[#22395A] flex items-center justify-between">
                <span className="text-[#7F93AF]">Coolant In:</span>
                <span className="text-white">20.0 °C</span>
              </div>
            </div>
          </div>

          {/* Scenario Parameters with Draft/Stale preview rules (Section 6.10) */}
          <div className="p-3 rounded-[10px] bg-[#0E1B2E] border border-[#22395A] space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-xs font-bold text-[#F2F6FB] uppercase tracking-wider font-mono">
                Scenario Inputs
              </span>
              {isDraftParam && (
                <span className="px-1.5 py-0.2 rounded text-[9px] font-mono bg-amber-500/20 text-amber-300 font-bold">
                  Draft Preview
                </span>
              )}
            </div>

            <div className="space-y-2 text-xs">
              <div>
                <label className="text-[10px] text-[#7F93AF] font-mono block mb-1">
                  Peak Load Torque (N·m)
                </label>
                <input
                  type="number"
                  value={loadTorqueDraft}
                  onChange={e => {
                    setLoadTorqueDraft(Number(e.target.value));
                    setIsDraftParam(true);
                  }}
                  className="w-full px-2 py-1 rounded bg-[#13253D] border border-[#22395A] text-white font-mono text-xs"
                />
              </div>

              <div>
                <label className="text-[10px] text-[#7F93AF] font-mono block mb-1">
                  Coolant Chamber (°C)
                </label>
                <input
                  type="number"
                  value={ambientTempDraft}
                  onChange={e => {
                    setAmbientTempDraft(Number(e.target.value));
                    setIsDraftParam(true);
                  }}
                  className="w-full px-2 py-1 rounded bg-[#13253D] border border-[#22395A] text-white font-mono text-xs"
                />
              </div>

              {isDraftParam && (
                <button
                  onClick={handleApplyRerun}
                  disabled={isRerunning}
                  className="w-full py-1.5 rounded-[6px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6] transition-colors"
                >
                  {isRerunning ? 'Run 20 Computing...' : 'Apply & Re-run (MOCK)'}
                </button>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* 3. TIMELINE SCRUBBER DOCK (Section 6.10: State Replay, not video) */}
      <div className="h-16 px-5 border-t border-[#22395A]/80 bg-[#0A1422]/95 backdrop-blur-md flex items-center justify-between gap-4 pointer-events-auto shrink-0 z-20">
        {/* Playback Controls */}
        <div className="flex items-center gap-2">
          <button
            onClick={() => setIsPlaying(p => !p)}
            className="w-8 h-8 rounded-full flex items-center justify-center bg-[#2F7BFF] hover:bg-[#3B82F6] text-white shadow-[0_0_15px_rgba(47,123,255,0.4)] transition-all"
            title={isPlaying ? 'Pause' : 'Play'}
          >
            {isPlaying ? '⏸' : '▶'}
          </button>

          <button
            onClick={() => handleScrub(0)}
            className="p-1.5 rounded bg-[#13253D] hover:bg-[#1E3A5F] text-[#D6E2F3] text-xs"
            title="Reset to 0s"
          >
            ⏹
          </button>

          <button
            onClick={() => handleScrub(currentTime - 0.5)}
            className="p-1.5 rounded bg-[#13253D] hover:bg-[#1E3A5F] text-[#D6E2F3] text-xs font-mono"
            title="Step -1"
          >
            -0.5s
          </button>

          <button
            onClick={() => handleScrub(currentTime + 0.5)}
            className="p-1.5 rounded bg-[#13253D] hover:bg-[#1E3A5F] text-[#D6E2F3] text-xs font-mono"
            title="Step +1"
          >
            +0.5s
          </button>

          {/* Speed Selectors */}
          <div className="flex items-center gap-1 font-mono text-[10px] ml-2">
            {([0.5, 1, 2, 4] as const).map(spd => (
              <button
                key={spd}
                onClick={() => setSpeedMultiplier(spd)}
                className={`px-1.5 py-0.5 rounded ${
                  speedMultiplier === spd
                    ? 'bg-[#2F7BFF] text-white font-bold'
                    : 'bg-[#13253D] text-[#7F93AF] hover:text-white'
                }`}
              >
                {spd}x
              </button>
            ))}
          </div>
        </div>

        {/* Reproducible State Scrubber */}
        <div className="flex-1 flex flex-col justify-center px-4 space-y-1">
          <div className="flex items-center justify-between text-[11px] font-mono text-[#7F93AF]">
            <span className="text-white font-bold">t = {currentTime.toFixed(2)} s</span>
            <div className="flex items-center gap-2">
              <span className="text-red-400 font-bold">Trip: 8.42 s</span>
              <span>•</span>
              <span>Total: {totalDuration} s</span>
            </div>
          </div>

          <div className="relative w-full flex items-center">
            <input
              type="range"
              min="0"
              max={totalDuration}
              step="0.05"
              value={currentTime}
              onChange={e => handleScrub(parseFloat(e.target.value))}
              className="w-full h-1.5 bg-[#13253D] rounded-lg appearance-none cursor-pointer accent-[#2F7BFF]"
            />
            {/* 8.42s Marker on scrubber */}
            <div
              className="absolute w-1.5 h-3 bg-red-500 rounded-sm pointer-events-none -top-0.5"
              style={{ left: `${(8.42 / totalDuration) * 100}%` }}
              title="Thermal failure trip: 8.42s"
            ></div>
          </div>
        </div>

        {/* Freeze & Branch Actions */}
        <div className="flex items-center gap-2">
          <button
            onClick={async () => {
              await engine.freeze('run-1', currentTime);
              if (onForkBranch) onForkBranch();
            }}
            className="px-3 py-1.5 rounded-[8px] text-xs font-mono font-medium text-white bg-[#8B5CF6]/30 border border-[#8B5CF6]/50 hover:bg-[#8B5CF6]/50 transition-all shadow-[0_0_12px_rgba(139,92,246,0.3)]"
          >
            ❄ Freeze & Branch
          </button>
        </div>
      </div>
    </div>
  );
};
