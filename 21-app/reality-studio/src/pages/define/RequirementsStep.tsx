import React, { useState } from 'react';
import { PreviewTag, ExampleTag } from '../../components/ui/UiKit';

export interface RequirementsStepProps {
  promptText: string;
  onContinue: () => void;
}

export const RequirementsStep: React.FC<RequirementsStepProps> = ({ promptText, onContinue }) => {
  const [briefs, setBriefs] = useState([
    { id: 'b-sys', title: 'System', status: 'Ready', desc: 'Closed fluid loop actuator driven by DC electrical supply.' },
    { id: 'b-cmp', title: 'Components', status: '4 proposed', desc: 'Power source, electric drive assembly, sensor unit, fluid displacement unit.' },
    { id: 'b-con', title: 'Connections', status: 'Ready', desc: '48 V DC link, mechanical shaft coupling, sensor feedback wire.' },
    { id: 'b-env', title: 'Environment', status: 'Draft', desc: 'Ambient 25.0 °C, atmospheric pressure 101.3 kPa, convection cooling.' },
    { id: 'b-scn', title: 'Scenario', status: '1 proposed', desc: 'High load dynamic ramp step at t=4.20s to evaluate thermal margin.' },
    { id: 'b-req', title: 'Requirements', status: 'Ready', desc: 'Stator winding temperature limit < 120.0 °C; deterministic bitwise replay.' }
  ]);

  const [questions, setQuestions] = useState([
    {
      id: 'q1',
      text: 'Specify continuous hydraulic flow rate requirement:',
      defaultVal: '4.5 L/min at 3.5 bar',
      answered: false,
      value: ''
    },
    {
      id: 'q2',
      text: 'Acceptable rotor angular speed tolerance:',
      defaultVal: '±50 RPM (Tolerance tier)',
      answered: false,
      value: ''
    }
  ]);

  const extractedRequirements = [
    { text: 'Electrical Supply Voltage', value: '48.0', unit: 'V', source: 'User-stated' },
    { text: 'Thermal Safety Threshold', value: '120.0', unit: '°C', source: 'User-stated' },
    { text: 'Calculated Nominal Current', value: '62.5', unit: 'A', source: 'Inferred', calc: '3.0 kW at 48 V is about 62.5 A' }
  ];

  return (
    <div className="w-full max-w-6xl mx-auto p-4 space-y-4 my-auto z-10 overflow-y-auto max-h-[85vh]">
      {/* Top Banner: Interpreting status */}
      <div className="p-3 rounded-[12px] bg-[#0E1B2E] border border-[#22395A] flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="w-2 h-2 rounded-full bg-[#22C55E]" />
          <span className="text-xs font-semibold text-[#F2F6FB]">Formal Interpretation Complete</span>
          <span className="text-[11px] text-[#7F93AF] font-mono">• 6 briefs synthesized from input brief</span>
        </div>
        <button
          onClick={onContinue}
          className="px-4 py-1.5 rounded-[8px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6] transition-all flex items-center gap-1.5 shadow-[0_0_12px_rgba(47,123,255,0.4)]"
        >
          <span>Continue to Architecture</span>
          <span>→</span>
        </button>
      </div>

      {/* Main 3-Column Layout: Left Prompt Brief, Center 6 Cards, Right Extracted Reqs */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-4">
        {/* Left: Your Brief Text (3 Cols) */}
        <div className="lg:col-span-3 rounded-[14px] bg-[#0A1422] border border-[#22395A] p-4 flex flex-col justify-between">
          <div className="space-y-2">
            <span className="text-xs font-bold text-[#F2F6FB]">Your Input Brief</span>
            <div className="p-3 rounded-[8px] bg-[#0E1B2E] border border-[#22395A] text-xs text-[#D6E2F3] font-mono leading-relaxed">
              "{promptText || 'A pump driven by an electric motor from a 48 V supply...'}"
            </div>
          </div>
          <div className="text-[10px] text-[#7F93AF] mt-3">
            Source: User specification prompt
          </div>
        </div>

        {/* Center: Six Brief Cards (6 Cols) */}
        <div className="lg:col-span-6 grid grid-cols-1 sm:grid-cols-2 gap-3">
          {briefs.map(b => (
            <div key={b.id} className="p-3 rounded-[12px] bg-[#0A1422] border border-[#22395A] space-y-1.5 flex flex-col justify-between">
              <div className="flex items-center justify-between">
                <span className="text-xs font-bold text-[#F2F6FB]">{b.title}</span>
                <span className="px-1.5 py-0.2 rounded text-[10px] font-mono text-[#22D3EE] bg-[#22D3EE]/10 border border-[#22D3EE]/30">
                  {b.status}
                </span>
              </div>
              <p className="text-[11px] text-[#7F93AF] leading-relaxed">{b.desc}</p>
              <button
                onClick={() => alert(`Re-running brief for ${b.title}...`)}
                className="text-[10px] text-[#2F7BFF] hover:underline text-left font-mono pt-1"
              >
                Edit brief →
              </button>
            </div>
          ))}
        </div>

        {/* Right: Extracted Requirements & CALCULATED checks (3 Cols) */}
        <div className="lg:col-span-3 rounded-[14px] bg-[#0A1422] border border-[#22395A] p-4 space-y-3">
          <span className="text-xs font-bold text-[#F2F6FB] border-b border-[#22395A] pb-2 block">
            Extracted Requirements
          </span>
          <div className="space-y-2.5">
            {extractedRequirements.map(req => (
              <div key={req.text} className="p-2.5 rounded-[8px] bg-[#0E1B2E] border border-[#22395A] space-y-1">
                <div className="flex justify-between text-[11px] text-[#7F93AF]">
                  <span>{req.text}</span>
                  <span className="font-mono text-[9px] text-[#22D3EE] uppercase">{req.source}</span>
                </div>
                <div className="text-sm font-bold text-[#F2F6FB] font-mono">
                  {req.value} <span className="text-xs font-normal text-[#22D3EE]">{req.unit}</span>
                </div>
                {req.calc && (
                  <div className="text-[10px] text-emerald-400 font-mono bg-emerald-950/30 p-1 rounded border border-emerald-500/20">
                    CALCULATED: {req.calc}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Bottom: Clarifying Question Cards (Up to 3 cards - Section 6.4) */}
      <div className="rounded-[14px] bg-[#0E1B2E] border border-[#22395A] p-4 space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span className="text-xs font-bold text-[#F2F6FB]">Clarifying Questions</span>
            <span className="text-[10px] px-1.5 py-0.2 rounded bg-purple-500/20 text-purple-300 font-mono">
              Rule-based suggestions
            </span>
          </div>
          <span className="text-[10px] text-[#7F93AF]">Skip sets parameter to UNKNOWN</span>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          {questions.map((q, idx) => (
            <div key={q.id} className="p-3 rounded-[10px] bg-[#0A1422] border border-[#22395A] space-y-2">
              <div className="text-xs text-[#D6E2F3] font-medium">{q.text}</div>
              <div className="flex items-center gap-2 text-xs">
                <button
                  onClick={() => alert(`Applied default: ${q.defaultVal}`)}
                  className="px-2.5 py-1 rounded-[6px] bg-[#13253D] hover:bg-[#1E3A5F] border border-[#22395A] text-[11px] text-[#22D3EE] font-mono"
                >
                  Use default: {q.defaultVal}
                </button>
                <button
                  onClick={() => alert(`Parameter marked UNKNOWN`)}
                  className="px-2.5 py-1 rounded-[6px] text-[11px] text-[#7F93AF] hover:text-white"
                >
                  Skip (UNKNOWN)
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
