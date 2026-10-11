import { useEffect, useRef } from 'react';
import * as THREE from 'three';
import { SceneManager } from '@helqix/visual-runtime';
import { EngineeringObject } from '@helqix/visual-runtime';
import { InteractionController } from '@helqix/visual-runtime';
import type { RealityConnection, RealityInterface } from './api';
import type { RuntimeEntityState } from './engineering/visualStateBinding';

export interface RuntimeEntity { id: string; name: string; componentType: string }
export type RuntimeViewPreset = 'perspective' | 'top' | 'front' | 'side' | 'isometric';
export const EMPTY_RUNTIME_ENTITIES: RuntimeEntityState[] = [];
export const EMPTY_RUNTIME_INTERFACES: RealityInterface[] = [];
export const EMPTY_RUNTIME_CONNECTIONS: RealityConnection[] = [];

function statusColor(status: RuntimeEntityState['status']): number {
  switch (status) {
    case 'failed': return 0xef4444;
    case 'paused': return 0xf59e0b;
    case 'running': return 0x38bdf8;
    case 'completed': return 0x22c55e;
    case 'stopped': return 0x94a3b8;
    case 'idle': return 0x94a3b8;
    default: return 0x64748b;
  }
}

function isActive(status: RuntimeEntityState['status']): boolean {
  return status === 'running';
}

