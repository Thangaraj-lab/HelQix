// What the first build of a project starts from, kept per project until it is committed:
// the user's prompt, or the scratch path. Session storage, so it never leaves this browser tab.
export type BuildInput = { kind: 'prompt'; text: string } | { kind: 'scratch' };

const key = (projectId: string) => `helqix_build_input:${projectId}`;

export function readBuildInput(projectId: string): BuildInput | null {
  try {
    const raw = sessionStorage.getItem(key(projectId));
    if (!raw) return null;
    const v = JSON.parse(raw) as BuildInput;
    if (v.kind === 'scratch') return v;
    if (v.kind === 'prompt' && typeof v.text === 'string' && v.text.trim() !== '') return v;
    return null;
  } catch {
    return null;
  }
}

export function writeBuildInput(projectId: string, input: BuildInput): void {
  try {
    sessionStorage.setItem(key(projectId), JSON.stringify(input));
  } catch {
    // Storage blocked (private mode): the prompt is kept in component state only.
  }
}
