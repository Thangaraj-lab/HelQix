import React, { useEffect } from 'react';
import { useAppState } from '../../engineering/stateStore';
import { DeterminismChip } from '../ui/UiKit';

export const TimelineBar: React.FC<{
  showFreezeBtn?: boolean;
  showHashMatch?: boolean;
  className?: string;
}> = ({ showFreezeBtn = true, showHashMatch = false, className = '' }) => {
  const {
    currentTime,
    totalDuration,
    isPlaying,
    togglePlay,
    seekTo,
    freezeAtCurrent,
    freezeTime,
    playbackSpeed,
    determinism,
    replayHashMatch
  } = useAppState();

  const formatTime = (secs: number) => {
    const mins = Math.floor(secs / 60);
    const remainingSecs = (secs % 60).toFixed(2);
    return `${mins.toString().padStart(2, '0')}:${remainingSecs.padStart(5, '0')}`;
  };

  // Keyboard controls: Space to play/pause, arrow keys to step
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      if (e.code === 'Space') {
        e.preventDefault();
        togglePlay();
      } else if (e.code === 'ArrowLeft') {
        e.preventDefault();
        seekTo(currentTime - 0.2);
      } else if (e.code === 'ArrowRight') {
        e.preventDefault();
        seekTo(currentTime + 0.2);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [currentTime, togglePlay, seekTo]);

  const progressPercent = (currentTime / totalDuration) * 100;
  const failurePercent = (8.42 / totalDuration) * 100;

  return (
    <div className={`glass-panel p-3 px-4 flex flex-col gap-2 ${className}`}>
      <div className="flex items-center justify-between gap-4">
        {/* Playback Controls */}
        <div className="flex items-center gap-2">
          <button
            onClick={togglePlay}
            className="w-9 h-9 rounded-full flex items-center justify-center bg-blue-600/30 border border-blue-400/60 text-cyan-300 hover:bg-blue-600/50 hover:scale-105 transition-all shadow-[0_0_15px_rgba(47,123,255,0.4)]"
            title={isPlaying ? 'Pause (Space)' : 'Play (Space)'}
          >
            {isPlaying ? (
              <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                <rect x="6" y="4" width="4" height="16" rx="1" />
                <rect x="14" y="4" width="4" height="16" rx="1" />
              </svg>
            ) : (
              <svg className="w-4 h-4 ml-0.5" fill="currentColor" viewBox="0 0 24 24">
                <path d="M5 3l14 9-14 9V3z" />
              </svg>
            )}
          </button>

          <button
            onClick={() => seekTo(0)}
            className="ui-btn-icon"
            title="Stop & Reset"
          >
            <svg className="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 24 24">
              <rect x="4" y="4" width="16" height="16" rx="2" />
            </svg>
          </button>

          <button
            onClick={() => seekTo(currentTime - 0.5)}
            className="ui-btn-icon"
            title="Step Backward (Left Arrow)"
          >
            <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
              <path d="M11 19l-9-7 9-7v14zm11 0l-9-7 9-7v14z" />
            </svg>
          </button>

          <button
            onClick={() => seekTo(currentTime + 0.5)}
            className="ui-btn-icon"
            title="Step Forward (Right Arrow)"
          >
            <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
              <path d="M4 5l9 7-9 7V5zm11 0l9 7-9 7V5z" />
            </svg>
          </button>

          {/* Time Readout */}
          <div className="flex items-center gap-1.5 ml-2 font-mono text-xs">
            <span className="text-cyan-300 font-semibold num">{formatTime(currentTime)}</span>
            <span className="text-slate-500">/</span>
            <span className="text-slate-400 num">{formatTime(totalDuration)}</span>
          </div>
        </div>

        {/* Phase Chips */}
        <div className="flex items-center gap-1.5 text-[11px] font-medium">
          <button
            onClick={() => seekTo(0.5)}
            className="px-2.5 py-1 rounded-full bg-slate-800/80 border border-slate-700/80 text-slate-300 hover:border-slate-500 flex items-center gap-1.5"
          >
            <span className="w-1.5 h-1.5 rounded-full bg-slate-300"></span>
            Init
          </button>
          <button
            onClick={() => seekTo(4.2)}
            className="px-2.5 py-1 rounded-full bg-blue-950/80 border border-blue-500/50 text-cyan-300 hover:border-cyan-400 flex items-center gap-1.5 shadow-[0_0_10px_rgba(47,123,255,0.25)]"
          >
            <span className="w-1.5 h-1.5 rounded-full bg-cyan-400"></span>
            Run
          </button>
          <button
            onClick={() => seekTo(8.42)}
            className="px-2.5 py-1 rounded-full bg-rose-950/80 border border-rose-500/60 text-rose-300 hover:border-rose-400 flex items-center gap-1.5"
          >
            <span className="w-1.5 h-1.5 rounded-full bg-rose-500 animate-ping"></span>
            Event (8.42s)
          </button>
          <button
            onClick={() => seekTo(11.8)}
            className="px-2.5 py-1 rounded-full bg-emerald-950/80 border border-emerald-500/50 text-emerald-300 hover:border-emerald-400 flex items-center gap-1.5"
          >
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
            Result
          </button>
        </div>

        {/* Right side chips & freeze button */}
        <div className="flex items-center gap-2">
          {showFreezeBtn && (
            <button
              onClick={freezeAtCurrent}
              className="px-2.5 py-1 rounded text-xs font-medium bg-slate-800/90 border border-blue-500/40 text-blue-300 hover:bg-blue-600/20 hover:border-cyan-400 transition-colors flex items-center gap-1.5"
              title="Capture immutable state freeze snapshot"
            >
              <svg className="w-3 h-3 text-cyan-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M12 2v20M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6" />
              </svg>
              <span>Freeze here</span>
              {freezeTime !== null && (
                <span className="text-[10px] text-cyan-400 font-mono">({freezeTime.toFixed(1)}s)</span>
              )}
            </button>
          )}

          {showHashMatch && (
            <div className="px-2 py-0.5 rounded text-[11px] font-mono font-semibold bg-emerald-950/70 border border-emerald-500/50 text-emerald-300 flex items-center gap-1.5">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
              <span>Replay matches original: PASS</span>
            </div>
          )}

          <DeterminismChip level={determinism} />
        </div>
      </div>

      {/* Scrubbing Track & Sparkline */}
      <div className="relative w-full h-8 flex items-center">
        {/* Background Mini Waveform / Sparkline */}
        <svg className="absolute inset-0 w-full h-full opacity-25 pointer-events-none" preserveAspectRatio="none" viewBox="0 0 100 24">
          <path
            d="M0,16 Q15,10 25,14 T50,8 T75,18 T90,6 T100,12"
            fill="none"
            stroke="#22D3EE"
            strokeWidth="1.2"
          />
          <path
            d="M0,18 Q20,14 40,16 T70,12 T85,22 T100,16"
            fill="none"
            stroke="#8B5CF6"
            strokeWidth="0.8"
          />
        </svg>

        {/* Failure Marker Line */}
        <div
          className="absolute top-0 bottom-0 w-0.5 bg-rose-500 z-10 pointer-events-none"
          style={{ left: `${failurePercent}%` }}
        >
          <div className="absolute -top-1 -left-1 w-2.5 h-2.5 rounded-full bg-rose-500 shadow-[0_0_8px_#EF4444]" />
        </div>

        {/* Input Range Slider */}
        <input
          type="range"
          min="0"
          max={totalDuration}
          step="0.01"
          value={currentTime}
          onChange={(e) => seekTo(parseFloat(e.target.value))}
          className="w-full relative z-20"
        />

        {/* Playhead Glow Indicator */}
        <div
          className="absolute top-1/2 -translate-y-1/2 pointer-events-none z-30 transition-all duration-75"
          style={{ left: `calc(${progressPercent}% - 6px)` }}
        >
          <div className="w-3 h-3 rounded-full bg-cyan-400 shadow-[0_0_12px_#22D3EE] border border-white" />
        </div>
      </div>
    </div>
  );
};
