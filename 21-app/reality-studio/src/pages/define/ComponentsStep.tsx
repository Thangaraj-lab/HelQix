import React, { useState } from 'react';
import { ExampleTag, PlaceholderTag } from '../../components/ui/UiKit';

export interface ComponentsStepProps {
  onContinue: () => void;
}

export const ComponentsStep: React.FC<ComponentsStepProps> = ({ onContinue }) => {
  const [selectedCandidates, setSelectedCandidates] = useState<Record<string, string>>({
    'block-ps': 'c-ps-1',
    'block-act': 'c-act-1',
    'block-sens': 'c-sens-1',
    'block-disp': 'c-disp-placeholder'
  });

  const componentCategories = [
    {
      id: 'block-ps',
      subsystem: 'Power Supply Unit',
      candidates: [
        {
          id: 'c-ps-1',
          name: 'Power supply (Example)',
          specs: '48.0 V DC, 3.2 kW rating',
          source: 'Internal Component CAD Vault',
          asOf: '2026-10-25',
          maturity: 'Validated model',
          reason: 'Matches voltage (48 V) and continuous power demand',
          selected: true
        }
      ]
    },
    {
      id: 'block-act',
      subsystem: 'Drive Assembly',
      candidates: [
        {
          id: 'c-act-1',
          name: 'Electric motor (Example)',
          specs: '48 V DC nominal, 3000 RPM, 3.0 kW',
          source: 'Internal Component CAD Vault',
          asOf: '2026-10-25',
          maturity: 'Validated model',
          reason: 'Matches 48 V electrical port and rotational drive requirement',
          selected: true
        }
      ]
    },
    {
      id: 'block-sens',
      subsystem: 'Thermal Sensor Probe',
      candidates: [
        {
          id: 'c-sens-1',
          name: 'Temperature sensor (Example)',
          specs: '-40 to 150 °C range, RTD Pt100',
          source: 'Power Lab Telemetry Ingest',
          asOf: '2026-10-25',
          maturity: 'Validated model',
          reason: 'Operational limit covers 120.0 °C threshold requirement',
          selected: true
        }
      ]
    },
    {
      id: 'block-disp',
      subsystem: 'Fluid Displacement Assembly',
      candidates: [
        {
          id: 'c-disp-placeholder',
          name: 'Pump (no model) (Example)',
          specs: 'Displacement placeholder, 3.5 bar fluid head',
          source: 'Inferred Specification',
          asOf: '2026-10-25',
          maturity: 'Placeholder',
          reason: 'No geometric CAD source found in connected vaults; using generic placeholder geometry',
          selected: true
        }
      ]
    }
  ];

  return (
    <div className="w-full max-w-6xl mx-auto p-4 space-y-4 my-auto z-10 overflow-y-auto max-h-[85vh]">
      {/* Top Header */}
      <div className="p-3.5 rounded-[12px] bg-[#0E1B2E] border border-[#22395A] flex items-center justify-between">
        <div>
          <h2 className="text-sm font-bold text-[#F2F6FB]">Component Resolution & Sourcing</h2>
          <p className="text-[11px] text-[#7F93AF]">Select validated models from connected vaults, or define placeholders from scratch.</p>
        </div>
        <button
          onClick={onContinue}
          className="px-5 py-2 rounded-[8px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6] shadow-[0_0_16px_rgba(47,123,255,0.4)] transition-all flex items-center gap-1.5"
        >
          <span>Continue to Assemble</span>
          <span>→</span>
        </button>
      </div>

      {/* Categories Grid */}
      <div className="space-y-4">
        {componentCategories.map(cat => (
          <div key={cat.id} className="p-4 rounded-[14px] bg-[#0A1422] border border-[#22395A] space-y-3">
            <div className="flex items-center justify-between border-b border-[#22395A] pb-2">
              <span className="text-xs font-mono font-bold text-[#22D3EE] uppercase">{cat.subsystem}</span>
              <span className="text-[10px] text-[#7F93AF] font-mono">1 resolution candidate</span>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
              {/* Sourced Candidate Card */}
              {cat.candidates.map(cand => (
                <div
                  key={cand.id}
                  className="p-3.5 rounded-[10px] bg-[#0E1B2E] border border-[#2F7BFF]/50 shadow-[0_0_15px_rgba(47,123,255,0.15)] flex flex-col justify-between space-y-2"
                >
                  <div className="space-y-1">
                    <div className="flex items-start justify-between gap-2">
                      <div className="text-xs font-bold text-[#F2F6FB] flex items-center gap-1.5">
                        <span>{cand.name}</span>
                        <ExampleTag />
                      </div>
                      {cand.maturity === 'Placeholder' ? (
                        <PlaceholderTag label="PLACEHOLDER" />
                      ) : (
                        <span className="px-1.5 py-0.2 rounded text-[9px] font-mono bg-emerald-500/15 border border-emerald-500/30 text-emerald-400">
                          {cand.maturity}
                        </span>
                      )}
                    </div>
                    <div className="text-[11px] text-[#22D3EE] font-mono">{cand.specs}</div>
                    <div className="text-[10px] text-[#7F93AF]">Source: {cand.source} (as of {cand.asOf})</div>
                  </div>

                  {/* Visible Ranking Reason (Section 6.6) */}
                  <div className="p-2 rounded-[6px] bg-[#13253D] border border-[#22395A] text-[10px] text-[#D6E2F3] font-mono leading-tight">
                    Ranking reason: {cand.reason}
                  </div>

                  <button className="w-full py-1.5 rounded-[6px] text-xs font-semibold bg-[#2F7BFF]/25 border border-[#2F7BFF]/60 text-[#22D3EE]">
                    ✓ Selected
                  </button>
                </div>
              ))}

              {/* Define from scratch Card (Always present - Section 6.6) */}
              <div
                onClick={() => alert('Define from scratch: opens part parameter & port authoring panel.')}
                className="p-3.5 rounded-[10px] bg-[#0A1422] border border-dashed border-[#22395A] hover:border-[#22D3EE] transition-all cursor-pointer flex flex-col items-center justify-center text-center gap-1.5"
              >
                <span className="text-xl text-[#7F93AF]">+</span>
                <span className="text-xs font-bold text-[#D6E2F3]">Define from scratch</span>
                <span className="text-[10px] text-[#7F93AF] max-w-[180px]">Specify custom geometry primitives, ports, and relations</span>
              </div>

              {/* Import file Card (Always present - Section 6.6) */}
              <div
                onClick={() => alert('Import file: upload STEP AP242, glTF, or Parasolid file.')}
                className="p-3.5 rounded-[10px] bg-[#0A1422] border border-dashed border-[#22395A] hover:border-[#22D3EE] transition-all cursor-pointer flex flex-col items-center justify-center text-center gap-1.5"
              >
                <span className="text-xl text-[#7F93AF]">⬆</span>
                <span className="text-xs font-bold text-[#D6E2F3]">Import CAD / Geometry</span>
                <span className="text-[10px] text-[#7F93AF] max-w-[180px]">Load external geometry with automatic port recognition</span>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
