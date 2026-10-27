import React from 'react';
import { ExampleTag } from '../../components/ui/UiKit';

/**
 * Shared frame for the Define steps. The header's single primary button moves to the next step, so
 * steps do not carry their own "Continue". `example` marks content that is canned, not derived from
 * the user's description (it is stated once, at the top, rather than on every card).
 */
export const StepFrame: React.FC<{
  title: string;
  subtitle: string;
  example?: boolean;
  width?: string;
  children: React.ReactNode;
}> = ({ title, subtitle, example = false, width = 'max-w-[1120px]', children }) => (
  <div className="relative z-10 flex-1 min-h-0 overflow-y-auto">
    <div className={`${width} mx-auto px-6 py-7 space-y-5 animate-rise`}>
      <div className="flex items-end justify-between gap-4">
        <div>
          <h2 className="font-display font-light text-[22px] text-strong">{title}</h2>
          <p className="text-[13px] text-muted mt-1">{subtitle}</p>
        </div>
        {example && (
          <div role="note" className="flex items-center gap-2 text-[11px] text-muted shrink-0">
            <ExampleTag />
            <span>Example content. This step does not read your description yet.</span>
          </div>
        )}
      </div>
      {children}
    </div>
  </div>
);
