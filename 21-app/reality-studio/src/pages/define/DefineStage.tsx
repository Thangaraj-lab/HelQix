import React, { useState, useEffect } from 'react';
import { DefineStep } from '../studio/Studio';
import { IntentStep } from './IntentStep';
import { RequirementsStep } from './RequirementsStep';
import { ArchitectureStep } from './ArchitectureStep';
import { ComponentsStep } from './ComponentsStep';
import { AssembleStep } from './AssembleStep';
import { EXAMPLE_PROMPT } from '../../mock/examplePack';

export interface DefineStageProps {
  currentStep: DefineStep;
  onStepChange: (step: DefineStep) => void;
  onCommitToAssemble: () => void;
}

export const DefineStage: React.FC<DefineStageProps> = ({
  currentStep,
  onStepChange,
  onCommitToAssemble
}) => {
  const [promptText, setPromptText] = useState(EXAMPLE_PROMPT);

  useEffect(() => {
    const saved = sessionStorage.getItem('helqix_initial_prompt');
    if (saved) setPromptText(saved);
  }, []);

  switch (currentStep) {
    case 'intent':
      return (
        <IntentStep
          onContinue={text => {
            setPromptText(text);
            sessionStorage.setItem('helqix_initial_prompt', text);
            onStepChange('requirements');
          }}
          onStartScratch={() => {
            onStepChange('assemble');
          }}
        />
      );
    case 'requirements':
      return (
        <RequirementsStep
          promptText={promptText}
          onContinue={() => onStepChange('architecture')}
        />
      );
    case 'architecture':
      return (
        <ArchitectureStep
          onContinue={() => onStepChange('components')}
        />
      );
    case 'components':
      return (
        <ComponentsStep
          onContinue={() => onStepChange('assemble')}
        />
      );
    case 'assemble':
      return (
        <AssembleStep
          onCommitSuccess={() => {
            onCommitToAssemble();
          }}
          onRunSimulation={() => {
            onCommitToAssemble();
          }}
          onStartScratch={promptText === ''}
        />
      );
    default:
      return null;
  }
};