export function useVisualRuntime(
  entities: RuntimeEntityState[] = EMPTY_RUNTIME_ENTITIES,
  interfaces: RealityInterface[] = EMPTY_RUNTIME_INTERFACES,
  connections: RealityConnection[] = EMPTY_RUNTIME_CONNECTIONS,
  onSelectEntity?: (id: string | null) => void,
  selectedEntityId?: string | null,
  viewPreset: RuntimeViewPreset = 'isometric',
  showGrid = true,
  showAxes = true,
  fitRevision = 0,
) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const managerRef = useRef<SceneManager | null>(null);
  const interactionRef = useRef<InteractionController | null>(null);
  const cameraTargetRef = useRef(new THREE.Vector3());

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    let manager: SceneManager | null = null;
    let connectionGroup: THREE.Group | null = null;
    try {
      manager = new SceneManager({
        canvas,
        sceneConfig: { backgroundColor: 0x080b10, showGrid, showAxes, gridSizeMeters: 20 },
        enableInterpolation: true,
      });
      manager.setViewPreset(viewPreset, cameraTargetRef.current);
      managerRef.current = manager;
      const componentPositions = new Map<string, { position: THREE.Vector3; halfWidth: number }>();

      entities.forEach((entity, index) => {
        const group = new THREE.Group();
        const type = entity.componentType.toLowerCase();
        const baseColor = /battery|cell|pack/.test(type) ? 0x64748b : /motor|engine|pump/.test(type) ? 0x6b7280 : 0x71717a;
        const material = new THREE.MeshStandardMaterial({
          color: baseColor,
          metalness: 0.42,
          roughness: 0.42,
          emissive: statusColor(entity.status),
          emissiveIntensity: entity.status === 'failed' ? 0.55 : isActive(entity.status) ? 0.28 : entity.selected ? 0.18 : 0.04,
        });
        const accent = new THREE.MeshStandardMaterial({ color: entity.faults.length ? 0xef4444 : 0x334155, metalness: 0.5, roughness: 0.35 });

        if (/motor|engine|pump/.test(type)) {
          const motor = new THREE.Mesh(new THREE.CylinderGeometry(0.42, 0.42, 1.1, 28), material);
          motor.rotation.z = Math.PI / 2;
          motor.castShadow = true;
          group.add(motor);
          const shaft = new THREE.Mesh(new THREE.CylinderGeometry(0.12, 0.12, 0.28, 16), accent);
          shaft.rotation.z = Math.PI / 2;
          shaft.position.x = 0.65;
          group.add(shaft);
        } else {
          const bounds = /battery|cell|pack/.test(type) ? [1.25, 0.48, 0.82] : [0.92, 0.72, 0.76];
          const body = new THREE.Mesh(new THREE.BoxGeometry(...bounds as [number, number, number]), material);
          body.castShadow = true;
          body.receiveShadow = true;
          group.add(body);
          if (/battery|cell|pack/.test(type)) {
            for (let cell = -1; cell <= 1; cell++) {
              const seam = new THREE.Mesh(new THREE.BoxGeometry(0.015, 0.38, 0.74), accent);
              seam.position.x = cell * 0.35;
              group.add(seam);
            }
          }
        }

        if (entity.faults.length) {
          const faultRing = new THREE.Mesh(
            new THREE.TorusGeometry(0.72, 0.025, 8, 48),
            new THREE.MeshBasicMaterial({ color: 0xef4444 }),
          );
          faultRing.rotation.x = Math.PI / 2;
          faultRing.position.y = 0.08;
          group.add(faultRing);
        }
        if (entity.selected) {
          const selectionRing = new THREE.Mesh(
            new THREE.TorusGeometry(0.86, 0.018, 8, 48),
            new THREE.MeshBasicMaterial({ color: 0x22d3ee }),
          );
          selectionRing.rotation.x = Math.PI / 2;
          selectionRing.position.y = 0.03;
          group.add(selectionRing);
        }

        const columns = Math.max(1, Math.ceil(Math.sqrt(entities.length)));
        group.position.set(3.25 + (index % columns - (columns - 1) / 2) * 2.1, 0.5, (Math.floor(index / columns) - (Math.ceil(entities.length / columns) - 1) / 2) * 1.8);
        componentPositions.set(entity.id, { position: group.position.clone(), halfWidth: /battery|cell|pack/.test(type) ? 0.64 : 0.5 });

        const object = new EngineeringObject({
          id: entity.id,
          name: entity.name,
          subsystem: 'engineering-reality',
          category: entity.componentType,
          rootMesh: group,
          initialAttributes: {
            selected: entity.selected,
            health: entity.status === 'failed' ? 'failed' : entity.status === 'paused' ? 'warning' : entity.status === 'running' || entity.status === 'completed' ? 'nominal' : 'unknown',
          },
        });
        manager!.registry.register(object);
      });

      if (connections.length) {
        const interfaceIndex = new Map(interfaces.map((item) => [item.id, item]));
        const network = new THREE.Group();
        connectionGroup = network;
        network.name = 'Declared engineering connections';
        for (const connection of connections) {
          const sourcePort = interfaceIndex.get(connection.sourceInterfaceId);
          const targetPort = interfaceIndex.get(connection.targetInterfaceId);
          const source = sourcePort ? componentPositions.get(sourcePort.componentId) : undefined;
          const target = targetPort ? componentPositions.get(targetPort.componentId) : undefined;
          if (!sourcePort || !targetPort || !source || !target) continue;
          const sourceSide = sourcePort.direction === 'in' ? -1 : 1;
          const targetSide = targetPort.direction === 'out' ? 1 : -1;
          const start = new THREE.Vector3(source.position.x + sourceSide * source.halfWidth, 0.9, source.position.z);
          const end = new THREE.Vector3(target.position.x + targetSide * target.halfWidth, 0.9, target.position.z);
          const arch = 1.25 + Math.min(0.9, Math.abs(end.x - start.x) * 0.12);
          const curve = new THREE.CatmullRomCurve3([start, new THREE.Vector3(start.x + sourceSide * 0.28, arch, start.z), new THREE.Vector3(end.x + targetSide * 0.28, arch, end.z), end]);
          const color = /thermal|heat/i.test(sourcePort.domain) ? 0xe8a76e : /fluid|hydraulic/i.test(sourcePort.domain) ? 0x79bad1 : 0xc5e486;
          const wire = new THREE.Mesh(new THREE.TubeGeometry(curve, 32, 0.018, 7, false), new THREE.MeshStandardMaterial({ color, emissive: color, emissiveIntensity: 0.24, metalness: 0.4, roughness: 0.32 }));
          wire.name = `Declared connection: ${connection.connectionType}`;
          network.add(wire);
          for (const point of [start, end]) {
            const marker = new THREE.Mesh(new THREE.SphereGeometry(0.055, 10, 8), new THREE.MeshBasicMaterial({ color }));
            marker.position.copy(point);
            network.add(marker);
          }
        }
        if (network.children.length) manager.scene.threeScene.add(network);
      }

      const fitTarget = manager.fitToContent();
      cameraTargetRef.current.copy(fitTarget);
      if (onSelectEntity) {
        const interaction = new InteractionController(manager);
        interaction.onSelectObject = (object) => onSelectEntity(object?.id ?? null);
        interaction.syncCameraTarget(fitTarget);
        interactionRef.current = interaction;
      }
      manager.start();
    } catch (error) {
      console.warn('HELQIX visual runtime could not initialize.', error);
    }
    return () => {
      interactionRef.current?.dispose();
      interactionRef.current = null;
      managerRef.current = null;
      if (connectionGroup) {
        connectionGroup.traverse((node) => {
          if (!(node instanceof THREE.Mesh)) return;
          node.geometry.dispose();
          for (const material of Array.isArray(node.material) ? node.material : [node.material]) material.dispose();
        });
        manager?.scene.threeScene.remove(connectionGroup);
      }
      manager?.dispose();
    };
  }, [entities, interfaces, connections, onSelectEntity, selectedEntityId, viewPreset, showGrid, showAxes, fitRevision]);

  useEffect(() => {
    const manager = managerRef.current;
    if (!manager) return;
    manager.setViewPreset(viewPreset, cameraTargetRef.current);
  }, [viewPreset]);

  return canvasRef;
}

