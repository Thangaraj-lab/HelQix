import { GeometryKind } from '../contracts';

export interface ScenePort {
  id: string;
  kind: string;
  direction: 'in' | 'out' | 'bi';
}

/** A part of the project's own model, as a build or a committed revision reports it. */
export interface ScenePart {
  id: string;
  name: string;
  geometry: GeometryKind;
  /** Position in metres; drawn 1 scene unit per metre. */
  pos?: [number, number, number];
  placeholder?: boolean;
  ports?: ScenePort[];
}

export interface SceneConnection {
  id: string;
  from: { partId: string; portId: string };
  to: { partId: string; portId: string };
  kind: string;
}

export type Look = 'Realistic' | 'Engineering';
export type CameraPreset = 'Perspective' | 'Top' | 'Front' | 'Right' | 'Follow' | 'Orbit' | 'FPV';
export type Series = Record<string, [number, number][]>;
