import React from 'react';
import { Radio } from 'lucide-react';
import { Button, EmptyState, PreviewTag } from '../../components/ui/UiKit';
import { PanelCard, StageGrid } from '../../components/ui/StageGrid';

export interface ObserveStageProps {
  onProposeUpdate?: () => void;
}

// Observe (file 37 section 6.15): a real device beside its digital twin. The edge agent exists
// (08-observation), but no device is connected to this Studio, so there is nothing to show and
// nothing is invented. Observed data never overwrites the model; it proposes updates to it.
export const ObserveStage: React.FC<ObserveStageProps> = () => (
  <StageGrid
    left={
      <PanelCard title="Devices" aside={<PreviewTag />}>
        <p className="text-xs text-muted">No device is connected.</p>
      </PanelCard>
    }
    right={
      <PanelCard title="Residuals and anomalies" aside={<PreviewTag />}>
        <p className="text-xs text-muted">Residuals appear once observed values arrive beside simulated ones.</p>
        <p className="text-[11px] text-muted">Observed data never overwrites the model.</p>
        <Button variant="secondary" className="w-full" disabled>Propose model update</Button>
      </PanelCard>
    }
    top={<div className="max-w-md"><EmptyState title="Nothing observed yet" description="Connect a device through the edge agent to compare observed behaviour with the model. Until then this stage stays empty rather than showing sample data." /></div>}
    bottom={<div className="surface-glass px-4 py-3 text-xs text-muted flex items-center gap-2"><Radio size={14} />Observed against simulated chart: no observed series.</div>}
  />
);
