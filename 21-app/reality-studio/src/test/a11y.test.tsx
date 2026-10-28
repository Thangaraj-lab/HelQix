import React from 'react';
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, waitFor } from '@testing-library/react';
import axe from 'axe-core';
import { StudioStoreProvider } from '../stores';
import { AppShell } from '../app/AppShell';
import { LandingPage } from '../pages/LandingPage';
import { NewProjectPage } from '../pages/NewProjectPage';
import { OverviewPage } from '../pages/home/OverviewPage';

// Structural accessibility rules (names, roles, labels, landmarks, ARIA). Colour contrast needs a real
// layout engine, so it is checked by hand in the browser and is switched off here.
async function violations(container: HTMLElement) {
  const result = await axe.run(container, { rules: { 'color-contrast': { enabled: false }, region: { enabled: false } } });
  return result.violations.map(v => `${v.id}: ${v.help} (${v.nodes.length}) ${v.nodes.map(n => n.target.join(' ')).slice(0, 3).join(' | ')}`);
}

afterEach(cleanup);

describe('accessibility rules on the main pages', () => {
  it('Hero', async () => {
    const { container } = render(<StudioStoreProvider><LandingPage /></StudioStoreProvider>);
    expect(await violations(container)).toEqual([]);
  });
  it('Home', async () => {
    const { container } = render(<StudioStoreProvider><AppShell><OverviewPage /></AppShell></StudioStoreProvider>);
    await waitFor(() => expect(container.textContent).toContain('Projects'));
    expect(await violations(container)).toEqual([]);
  });
  it('New project', async () => {
    const { container } = render(<StudioStoreProvider><NewProjectPage /></StudioStoreProvider>);
    expect(await violations(container)).toEqual([]);
  });
  it('the check can fail: an unlabelled button is reported', async () => {
    const { container } = render(<button><svg /></button>);
    expect((await violations(container)).join()).toContain('button-name');
  });
});
