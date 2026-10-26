import React, { useState } from 'react';

export interface EvidenceSheetProps {
  isOpen: boolean;
  onClose: () => void;
}

export const EvidenceSheet: React.FC<EvidenceSheetProps> = ({ isOpen, onClose }) => {
  const [copied, setCopied] = useState<boolean>(false);

  if (!isOpen) return null;

  const rootHash = '0x789030e4f2a1b9c89134a6ef24d108bc';
  const verifierCmd = 'helqix verify pkg-789030.evidence';

  const handleCopy = () => {
    navigator.clipboard?.writeText(verifierCmd);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const packageTree = [
    { title: 'Model Revision AST (r1)', size: '42.1 kB', hash: '0x12a9...c4f1' },
    { title: 'Bitwise Simulation Frames (Run 19)', size: '1.24 MB', hash: '0x33b8...92a1' },
    { title: 'Dynamic Boundary Conditions', size: '12.8 kB', hash: '0x99e4...b8f0' },
    { title: 'Solver Convergence Log (RK4)', size: '85.2 kB', hash: '0x77c2...51e8' },
    { title: 'ASME V&V 40 Checklist Audit', size: '18.4 kB', hash: '0x44d1...88b2' },
    { title: 'Ed25519 Cryptographic Manifest', size: '1.02 kB', hash: '0x7890...08bc' }
  ];

  return (
    <div className="fixed inset-0 z-50 flex justify-end bg-black/60 backdrop-blur-sm animate-fade-in select-none">
      <div className="w-full max-w-lg h-full bg-[#0A1422] border-l border-[#22395A] p-6 flex flex-col justify-between shadow-[-10px_0_40px_rgba(0,0,0,0.8)] overflow-y-auto">
        <div className="space-y-5">
          {/* Header */}
          <div className="flex items-center justify-between border-b border-[#22395A] pb-4">
            <div className="space-y-1">
              <div className="flex items-center gap-2">
                <span className="text-sm font-bold text-white uppercase font-mono">
                  Evidence Package Sheet
                </span>
                <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-purple-500/20 text-purple-300">
                  Ed25519
                </span>
              </div>
              <p className="text-xs text-[#7F93AF]">
                Formal computational package for independent engineering verification.
              </p>
            </div>
            <button
              onClick={onClose}
              className="text-[#7F93AF] hover:text-white p-1 text-base font-mono"
            >
              ✕
            </button>
          </div>

          {/* Root Hash & Signature Status (Section 6.14) */}
          <div className="p-3.5 rounded-[12px] bg-[#0E1B2E] border border-[#22395A] space-y-2.5 font-mono text-xs">
            <div className="flex items-center justify-between">
              <span className="text-[#7F93AF]">Signature Status:</span>
              <span className="text-[#4ADE80] font-bold flex items-center gap-1.5">
                <span>✓</span>
                <span>Self-signed (Ed25519)</span>
              </span>
            </div>

            <div className="space-y-1">
              <span className="text-[#7F93AF] text-[10px]">Merkle Root Hash:</span>
              <div className="p-2 rounded bg-[#13253D] text-[11px] text-cyan-300 break-all border border-[#22395A]">
                {rootHash}
              </div>
            </div>

            {/* Verifier Command with Copy (Section 6.14) */}
            <div className="space-y-1 pt-1">
              <span className="text-[#7F93AF] text-[10px]">Independent Verifier Command:</span>
              <div className="flex items-center justify-between p-2 rounded bg-[#050B18] border border-[#22395A]">
                <code className="text-white text-[11px]">{verifierCmd}</code>
                <button
                  onClick={handleCopy}
                  className="px-2 py-0.5 rounded text-[10px] bg-[#2F7BFF]/20 border border-[#2F7BFF]/40 text-cyan-300 hover:bg-[#2F7BFF]/40 transition-colors"
                >
                  {copied ? 'Copied!' : 'Copy'}
                </button>
              </div>
            </div>
          </div>

          {/* Package Contents Tree (Section 6.14) */}
          <div className="space-y-2">
            <div className="flex items-center justify-between text-xs font-mono font-bold text-white uppercase">
              <span>Package Contents Tree (06 Items)</span>
              <span className="text-[#7F93AF]">1.38 MB Total</span>
            </div>

            <div className="space-y-1.5">
              {packageTree.map((item, idx) => (
                <div
                  key={idx}
                  className="p-2.5 rounded-[8px] bg-[#0E1B2E] border border-[#22395A] flex items-center justify-between text-xs font-mono"
                >
                  <div className="space-y-0.5 truncate pr-2">
                    <div className="text-white font-medium truncate">{item.title}</div>
                    <div className="text-[10px] text-[#7F93AF]">{item.hash}</div>
                  </div>
                  <span className="text-cyan-300 text-[11px] shrink-0">{item.size}</span>
                </div>
              ))}
            </div>
          </div>

          {/* Mandatory Honesty Note (Section 6.14 & Rule 2 & 10) */}
          <div className="p-3.5 rounded-[12px] bg-[#13253D] border border-blue-500/40 space-y-1 text-xs">
            <div className="text-white font-bold font-mono text-[11px] flex items-center gap-1.5">
              <span>ℹ</span>
              <span>Engineering Integrity Notice</span>
            </div>
            <p className="text-[#D6E2F3] text-[11px]">
              "Signing proves origin and integrity, not correct physics."
            </p>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex items-center gap-3 pt-4 border-t border-[#22395A]">
          <button
            onClick={() => alert('Evidence package exported to disk: pkg-789030.evidence')}
            className="flex-1 py-2.5 rounded-[8px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6] transition-all shadow-[0_0_20px_rgba(47,123,255,0.4)]"
          >
            Export Evidence (.evidence)
          </button>
          <button
            onClick={() => alert('PDF Engineering Report generated: helqix_r1_audit_report.pdf')}
            className="px-4 py-2.5 rounded-[8px] text-xs font-semibold text-[#D6E2F3] bg-[#13253D] border border-[#22395A] hover:text-white"
          >
            Generate Report (PDF)
          </button>
        </div>
      </div>
    </div>
  );
};
