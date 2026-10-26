import React, { useState } from 'react';
import { useAppState } from '../../engineering/stateStore';
import { PreviewTag, DeterminismChip, OriginStatusBadges } from '../../components/ui/HonestyBadges';

export const EdgeMonitoringPage: React.FC = () => {
  const { determinism } = useAppState();

  const [overlaySimulated, setOverlaySimulated] = useState(true);

  const edgeDevices = [
    { id: 'EDGE-JETSON-01', name: 'Rotor Dynamometer Cell A', status: 'Online', ip: '10.0.4.12', latency: '4.2 ms' },
    { id: 'EDGE-JETSON-02', name: 'Cryo-Thermal Chamber Testbed', status: 'Warning', ip: '10.0.4.15', latency: '12.8 ms' },
    { id: 'EDGE-PLC-03', name: 'High-Voltage SiC Inverter Rig', status: 'Online', ip: '10.0.4.18', latency: '2.1 ms' },
    { id: 'EDGE-SENS-04', name: 'Telemetry Bridge Unit 04', status: 'Offline', ip: '10.0.4.22', latency: 'Timeout' }
  ];

  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto overflow-y-auto">
      {/* Top Header */}
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-bold text-white tracking-wide">Edge Telemetry & Real-Time Physical Ingest</h1>
            <PreviewTag label="PREVIEW (Phase 7A)" />
          </div>
          <p className="text-xs text-slate-400 mt-0.5">High-frequency physical sensor observation stream paired with simulation digital twin.</p>
        </div>

        <div className="flex items-center gap-2">
          {/* Simulated vs Observed Overlay Toggle (Prompt 16) */}
          <button
            onClick={() => setOverlaySimulated(!overlaySimulated)}
            className={`px-3 py-1.5 rounded-lg text-xs font-semibold border transition-all flex items-center gap-1.5 ${
              overlaySimulated
                ? 'bg-blue-600/40 text-cyan-300 border-cyan-400 shadow-[0_0_12px_rgba(34,211,238,0.25)]'
                : 'bg-slate-900 border-slate-700 text-slate-400'
            }`}
          >
            <span>Simulated vs Observed Overlay</span>
            <span className={`w-2 h-2 rounded-full ${overlaySimulated ? 'bg-cyan-400' : 'bg-slate-600'}`} />
          </button>

          <DeterminismChip level={determinism} />
        </div>
      </div>

      {/* Main Grid: Left Stats, Center Abstract Device Map, Right Device List */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-4">
        {/* Left Stats: Active Devices, Error Devices, Signal Quality (3 Cols - Prompt 16) */}
        <div className="lg:col-span-3 flex flex-col gap-3">
          <div className="glass-panel p-4 space-y-1">
            <div className="flex items-center justify-between text-xs text-slate-400">
              <span>Active Edge Devices</span>
              <OriginStatusBadges origin="Observed" status="Verified" />
            </div>
            <div className="text-3xl font-bold text-white font-mono num">18</div>
            <div className="text-[11px] text-emerald-400 font-mono">18 / 20 connected</div>
          </div>

          <div className="glass-panel p-4 space-y-1">
            <div className="flex items-center justify-between text-xs text-slate-400">
              <span>Error / Warning Devices</span>
              <OriginStatusBadges origin="Observed" status="Verified" />
            </div>
            <div className="text-3xl font-bold text-rose-400 font-mono num">1</div>
            <div className="text-[11px] text-rose-300 font-mono">High latency jitter on Chamber</div>
          </div>

          <div className="glass-panel p-4 space-y-1">
            <div className="flex items-center justify-between text-xs text-slate-400">
              <span>Telemetry Ingest Quality</span>
              <OriginStatusBadges origin="Derived" status="Verified" />
            </div>
            <div className="text-3xl font-bold text-cyan-300 font-mono num">99.4%</div>
            <div className="text-[11px] text-slate-400 font-mono">Drop rate &lt; 0.02%</div>
          </div>
        </div>

        {/* Center: Abstract Device Map (6 Cols - Prompt 16 strictly no real-world claim) */}
        <div className="lg:col-span-6 glass-panel p-4 relative flex flex-col justify-between overflow-hidden min-h-[260px]">
          <div className="flex items-center justify-between border-b border-slate-800 pb-2 z-10">
            <span className="text-xs font-bold text-white">Abstract Device Topology Network</span>
            <span className="text-[10px] font-mono text-cyan-400">STAR-MESH ARCHITECTURE</span>
          </div>

          {/* Abstract topological node canvas */}
          <div className="relative w-full h-44 flex items-center justify-center my-auto">
            {/* Hub node */}
            <div className="w-14 h-14 rounded-full bg-blue-600/30 border-2 border-cyan-400 flex items-center justify-center text-xs font-mono font-bold text-white shadow-[0_0_20px_#22D3EE] z-10">
              HUB
            </div>

            {/* Satellite nodes */}
            {[
              { label: 'E1', top: '15%', left: '20%', color: 'border-emerald-400 text-emerald-300' },
              { label: 'E2', top: '18%', right: '22%', color: 'border-amber-400 text-amber-300' },
              { label: 'E3', bottom: '15%', left: '25%', color: 'border-emerald-400 text-emerald-300' },
              { label: 'E4', bottom: '18%', right: '20%', color: 'border-rose-400 text-rose-300' }
            ].map((node, idx) => (
              <div
                key={idx}
                style={{ top: node.top, left: node.left, right: node.right, bottom: node.bottom }}
                className={`absolute w-9 h-9 rounded-full bg-slate-900 border ${node.color} flex items-center justify-center text-[11px] font-mono font-bold shadow-[0_0_10px_rgba(47,123,255,0.3)]`}
              >
                {node.label}
              </div>
            ))}
          </div>

          <div className="text-[10px] text-slate-500 font-mono text-center z-10">
            Note: Diagram represents logical network fabric; no geographical positioning implied.
          </div>
        </div>

        {/* Right: Live Feed of Devices (3 Cols - Prompt 16) */}
        <div className="lg:col-span-3 glass-panel p-4 flex flex-col gap-2.5 overflow-y-auto">
          <span className="text-xs font-bold text-white border-b border-slate-800 pb-2">
            Connected Edge Feed
          </span>

          <div className="space-y-2">
            {edgeDevices.map(dev => (
              <div key={dev.id} className="p-2.5 rounded-xl bg-slate-900/60 border border-slate-800 space-y-1">
                <div className="flex items-center justify-between text-xs">
                  <span className="font-semibold text-slate-200 truncate">{dev.id}</span>
                  <span
                    className={`px-1.5 py-0.2 rounded text-[10px] font-semibold border ${
                      dev.status === 'Online'
                        ? 'bg-emerald-500/20 text-emerald-400 border-emerald-500/40'
                        : dev.status === 'Warning'
                        ? 'bg-amber-500/20 text-amber-400 border-amber-500/40'
                        : 'bg-rose-500/20 text-rose-400 border-rose-500/40'
                    }`}
                  >
                    {dev.status}
                  </span>
                </div>
                <div className="text-[11px] text-slate-400 truncate">{dev.name}</div>
                <div className="flex justify-between text-[10px] font-mono text-slate-500 pt-0.5">
                  <span>{dev.ip}</span>
                  <span>{dev.latency}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Bottom Sensor Data Line Charts (Temperature, Vibration, Load, Power) - Prompt 16 */}
      <div className="glass-panel p-4 space-y-3">
        <div className="flex items-center justify-between border-b border-slate-800 pb-2">
          <div className="flex items-center gap-3">
            <span className="text-xs font-bold text-white">Physical Sensor Observation Telemetry</span>
            <OriginStatusBadges origin="Observed" status="Verified" />
          </div>

          {/* Residual Readout */}
          <div className="flex items-center gap-2 text-xs font-mono">
            <span className="text-slate-400">Mean Model Residual:</span>
            <span className="text-cyan-300 font-bold">Δ = 0.42 °C / ±0.08 mm</span>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          {[
            { label: 'Rotor Temperature', observed: '92.4 °C', simulated: '91.8 °C', color: '#EF4444' },
            { label: 'Shaft Vibration', observed: '21.4 mm/s', simulated: '20.9 mm/s', color: '#22D3EE' },
            { label: 'Mechanical Load', observed: '85.4 N·m', simulated: '85.4 N·m', color: '#8B5CF6' },
            { label: 'Electrical Power Draw', observed: '64.5 kW', simulated: '64.1 kW', color: '#F59E0B' }
          ].map(sensor => (
            <div key={sensor.label} className="p-3 rounded-xl bg-slate-900/60 border border-slate-800 space-y-2">
              <div className="flex justify-between text-xs">
                <span className="text-slate-300 font-medium">{sensor.label}</span>
                <span className="text-[10px] font-mono text-slate-400">1000 Hz</span>
              </div>

              <div className="flex items-baseline justify-between text-xs font-mono">
                <div>
                  <span className="text-[10px] text-slate-500 block">Observed:</span>
                  <span className="text-white font-bold">{sensor.observed}</span>
                </div>
                {overlaySimulated && (
                  <div className="text-right">
                    <span className="text-[10px] text-cyan-400 block">Simulated:</span>
                    <span className="text-cyan-300 font-bold">{sensor.simulated}</span>
                  </div>
                )}
              </div>

              {/* Sparkline curve */}
              <svg className="w-full h-10" preserveAspectRatio="none" viewBox="0 0 100 24">
                <path d="M0,18 Q25,16 50,14 T75,8 T100,6" fill="none" stroke={sensor.color} strokeWidth="1.5" />
                {overlaySimulated && (
                  <path d="M0,19 Q25,17 50,15 T75,9 T100,7" fill="none" stroke="#38BDF8" strokeWidth="1" strokeDasharray="2,2" />
                )}
              </svg>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
