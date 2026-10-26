import React from 'react';

export interface ActivityDrawerProps {
  isOpen: boolean;
  onClose: () => void;
}

export const ActivityDrawer: React.FC<ActivityDrawerProps> = ({ isOpen, onClose }) => {
  if (!isOpen) return null;

  const activities = [
    { title: 'Checkpoint cp-800 Created', time: '2 min ago', desc: 'State snapshot at t=8.00s saved with hash 0x789030', kind: 'checkpoint' },
    { title: 'Simulation Trip Flagged', time: '5 min ago', desc: 'Thermal threshold 120.0 °C exceeded on stator at 8.42s', kind: 'warning' },
    { title: 'Model Revision r1 Committed', time: '12 min ago', desc: '4 parts, 3 ports, 3 connections committed to project', kind: 'commit' },
    { title: 'Unit Discrepancy Auto-Corrected', time: '14 min ago', desc: 'Rated voltage parameter updated to 48.0 V DC nominal', kind: 'fix' }
  ];

  return (
    <div className="fixed inset-0 z-50 flex justify-end bg-black/60 backdrop-blur-sm select-none animate-fade-in">
      <div className="w-full max-w-md h-full bg-[#0A1422] border-l border-[#22395A] p-6 flex flex-col justify-between shadow-[-10px_0_40px_rgba(0,0,0,0.8)] overflow-y-auto">
        <div className="space-y-4">
          <div className="flex items-center justify-between border-b border-[#22395A] pb-3">
            <div className="flex items-center gap-2">
              <span className="text-white text-base">🔔</span>
              <span className="text-sm font-bold text-white font-mono uppercase">Activity Log</span>
            </div>
            <button onClick={onClose} className="text-[#7F93AF] hover:text-white p-1 text-base">✕</button>
          </div>

          <div className="space-y-2">
            {activities.map((act, idx) => (
              <div key={idx} className="p-3 rounded-[10px] bg-[#0E1B2E] border border-[#22395A] space-y-1 text-xs">
                <div className="flex items-center justify-between font-mono">
                  <span className="text-white font-semibold">{act.title}</span>
                  <span className="text-[10px] text-[#7F93AF]">{act.time}</span>
                </div>
                <div className="text-[11px] text-[#7F93AF]">{act.desc}</div>
              </div>
            ))}
          </div>
        </div>

        <button onClick={onClose} className="w-full py-2 rounded-[8px] text-xs font-semibold text-white bg-[#13253D] hover:bg-[#1E3A5F]">
          Close Log
        </button>
      </div>
    </div>
  );
};
