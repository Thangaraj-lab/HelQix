import React from 'react';
import { navigate } from '../app/router';

// Hero (file 35 section 12, route /). New Project goes to /new, which creates the project and opens
// the Prompt screen: no modal, no domain question, no model-type cards (file 36 M2).
export const LandingPage: React.FC = () => {
  return (
    <div className="relative w-screen h-screen overflow-hidden bg-[#020611] select-none flex flex-col justify-between items-center text-white font-['Outfit']">
      {/* 1. Cinematic Horizon & Glowing Portal Ring Background (Attached Image Replica) */}
      <div
        className="absolute inset-0 bg-cover bg-center bg-no-repeat pointer-events-none"
        style={{
          backgroundImage: `url('/assets/helqix-portal.jpg')`
        }}
      >
        {/* Deep ambient vignette & subtle ray overlays */}
        <div className="absolute inset-0 bg-gradient-to-t from-[#020611] via-transparent to-[#020611]/60" />
        <div className="absolute inset-0 bg-[radial-gradient(circle_at_50%_45%,rgba(34,211,238,0.12)_0%,transparent_60%)]" />
        {/* The image has a wordmark and pill painted in; mask them so only the real (accessible) ones show. */}
        <div className="absolute inset-0 bg-[radial-gradient(ellipse_36%_17%_at_50%_49%,#030a18_55%,rgba(3,10,24,0.85)_72%,transparent_100%)]" />
      </div>

      {/* 2. Top Minimalist Header */}
      <header className="relative z-20 w-full px-8 py-6 flex items-center justify-between">
        {/* Brand Mark */}
        <div className="flex items-center gap-3">
          <div className="w-7 h-7 rounded-md bg-[#2F7BFF] flex items-center justify-center text-white font-mono text-xs font-bold shadow-[0_0_15px_rgba(47,123,255,0.8)]">
            ⬡
          </div>
          <span className="font-['Space_Grotesk'] font-bold tracking-[0.25em] text-[#F2F6FB] text-sm">
            HELQIX
          </span>
        </div>


        {/* Top Right Action: Launch Studio Directly or New Project */}
        <div className="flex items-center gap-3">
          <button
            onClick={() => navigate('/home')}
            className="px-3.5 py-1.5 rounded-lg text-xs font-mono text-[#D6E2F3] hover:text-white hover:bg-white/5 border border-white/10 transition-all"
          >
            Open Studio
          </button>
          <button
            onClick={() => navigate('/new')}
            className="px-4 py-1.5 rounded-lg text-xs font-mono font-semibold text-white bg-gradient-to-r from-[#2F7BFF] to-[#06B6D4] hover:from-[#3B82F6] hover:to-[#22D3EE] shadow-[0_0_20px_rgba(47,123,255,0.4)] transition-all flex items-center gap-1.5"
          >
            <span>+</span>
            <span>New Project</span>
          </button>
        </div>
      </header>

      {/* 3. Center Hero: Exact Attached Image Branding (H E L Q I X with glowing Q & pill button) */}
      <main className="relative z-20 flex-1 flex flex-col items-center justify-center text-center px-4 my-auto">
        {/* Main Brand Title: H E L Q I X */}
        <div className="flex items-center justify-center tracking-[0.35em] text-5xl sm:text-7xl md:text-8xl font-['Space_Grotesk'] font-light text-white select-none drop-shadow-[0_0_35px_rgba(47,123,255,0.4)]">
          <span>H</span>
          <span className="ml-4 md:ml-8">E</span>
          <span className="ml-4 md:ml-8">L</span>
          {/* Distinctive Glowing Q with luminous cyan ring and angled tail */}
          <span className="relative ml-4 md:ml-8 inline-flex items-center justify-center">
            <span className="text-[#22D3EE] font-medium drop-shadow-[0_0_25px_rgba(34,211,238,0.9)]">
              Q
            </span>
            <span className="absolute inset-0 rounded-full border border-[#22D3EE]/60 blur-[2px] animate-pulse pointer-events-none" />
          </span>
          <span className="ml-4 md:ml-8">I</span>
          <span className="ml-4 md:ml-8">X</span>
        </div>

        {/* Subtitle */}
        <p className="mt-4 text-xs md:text-sm font-['Space_Grotesk'] tracking-[0.2em] text-[#7F93AF] uppercase">
          Physical reality engineering with mathematical proof
        </p>

        {/* Centered Pill Button with Arrow: [   →   ] */}
        <div className="mt-10">
          <button
            onClick={() => navigate('/new')}
            aria-label="Create New Project"
            className="group relative px-14 py-3.5 rounded-full border border-[#2F7BFF]/60 bg-[#0A1422]/50 backdrop-blur-md hover:bg-[#2F7BFF]/25 hover:border-[#22D3EE] shadow-[0_0_25px_rgba(47,123,255,0.35)] hover:shadow-[0_0_45px_rgba(34,211,238,0.8)] transition-all duration-300 flex items-center justify-center cursor-pointer"
          >
            {/* Subtle glow beacon */}
            <span className="absolute inset-0 rounded-full bg-gradient-to-r from-transparent via-[#22D3EE]/15 to-transparent opacity-0 group-hover:opacity-100 transition-opacity" />
            
            {/* Arrow icon */}
            <span className="text-2xl text-[#22D3EE] group-hover:translate-x-2 transition-transform duration-300 font-light">
              →
            </span>
          </button>
        </div>
      </main>

      {/* 4. Footer: what the product is today, without claims it has not earned */}
      <footer className="relative z-20 w-full max-w-5xl px-8 py-5 border-t border-[#22395A]/40 flex items-center justify-between text-xs font-mono text-[#7F93AF]">
        <span className="text-white">Engineering truth, with proof</span>
        <span>Prototype · results are labelled Simulated or Observed, and Unverified until checked</span>
      </footer>

    </div>
  );
};
