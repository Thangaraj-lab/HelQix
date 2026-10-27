// The one place that chooses the engine. Default is the deterministic mock (every Value carries
// mock: true and screens show MOCK). VITE_ENGINE=real uses the HELQIX API through the dev proxy.
import type { EngineClient } from '../contracts';
import { MockEngineClient } from '../mock/MockEngineClient';
import { RealEngineClient } from './RealEngineClient';

export type EngineMode = 'mock' | 'real';

export const engineMode: EngineMode = import.meta.env.VITE_ENGINE === 'real' ? 'real' : 'mock';

export const engine: EngineClient = engineMode === 'real' ? new RealEngineClient('/api') : MockEngineClient;