export function useVehicleRuntime(distanceMeters: number, totalDistanceMeters: number) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const width = canvas.clientWidth || 640;
    const height = canvas.clientHeight || 260;
    const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setSize(width, height, false);
    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x080b10);
    const camera = new THREE.PerspectiveCamera(36, width / height, 0.1, 100);
    camera.position.set(5.4, 3.2, 6.4);
    camera.lookAt(0, 0.35, 0);
    const light = new THREE.DirectionalLight(0xffffff, 2.2);
    light.position.set(4, 7, 4);
    scene.add(light, new THREE.AmbientLight(0xffffff, 0.65));
    const ground = new THREE.Mesh(new THREE.PlaneGeometry(18, 6), new THREE.MeshStandardMaterial({ color: 0x111827, roughness: 0.92, metalness: 0.05 }));
    ground.rotation.x = -Math.PI / 2;
    scene.add(ground);
    const road = new THREE.Mesh(new THREE.PlaneGeometry(18, 2.2), new THREE.MeshStandardMaterial({ color: 0x1f2937, roughness: 1 }));
    road.rotation.x = -Math.PI / 2;
    road.position.y = 0.006;
    scene.add(road);
    const vehicle = new THREE.Group();
    const body = new THREE.Mesh(new THREE.BoxGeometry(1.8, 0.38, 0.86), new THREE.MeshStandardMaterial({ color: 0x64748b, metalness: 0.55, roughness: 0.28 }));
    body.position.y = 0.35;
    vehicle.add(body);
    const cabin = new THREE.Mesh(new THREE.BoxGeometry(0.9, 0.34, 0.76), new THREE.MeshStandardMaterial({ color: 0x334155, metalness: 0.2, roughness: 0.2 }));
    cabin.position.set(-0.1, 0.67, 0);
    vehicle.add(cabin);
    for (const x of [-0.62, 0.62]) for (const z of [-0.47, 0.47]) {
      const wheel = new THREE.Mesh(new THREE.CylinderGeometry(0.2, 0.2, 0.12, 18), new THREE.MeshStandardMaterial({ color: 0x0f172a, roughness: 0.9 }));
      wheel.rotation.x = Math.PI / 2;
      wheel.position.set(x, 0.2, z);
      vehicle.add(wheel);
    }
    const normalized = totalDistanceMeters > 0 ? Math.max(0, Math.min(1, distanceMeters / totalDistanceMeters)) : 0;
    vehicle.position.x = -6.8 + normalized * 13.6;
    scene.add(vehicle);
    renderer.render(scene, camera);
    return () => { renderer.dispose(); scene.traverse((object) => { if (object instanceof THREE.Mesh) { object.geometry.dispose(); if (Array.isArray(object.material)) object.material.forEach((material) => material.dispose()); else object.material.dispose(); } }); };
  }, [distanceMeters, totalDistanceMeters]);
  return canvasRef;
}
