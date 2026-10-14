import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { App } from './App';

function installGateway(projects: Array<Record<string, unknown>>) {
  vi.stubGlobal('fetch', vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    if (url.endsWith('/health/live')) return new Response(JSON.stringify({ status: 'live' }));
    if (url.includes('/v1/projects?')) return new Response(JSON.stringify({ data: projects }));
    return new Response(JSON.stringify({}));
  }));
}
afterEach(() => vi.unstubAllGlobals());

describe('Reality Studio', () => {
  it('opens the direct local studio without presenting a login screen', async () => {
    installGateway([]); render(<App />);
    expect(await screen.findByRole('heading', { name: /give every product/i })).toBeTruthy();
    expect(screen.queryByText(/welcome back/i)).toBeNull();
    expect(screen.getByRole('button', { name: /create engineering reality/i })).toBeTruthy();
  });
  it('shows focused studio spaces for a saved project', async () => {
    installGateway([{ id: 'project-1', kind: 'project', project_id: null, version: 1, payload: { name: 'Prototype', domain: 'general engineering', engineering_reality: { components: [] } }, created_at: '', updated_at: '' }]); render(<App />);
    const navigation = await screen.findByRole('navigation', { name: /reality studio spaces/i });
    fireEvent.click(within(navigation).getByRole('button', { name: /simulation/i })); expect(screen.getByRole('heading', { name: /move the actual representation from recorded results/i })).toBeTruthy();
    fireEvent.click(within(navigation).getByRole('button', { name: /connect/i })); expect(screen.getByRole('heading', { name: /one reality across desk, field, edge, and compute/i })).toBeTruthy();
  });
});
