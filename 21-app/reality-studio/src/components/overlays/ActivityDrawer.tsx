import React, { useEffect, useState } from 'react';
import { GitCommitHorizontal } from 'lucide-react';
import { Project } from '../../contracts';
import { engine } from '../../engine';
import { Drawer, EmptyState } from '../ui/UiKit';

export interface ActivityDrawerProps {
  isOpen: boolean;
  onClose: () => void;
  projectId: string;
}

// Activity: what the engine recorded for this project. Today that is its committed revisions.
export const ActivityDrawer: React.FC<ActivityDrawerProps> = ({ isOpen, onClose, projectId }) => {
  const [project, setProject] = useState<Project | null>(null);

  useEffect(() => {
    if (!isOpen) return;
    engine.listProjects().then(list => setProject(list.find(p => p.id === projectId) ?? null)).catch(() => setProject(null));
  }, [isOpen, projectId]);

  const revisions = project?.modelRevisions ?? [];

  return (
    <Drawer isOpen={isOpen} onClose={onClose} title="Activity">
      {revisions.length === 0 ? (
        <EmptyState title="No activity yet" description="Committing a model revision, running a simulation or freezing a state shows up here." />
      ) : (
        <ol className="space-y-4">
          {[...revisions].reverse().map((_, i) => {
            const n = revisions.length - i;
            return (
              <li key={n} className="flex gap-3">
                <GitCommitHorizontal size={16} className="text-accent mt-0.5 shrink-0" />
                <div>
                  <p className="text-[13px] text-strong">Model r{n} committed</p>
                  {n === revisions.length && project && <p className="text-xs text-muted">{new Date(project.updatedAt).toLocaleString()}</p>}
                </div>
              </li>
            );
          })}
        </ol>
      )}
    </Drawer>
  );
};
