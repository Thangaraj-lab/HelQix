import React, { useState } from 'react';
import { PreviewTag, MockTag, Panel } from '../../components/ui/UiKit';

export interface VerifyStageProps {
  onOpenEvidenceSheet: () => void;
}

export const VerifyStage: React.FC<VerifyStageProps> = ({ onOpenEvidenceSheet }) => {
  const [selectedReqId, setSelectedReqId] = useState<string>('req-1');
  const [isValidating, setIsValidating] = useState<boolean>(false);
  const [hasValidated, setHasValidated] = useState<boolean>(false);

  // Requirements with Pass/Warning/Not-Run states (Section 6.13)
  const requirements = [
    { id: 'req-1', text: 'Winding temperature below 120.0 °C', state: 'pass', value: '105.8 °C', limit: '120.0 °C' },
    { id: 'req-2', text: 'Mechanical output power > 2.5 kW', state: 'pass', value: '2.98 kW', limit: '2.50 kW' },
    { id: 'req-3', text: 'Coolant flow capacity margin > 10%', state: 'warning', value: '11.2 %', limit: '10.0 %' },
    { id: 'req-4', text: 'Acoustic vibration level < 65 dBA', state: 'not-run', value: 'Pending', limit: '65.0 dBA' }
  ];

  // Credibility Checklist (ASME V&V 40 style - Section 6.13)
  const credibilityChecklist = [
    { title: 'Validation error against reference data', status: 'missing', note: 'Physical bench reference data not connected' },
    { title: 'Uncertainty quantification bounds', status: 'missing', note: 'Stochastic tolerance sweep not yet completed' },
    { title: 'Numerical solver determinism check', status: 'passed', note: 'Bitwise identical state hashes verified' },
    { title: 'Spatial & temporal grid convergence', status: 'passed', note: 'dt = 0.001s timestep convergence confirmed' },
    { title: 'Independent engineering peer review', status: 'missing', note: 'Cryptographic signature pending sign-off' }
  ];

  // Six Included Items in Evidence Package (Section 6.13)
  const evidenceItems = [
    'Model Revision r1 AST Snapshot',
    'Solver Execution Log (mock run)',
    'Intervention Branch R1 Diff Ops',
    'Deterministic State Replay Hashes',
    'Unit Verification Audit Report',
    'Ed25519 Origin Signatures'
  ];

  const handleRunValidation = async () => {
    setIsValidating(true);
    await new Promise(r => setTimeout(r, 600));
    setIsValidating(false);
    setHasValidated(true);
  };

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-transparent z-10 pointer-events-none select-none">
      {/* 1. TOP VERIFY BAR (Section 6.13) */}
      <div className="h-11 px-5 flex items-center justify-between border-b border-[#22395A]/80 bg-[#0A1422]/90 backdrop-blur-md pointer-events-auto shrink-0">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <span className="text-xs font-semibold text-[#F2F6FB]">Verify Stage</span>
            <PreviewTag label="PREVIEW" />
            <MockTag />
          </div>

          <span className="text-[#22395A]">|</span>

          <span className="text-[11px] font-mono text-[#7F93AF]">
            Credibility checklist: <strong className="text-amber-400">3 gaps missing</strong>
          </span>
        </div>

        {/* Action Button: Run validation / Export evidence */}
        <div className="flex items-center gap-2.5">
          {hasValidated ? (
            <button
              onClick={onOpenEvidenceSheet}
              className="px-4 py-1.5 rounded-[8px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6] shadow-[0_0_20px_rgba(47,123,255,0.4)] transition-all flex items-center gap-1.5"
            >
              <span>📄</span>
              <span>Export evidence package →</span>
            </button>
          ) : (
            <button
              onClick={handleRunValidation}
              disabled={isValidating}
              className="px-4 py-1.5 rounded-[8px] text-xs font-semibold text-white bg-gradient-to-r from-[#2F7BFF] to-[#8B5CF6] hover:from-[#3B82F6] hover:to-[#7C3AED] shadow-[0_0_20px_rgba(47,123,255,0.4)] transition-all flex items-center gap-1.5"
            >
              <span>✦</span>
              <span>{isValidating ? 'Validating checklist...' : 'Run validation checklist'}</span>
            </button>
          )}
        </div>
      </div>

      {/* 2. THREE-COLUMN VERIFY BODY */}
      <div className="flex-1 flex overflow-hidden">
        {/* LEFT PANEL: REQUIREMENTS WITH STATES (Section 6.13) */}
        <div className="w-80 border-r border-[#22395A]/80 bg-[#0A1422]/85 backdrop-blur-md flex flex-col justify-between shrink-0 pointer-events-auto p-4 space-y-3 overflow-y-auto">
          <div className="space-y-3">
            <div className="border-b border-[#22395A] pb-2 flex items-center justify-between">
              <span className="text-xs font-bold text-[#F2F6FB] uppercase tracking-wider font-mono">
                System Requirements
              </span>
              <span className="text-[10px] font-mono text-[#22D3EE]">V&V Trace</span>
            </div>

            <div className="space-y-2">
              {requirements.map(req => {
                const isSelected = selectedReqId === req.id;
                return (
                  <div
                    key={req.id}
                    onClick={() => setSelectedReqId(req.id)}
                    className={`p-3 rounded-[10px] border cursor-pointer transition-all space-y-1.5 ${
                      isSelected
                        ? 'bg-[#2F7BFF]/20 border-[#2F7BFF] shadow-[0_0_12px_rgba(47,123,255,0.25)]'
                        : 'bg-[#0E1B2E]/60 border-[#22395A] hover:bg-[#13253D]/80'
                    }`}
                  >
                    <div className="flex items-center justify-between text-xs">
                      <span className="font-semibold text-white truncate">{req.text}</span>
                      <span
                        className={`text-[9px] px-1.5 py-0.2 rounded font-mono uppercase font-bold ${
                          req.state === 'pass'
                            ? 'bg-[#22C55E]/15 text-[#4ADE80] border border-[#22C55E]/40'
                            : req.state === 'warning'
                            ? 'bg-amber-500/15 text-amber-300 border border-amber-500/40'
                            : 'bg-slate-700 text-slate-400'
                        }`}
                      >
                        {req.state}
                      </span>
                    </div>

                    <div className="flex items-center justify-between text-[10px] font-mono text-[#7F93AF]">
                      <span>Observed: <strong className="text-white">{req.value}</strong></span>
                      <span>Limit: {req.limit}</span>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          <div className="p-2.5 rounded-[8px] bg-[#13253D] border border-[#22395A] text-[10px] font-mono text-[#7F93AF]">
            Note: Requirements validation runs deterministically against solver outputs. Physical telemetry sync required for full sign-off.
          </div>
        </div>

        {/* CENTER VIEWPORT OVERLAY: 3D REQUIREMENT MARKERS (Section 6.13) */}
        <div className="flex-1 relative flex flex-col justify-between p-4 pointer-events-none">
          <div className="p-3 rounded-[12px] bg-[#0A1422]/85 backdrop-blur-md border border-[#22395A] max-w-sm pointer-events-auto space-y-1">
            <div className="text-xs font-bold text-[#F2F6FB] uppercase font-mono tracking-wider">
              REQUIREMENT SPATIAL MARKERS
            </div>
            <div className="text-[11px] text-[#7F93AF] font-mono">
              Hover pins on 3D geometry to trace sensor validation points
            </div>
          </div>

          {/* Bottom Card: Evidence Package Composer (Six Items) */}
          <div className="p-3 rounded-[14px] bg-[#0A1422]/90 backdrop-blur-md border border-[#22395A] pointer-events-auto space-y-2">
            <div className="flex items-center justify-between border-b border-[#22395A]/80 pb-2">
              <div className="flex items-center gap-2">
                <span className="text-xs font-bold text-white uppercase font-mono">
                  Evidence Package Composer (06 Items Included)
                </span>
                <span className="text-[9px] px-1.5 py-0.2 rounded bg-blue-500/20 text-cyan-300 font-mono">
                  ASME V&V 40
                </span>
              </div>
              <button
                onClick={onOpenEvidenceSheet}
                className="px-3 py-1 rounded-[6px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6]"
              >
                Sign and export →
              </button>
            </div>

            <div className="grid grid-cols-3 gap-2 text-xs font-mono">
              {evidenceItems.map((item, idx) => (
                <div key={idx} className="p-1.5 rounded bg-[#0E1B2E] border border-[#22395A] flex items-center gap-2 text-[#D6E2F3]">
                  <span className="text-[#22C55E]">✓</span>
                  <span className="truncate">{item}</span>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* RIGHT PANEL: CREDIBILITY CHECKLIST & HONESTY CARD (Section 6.13) */}
        <div className="w-80 border-l border-[#22395A]/80 bg-[#0A1422]/85 backdrop-blur-md flex flex-col justify-between shrink-0 pointer-events-auto p-4 space-y-4 overflow-y-auto">
          <div className="space-y-3">
            <div className="border-b border-[#22395A] pb-2 flex items-center justify-between">
              <span className="text-xs font-bold text-[#F2F6FB] uppercase tracking-wider font-mono">
                Credibility Checklist
              </span>
              <span className="text-[10px] font-mono text-cyan-300">V&V Gaps</span>
            </div>

            <div className="space-y-2">
              {credibilityChecklist.map((c, idx) => (
                <div key={idx} className="p-2.5 rounded-[8px] bg-[#0E1B2E] border border-[#22395A] space-y-1">
                  <div className="flex items-center justify-between text-xs">
                    <span className="font-semibold text-white truncate">{c.title}</span>
                    <span
                      className={`text-[9px] font-mono px-1 rounded uppercase ${
                        c.status === 'passed' ? 'text-[#4ADE80] bg-[#22C55E]/15' : 'text-amber-300 bg-amber-500/15'
                      }`}
                    >
                      {c.status}
                    </span>
                  </div>
                  <div className="text-[10px] font-mono text-[#7F93AF]">{c.note}</div>
                </div>
              ))}
            </div>
          </div>

          {/* Fact-based Honesty Card (Section 6.13: "Not yet verified for this use") */}
          <div className="p-3.5 rounded-[12px] bg-amber-950/40 border border-amber-500/50 space-y-1.5">
            <div className="flex items-center gap-2 text-amber-300 font-bold text-xs uppercase font-mono">
              <span>⚠</span>
              <span>Not yet verified for this use</span>
            </div>
            <p className="text-[11px] text-[#D6E2F3]">
              While mathematical convergence and determinism are proven, independent peer review and physical bench reference comparison remain unfulfilled.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};
