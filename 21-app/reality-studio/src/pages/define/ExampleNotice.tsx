import React from 'react';
import { ExampleTag } from '../../components/ui/UiKit';

// Shown on Define steps whose content is the canned example, not derived from the user's input.
export const ExampleNotice: React.FC = () => (
  <div role="note" className="flex items-center gap-2 px-3 py-2 rounded-[8px] bg-amber-500/10 border border-amber-500/30 text-[11px] text-amber-200">
    <ExampleTag />
    <span>Example content. This step does not read your description yet.</span>
  </div>
);
