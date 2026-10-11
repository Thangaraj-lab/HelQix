import { useEffect, useRef } from 'react';
import * as THREE from 'three';
import { SceneManager } from '@helqix/visual-runtime';
import { EngineeringObject } from '@helqix/visual-runtime';
import { InteractionController } from '@helqix/visual-runtime';
import type { RealityConnection, RealityInterface } from './api';

export interface RuntimeEntity { id: string; name: string; componentType: string }
export type RuntimeViewPreset = 'perspective' | 'top' | 'front' | 'side' | 'isometric';
export const EMPTY_RUNTIME_ENTITIES: RuntimeEntity[] = [];
export const EMPTY_RUNTIME_INTERFACES: RealityInterface[] = [];
export const EMPTY_RUNTIME_CONNECTIONS: RealityConnection[] = [];

export function useVisualRuntime(
  entities: RuntimeEntity[] = EMPTY_RUNTIME_ENTITIES,
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
        sceneConfig: { backgroundColor: 0x12130f, showGrid, showAxes, gridSizeMeters: 20 },
        enableInterpolation: true,
      });
      manager.setViewPreset(viewPreset, cameraTargetRef.current);
      managerRef.current = manager;
      const componentPositions = new Map<string, { position: THREE.Vector3; halfWidth: number }>();
      entities.forEach((entity, index) => {
        const group = new THREE.Group();
        const type = entity.componentType.toLowerCase();
        const material = new THREE.MeshStandardMaterial({ color: /battery|cell|pack/.test(type) ? 0xb4ce76 : /motor|engine|pump/.test(type) ? 0x95a59c : 0x9ba57d, metalness: 0.34, roughness: 0.42 });
        const accent = new THREE.MeshStandardMaterial({ color: 0x34392d, metalness: 0.5, roughness: 0.35 });
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
        const columns = Math.max(1, Math.ceil(Math.sqrt(entities.length)));
        group.position.set(3.25 + (index % columns - (columns - 1) / 2) * 2.1, 0.5, (Math.floor(index / columns) - (Math.ceil(entities.length / columns) - 1) / 2) * 1.8);
        componentPositions.set(entity.id, { position: group.position.clone(), halfWidth: /battery|cell|pack/.test(type) ? 0.64 : 0.5 });
        manager!.registry.register(new EngineeringObject({ id: entity.id, name: entity.name, subsystem: 'engineering-reality', category: entity.componentType, rootMesh: group }));
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
          const curve = new THREE.CatmullRomCurve3([
            start,
            new THREE.Vector3(start.x + sourceSide * 0.28, arch, start.z),
            new THREE.Vector3(end.x + targetSide * 0.28, arch, end.z),
            end,
          ]);
          const color = /thermal|heat/i.test(sourcePort.domain) ? 0xe8a76e : /fluid|hydraulic/i.test(sourcePort.domain) ? 0x79bad1 : 0xc5e486;
          const wire = new THREE.Mesh(
            new THREE.TubeGeometry(curve, 32, 0.018, 7, false),
            new THREE.MeshStandardMaterial({ color, emissive: color, emissiveIntensity: 0.24, metalness: 0.4, roughness: 0.32 }),
          );
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
      // WebGL may be unavailable on this device; the engineering workspace remains usable.
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
  }, [entities, interfaces, connections, onSelectEntity]);
  useEffect(() => {
    const manager = managerRef.current;
    if (!manager) return;
    manager.scene.setGridVisibility(showGrid);
    manager.scene.setAxesVisibility(showAxes);
    manager.renderSingleFrame();
  }, [showGrid, showAxes]);
  useEffect(() => {
    const manager = managerRef.current;
    if (!manager) return;
    manager.setViewPreset(viewPreset, cameraTargetRef.current);
    interactionRef.current?.syncCameraTarget(cameraTargetRef.current);
    manager.renderSingleFrame();
  }, [viewPreset]);
  useEffect(() => {
    const manager = managerRef.current;
    if (!manager) return;
    if (fitRevision > 0) {
      const target = manager.fitToContent();
      cameraTargetRef.current.copy(target);
      interactionRef.current?.syncCameraTarget(target);
    } else {
      manager.renderSingleFrame();
    }
  }, [fitRevision]);
  useEffect(() => {
    const manager = managerRef.current;
    if (!manager) return;
    for (const entity of manager.registry.getAll()) {
      entity.applyAttributes({ selected: entity.id === selectedEntityId });
    }
    manager.renderSingleFrame();
  }, [selectedEntityId, entities]);
  return canvasRef;
}

/** Maps recorded cumulative travel to the bounded length of the replay track. */
export function recordedDistanceTrackPosition(distanceMeters: number, totalDistanceMeters: number): number {
  if (!Number.isFinite(distanceMeters) || !Number.isFinite(totalDistanceMeters) || totalDistanceMeters <= 0) return 0;
  return THREE.MathUtils.clamp(distanceMeters / totalDistanceMeters, 0, 1);
}

