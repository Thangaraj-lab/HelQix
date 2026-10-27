import { engine } from '../engine';
import { BuildInput, writeBuildInput } from './draft';
import { navigate } from './router';

/**
 * Creates a project and opens its Studio on the Prompt screen, carrying what the user already typed
 * (Hero and Home both have a prompt box). Scratch goes straight to Assemble with an empty canvas.
 */
export async function startProject(input?: BuildInput): Promise<void> {
  const project = await engine.createProject();
  const id = encodeURIComponent(project.id);
  if (input) writeBuildInput(project.id, input);
  if (input?.kind === 'scratch') navigate(`/p/${id}?stage=define&step=assemble`);
  else navigate(`/p/${id}?stage=define&step=intent`);
}
