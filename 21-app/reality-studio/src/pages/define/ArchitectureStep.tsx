import React from 'react';

export interface ArchitectureStepProps {
  onContinue: () => void;
}

export const ArchitectureStep: React.FC<ArchitectureStepProps> = ({ onContinue }) => {
  const subsystems = [
    {
      id: 'sub-elec',
      name: 'Electrical Subsystem',
      blocks: [
        { id: 'blk-ps', name: 'Power Supply Unit', ports: ['DC Out (48 V)'] }
      ]
    },
    {
      id: 'sub-drive',
      name: 'Drive Subsystem',
      blocks: [
        { id: 'blk-mot', name: 'Actuator Unit', ports: ['DC In', 'Shaft Coupling'] },
        { id: 'blk-sens', name: 'Thermal Sensor Probe', ports: ['Analog Out'] }
      ]
    },
    {
      id: 'sub-fluid',
      name: 'Fluid Subsystem',
      blocks: [
        { id: 'blk-pump', name: 'Fluid Displacement Assembly', ports: ['Shaft Input', 'Inlet', 'Outlet'] }
      ]
    }
  ];

  const linkTypes = [
    { name: 'Electrical DC', color: '#2F7BFF', dash: '' },
    { name: 'Mechanical Torque', color: '#22D3EE', dash: '' },
    { name: 'Telemetry Signal', color: '#8B5CF6', dash: '3,3' },
    { name: 'Thermal Dissipation', color: '#F59E0B', dash: '2,2' }
  ];

  return (
    <div className="w-full max-w-5xl mx-auto p-6 space-y-4 my-auto z-10">
      {/* Top Action Header */}
      <div className="p-3.5 rounded-[12px] bg-[#0E1B2E] border border-[#22395A] flex items-center justify-between">
        <div>
          <h2 className="text-sm font-bold text-[#F2F6FB]">System Architecture Block Diagram</h2>
          <p className="text-[11px] text-[#7F93AF]">Synthesized logical topology derived from engineering briefs (no physics claims).</p>
        </div>
        <button
          onClick={onContinue}
          className="px-5 py-2 rounded-[8px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6] shadow-[0_0_16px_rgba(47,123,255,0.4)] transition-all flex items-center gap-1.5"
        >
          <span>Continue to Components</span>
          <span>→</span>
        </button>
      </div>

      {/* Center Block Diagram over Translucent Backdrop */}
      <div className="p-8 rounded-[16px] bg-[#0A1422]/90 border border-[#22395A] shadow-[0_16px_48px_rgba(0,0,0,0.5)] relative overflow-hidden flex flex-col justify-between min-h-[380px]">
        {/* Background Grid Pattern */}
        <div className="absolute inset-0 bg-[radial-gradient(#22395A_1px,transparent_1px)] [background-size:16px_16px] opacity-40 pointer-events-none" />

        {/* Subsystem Blocks */}
        <div className="relative z-10 grid grid-cols-1 md:grid-cols-3 gap-6 my-auto">
          {subsystems.map(sub => (
            <div
              key={sub.id}
              className="p-4 rounded-[12px] border-2 border-dashed border-[#22395A] bg-[#0E1B2E]/60 space-y-3"
            >
              <div className="text-xs font-mono font-bold text-[#22D3EE] uppercase tracking-wider">
                {sub.name}
              </div>

              <div className="space-y-2">
                {sub.blocks.map(b => (
                  <div
                    key={b.id}
                    className="p-3 rounded-[8px] bg-[#13253D] border border-[#22395A] space-y-2"
                  >
                    <div className="text-xs font-bold text-[#F2F6FB]">{b.name}</div>
                    <div className="flex flex-wrap gap-1">
                      {b.ports.map(p => (
                        <span
                          key={p}
                          className="px-1.5 py-0.5 rounded text-[9px] font-mono bg-black/40 text-[#7F93AF] border border-[#22395A]"
                        >
                          {p}
                        </span>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>

        {/* Bottom Link Type Legend */}
        <div className="relative z-10 pt-4 border-t border-[#22395A] flex items-center justify-between text-xs">
          <span className="text-[#7F93AF] font-mono text-[11px]">Inter-Subsystem Coupling:</span>
          <div className="flex items-center gap-4">
            {linkTypes.map(lt => (
              <div key={lt.name} className="flex items-center gap-1.5 font-mono text-[11px]">
                <span className="w-3 h-0.5" style={{ backgroundColor: lt.color }} />
                <span className="text-[#D6E2F3]">{lt.name}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};