/** Renders a vehicle proxy whose position and wheel motion are driven only by recorded distance. */
export function useVehicleRuntime(distanceMeters: number, totalDistanceMeters: number) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const managerRef = useRef<SceneManager | null>(null);
  const vehicleRef = useRef<EngineeringObject | null>(null);
  const wheelsRef = useRef<THREE.Mesh[]>([]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    let manager: SceneManager | null = null;
    try {
      manager = new SceneManager({ canvas, sceneConfig: { backgroundColor: 0x171812, showGrid: false, showAxes: false, gridSizeMeters: 14 } });
      manager.setViewPreset('isometric');

      const road = new THREE.Group();
      const asphalt = new THREE.Mesh(new THREE.BoxGeometry(14, 0.08, 3.3), new THREE.MeshStandardMaterial({ color: 0x30312a, roughness: 0.94 }));
      asphalt.position.y = -0.08;
      asphalt.receiveShadow = true;
      road.add(asphalt);
      const edgeMaterial = new THREE.MeshStandardMaterial({ color: 0x82905f, emissive: 0x3b492a, emissiveIntensity: 0.22 });
      for (const z of [-1.63, 1.63]) {
        const edge = new THREE.Mesh(new THREE.BoxGeometry(14, 0.035, 0.045), edgeMaterial);
        edge.position.set(0, -0.015, z);
        road.add(edge);
      }
      const dashMaterial = new THREE.MeshStandardMaterial({ color: 0xb8c18e, emissive: 0x4b5136, emissiveIntensity: 0.12 });
      for (let x = -6; x <= 6; x += 1.5) {
        const dash = new THREE.Mesh(new THREE.BoxGeometry(0.72, 0.018, 0.035), dashMaterial);
        dash.position.set(x, -0.025, 0);
        road.add(dash);
      }
      manager.scene.environmentGroup.add(road);

      const car = new THREE.Group();
      const bodyMaterial = new THREE.MeshStandardMaterial({ color: 0xc4e47c, metalness: 0.28, roughness: 0.36 });
      const glassMaterial = new THREE.MeshStandardMaterial({ color: 0x52604b, metalness: 0.44, roughness: 0.2 });
      const tireMaterial = new THREE.MeshStandardMaterial({ color: 0x171815, roughness: 0.82 });
      const hubMaterial = new THREE.MeshStandardMaterial({ color: 0xa8b18b, metalness: 0.78, roughness: 0.28 });
      const chassis = new THREE.Mesh(new THREE.BoxGeometry(2.35, 0.38, 1.08), bodyMaterial);
      chassis.position.y = 0.55;
      chassis.castShadow = true;
      car.add(chassis);
      const cabin = new THREE.Mesh(new THREE.BoxGeometry(1.15, 0.52, 0.88), glassMaterial);
      cabin.position.set(-0.05, 0.96, 0);
      cabin.castShadow = true;
      car.add(cabin);
      const hood = new THREE.Mesh(new THREE.BoxGeometry(0.64, 0.13, 0.92), bodyMaterial);
      hood.position.set(0.79, 0.78, 0);
      car.add(hood);
      const wheels: THREE.Mesh[] = [];
      for (const x of [-0.74, 0.76]) for (const z of [-0.59, 0.59]) {
        const wheel = new THREE.Mesh(new THREE.CylinderGeometry(0.31, 0.31, 0.18, 24), tireMaterial);
        wheel.rotation.x = Math.PI / 2;
        wheel.position.set(x, 0.33, z);
        wheel.castShadow = true;
        const hub = new THREE.Mesh(new THREE.CylinderGeometry(0.13, 0.13, 0.19, 16), hubMaterial);
        hub.rotation.x = Math.PI / 2;
        hub.position.copy(wheel.position);
        car.add(wheel, hub);
        wheels.push(wheel);
      }
      const frontLights = new THREE.Mesh(new THREE.BoxGeometry(0.06, 0.12, 0.54), new THREE.MeshBasicMaterial({ color: 0xe9f3be }));
      frontLights.position.set(1.19, 0.61, 0);
      car.add(frontLights);
      const vehicle = new EngineeringObject({ id: 'recorded-vehicle', name: 'Recorded vehicle proxy', subsystem: 'trajectory-replay', category: 'vehicle', rootMesh: car });
      manager.registry.register(vehicle);
      managerRef.current = manager;
      vehicleRef.current = vehicle;
      wheelsRef.current = wheels;
      manager.start();
    } catch (error) {
      console.warn('HELQIX vehicle trajectory viewport could not initialize.', error);
    }
    return () => {
      manager?.scene.threeScene.traverse((node) => {
        if (!(node instanceof THREE.Mesh)) return;
        node.geometry.dispose();
        for (const material of Array.isArray(node.material) ? node.material : [node.material]) material.dispose();
      });
      manager?.dispose();
      managerRef.current = null;
      vehicleRef.current = null;
      wheelsRef.current = [];
    };
  }, []);

  useEffect(() => {
    const position = recordedDistanceTrackPosition(distanceMeters, totalDistanceMeters);
    if (vehicleRef.current) vehicleRef.current.root3D.position.x = -4.8 + position * 9.6;
    for (const wheel of wheelsRef.current) wheel.rotation.z = -distanceMeters / 0.31;
    managerRef.current?.renderSingleFrame();
  }, [distanceMeters, totalDistanceMeters]);

  return canvasRef;
}
