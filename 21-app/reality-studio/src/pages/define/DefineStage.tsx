import React, { useState } from 'react';
import { ModelRevision } from '../../contracts';
import type { ScenePart } from '../../scene/HeroScene';
import { DefineStep } from '../studio/Studio';
import { BuildInput, readBuildInput, writeBuildInput } from '../../app/draft';
import { IntentStep } from './IntentStep';
import { RequirementsStep } from './RequirementsStep';
import { ArchitectureStep } from './ArchitectureStep';
import { ComponentsStep } from './ComponentsStep';
import { AssembleStep } from './AssembleStep';

export interface DefineStageProps {
  projectId: string;
  currentStep: DefineStep;
  /** Revision label from the URL (rev), or null while the model is still a draft. */
  committedRev: string | null;
  onStepChange: (step: DefineStep) => void;
  onCommitted: (revision: ModelRevision) => void;
  onRunSimulation: () => void;
  onPartsChange: (parts: ScenePart[]) => void;
}

export const DefineStage: React.FC<DefineStageProps> = ({
  projectId,
  currentStep,
  committedRev,
  onStepChange,
  onCommitted,
  onRunSimulation,
  onPartsChange
}) => {
  const [input, setInput] = useState<BuildInput | null>(() => readBuildInput(projectId));

  const choose = (next: BuildInput, step: DefineStep) => {
    writeBuildInput(projectId, next);
    setInput(next);
    onStepChange(step);
  };

  const promptText = input?.kind === 'prompt' ? input.text : '';

  switch (currentStep) {
    case 'intent':
      return (
        <IntentStep
          projectId={projectId}
          initialText={promptText}
          onContinue={text => choose({ kind: 'prompt', text }, 'requirements')}
          onStartScratch={() => choose({ kind: 'scratch' }, 'assemble')}
        />
      );
    case 'requirements':
      return <RequirementsStep promptText={promptText} onContinue={() => onStepChange('architecture')} />;
    case 'architecture':
      return <ArchitectureStep onContinue={() => onStepChange('components')} />;
    case 'components':
      return <ComponentsStep onContinue={() => onStepChange('assemble')} />;
    case 'assemble':
      if (!input) {
        // Deep link straight to Assemble with nothing chosen yet: ask, rather than invent a prompt.
        return (
          <div className="m-auto z-10 rounded-[16px] bg-[#0E1B2E]/90 border border-[#22395A] p-6 space-y-3 text-sm text-[#D6E2F3] max-w-md">
            <p className="font-semibold text-[#F2F6FB]">Nothing to assemble yet.</p>
            <p className="text-xs text-[#7F93AF]">Describe what you want to engineer, or start from scratch.</p>
            <div className="flex gap-2">
              <button className="px-3 py-1.5 rounded-md bg-[#2F7BFF] text-white text-xs" onClick={() => onStepChange('intent')}>Describe it</button>
              <button className="px-3 py-1.5 rounded-md border border-[#22395A] text-xs" onClick={() => choose({ kind: 'scratch' }, 'assemble')}>Start from scratch</button>
            </div>
          </div>
        );
      }
      return (
        <AssembleStep
          projectId={projectId}
          prompt={input.kind === 'prompt' ? input.text : null}
          committedRev={committedRev}
          onCommitSuccess={onCommitted}
          onRunSimulation={onRunSimulation}
          onPartsChange={onPartsChange}
        />
      );
    default:
      return null;
  }
};
