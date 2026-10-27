import React from 'react';
import { useStudio } from '../../stores';
import { Drawer, SectionLabel, Tabs } from '../ui/UiKit';

export interface SettingsSheetProps {
  isOpen: boolean;
  onClose: () => void;
  onToggleTheme: () => void;
  currentTheme: 'dark' | 'light';
}

// Settings sheet: only settings that change something in this build.
export const SettingsSheet: React.FC<SettingsSheetProps> = ({ isOpen, onClose, onToggleTheme, currentTheme }) => {
  const { session } = useStudio();

  return (
    <Drawer isOpen={isOpen} onClose={onClose} title="Settings">
      <div className="space-y-7">
        <section className="space-y-2.5">
          <SectionLabel>Theme</SectionLabel>
          <Tabs
            tabs={[{ id: 'dark', label: 'Dark' }, { id: 'light', label: 'Light' }]}
            activeTab={currentTheme}
            onChange={id => id !== currentTheme && onToggleTheme()}
          />
        </section>

        <section className="space-y-2.5">
          <SectionLabel>Rendering quality</SectionLabel>
          <Tabs
            tabs={(['Auto', 'High', 'Low'] as const).map(t => ({ id: t, label: t }))}
            activeTab={session.qualityTier}
            onChange={id => session.setQualityTier(id as 'Auto' | 'High' | 'Low')}
          />
          <p className="text-xs text-muted">Lower tiers turn off shadows and antialiasing to keep the canvas smooth on modest hardware.</p>
        </section>
      </div>
    </Drawer>
  );
};
