import React, { useState } from 'react';
import { useAppState } from '../../engineering/stateStore';
import { HeroScene } from '../../scene/HeroScene';
import { PreviewTag, DeterminismChip, OriginStatusBadges } from '../../components/ui/HonestyBadges';
import { MOCK_VALIDATION_RULES, ValidationRow } from '../../mock/engineeringData';

export const VerifyValidationPage: React.FC = () => {
  const { qualityTier, determinism, showToast } = useAppState();

  const [selectedRule, setSelectedRule] = useState<ValidationRow>(MOCK_VALIDATION_RULES[4]); // Failing rule

  const allPassed = MOCK_VALIDATION_RULES.every(r => r.passed);

  return (
    <div className="relative w-full h-full flex flex-col justify-between overflow-hidden p-3 gap-3">
      {/* Top Header */}
      <div className="flex items-center justify-between z-20 shrink-0">
        <div className="flex items-center gap-2">
          <span className="text-xs font-bold font-mono text-cyan-300">
            VERIFY // ASME V&V 40 CREDIBILITY & FORMAL EVIDENCE
          </span>
          <PreviewTag label="PREVIEW (Phase 5)" />
        </div>

        <div className="flex items-center gap-2">
          {/* Status only says PASS if all criteria pass (Honesty rule!) */}
          <span
            className={`px-3 py-1 rounded-full text-xs font-bold font-mono border ${
              allPassed
                ? 'bg-emerald-950/80 border-emerald-500/60 text-emerald-300'
                : 'bg-rose-950/80 border-rose-500/60 text-rose-300 shadow-[0_0_12px_rgba(239,68,68,0.4)]'
            }`}
          >
            {allPassed ? 'STATUS: PASS' : 'STATUS: REVIEW FAILED (1 REQ)'}
          </span>
          <DeterminismChip level={determinism} />
        </div>
      </div>

      {/* Main Grid: Left Checklist, Center 3D Shield, Right Validation Report Card */}
      <div className="flex-1 grid grid-cols-12 gap-3 min-h-0">
        {/* Left: Validation Rules Checklist (4 Cols - Prompt 14) */}
        <div className="col-span-4 glass-panel p-3.5 flex flex-col gap-2.5 overflow-y-auto">
          <div className="border-b border-slate-800 pb-2 flex items-center justify-between">
            <span className="text-xs font-bold text-white">Validation Criteria Checklist</span>
            <span className="text-[10px] font-mono text-rose-400">1 Failing Req</span>
          </div>

          <div className="space-y-2">
            {MOCK_VALIDATION_RULES.map(rule => {
              const isSelected = selectedRule.id === rule.id;
              return (
                <button
                  key={rule.id}
                  onClick={() => setSelectedRule(rule)}
                  className={`w-full p-2.5 rounded-xl text-left text-xs transition-all flex items-start gap-2.5 border ${
                    isSelected
                      ? 'bg-blue-600/30 border-cyan-400/60 shadow-[0_0_12px_rgba(34,211,238,0.2)]'
                      : rule.passed
                      ? 'bg-slate-900/60 border-slate-800 hover:border-slate-700'
                      : 'bg-rose-950/30 border-rose-500/40 hover:border-rose-400'
                  }`}
                >
                  <span
                    className={`w-4 h-4 rounded-full mt-0.5 shrink-0 flex items-center justify-center font-bold text-[10px] ${
                      rule.passed ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/40' : 'bg-rose-500/20 text-rose-400 border border-rose-500/60'
                    }`}
                  >
                    {rule.passed ? '✓' : '✕'}
                  </span>
                  <div className="flex-1">
                    <div className={rule.passed ? 'font-semibold text-slate-200' : 'font-bold text-rose-300'}>
                      {rule.name}
                    </div>
                    <div className="text-[11px] text-slate-400 font-mono mt-0.5">{rule.count}</div>
                  </div>
                </button>
              );
            })}
          </div>
        </div>

        {/* Center: Large Shield on 3D Model (4 Cols - Prompt 14) */}
        <div className="col-span-4 relative rounded-2xl overflow-hidden border border-blue-500/20 bg-[#050B18] flex items-center justify-center">
          <div className="absolute inset-0">
            <HeroScene
              variant="workspace"
              interactive={true}
              qualityTier={qualityTier}
            />
          </div>

          {/* Large Shield Overlay */}
          <div className="relative z-10 p-6 rounded-3xl glass-panel-elevated border border-cyan-400/40 flex flex-col items-center gap-3 shadow-[0_0_40px_rgba(47,123,255,0.35)]">
            <div className="w-20 h-20 rounded-2xl bg-gradient-to-tr from-blue-600 to-cyan-400 flex items-center justify-center text-white shadow-[0_0_25px_rgba(34,211,238,0.5)]">
              <svg className="w-10 h-10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
                <polyline points="9 12 11 14 15 10" />
              </svg>
            </div>
            <div className="text-center">
              <div className="text-sm font-bold text-white">ASME V&V 40 Engine</div>
              <div className="text-[11px] text-cyan-300 font-mono mt-0.5">Formal Proof Inspector</div>
            </div>
          </div>
        </div>

        {/* Right: Validation Report Card (4 Cols - Prompt 14) */}
        <div className="col-span-4 glass-panel p-3.5 flex flex-col justify-between gap-3 overflow-y-auto">
          <div className="space-y-3">
            <div className="border-b border-slate-800 pb-2">
              <span className="text-xs font-bold text-white">Criterion Audit Report</span>
              <div className="text-[11px] text-cyan-300 font-medium mt-0.5">{selectedRule.name}</div>
            </div>

            <div className="space-y-2 text-xs">
              <div className="p-2 rounded bg-slate-900/60 border border-slate-800">
                <span className="text-slate-500 text-[10px] block">Context of Use</span>
                <span className="text-slate-200">{selectedRule.context}</span>
              </div>

              <div className="p-2 rounded bg-slate-900/60 border border-slate-800">
                <span className="text-slate-500 text-[10px] block">Model Error vs Threshold</span>
                <span className={`font-mono font-bold ${selectedRule.passed ? 'text-emerald-400' : 'text-rose-400'}`}>
                  {selectedRule.errorVsThreshold}
                </span>
              </div>

              <div className="grid grid-cols-2 gap-2">
                <div className="p-2 rounded bg-slate-900/60 border border-slate-800">
                  <span className="text-slate-500 text-[10px] block">Uncertainty Bound</span>
                  <span className="font-mono text-white num">{selectedRule.uncertainty}</span>
                </div>
                <div className="p-2 rounded bg-slate-900/60 border border-slate-800">
                  <span className="text-slate-500 text-[10px] block">Credibility Level</span>
                  <span className="font-mono text-cyan-300">{selectedRule.credibility}</span>
                </div>
              </div>
            </div>

            {/* Signed status line */}
            <div className="p-2 rounded-lg bg-blue-950/30 border border-blue-500/20 text-[10px] font-mono text-slate-400 space-y-0.5">
              <div>Signed: Yes (ECDSA P-256)</div>
              <div>Verifier: `helqix verify --rigorous --seed 8421`</div>
            </div>
          </div>

          <button
            onClick={() => showToast('Generating tamper-proof evidence package archive...', 'success')}
            className="w-full ui-btn ui-btn-primary py-3 justify-center text-xs font-semibold shadow-[0_0_20px_rgba(47,123,255,0.4)]"
          >
            <span>Generate Evidence Package</span>
            <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" y1="15" x2="12" y2="3" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
};
